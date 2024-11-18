use core::u32;

use crate::{
    distribute_emission::update_pending_emission,
    subnet_consensus::{util::params::ConsensusParams, yuma::YumaEpoch},
};
use pallet_subspace::UseWeightsEncryption;
use sp_runtime::traits::Get;
use subnet_consensus::util::params::ModuleKey;
use types::KeylessBlockWeights;

// TODO: all logic of canceling has to completelly match what is in the offchain worker code !!
// We can not cancel if offchain worker is not explicityl "aware it should have send weights"
// We also have to make sure that the block of assigning is handeled correctly when offchain worker
// sends the weights We need to completelly clear the subnet decryption data once weights are
// received

use super::*;

impl<T: Config> Pallet<T> {
    #[must_use = "Check if active nodes list is empty before proceeding"]
    pub fn get_active_nodes(block: u64) -> Option<Vec<SubnetDecryptionInfo<T>>> {
        let authority_nodes = DecryptionNodes::<T>::get();
        let keep_alive_interval =
            T::PingInterval::get().saturating_mul(T::MissedPingsForInactivity::get() as u64);
        let active_nodes: Vec<_> = authority_nodes
            .into_iter()
            .filter(|node| {
                // Check if the node is within the keep-alive interval
                let is_alive = block.saturating_sub(node.last_keep_alive) <= keep_alive_interval;

                // Check if the node is not in the banned list
                let is_not_banned = !BannedDecryptionNodes::<T>::contains_key(&node.node_id);

                // Node is considered active if it's both alive and not banned
                is_alive && is_not_banned
            })
            .collect();

        if active_nodes.is_empty() {
            log::warn!(
                "No active and unbanned encryption nodes found within the last {} blocks",
                keep_alive_interval
            );
            None
        } else {
            Some(active_nodes)
        }
    }

    pub fn distribute_subnets_to_nodes(block: u64) {
        // Filter out nodes that haven't sent a ping within required interval
        let active_nodes = match Self::get_active_nodes(block) {
            Some(nodes) => nodes,
            None => return,
        };

        for netuid in pallet_subspace::N::<T>::iter_keys() {
            if !UseWeightsEncryption::<T>::get(netuid) {
                continue;
            }

            let data = SubnetDecryptionData::<T>::get(netuid);
            if data.is_some_and(|_data| true) {
                return;
            }

            let mut current = DecryptionNodeCursor::<T>::get() as usize;
            if current >= active_nodes.len() {
                current = 0;
            }

            if let Some(node_info) = active_nodes.get(current) {
                SubnetDecryptionData::<T>::set(
                    netuid,
                    Some(SubnetDecryptionInfo {
                        node_id: node_info.node_id.clone(),
                        node_public_key: node_info.node_public_key.clone(),
                        block_assigned: block,
                        last_keep_alive: block,
                    }),
                );

                DecryptionNodeCursor::<T>::set((current.saturating_add(1)) as u16);
            }
        }
    }

    /// 1. TODO: step 4. verify the zk proofs, if only one zk proof is invalid, you will ban the
    /// offchain worker
    ///
    /// 2. TODO: add a test where some of the decrypted weights will be empty, and expect everything
    ///    to be handeled correctly
    pub fn handle_decrypted_weights(netuid: u16, weights: Vec<BlockWeights>) {
        log::info!("Received decrypted weights for subnet {netuid}");
        let info = match SubnetDecryptionData::<T>::get(netuid) {
            Some(info) => info,
            None => {
                log::error!(
                    "subnet {netuid} received decrypted weights to run but has no decryption data."
                );
                return;
            }
        };

        let valid_weights: Vec<KeylessBlockWeights> = weights
            .into_iter()
            .filter_map(|(block, block_weights)| {
                if let Some(params) = ConsensusParameters::<T>::get(netuid, block) {
                    let valid_block_weights = block_weights
                        .into_iter()
                        .filter_map(|(uid, weights, received_key)| {
                            Self::validate_weight_entry(
                                netuid,
                                &params,
                                block,
                                uid,
                                &weights,
                                &received_key,
                            )
                            .map(|_| (uid, weights))
                        })
                        .collect::<Vec<_>>();

                    // We allow empty vectors
                    Some((block, valid_block_weights))
                } else {
                    None
                }
            })
            .collect();

        log::info!(
            "Received {} valid decrypted weights for subnet {}",
            valid_weights.len(),
            netuid
        );

        Self::update_decrypted_weights(netuid, valid_weights);
        match Self::process_decrypted_weights(netuid) {
            Ok(()) => {
                log::info!("decrypted weights have been processed for {netuid}")
            }
            Err(err) => {
                log::error!("error: {err:?} in processing decrypted weights for subnet {netuid} ")
            }
        }
        Self::rotate_decryption_node_if_needed(netuid, info);
    }

    /// TODO: For this fn to work properely make sure that the decrypted weights extend their first
    /// weights by the `Weights` and then "continue extending themselves"
    fn process_decrypted_weights(netuid: u16) -> Result<(), &'static str> {
        let weights = DecryptedWeights::<T>::get(netuid);
        if let Some(weights) = weights {
            // Sorts from oldest weights to newest
            let mut sorted_weights = weights;
            sorted_weights.sort_by_key(|(block, _)| *block);

            let mut accumulated_emission: u64 = 0;

            for (block, weights) in sorted_weights {
                let consensus_type =
                    SubnetConsensusType::<T>::get(netuid).ok_or("Invalid network ID")?;
                if consensus_type != pallet_subnet_emission_api::SubnetConsensus::Yuma {
                    return Err("Unsupported consensus type");
                }

                // Extend the weight storage of the subnet with the new weights
                for (uid, weights) in weights.clone() {
                    Weights::<T>::set(netuid, uid, Some(weights));
                }

                let mut params = ConsensusParameters::<T>::get(netuid, block).ok_or_else(|| {
                    log::error!("no params found for netuid {netuid} block {block}");
                    "Missing consensus parameters"
                })?;

                params.token_emission = params.token_emission.saturating_add(accumulated_emission);
                let new_emission = params.token_emission;

                match YumaEpoch::new(netuid, params.clone()).run(weights) {
                    Ok(output) => {
                        accumulated_emission = 0;
                        output.apply()
                    }
                    Err(err) => {
                        log::error!(
                            "could not run yuma consensus for {netuid} block {block}: {err:?}"
                        );
                        accumulated_emission = new_emission;
                    }
                }
            }

            // If the last consensus that we were processing had an error we directly update the
            // pending emision storage of the subnet
            if accumulated_emission > 0 {
                update_pending_emission::<T>(netuid, &accumulated_emission);
            }

            // --- Clear All Of the Relevant Storages ---
            // We avoid subnet decryption data, as node rotation has to handle that

            Self::cleanup_subnet_wc_state(netuid, false, false); // don't increase pending emisison, don't deletete node assignement

            Ok(())
        } else {
            log::info!("No decrypted weights");
            Ok(())
        }
    }

    fn weights_to_blob(weights: &[(u16, u16)]) -> Vec<u8> {
        let mut encoded = Vec::new();
        encoded.extend((weights.len() as u32).to_be_bytes());
        encoded.extend(weights.iter().flat_map(|(uid, weight)| {
            sp_std::vec![uid.to_be_bytes(), weight.to_be_bytes()].into_iter().flatten()
        }));

        encoded
    }

    #[inline]
    fn validate_weight_entry(
        netuid: u16,
        params: &ConsensusParams<T>,
        block: u64,
        uid: u16,
        weights: &[(u16, u16)],
        received_key: &[u8],
    ) -> Option<()> {
        if weights.is_empty() {
            return Some(());
        }

        let module_key = params.get_module_key_by_uid(uid)?;
        let module = params.modules.get(&ModuleKey(module_key.clone()))?;

        // --- Veify the hash ---

        let hash = sp_io::hashing::sha2_256(&Self::weights_to_blob(weights)[..]).to_vec();
        if hash != module.weight_hash {
            log::error!(
                "incoherent hash received for module {uid} on block {block} in subnet {netuid}"
            );
            return None;
        }

        // --- Veify the validator key ---

        // It is not possible to somehow "time the delegation" and then successfully weight copy,
        // because all of the consensus parameters are essentially "snapshotted" at one time, so the
        // person was either delegating to someone, and uses their weights, paying the fee, or they
        // were not delegating, and they use their own weights.
        let key = match &module.delegated_to {
            Some((key, _fee)) => key,
            None => &module_key,
        };
        // In the scenario where someone would just try to copy the encrypted weights of other
        // validator, his weights would be discarded, because the key would not match the key of the
        // module
        if key.encode() != received_key {
            log::warn!("Key mismatch for module {uid}");
            return None;
        }

        let (uids, values): (Vec<_>, Vec<_>) = weights.iter().copied().unzip();

        Self::validate_input(uid, &uids, &values, netuid).ok()
    }

    /// TODO: we should be able to get rid of the `DecryptedWeights` and return the result directly
    /// for processing. if we extend with the `Weights` storage, everything should work as expected
    ///
    /// TODO: the first decrypted weights here need to be extended by the
    /// `Weights` storage. This should not be a problem even for activity cutoff, as the last
    /// updates are "cached" in the consensus parameters, therefore the consensus is able to
    /// determine which of those standalone extended weights would still be valid
    fn update_decrypted_weights(netuid: u16, valid_weights: Vec<KeylessBlockWeights>) {
        // Extends cached weights with new weights, including blocks with empty weight vectors
        DecryptedWeights::<T>::mutate(netuid, |cached| match cached {
            Some(cached) => cached.extend(valid_weights),
            None => *cached = Some(valid_weights),
        });
    }

    fn rotate_decryption_node_if_needed(netuid: u16, info: SubnetDecryptionInfo<T>) {
        let block_number = pallet_subspace::Pallet::<T>::get_current_block_number();
        if block_number.saturating_sub(info.block_assigned)
            < T::DecryptionNodeRotationInterval::get()
        {
            return;
        }

        let current = DecryptionNodeCursor::<T>::get() as usize;
        let active_nodes = match Self::get_active_nodes(block_number) {
            Some(nodes) => nodes,
            None => return,
        };

        let new_node =
            active_nodes.get(current.checked_rem(active_nodes.len()).unwrap_or(0)).cloned();

        if let Some(new_node) = new_node {
            SubnetDecryptionData::<T>::set(
                netuid,
                Some(SubnetDecryptionInfo {
                    node_id: new_node.node_id,
                    node_public_key: new_node.node_public_key,
                    block_assigned: block_number,
                    last_keep_alive: block_number,
                }),
            );
            DecryptionNodeCursor::<T>::set((current.saturating_add(1)) as u16);
        }
    }

    /// Adds a new active authority node to the list of active authority nodes.
    /// If the node is already in the list, it will be updated with a new time.
    pub fn handle_authority_node_ping(account_id: T::AccountId) {
        log::info!(
            "Starting authority node ping handling for account: {:?}",
            account_id
        );

        let current_block = pallet_subspace::Pallet::<T>::get_current_block_number();

        // Find matching public key using functional approach
        let public_key = Authorities::<T>::get()
            .into_iter()
            .find_map(|(auth_id, pub_key)| (auth_id == account_id).then_some(pub_key));

        let Some(public_key) = public_key else {
            log::info!("No matching public key found for account {:?}", account_id);
            return;
        };

        // Update active nodes list
        DecryptionNodes::<T>::mutate(|nodes| {
            match nodes.iter_mut().find(|node| node.node_id == account_id) {
                Some(node) => {
                    log::info!(
                        "Updating existing node's last_keep_alive from {} to {}",
                        node.last_keep_alive,
                        current_block
                    );
                    node.last_keep_alive = current_block;
                }
                None => {
                    log::info!("Adding new authority node to active nodes list");
                    nodes.push(SubnetDecryptionInfo {
                        node_id: account_id.clone(),
                        node_public_key: public_key,
                        last_keep_alive: current_block,
                        block_assigned: current_block,
                    });
                }
            }
        });

        // Update subnet decryption data directly from storage
        SubnetDecryptionData::<T>::iter()
            .filter(|(_, info)| info.node_id == account_id)
            .for_each(|(netuid, mut info)| {
                log::info!(
                    "Updating last_keep_alive for subnet {} decryption node",
                    netuid
                );
                info.last_keep_alive = current_block;
                SubnetDecryptionData::<T>::insert(netuid, info);
            });

        log::info!("Authority node ping handling completed successfully");
    }

    /// Returns a tuple of subnet UIDs (with_encryption, without_encryption) where:
    /// - First vector contains subnets that use weight encryption and have matching keys (if acc_id
    ///   is Some)
    /// - Second vector contains subnets that don't use encryption but still have matching keys (if
    ///   acc_id is Some). Both require the subnet to have existing encrypted weights.
    pub fn get_valid_subnets(acc_id: Option<&T::AccountId>) -> (Vec<u16>, Vec<u16>) {
        let (with_encryption, without_encryption): (Vec<_>, Vec<_>) =
            SubnetDecryptionData::<T>::iter()
                .filter(|(netuid, data)| {
                    let key_match = acc_id.map_or(true, |id| &data.node_id == id);
                    let has_encrypted_weights = WeightEncryptionData::<T>::iter_prefix(*netuid)
                        .any(|(_, value)| !value.encrypted.is_empty());

                    key_match && has_encrypted_weights
                })
                .map(|(netuid, _)| netuid)
                .partition(|netuid| pallet_subspace::UseWeightsEncryption::<T>::get(*netuid));

        (with_encryption, without_encryption)
    }

    pub fn cancel_expired_offchain_workers(block_number: u64) {
        // TODO: make sure that this is the boundry even for the subnet owner
        // / the offchain worker has max lenght this exact interval present in the
        // `is_copying_irrational` otherwise we could run into race conditions
        let max_inactivity_blocks =
            T::PingInterval::get().saturating_mul(T::MaxFailedPings::get() as u64);

        // Get only subnets that use encryption and have encrypted weights
        let (with_encryption, _) = Self::get_valid_subnets(None);

        with_encryption
            .into_iter()
            .filter_map(|subnet_id| {
                SubnetDecryptionData::<T>::get(subnet_id).map(|info| (subnet_id, info))
            })
            .filter(|(_, info)| {
                block_number.saturating_sub(info.last_keep_alive) > max_inactivity_blocks
                    || block_number.saturating_sub(info.block_assigned)
                        > T::MaxEncryptionDuration::get().saturating_add(100) // TODO: the buffer
                                                                              // has to be a
                                                                              // constant defined in
                                                                              // the runtime
            })
            .for_each(|(subnet_id, info)| Self::cancel_offchain_worker(subnet_id, &info));
    }

    /// Cleans up weight copying state of a subnet by removing weights and parameters.
    /// If increase_pending_emission is true, returns tokens to pending emissions.
    /// Returns the total emission amount that was processed.
    fn cleanup_subnet_wc_state(
        subnet_id: u16,
        increase_pending_emission: bool,
        clear_node_assing: bool,
    ) -> u64 {
        // --- Cleanup The Core ---

        // Clear decrypted weights
        DecryptedWeights::<T>::remove(subnet_id);
        // Clear hashes & encrypted weights
        let _ = WeightEncryptionData::<T>::clear_prefix(subnet_id, u32::MAX, None);
        // Sum up and clear ConsensusParameters
        let total_emission = ConsensusParameters::<T>::iter_prefix(subnet_id)
            .fold(0u64, |acc, (_, params)| {
                acc.saturating_add(params.token_emission)
            });
        // Clear ConsensusParameters
        let _ = ConsensusParameters::<T>::clear_prefix(subnet_id, u32::MAX, None);

        // --- Cleanup The Conditionals ---

        // Add tokens back to pending emission if requested
        if increase_pending_emission {
            update_pending_emission::<T>(subnet_id, &total_emission)
        }
        if clear_node_assing {
            // Remove the subnet's decryption data
            SubnetDecryptionData::<T>::remove(subnet_id);
        }

        total_emission
    }

    /// Cleans up all hanging subnets (subnets that have turned their weight encryption off)
    /// by removing their weight copying state.
    /// Recycles / Burns the pending emission, this aims to disincentivize subnet owners from
    /// switching the parameter, unless absolutelly neccessary Returns the number of subnets
    /// that were cleaned up.
    pub fn clear_hanging_subnet_state() -> usize {
        let (_, hanging_subnets) = Self::get_valid_subnets(None);

        for netuid in &hanging_subnets {
            let _ = Self::cleanup_subnet_wc_state(*netuid, false, true);
        }

        hanging_subnets.len()
    }

    /// Cancels an offchain worker for a subnet by cleaning up its weight copying state,
    /// banning the worker, and reassigning the subnet to a different worker.
    fn cancel_offchain_worker(subnet_id: u16, info: &SubnetDecryptionInfo<T>) {
        let _ = Self::cleanup_subnet_wc_state(subnet_id, true, true);

        // Additional operations specific to canceling offchain worker
        let current_block = pallet_subspace::Pallet::<T>::get_current_block_number();

        // Ban the offchain worker
        Self::ban_offchain_worker(&info.node_id);

        // Reassign the subnet to a different offchain worker
        Self::distribute_subnets_to_nodes(current_block);

        // Emit an event
        Self::deposit_event(Event::<T>::DecryptionNodeCanceled {
            subnet_id,
            node_id: info.node_id.clone(),
        });
    }

    fn ban_offchain_worker(node_id: &T::AccountId) {
        let ban_duration = T::OffchainWorkerBanDuration::get();
        let current_block = pallet_subspace::Pallet::<T>::get_current_block_number();
        let ban_expiry = current_block.saturating_add(ban_duration);

        BannedDecryptionNodes::<T>::insert(node_id, ban_expiry);
    }
}
