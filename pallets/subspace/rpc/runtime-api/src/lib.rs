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
    pub stake_from: Vec<(AccountId, u64)>, /* map of key to stake on this module/key * (includes
                                            * delegations) */
    pub emission: u64,
    pub incentive: u16,
    pub dividends: u16,
    pub weights: Vec<(u16, u16)>, // Vec of (uid, weight)
}

#[derive(Decode, Encode, PartialEq, Eq, Clone, Debug, TypeInfo, Serialize, Deserialize)]
pub struct ModuleParams {
    pub name: String,
    pub address: String,
    pub delegation_fee: Percent, // delegate_fee
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

    // max
    pub max_name_length: u16,             // max length of a network name
    pub max_allowed_subnets: u16,         // max number of subnets allowed
    pub max_allowed_modules: u16,         // max number of modules allowed per subnet
    pub max_registrations_per_block: u16, // max number of registrations per block
    pub max_allowed_weights: u16,         // max number of weights per module
    pub max_proposals: u64,               // max number of proposals per block
    pub max_burn: u64,                    // max burn allowed

    // mins
    pub min_burn: u64,                 // min burn required
    pub min_stake: u64,                // min stake required
    pub floor_delegation_fee: Percent, // min delegation fee
    pub min_weight_stake: u64,         // min weight stake required

    // other
    pub target_registrations_per_interval: u16, // desired number of registrations per interval
    pub target_registrations_interval: u16,     /* the number of blocks that defines the
                                                 * registration interval */
    pub adjustment_alpha: u64, // adjustment alpha
    pub unit_emission: u64,    // emission per block
    pub tx_rate_limit: u64,    // tx rate limit
    pub vote_threshold: u16,   // out of 100
    pub vote_mode: String,     // out of 100
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
    // --- parameters
    pub founder: AccountId,
    pub founder_share: u16,   // out of 100
    pub immunity_period: u16, // immunity period
    pub incentive_ratio: u16, // out of 100
    pub max_allowed_uids: u16, /* max number of weights allowed to be registered in this
                               * pub max_allowed_uids: u16, // max number of uids
                               * allowed to be registered in this subne */
    pub max_allowed_weights: u16, /* max number of weights allowed to be registered in this
                                   * pub max_allowed_uids: u16, // max number of uids
                                   * allowed to be registered in this subnet */
    pub min_allowed_weights: u16, // min number of weights allowed to be registered in this
    pub max_stake: u64,           // max stake allowed
    pub max_weight_age: u64,      // max age of a weight
    pub min_stake: u64,           // min stake required
    pub name: String,
    pub tempo: u16, // how many blocks to wait before rewarding models
    pub trust_ratio: u16,
    pub vote_threshold: u16, // out of 100
    pub vote_mode: String,
}

#[derive(Decode, Encode, PartialEq, Eq, Clone, Debug, TypeInfo, Serialize, Deserialize)]
pub struct SubnetState {
    pub emission: u64,
    pub n_uids: u16, //number of uids
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
