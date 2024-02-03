#![cfg_attr(not(feature = "std"), no_std)]

use sp_runtime::{
	MultiSignature,
	sp_std::prelude::Vec,
	traits::{
		Verify,
		IdentifyAccount
	},
};
use sp_arithmetic::per_things::Percent;

use codec::{
	Decode,
	Encode,
};
use scale_info::TypeInfo;
use serde::{
	Deserialize,
	Serialize
};

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

sp_api::decl_runtime_apis! {
	pub trait SubspaceRuntimeApi
	{
		fn get_module_info(key: AccountId, netuid: u16) -> ModuleInfo;
	}
}