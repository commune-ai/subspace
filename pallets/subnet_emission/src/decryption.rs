use core::ops::Index;

use pallet_subspace::UseWeightsEncrytyption;
use sp_runtime::traits::Get;
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

        let mut valid_weights: Vec<(u64, Vec<(u16, Vec<(u16, u16)>)>)> = Vec::new();

        for (block, weights) in weights.into_iter() {
            let mut valid_block_weights: Vec<(u16, Vec<(u16, u16)>)> = Vec::new();

            for (uid, weights) in weights {
                let Some(params) = ConsensusParameters::<T>::get(netuid, block) else {
                    log::error!("could not find required consensus parameters for block {block} in subnet {netuid}");
                    continue;
                };

                let Some(module_key) = pallet_subspace::Pallet::<T>::get_key_for_uid(netuid, uid)
                else {
                    log::error!("could not find module {uid} key in subnet {netuid}");
                    continue;
                };

                let Some(module) = params.modules.get(&ModuleKey(module_key)) else {
                    log::error!("could not find required module {uid} parameter for block {block} in subnet {netuid}");
                    continue;
                };

                let hash =
                    sp_io::hashing::sha2_256(&Self::weights_to_blob(&weights[..])[..]).to_vec();

                if hash != module.weight_hash {
                    log::error!("incoherent hash received for module {uid} on block {block} in subnet {netuid}");
                    continue;
                }

                if let None = Self::validate_weights(uid, &weights, netuid) {
                    log::error!("validation failed for module {uid} weights on block {block} in subnet {netuid}");
                    continue;
                }

                valid_block_weights.push((uid, weights));
            }

            valid_weights.push((block, valid_block_weights));
        }

        match DecryptedWeights::<T>::get(netuid) {
            Some(mut cached) => {
                cached.extend(valid_weights);
                DecryptedWeights::<T>::set(netuid, Some(cached));
            }
            None => DecryptedWeights::<T>::set(netuid, Some(valid_weights)),
        }

        let block_number = pallet_subspace::Pallet::<T>::get_current_block_number();
        if block_number - info.block_assigned < T::DecryptionNodeRotationInterval::get() {
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

    fn weights_to_blob(weights: &[(u16, u16)]) -> Vec<u8> {
        let mut encoded = Vec::new();
        encoded.extend((weights.len() as u32).to_be_bytes());
        encoded.extend(weights.iter().flat_map(|(uid, weight)| {
            sp_std::vec![uid.to_be_bytes(), weight.to_be_bytes()]
                .into_iter()
                .flat_map(|a| a)
        }));

        encoded
    }

    fn validate_weights(uid: u16, weights: &Vec<(u16, u16)>, netuid: u16) -> Option<()> {
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
