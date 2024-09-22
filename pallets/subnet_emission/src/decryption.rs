use core::ops::Index;

use pallet_subspace::UseWeightsEncrytyption;
use subnet_consensus::util::params::ModuleKey;

use super::*;

#[derive(Clone, Encode, Decode, TypeInfo)]
pub struct DecryptionNodeInfo {
    public_key: PublicKey,
    last_keep_alive: u64,
}

#[derive(Clone, Encode, Decode, TypeInfo)]
pub struct SubnetDecryptionInfo {
    pub node_id: u16,
    pub node_public_key: PublicKey,
    pub block_assigned: u64,
}

impl<T: Config> Pallet<T> {
    pub fn run_decrypted_weights() {
        for (netuid, mut weights) in DecryptedWeights::<T>::iter() {
            weights.sort_by_key(|(block, _)| *block);

            for (block, weights) in weights {
                if let Err(err) = Self::execute_decrypted_weights(netuid, block, weights) {
                    log::error!("could not execute decrypted weights for block {block} in netuid {netuid}: {err}");
                }
            }
        }

        // ASSUMPTION
        let _ = DecryptedWeights::<T>::clear(u32::MAX, None);
    }

    pub fn distribute_subnets_to_nodes(block: u64) {
        let authority_node_count = DecryptionNodes::<T>::get().len();
        if authority_node_count < 1 {
            log::warn!("no encryption nodes found");
            return;
        }

        for netuid in pallet_subspace::N::<T>::iter_keys() {
            if !UseWeightsEncrytyption::<T>::get(netuid) {
                continue;
            }

            let data = SubnetDecryptionData::<T>::get(netuid);
            if data.is_some_and(|_data| true /* TODO: check if shouldn't rotate */) {
                return;
            }

            let mut current = DecryptionNodeCursor::<T>::get();
            if current as usize >= authority_node_count {
                current = 0;
            }

            let opt = DecryptionNodes::<T>::get().get(current as usize).cloned();
            let Some(node_info) = opt else {
                log::error!("internal error");
                continue;
            };

            SubnetDecryptionData::<T>::set(
                netuid,
                Some(SubnetDecryptionInfo {
                    node_id: current,
                    node_public_key: node_info.public_key,
                    block_assigned: block,
                }),
            );

            DecryptionNodeCursor::<T>::set(current + 1);
        }
    }

    pub fn do_handle_decrypted_weights(
        netuid: u16,
        weights: Vec<(u64, Vec<(u16, Vec<(u16, u16)>)>)>,
    ) {
        let Some(info) = SubnetDecryptionData::<T>::get(netuid) else {
            log::error!(
                "subnet {netuid} received decrypted weights to run but has no decryption data."
            );
            return;
        };

        for (block, weights) in &weights {
            for (uid, weights) in weights {
                let Some(params) = ConsensusParameters::<T>::get(netuid, block) else {
                    log::error!("could not find required consensus parameters for block {block} in subnet {netuid}");
                    continue;
                };

                let Some(module_key) = pallet_subspace::Pallet::<T>::get_key_for_uid(netuid, *uid)
                else {
                    log::error!("could not find module {uid} key in subnet {netuid}");
                    continue;
                };

                let Some(module) = params.modules.get(&ModuleKey(module_key)) else {
                    log::error!("could not find required module {uid} parameter for block {block} in subnet {netuid}");
                    continue;
                };

                let Some(hash) = ow_extensions::offworker::hash_weight(weights.clone()) else {
                    log::error!("could hash required module {uid}'s weights for block {block} in subnet {netuid}");
                    continue;
                };

                if hash != module.weight_unencrypted_hash {
                    log::error!("incoherent weight hashes for module {uid} on block {block} in subnet {netuid}");
                    // WHAT TO DO HERE?
                    continue;
                }
            }
        }

        match DecryptedWeights::<T>::get(netuid) {
            Some(mut cached) => {
                cached.extend(weights);
                DecryptedWeights::<T>::set(netuid, Some(cached));
            }
            None => DecryptedWeights::<T>::set(netuid, Some(weights)),
        }

        let block_number = pallet_subspace::Pallet::<T>::get_current_block_number();
        if block_number - info.block_assigned < 100 {
            // TODO check this number
            return;
        }

        let mut current = DecryptionNodeCursor::<T>::get();
        if current as usize >= DecryptionNodes::<T>::get().len() {
            current = 0;
        }

        let Some(new_node) = DecryptionNodes::<T>::get().get(current as usize).cloned() else {
            // shouldn't happen, maybe log
            return;
        };

        SubnetDecryptionData::<T>::set(
            netuid,
            Some(SubnetDecryptionInfo {
                node_id: current,
                node_public_key: new_node.public_key,
                block_assigned: block_number,
            }),
        );
    }

    /// Executes consensus for the oldest set of parameters in the given netuid.
    /// Currently only supports Yuma consensus.
    ///
    /// This function:
    /// 1. Retrieves the oldest ConsensusParameters for the specified netuid
    /// 2. Executes the Yuma consensus using these parameters
    /// 3. Applies the consensus results
    /// 4. Deletes the processed ConsensusParameters from storage
    ///
    /// Parameters:
    /// - netuid: The network ID
    /// - weights: The decrypted weights to be used in the consensus
    ///
    /// Returns:
    /// - Ok(()) if successful
    /// - Err with a descriptive message if an error occurs
    pub fn execute_decrypted_weights(
        netuid: u16,
        block: u64,
        weights: Vec<(u16, Vec<(u16, u16)>)>,
    ) -> Result<(), &'static str> {
        // Check if the given netuid is running Yuma consensus
        let consensus_type = SubnetConsensusType::<T>::get(netuid).ok_or("Invalid network ID")?;

        if consensus_type != pallet_subnet_emission_api::SubnetConsensus::Yuma {
            return Err("Unsupported consensus type");
        }

        // Retrieve the oldest ConsensusParameters
        let (oldest_epoch, oldest_params) = ConsensusParameters::<T>::iter_prefix(netuid)
            .min_by_key(|(k, _)| *k)
            .ok_or("No consensus parameters found")?;

        // Initialize Yuma epoch with the oldest parameters
        let mut yuma_epoch =
            crate::subnet_consensus::yuma::YumaEpoch::<T>::new(netuid, oldest_params);

        // // Execute Yuma consensus
        // let emission_to_drain =
        //     Self::get_emission_to_drain(netuid).map_err(|_| "Failed to get emission to drain")?;
        // yuma_epoch.run(weights).map_err(|_| "Failed to run Yuma consensus")?;

        // // Apply consensus results
        // yuma_epoch
        //     .params
        //     .apply(netuid)
        //     .map_err(|_| "Failed to apply consensus results")?;

        // // Delete the processed ConsensusParameters from storage
        // ConsensusParameters::<T>::remove(netuid, oldest_epoch);

        Ok(())
    }

    pub fn do_handle_authority_node_keep_alive(public_key: (Vec<u8>, Vec<u8>)) {
        if !Self::is_node_authorized(&public_key) {
            // TODO what to do here?
            return;
        }

        let mut authority_nodes = DecryptionNodes::<T>::get();

        let index = authority_nodes
            .iter()
            .position(|info| info.public_key == public_key)
            .unwrap_or(authority_nodes.len());
        authority_nodes.insert(
            index,
            DecryptionNodeInfo {
                public_key,
                last_keep_alive: pallet_subspace::Pallet::<T>::get_current_block_number(),
            },
        );

        DecryptionNodes::<T>::set(authority_nodes);
    }

    fn is_node_authorized(public_key: &(Vec<u8>, Vec<u8>)) -> bool {
        AuthorizedPublicKeys::<T>::get().iter().any(|node| node == public_key)
    }
}
