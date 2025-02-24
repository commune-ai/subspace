#![no_std]

use parity_scale_codec::{Decode, Encode, MaxEncodedLen};
use scale_info::{prelude::vec::Vec, TypeInfo};

use frame_support::dispatch::DispatchResult;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, TypeInfo, Decode, Encode, MaxEncodedLen)]
pub enum SubnetConsensus {
    // Default
    #[default]
    Yuma,
    // System
    Linear,
    Treasury,
    // Pricing
    Root,
}

pub type SubnetWeights = Vec<(u16, Vec<(u16, u16)>)>;

pub trait SubnetEmissionApi<AccountId> {
    fn get_lowest_emission_netuid(ignore_subnet_immunity: bool) -> Option<u16>;

    fn set_emission_storage(netuid: u16, emission: u64);

    fn create_yuma_subnet(netuid: u16);

    fn can_remove_subnet(netuid: u16) -> bool;

    fn is_mineable_subnet(netuid: u16) -> bool;

    fn get_consensus_netuid(subnet_consensus: SubnetConsensus) -> Option<u16>;

    fn get_subnet_consensus_type(netuid: u16) -> Option<SubnetConsensus>;

    fn set_subnet_consensus_type(netuid: u16, subnet_consensus: Option<SubnetConsensus>);

    fn get_weights(netuid: u16, uid: u16) -> Option<Vec<(u16, u16)>>;

    fn set_weights(
        netuid: u16,
        uid: u16,
        weights: Option<Vec<(u16, u16)>>,
    ) -> Option<Vec<(u16, u16)>>;

    fn clear_subnet_includes(netuid: u16);

    fn clear_module_includes(
        netuid: u16,
        uid: u16,
        replace_uid: u16,
        module_key: &AccountId,
        replace_key: &AccountId,
    ) -> DispatchResult;
}
