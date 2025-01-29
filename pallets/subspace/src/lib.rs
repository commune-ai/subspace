#![allow(deprecated, non_camel_case_types, non_snake_case)]
#![cfg_attr(not(feature = "std"), no_std)]
#![recursion_limit = "512"]

use frame_support::{
    dispatch, ensure,
    pallet_macros::import_section,
    traits::{tokens::WithdrawReasons, ConstU32, Currency, ExistenceRequirement},
    PalletId,
};

use frame_system::{self as system, ensure_signed};
pub use pallet::*;
use sp_std::collections::btree_set::BTreeSet;

pub use self::{params::global};
use frame_support::pallet_prelude::Weight;
use parity_scale_codec::{Decode, Encode};
use sp_std::marker::PhantomData;

// ---------------------------------
//	Benchmark Imports
// ---------------------------------

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

// ---------------------------------
// Pallet Imports
// ---------------------------------
pub mod math;
pub mod staking;
pub mod global
pub mod module
pub mod weights
pub mod migrations;
pub mod rpc;
pub mod errors;
pub mod events;
pub mod genesis;
pub mod hooks;


pub use crate::params::{
    global::GlobalParams,
    module::{ModuleChangeset, ModuleParams},
};
#[import_section(genesis::genesis)]
#[import_section(errors::errors)]
#[import_section(events::events)]
#[import_section(dispatches::dispatches)]
#[import_section(hooks::hooks)]
#[import_section(config::config)]
#[frame_support::pallet]
pub mod pallet {
    #![allow(deprecated, clippy::let_unit_value, clippy::too_many_arguments)]
    use super::*;
    pub use weights::WeightInfo;
    use frame_support::{
        dispatch::DispatchResult,
        pallet_prelude::{ValueQuery, *},
        traits::Currency,
        Identity,
    };
    use frame_system::pallet_prelude::*;
    use pallet_governance_api::{GovernanceConfiguration, VoteMode};
    use sp_arithmetic::per_things::Percent;
    use sp_core::{ConstU16, ConstU64, ConstU8};
    pub use sp_std::{vec, vec::Vec};
    use substrate_fixed::types::I64F64;

    pub type BalanceOf<T> =
        <<T as Config>::Currency as Currency<<T as system::Config>::AccountId>>::Balance;

    pub type AccountIdOf<T> = <T as frame_system::Config>::AccountId;

    #[cfg(feature = "testnet")]
    const STORAGE_VERSION: StorageVersion = StorageVersion::new(34);

    #[cfg(not(feature = "testnet"))]
    const STORAGE_VERSION: StorageVersion = StorageVersion::new(15);

    #[pallet::pallet]
    #[pallet::storage_version(STORAGE_VERSION)]
    #[pallet::without_storage_info]
    pub struct Pallet<T>(_);

    #[pallet::storage]
    pub type N<T> = StorageMap<_, Identity, u16, u16, ValueQuery>;

    #[pallet::type_value]
    pub fn DefaultKey<T: Config>() -> T::AccountId {
        T::AccountId::decode(&mut sp_runtime::traits::TrailingZeroInput::zeroes()).unwrap()
    }

    // --- Module ---
    #[pallet::storage]
    pub type Keys<T: Config> = StorageMap<_, Identity, u16, T::AccountId>;


    #[pallet::storage]
    pub type Name<T: Config> =
        StorageMap<_, Twox64Concat, T::AccountId, Vec<u8>, ValueQuery>;

    #[pallet::storage]
    pub type Url<T: Config> =
        StorageMap<_, Twox64Concat, T::AccountId, Vec<u8>, ValueQuery>;

    #[pallet::storage]
    pub type Metadata<T: Config> =
         StorageMap<_, Twox64Concat, T::AccountId, Vec<u64>, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn get_emission_for)]
    pub type Emission<T: Config> = StorageMap<_, Twox64Concat, T::AccountId,, Vec<u64>, ValueQuery>;
    // --- Rootnet ---

    /// Maximum allowed length for names
    #[pallet::storage]
    pub type MaxNameLength<T: Config> = StorageValue<_, u16, ValueQuery, ConstU16<32>>;

    /// Maximum allowed modules globally
    #[pallet::storage]
    pub type MaxAllowedModules<T: Config> = StorageValue<_, u16, ValueQuery, ConstU16<10_000>>;

    // --- Staking Storage ---

    /// Maps (from_account, to_account) to stake amount
    #[pallet::storage]
    pub type StakeFrom<T: Config> =
        StorageDoubleMap<_, Identity, T::AccountId, Identity, T::AccountId, u64, ValueQuery>;

    /// Maps (to_account, from_account) to stake amount
    #[pallet::storage]
    pub type StakeTo<T: Config> =
        StorageDoubleMap<_, Identity, T::AccountId, Identity, T::AccountId, u64, ValueQuery>;

    /// Total stake in the system
    #[pallet::storage]
    pub type TotalStake<T> = StorageValue<_, u64, ValueQuery>;

    /// Global minimum allowed stake
    #[pallet::storage]
    pub type MinimumAllowedStake<T> = StorageValue<_, u64, ValueQuery, ConstU64<500000000>>;

}
