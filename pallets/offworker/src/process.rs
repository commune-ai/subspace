use super::*;

impl<T: Config> Pallet<T> {
    pub fn get_valid_subnets(public_key: (Vec<u8>, Vec<u8>)) -> Vec<u16> {
        pallet_subnet_emission::SubnetDecryptionData::<T>::iter()
            .filter(|(netuid, data)| {
                pallet_subspace::UseWeightsEncrytyption::<T>::get(*netuid)
                    && data.node_public_key == public_key
            })
            .map(|(netuid, _)| netuid)
            .collect()
    }

    pub fn process_subnets(subnets: Vec<u16>) {
        subnets
            .into_iter()
            .filter_map(|subnet_id| {
                let params: Vec<(u64, ConsensusParams<T>)> =
                    ConsensusParameters::<T>::iter_prefix(subnet_id).collect();

                let max_block = params.iter().map(|(block, _)| *block).max().unwrap_or(0);

                if !Self::should_process_subnet(subnet_id, max_block) {
                    return None;
                }

                let (epochs, result) = process_consensus_params::<T>(subnet_id, params);

                if epochs.is_empty() {
                    return None;
                }

                Some((subnet_id, epochs, result.delta))
            })
            .for_each(|(subnet_id, epochs, delta)| {
                if let Err(err) = Self::do_send_weights(subnet_id, epochs, delta) {
                    log::error!(
                        "couldn't send weights to runtime for subnet {}: {}",
                        subnet_id,
                        err
                    );
                }
            });
    }

    fn should_process_subnet(subnet_id: u16, max_block: u64) -> bool {
        let storage_key = format!("last_processed_block:{subnet_id}");
        let storage = StorageValueRef::persistent(storage_key.as_bytes());
        let last_processed_block = storage.get::<u64>().ok().flatten().unwrap_or(0);

        if last_processed_block < max_block {
            storage.set(&max_block);
            true
        } else {
            false
        }
    }
}
