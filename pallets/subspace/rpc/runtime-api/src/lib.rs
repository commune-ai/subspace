#![cfg_attr(not(feature = "std"), no_std)]

// pub type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;

// #[struct_versioning::versioned(version = 2, upper)]
// #[derive(Encode, Decode, Clone, PartialEq, TypeInfo, Serialize, Deserialize)]
// pub struct ModuleStats<AccountId> {
// 	last_update: u64,
// 	registration_block: u64,
// 	stake_from: Vec<(AccountId, u64)>, /* map of key to stake on this module/key * (includes delegations) */
// 	emission: u64,
// 	incentive: u16,
// 	dividends: u16,
// 	weights: Vec<(u16, u16)>, // Vec of (uid, weight)
// }

use sp_runtime::DispatchError;

type Result<T> = core::result::Result<T, DispatchError>;

sp_api::decl_runtime_apis! {
	pub trait SubspaceRuntimeApi {
		fn get_burn_rate() -> u16;
	}
}