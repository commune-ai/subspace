#![cfg_attr(not(feature = "std"), no_std)]

use sp_runtime::{DispatchError, MultiSignature, traits::{Verify, IdentifyAccount}};
use sp_runtime::{sp_std::prelude::Vec, ArithmeticError};
use parity_scale_codec::{Decode, Encode, EncodeLike, MaxEncodedLen};
use scale_info::TypeInfo;
use serde::{Deserialize, Serialize};
use sp_arithmetic::per_things::Percent;

type Result<T> = core::result::Result<T, DispatchError>;
type Signature = MultiSignature;
pub type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;

#[derive(Decode, Encode, PartialEq, Eq, Clone, Debug, TypeInfo, Serialize, Deserialize)]
pub struct ModuleStats{
	pub last_update: u64,
	pub registration_block: u64,
	pub stake_from: Vec<(AccountId, u64)>, /* map of key to stake on this module/key * (includes delegations) */
	pub emission: u64,
	pub incentive: u16,
	pub dividends: u16,
	pub weights: Vec<(u16, u16)>, // Vec of (uid, weight)
}

#[derive(Decode, Encode, PartialEq, Eq, Clone, Debug, TypeInfo, Serialize, Deserialize)]
pub struct ModuleParams {
	pub name: Vec<u8>,
	pub address: Vec<u8>,
	pub delegation_fee: Percent, // delegate_fee
	pub controller: AccountId,
}

#[derive(Decode, Encode, PartialEq, Eq, Clone, Debug, TypeInfo, Serialize, Deserialize)]
pub struct ModuleInfo {
	pub params: ModuleParams,
	pub stats: ModuleStats,
}

#[derive(Decode, Encode, PartialEq, Eq, Clone, Debug, TypeInfo, Serialize, Deserialize)]
#[scale_info(skip_type_params(T))]
pub struct SubnetParams {
	pub founder: AccountId,
	pub founder_share: u16,
	pub immunity_period: u16,
	pub incentive_ratio : u16,
	pub max_allowed_uids: u16,
	pub max_allowed_weights: u16,
	pub min_allowed_weights: u16,
	pub max_stake: u64,
	pub max_weight_age: u64,
	pub min_stake: u64,
	pub name: Vec<u8>,
	pub self_vote: bool,
	pub tempo: u16,
	pub trust_ratio: u16,
	pub vote_threshold: u16,
	pub vote_mode: Vec<u8>,
}

#[derive(Decode, Encode, PartialEq, Eq, Clone, Debug, TypeInfo, Serialize, Deserialize)]
pub struct SubnetInfo {
	pub params: SubnetParams,
	pub netuid: u16,
	pub n: u16,
	pub stake: u64,
	pub emission: u64,
	pub founder: AccountId,
}

sp_api::decl_runtime_apis! {
	pub trait SubspaceRuntimeApi
	{
		fn get_module_info(key: AccountId, netuid: u16) -> ModuleInfo;

		fn get_subnet_info(netuid: u16) -> SubnetInfo;
	}
}