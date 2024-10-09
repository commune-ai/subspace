use super::*;

impl<T: Config> Pallet<T> {
    pub fn get_valid_subnets(public_key: (Vec<u8>, Vec<u8>)) -> Vec<u16> {
        pallet_subnet_emission::SubnetDecryptionData::<T>::iter()
            .filter(|(netuid, data)| {
                let use_weights = pallet_subspace::UseWeightsEncrytyption::<T>::get(*netuid);
                let key_match = &data.node_public_key == &public_key;
                use_weights && key_match
            })
            .map(|(netuid, _)| netuid)
            .collect()
    }

    pub fn process_subnets(subnets: Vec<u16>) {
        for subnet_id in subnets {
            let params: Vec<(u64, ConsensusParams<T>)> =
                ConsensusParameters::<T>::iter_prefix(subnet_id).collect();

            let max_block = params.iter().map(|(block, _)| *block).max().unwrap_or(0);

            let (last_processed_block, simulation_result) = Self::get_subnet_state(subnet_id);

            dbg!(last_processed_block);

            if last_processed_block >= max_block {
                log::info!(
                    "Skipping subnet {} as it has already been processed",
                    subnet_id
                );
                continue;
            }

            log::info!(
                "Processing subnet {} from block {} to {}",
                subnet_id,
                last_processed_block,
                max_block
            );

            let new_params: Vec<_> =
                params.into_iter().filter(|(block, _)| *block > last_processed_block).collect();

            let (epochs, result) =
                process_consensus_params::<T>(subnet_id, new_params, simulation_result);

            if !epochs.is_empty() {
                if let Err(err) = Self::do_send_weights(subnet_id, epochs, result.delta) {
                    log::error!(
                        "Couldn't send weights to runtime for subnet {}: {}",
                        subnet_id,
                        err
                    );
                } else {
                    // Save, and wait for another round of processing, to try to send weights again
                    Self::save_subnet_state(subnet_id, max_block, result.simulation_result.clone());
                }
            }
            Self::save_subnet_state(subnet_id, max_block, result.simulation_result);
        }
    }

    fn get_subnet_state(subnet_id: u16) -> (u64, ConsensusSimulationResult<T>) {
        let storage_key = format!("subnet_state:{subnet_id}");
        let storage = StorageValueRef::persistent(storage_key.as_bytes());
        storage
            .get::<(u64, ConsensusSimulationResult<T>)>()
            .unwrap_or_else(|_| {
                log::warn!(
                    "Failed to retrieve subnet state for subnet {}. Starting from the beginning.",
                    subnet_id
                );
                Some((0, ConsensusSimulationResult::default()))
            })
            .unwrap_or_else(|| {
                log::warn!(
                    "Subnet state not found for subnet {}. Starting from the beginning.",
                    subnet_id
                );
                (0, ConsensusSimulationResult::default())
            })
    }

    fn save_subnet_state(
        subnet_id: u16,
        last_processed_block: u64,
        simulation_result: ConsensusSimulationResult<T>,
    ) {
        let storage_key = format!("subnet_state:{subnet_id}");
        let storage = StorageValueRef::persistent(storage_key.as_bytes());
        storage.set(&(last_processed_block, simulation_result));
    }
}
