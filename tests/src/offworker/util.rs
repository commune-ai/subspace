use crate::mock::*;
use frame_support::pallet_prelude::BoundedVec;
use pallet_subnet_emission::{
    types::{DecryptionNodeInfo, PublicKey},
    Authorities, Config, DecryptionNodes, SubnetConsensusType, SubnetDecryptionData,
};
use pallet_subnet_emission_api::SubnetConsensus;
use pallet_subspace::{
    BondsMovingAverage, FounderShare, MaxAllowedUids, MaxAllowedWeights, MaxEncryptionPeriod,
    MaxRegistrationsPerBlock, MaxWeightAge, MinValidatorStake, Tempo, UseWeightsEncryption,
};

pub fn setup_subnet(netuid: u16, tempo: u64) {
    register_subnet(u32::MAX, 0).unwrap();
    zero_min_burn();
    SubnetConsensusType::<Test>::set(netuid, Some(SubnetConsensus::Yuma));
    Tempo::<Test>::insert(netuid, tempo as u16);

    BondsMovingAverage::<Test>::insert(netuid, 0);
    UseWeightsEncryption::<Test>::set(netuid, true);

    MaxWeightAge::<Test>::set(netuid, 50_000);
    MinValidatorStake::<Test>::set(netuid, to_nano(10));

    // Things that should never expire / exceed
    MaxEncryptionPeriod::<Test>::set(netuid, u64::MAX);
    MaxRegistrationsPerBlock::<Test>::set(u16::MAX);
    MaxAllowedUids::<Test>::set(netuid, u16::MAX);
    MaxAllowedWeights::<Test>::set(netuid, u16::MAX);
    FounderShare::<Test>::set(netuid, 0);
}

/// Updates all authority nodes with new account id
pub fn update_authority_and_decryption_node<T: Config>(subnet_id: u16, new_acc_id: T::AccountId) {
    // Update Authorities
    Authorities::<T>::mutate(|authorities| {
        if let Some(authority) = authorities.iter_mut().next() {
            authority.0 = new_acc_id.clone();
        }
    });

    // Update DecryptionNodes
    DecryptionNodes::<T>::mutate(|nodes| {
        if let Some(node) = nodes.iter_mut().next() {
            node.account_id = new_acc_id.clone();
        }
    });

    SubnetDecryptionData::<T>::mutate(subnet_id, |subnet_info| {
        if let Some(info) = subnet_info {
            // Only update the node_id, keep the existing public_key
            info.node_id = new_acc_id;
            // The public_key and block_assigned remain unchanged
        } else {
            // This should never happen
            panic!("Subnet info not found");
        }
    });
}

/// Function that initializes authorites with initial values
/// THe acc id of the node is sample, it will be updated later
#[must_use]
pub fn initialize_authorities(
    public_key: PublicKey,
    first_block: BlockNumber,
) -> DecryptionNodeInfo<Test> {
    let acc_id = u32::MAX;

    let authorities: BoundedVec<(AccountId, PublicKey), <Test as Config>::MaxAuthorities> =
        vec![(acc_id, (public_key.0.to_vec(), public_key.1.to_vec()))]
            .try_into()
            .expect("Should not exceed max authorities");

    Authorities::<Test>::put(authorities);

    let decryption_info = DecryptionNodeInfo {
        account_id: acc_id,
        public_key,
        last_keep_alive: first_block,
    };
    let decryption_nodes = vec![decryption_info.clone()];
    DecryptionNodes::<Test>::set(decryption_nodes);

    decryption_info
}
