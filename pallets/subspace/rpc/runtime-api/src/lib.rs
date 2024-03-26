#![cfg_attr(not(feature = "std"), no_std)]

use parity_scale_codec::{Decode, Encode};
use scale_info::{prelude::string::String, TypeInfo};
use serde::{Deserialize, Serialize};
use sp_arithmetic::per_things::Percent;
use sp_runtime::{
    sp_std::prelude::Vec,
    traits::{IdentifyAccount, Verify},
    MultiSignature,
};

type Signature = MultiSignature;
pub type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;

#[derive(Decode, Encode, PartialEq, Eq, Clone, Debug, TypeInfo, Serialize, Deserialize)]
pub struct ModuleStats {
    pub last_update: u64,
    pub registration_block: u64,
    pub stake_from: Vec<(AccountId, u64)>,
    pub emission: u64,
    pub incentive: u16,
    pub dividends: u16,
    pub weights: Vec<(u16, u16)>,
}

#[derive(Decode, Encode, PartialEq, Eq, Clone, Debug, TypeInfo, Serialize, Deserialize)]
pub struct ModuleParams {
    pub name: String,
    pub address: String,
    pub delegation_fee: Percent,
    pub controller: AccountId,
}

#[derive(Decode, Encode, PartialEq, Eq, Clone, Debug, TypeInfo, Serialize, Deserialize)]
pub struct ModuleInfo {
    pub uid: u16,
    pub params: ModuleParams,
    pub stats: ModuleStats,
}

#[derive(Decode, Encode, PartialEq, Eq, Clone, Debug, TypeInfo, Serialize, Deserialize)]
pub struct KeyStakeToInfo {
    pub netuid: u16,
    pub subnet_name: String,
    pub stake_to_module: Vec<(String, u64)>,
}

#[derive(Decode, Encode, PartialEq, Eq, Clone, Debug, TypeInfo, Serialize, Deserialize)]
pub struct KeyInfo {
    pub balance: u64,
    pub total_stake: u64,
    pub stake_to: Vec<KeyStakeToInfo>,
}

#[derive(Decode, Encode, PartialEq, Eq, Clone, Debug, TypeInfo, Serialize, Deserialize)]
pub struct GlobalParams {
    pub burn_rate: u16,

    pub max_name_length: u16,
    pub max_allowed_subnets: u16,
    pub max_allowed_modules: u16,
    pub max_registrations_per_block: u16,
    pub max_allowed_weights: u16,
    pub max_proposals: u64,
    pub max_burn: u64,

    pub min_burn: u64,
    pub min_stake: u64,
    pub floor_delegation_fee: Percent,
    pub min_weight_stake: u64,

    pub target_registrations_per_interval: u16,
    pub target_registrations_interval: u16,

    pub adjustment_alpha: u64,
    pub unit_emission: u64,
    pub tx_rate_limit: u64,
    pub vote_threshold: u16,
    pub vote_mode: String,
}

#[derive(Decode, Encode, PartialEq, Eq, Clone, Debug, TypeInfo, Serialize, Deserialize)]
pub struct GlobalState {
    // status
    pub registrations_per_block: u16,
    pub total_subnets: u16,
}

#[derive(Decode, Encode, PartialEq, Eq, Clone, Debug, TypeInfo, Serialize, Deserialize)]
pub struct GlobalInfo {
    pub params: GlobalParams,
    pub stats: GlobalState,
}

#[derive(Decode, Encode, PartialEq, Eq, Clone, Debug, TypeInfo, Serialize, Deserialize)]
pub struct SubnetParams {
    pub founder: AccountId,
    pub founder_share: u16,
    pub immunity_period: u16,
    pub incentive_ratio: u16,
    pub max_allowed_uids: u16,
    pub max_allowed_weights: u16,
    pub min_allowed_weights: u16,
    pub max_stake: u64,
    pub max_weight_age: u64,
    pub min_stake: u64,
    pub name: String,
    pub tempo: u16,
    pub trust_ratio: u16,
    pub vote_threshold: u16,
    pub vote_mode: String,
}

#[derive(Decode, Encode, PartialEq, Eq, Clone, Debug, TypeInfo, Serialize, Deserialize)]
pub struct SubnetState {
    pub emission: u64,
    pub n_uids: u16,
    pub pending_emission: u64,

    pub total_stake: u64,
}

#[derive(Decode, Encode, PartialEq, Eq, Clone, Debug, TypeInfo, Serialize, Deserialize)]
pub struct SubnetInfo {
    pub params: SubnetParams,
    pub stats: SubnetState,
}

sp_api::decl_runtime_apis! {
    pub trait SubspaceRuntimeApi
    {
        fn get_global_info() -> GlobalInfo;

        fn get_subnet_info(netuid: u16) -> SubnetInfo;

        fn get_module_info(key: AccountId, netuid: u16) -> ModuleInfo;

        fn get_key_info(key: AccountId) -> KeyInfo;
    }
}
