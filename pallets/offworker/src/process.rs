use super::*;

impl<T: Config> Pallet<T> {
    pub fn get_valid_subnets(public_key: (Vec<u8>, Vec<u8>)) -> Vec<u16> {
        SubnetDecryptionData::<T>::iter()
            .filter(|(netuid, data)| {
                let use_weights = pallet_subspace::UseWeightsEncryption::<T>::get(*netuid);
                let key_match = data.node_public_key == public_key;
                use_weights && key_match
            })
            .map(|(netuid, _)| netuid)
            .collect()
    }

    pub fn process_subnets(subnets: Vec<u16>, current_block: u64) {
        subnets.into_iter().for_each(|subnet_id| {
            let params = ConsensusParameters::<T>::iter_prefix(subnet_id).collect::<Vec<_>>();
            let max_block = params.iter().map(|(block, _)| *block).max().unwrap_or(0);

            let copier_margin = CopierMargin::<T>::get(subnet_id);
            let max_encryption_period =
                T::MaxEncryptionTime::get().min(MaxEncryptionPeriod::<T>::get(subnet_id));

            let (last_processed_block, simulation_result) = Self::get_subnet_state(
                subnet_id,
                current_block,
                copier_margin,
                max_encryption_period,
            );

            if last_processed_block >= max_block {
                log::info!(
                    "Skipping subnet {} as it has already been processed",
                    subnet_id
                );
                return;
            }

            log::info!(
                "Processing subnet {} from block {} to {}",
                subnet_id,
                last_processed_block,
                max_block
            );

            let new_params = params
                .into_iter()
                .filter(|(block, _)| *block > last_processed_block)
                .collect::<Vec<_>>();

            let (epochs, result) =
                process_consensus_params::<T>(subnet_id, new_params, simulation_result);

            if epochs.is_empty() {
                Self::save_subnet_state(subnet_id, max_block, result.simulation_result);
            } else if let Err(err) = Self::do_send_weights(subnet_id, epochs, result.delta) {
                log::error!(
                    "Couldn't send weights to runtime for subnet {}: {}",
                    subnet_id,
                    err
                );
            }
        });
    }

    fn get_subnet_state(
        subnet_id: u16,
        current_block: u64,
        copier_margin: I64F64,
        max_encryption_period: u64,
    ) -> (u64, ConsensusSimulationResult<T>) {
        let storage_key = alloc::format!("subnet_state:{subnet_id}");
        let storage = StorageValueRef::persistent(storage_key.as_bytes());
        let default = || {
            (
                0u64,
                ConsensusSimulationResult {
                    cumulative_avg_delegate_divs: IrrationalityDelta::<T>::get(subnet_id),
                    creation_block: current_block,
                    copier_margin,
                    max_encryption_period,
                    ..Default::default()
                },
            )
        };
        storage.get::<(u64, ConsensusSimulationResult<T>)>().map_or_else(
            |_| {
                log::warn!(
                    "Failed to retrieve subnet state for subnet {}. Starting from the beginning.",
                    subnet_id
                );
                default()
            },
            |opt| {
                opt.unwrap_or_else(|| {
                    log::warn!(
                        "Subnet state not found for subnet {}. Starting from the beginning.",
                        subnet_id
                    );
                    default()
                })
            },
        )
    }

    fn save_subnet_state(
        subnet_id: u16,
        last_processed_block: u64,
        simulation_result: ConsensusSimulationResult<T>,
    ) {
        let storage_key = alloc::format!("subnet_state:{subnet_id}");
        let storage = StorageValueRef::persistent(storage_key.as_bytes());
        storage.set(&(last_processed_block, simulation_result));
    }

    pub fn delete_subnet_state(subnet_id: u16) {
        let storage_key = alloc::format!("subnet_state:{subnet_id}");
        let mut storage = StorageValueRef::persistent(storage_key.as_bytes());
        storage.clear();
    }
}
