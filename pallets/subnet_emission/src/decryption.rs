use pallet_subspace::UseWeightsEncrytyption;
use sp_runtime::traits::Get;
use subnet_consensus::util::params::ModuleKey;

use super::*;

impl<T: Config> Pallet<T> {
    #[must_use = "Check if active nodes list is empty before proceeding"]
    fn get_active_nodes(block: u64) -> Option<Vec<DecryptionNodeInfo<T>>> {
        let authority_nodes = DecryptionNodes::<T>::get();
        let keep_alive_interval = T::MaxKeepAlive::get();

        let active_nodes: Vec<_> = authority_nodes
            .into_iter()
            .filter(|node| block.saturating_sub(node.last_keep_alive) <= keep_alive_interval)
            .collect();

        if active_nodes.is_empty() {
            log::warn!(
                "No active encryption nodes found within the last {} blocks",
                keep_alive_interval
            );
            None
        } else {
            Some(active_nodes)
        }
    }

    pub fn distribute_subnets_to_nodes(block: u64) {
        // Filter out nodes that haven't sent a keep-alive within the keep_alive_intervals
        let active_nodes = match Self::get_active_nodes(block) {
            Some(nodes) => nodes,
            None => return,
        };

        for netuid in pallet_subspace::N::<T>::iter_keys() {
            if !UseWeightsEncrytyption::<T>::get(netuid) {
                continue;
            }

            let data = SubnetDecryptionData::<T>::get(netuid);
            if data.is_some_and(|_data| true /* TODO: check if shouldn't rotate */) {
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
                        node_id: node_info.account_id.clone(),
                        node_public_key: node_info.public_key.clone(),
                        block_assigned: block,
                    }),
                );

                DecryptionNodeCursor::<T>::set((current + 1) as u16);
            }
        }
    }

    pub fn do_handle_decrypted_weights(netuid: u16, weights: Vec<BlockWeights>) {
        let info = match SubnetDecryptionData::<T>::get(netuid) {
            Some(info) => info,
            None => {
                log::error!(
                    "subnet {netuid} received decrypted weights to run but has no decryption data."
                );
                return;
            }
        };

        let valid_weights: Vec<(u64, Vec<(u16, Vec<(u16, u16)>)>)> = weights
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

        if let Some((_, weights)) = valid_weights.iter().max_by_key(|&(key, _)| key) {
            for &(uid, ref weights) in weights {
                Weights::<T>::set(netuid, uid, Some(weights.clone()));
            }
        }

        Self::update_decrypted_weights(netuid, valid_weights);
        Self::rotate_decryption_node_if_needed(netuid, info);
    }

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
        let module_key = pallet_subspace::Pallet::<T>::get_key_for_uid(netuid, uid)?;
        let module = params.modules.get(&ModuleKey(module_key))?;

        let hash = sp_io::hashing::sha2_256(&Self::weights_to_blob(weights)[..]).to_vec();
        if hash != module.weight_hash {
            log::error!(
                "incoherent hash received for module {uid} on block {block} in subnet {netuid}"
            );
            return None;
        }

        let key = pallet_subspace::Pallet::<T>::get_key_for_uid(netuid, uid)?;
        if key.encode() != received_key {
            log::error!("Key mismatch for module {uid}");
            return None;
        }

        Self::validate_weights(uid, weights, netuid)
    }

    fn update_decrypted_weights(
        netuid: u16,
        valid_weights: Vec<(u64, Vec<(u16, Vec<(u16, u16)>)>)>,
    ) {
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

        let new_node = active_nodes.get(current % active_nodes.len()).cloned();

        if let Some(new_node) = new_node {
            SubnetDecryptionData::<T>::set(
                netuid,
                Some(SubnetDecryptionInfo {
                    node_id: new_node.account_id,
                    node_public_key: new_node.public_key,
                    block_assigned: block_number,
                }),
            );
            DecryptionNodeCursor::<T>::set((current + 1) as u16);
        }
    }

    /// Adds a new active authority node to the list of active authority nodes.
    /// If the node is already in the list, it will be updated with a new time.
    pub fn do_handle_authority_node_keep_alive(public_key: (Vec<u8>, Vec<u8>)) {
        // Get the current list of active authority nodes
        let mut active_authority_nodes = DecryptionNodes::<T>::get();

        // Get the current block number
        let current_block = pallet_subspace::Pallet::<T>::get_current_block_number();

        // Find the matching account id for the given public key
        let authorities = Authorities::<T>::get();
        let account_id = authorities
            .iter()
            .find(|(_, auth_public_key)| auth_public_key == &public_key)
            .map(|(account, _)| account.clone());

        if let Some(account_id) = account_id {
            // Update or add the authority node
            if let Some(node) =
                active_authority_nodes.iter_mut().find(|node| node.account_id == account_id)
            {
                // Update existing node
                node.last_keep_alive = current_block;
            } else {
                // Add new node
                active_authority_nodes.push(DecryptionNodeInfo {
                    account_id,
                    public_key,
                    last_keep_alive: current_block,
                });
            }

            // Update the storage
            DecryptionNodes::<T>::set(active_authority_nodes);
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

    fn validate_weights(uid: u16, weights: &[(u16, u16)], netuid: u16) -> Option<()> {
        let (uids, values) = weights.iter().copied().collect::<(Vec<u16>, Vec<u16>)>();

        let len = uids.len();
        if len != values.len() {
            return None;
        }

        let mut seen = sp_std::collections::btree_set::BTreeSet::new();
        if uids.iter().any(|item| !seen.insert(item)) {
            return None;
        }

        if uids.contains(&uid) {
            return None;
        }

        let min_allowed_length =
            pallet_subspace::Pallet::<T>::get_min_allowed_weights(netuid) as usize;
        let max_allowed_length = pallet_subspace::MaxAllowedWeights::<T>::get(netuid) as usize; //.min(N::<T>::get(netuid)) as usize;

        if len < min_allowed_length || len > max_allowed_length {
            return None;
        }

        Some(())
    }
}

// TODO: integrate

// #[pallet::storage]
// pub type BannedOffchainWorkers<T: Config> =
//     StorageMap<_, Blake2_128Concat, T::AccountId, u64>;

// #[pallet::storage]
// pub type PendingEmissionIncrease<T: Config> = StorageValue<_, u64, ValueQuery>;

// #[pallet::config]
// pub trait Config: frame_system::Config {
//     // ... other config items ...

//     /// The maximum number of blocks an offchain worker can be inactive before being cancelled
//     type MaxOffchainWorkerInactivity: Get<u64>;

//     /// The duration (in blocks) for which an offchain worker is banned after being cancelled
//     type OffchainWorkerBanDuration: Get<u64>;
// }

// fn cancel_expired_offchain_workers(block_number: u64) {
//        let max_inactivity_blocks = T::MaxOffchainWorkerInactivity::get();
//        let pending_emission_increase = PendingEmissionIncrease::<T>::get();

//        for netuid in pallet_subspace::N::<T>::iter_keys() {
//            if !pallet_subspace::UseWeightsEncrytyption::<T>::get(netuid) {
//                continue;
//            }

//            if let Some(info) = SubnetDecryptionData::<T>::get(netuid) {
//                if block_number.saturating_sub(info.block_assigned) > max_inactivity_blocks {
//                    // Cancel the offchain worker
//                    Self::cancel_offchain_worker(netuid, &info, pending_emission_increase);
//                }
//            }
//        }
//    }

//    fn cancel_offchain_worker(
//        netuid: u16,
//        info: &SubnetDecryptionInfo<T>,
//        pending_emission_increase: u64,
//    ) {
//        // Clear encrypted weights
//        DecryptedWeights::<T>::remove(netuid);

//        // Clear hashes (assuming they're stored in a separate storage item)
//        WeightHashes::<T>::remove(netuid);

//        // Clear ConsensusParameters
//        let block_number = pallet_subspace::Pallet::<T>::get_current_block_number();
//        ConsensusParameters::<T>::remove(netuid, block_number);

//        // Add tokens back to pending emission
//        PendingEmission::<T>::mutate(netuid, |emission| {
//            *emission = emission.saturating_add(pending_emission_increase);
//        });

//        // Ban the offchain worker
//        Self::ban_offchain_worker(&info.node_id);

//        // Reassign the subnet to a different offchain worker
//        Self::reassign_subnet(netuid, block_number);

//        // Emit an event
//        Self::deposit_event(Event::<T>::OffchainWorkerCancelled(netuid, info.node_id.clone()));
//    }

//    fn ban_offchain_worker(node_id: &T::AccountId) {
//        let ban_duration = T::OffchainWorkerBanDuration::get();
//        let current_block = pallet_subspace::Pallet::<T>::get_current_block_number();
//        let ban_expiry = current_block.saturating_add(ban_duration);

//        BannedOffchainWorkers::<T>::insert(node_id, ban_expiry);
//    }

//    fn reassign_subnet(netuid: u16, block_number: u64) {
//        let active_nodes = match Self::get_active_nodes(block_number) {
//            Some(nodes) => nodes,
//            None => return,
//        };

//        let mut current = DecryptionNodeCursor::<T>::get() as usize;
//        if current >= active_nodes.len() {
//            current = 0;
//        }

//        if let Some(node_info) = active_nodes.get(current) {
//            SubnetDecryptionData::<T>::set(
//                netuid,
//                Some(SubnetDecryptionInfo {
//                    node_id: node_info.account_id.clone(),
//                    node_public_key: node_info.public_key.clone(),
//                    block_assigned: block_number,
//                }),
//            );

//            DecryptionNodeCursor::<T>::set((current + 1) as u16);
//        }
//    }
