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

    // TODO: step 4. verify the zk proofs, if only one zk proof is invalid, you will ban the
    // offchain worker
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
                let valid_block_weights = block_weights
                    .into_iter()
                    .filter_map(|(uid, weights, received_key)| {
                        Self::validate_weight_entry(netuid, block, uid, &weights, &received_key)
                            .map(|_| (uid, weights))
                    })
                    .collect::<Vec<_>>();

                if valid_block_weights.is_empty() {
                    None
                } else {
                    Some((block, valid_block_weights))
                }
            })
            .collect();

        log::info!(
            "Received {} valid decrypted weights for subnet {}",
            valid_weights.len(),
            netuid
        );

        // It is fine to inset the weights directly here. as the subnet is obviously using weights
        // encryption, we do not need to bother with additional checks. these weights are primarly
        // just stored for conseistency between subnets that use the weight encryption and those
        // that do not
        if let Some((_, weights)) = valid_weights.iter().max_by_key(|&(key, _)| key) {
            for &(uid, ref weights) in weights {
                Weights::<T>::set(netuid, uid, Some(weights.clone()));
            }
        }
        Self::update_decrypted_weights(netuid, valid_weights);
        Self::rotate_decryption_node_if_needed(netuid, info);
    }

    #[inline]
    fn validate_weight_entry(
        netuid: u16,
        block: u64,
        uid: u16,
        weights: &[(u16, u16)],
        received_key: &[u8],
    ) -> Option<()> {
        if weights.is_empty() {
            return Some(());
        }

        let params = ConsensusParameters::<T>::get(netuid, block)?;
        let module_key = params.get_module_key_by_uid(uid)?;
        let module = params.modules.get(&ModuleKey(module_key.clone()))?;

        let hash = sp_io::hashing::sha2_256(&Self::weights_to_blob(weights)[..]).to_vec();
        if hash != module.weight_hash {
            log::error!(
                "incoherent hash received for module {uid} on block {block} in subnet {netuid}"
            );
            return None;
        }

        let key = match &module.delegated_to {
            Some((key, _fee)) => key,
            None => &module_key,
        };
        if key.encode() != received_key {
            log::warn!("Key mismatch for module {uid}");
            return None;
        }

        Self::validate_weights(uid, weights, netuid)
    }

    #[inline]
    fn validate_weights(uid: u16, weights: &[(u16, u16)], netuid: u16) -> Option<()> {
        let (uids, values): (Vec<_>, Vec<_>) = weights.iter().copied().unzip();

        // Early return if lengths don't match
        if uids.len() != values.len() {
            return None;
        }

        // Check for duplicates and self-referencing
        let unique_uids = uids.iter().collect::<sp_std::collections::btree_set::BTreeSet<_>>();
        if unique_uids.len() != uids.len() || uids.contains(&uid) {
            return None;
        }

        // Check length constraints
        let min_allowed_length =
            pallet_subspace::Pallet::<T>::get_min_allowed_weights(netuid) as usize;
        let max_allowed_length = pallet_subspace::MaxAllowedWeights::<T>::get(netuid) as usize;

        if !(min_allowed_length..=max_allowed_length).contains(&uids.len()) {
            return None;
        }
        Some(())
    }

    fn update_decrypted_weights(netuid: u16, valid_weights: Vec<KeylessBlockWeights>) {
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

    fn weights_to_blob(weights: &[(u16, u16)]) -> Vec<u8> {
        let mut encoded = Vec::new();
        encoded.extend((weights.len() as u32).to_be_bytes());
        encoded.extend(weights.iter().flat_map(|(uid, weight)| {
            sp_std::vec![uid.to_be_bytes(), weight.to_be_bytes()].into_iter().flatten()
        }));

        encoded
    }

    /// Returns a tuple of subnet UIDs (with_encryption, without_encryption) where:
    /// - First vector contains subnets that use weight encryption and have matching keys (if acc_id
    ///   is Some)
    /// - Second vector contains subnets that don't use encryption but still have matching keys (if
    ///   acc_id is Some)
    /// Both require the subnet to have existing encrypted weights.
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
                        > T::MaxEncryptionDuration::get().saturating_add(100)
            })
            .for_each(|(subnet_id, info)| Self::cancel_offchain_worker(subnet_id, &info));
    }

    /// Cancels an offchain worker for a specific subnet and handles the associated cleanup.
    ///
    /// This function performs the following actions:
    /// 1. Clears all encrypted weights for the subnet.
    /// 2. Clears all decrypted weight hashes for the subnet.
    /// 3. Sums up the total token emission from all consensus parameters for the subnet.
    /// 4. Clears all consensus parameters for the subnet.
    /// 5. Adds the total token emission back to the pending emission for the subnet.
    /// 6. Bans the offchain worker associated with the subnet.
    /// 7. Removes the subnet's decryption data.
    /// 8. Reassigns the subnet to a different offchain worker.
    /// 9. Emits a `DecryptionNodeCanceled` event.
    fn cancel_offchain_worker(subnet_id: u16, info: &SubnetDecryptionInfo<T>) {
        // Decrypted Weights (there should not be any, but make sure)
        DecryptedWeights::<T>::remove(subnet_id);

        // Clear hashes & encrypted weights
        let _ = WeightEncryptionData::<T>::clear_prefix(subnet_id, u32::MAX, None);

        // Clear ConsensusParameters and sum up token emission
        let current_block = pallet_subspace::Pallet::<T>::get_current_block_number();
        let mut total_emission = 0u64;

        // We need to iterate through all ConsensusParameters to sum up the token emission before
        // clearing
        for (_, params) in ConsensusParameters::<T>::iter_prefix(subnet_id) {
            total_emission = total_emission.saturating_add(params.token_emission);
        }

        // Now clear the ConsensusParameters
        let _ = ConsensusParameters::<T>::clear_prefix(subnet_id, u32::MAX, None);

        // Add tokens back to pending emission
        PendingEmission::<T>::mutate(subnet_id, |emission| {
            *emission = emission.saturating_add(total_emission);
        });

        // Ban the offchain worker
        Self::ban_offchain_worker(&info.node_id);

        // Remove the subnet's decryption data
        SubnetDecryptionData::<T>::remove(subnet_id);

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
