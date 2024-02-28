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
pub struct ModuleState{
	pub uid: u16,
	pub module_key: AccountId,
	pub incentive: u16,
	pub trust: u16,
	pub dividend: u16,
	pub emission: u64,
	pub last_update: u64,
	pub registration_block: u64,
	pub stake: u64,
	pub stake_from: Vec<(AccountId, u64)>,
	pub profit_shares: Vec<(AccountId, u16)>
}

#[derive(Decode, Encode, PartialEq, Eq, Clone, Debug, TypeInfo, Serialize, Deserialize)]
pub struct ModuleParams {
	pub name: Vec<u8>,
	pub address: Vec<u8>,
	pub delegation_fee: Percent, // delegate_fee
	pub controller: AccountId,
	pub weights: Vec<(u16, u16)>
}

#[derive(Decode, Encode, PartialEq, Eq, Clone, Debug, TypeInfo, Serialize, Deserialize)]
pub struct ModuleInfo {
	pub params: ModuleParams,
	pub state: ModuleState,
}


// sp_api::decl_runtime_apis! {
// 	pub trait SubspaceRuntimeApi where
// 	AccountId: <<Signature as Verify>::Signer as IdentifyAccount>::AccountId
// 	{
// 		fn get_module_info() -> Result<ModuleInfo>;
// 	}
// }

sp_api::decl_runtime_apis! {
	pub trait SubspaceRuntimeApi
	{
		fn get_burn_rate() -> u16;

		fn get_module_info(key: AccountId, netuid: u16) -> ModuleInfo;
	}
}