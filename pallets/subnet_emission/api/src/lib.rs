#![no_std]

use parity_scale_codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;

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

pub trait SubnetEmissionApi {
    fn get_unit_emission() -> u64;

    fn set_unit_emission(unit_emission: u64);

    fn get_lowest_emission_netuid(ignore_subnet_immunity: bool) -> Option<u16>;

    fn remove_subnet_emission_storage(netuid: u16);

    fn set_subnet_emission_storage(netuid: u16, emission: u64);

    fn create_yuma_subnet(netuid: u16);

    fn remove_yuma_subnet(netuid: u16);

    fn can_remove_subnet(netuid: u16) -> bool;

    fn is_mineable_subnet(netuid: u16) -> bool;

    fn get_consensus_netuid(subnet_consensus: SubnetConsensus) -> Option<u16>;

    fn get_pending_emission(netuid: u16) -> u64;

    fn set_pending_emission(netuid: u16, pending_emission: u64);

    fn get_subnet_emission(netuid: u16) -> u64;

    fn set_subnet_emission(netuid: u16, subnet_emission: u64);

    fn get_subnet_consensus_type(netuid: u16) -> Option<SubnetConsensus>;

    fn set_subnet_consensus_type(netuid: u16, subnet_consensus: Option<SubnetConsensus>);
}
