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
pub mod network {
    pub mod module;
    pub mod registration;
    pub mod staking;
}

pub mod params {
    pub mod burn;
    pub mod global;
    pub mod module;
}

pub mod selections {
    pub mod weights
}


pub mod migrations;
pub mod rpc;
pub mod selections;

pub use crate::params::{
    burn::{BurnType, GeneralBurnConfiguration},
    global::GlobalParams,
    module::{ModuleChangeset, ModuleParams},
};
use selections::{config, dispatches, errors, events, genesis, hooks};

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
    pub use selections::weights::WeightInfo;
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

    #[pallet::storage]
    pub type Founder<T: Config> =
        StorageMap<_, Identity, u16, T::AccountId, ValueQuery, DefaultKey<T>>;

    #[pallet::storage]
    pub type ImmunityPeriod<T> =
        StorageMap<_, Identity, u16, u16, ValueQuery, ImmunityPeriodDefaultValue>;

    #[pallet::storage]
    pub type MinAllowedWeights<T> =
        StorageMap<_, Identity, u16, u16, ValueQuery, MinAllowedWeightsDefaultValue>;

    #[pallet::storage]
    pub type MaxWeightAge<T> =
        StorageMap<_, Identity, u16, u64, ValueQuery, MaxWeightAgeDefaultValue>;

    #[pallet::storage]
    pub type MaxAllowedWeights<T> =
        StorageMap<_, Identity, u16, u16, ValueQuery, MaxAllowedWeightsDefaultValue>;

    #[pallet::storage]
    pub type Tempo<T> = StorageMap<_, Identity, u16, u16, ValueQuery, TempoDefaultValue>;

    // --- Module ---
    #[pallet::storage]
    pub type Keys<T: Config> = StorageMap<_, Identity, u16, T::AccountId>;

    #[pallet::storage]
    pub type FounderShare<T: Config> =
        StorageMap<_, Twox64Concat, T::AccountId,  u16, ValueQuery, DefaultFounderShare<T>>;

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

    #[pallet::storage]
    pub type RegistrationBlock<T: Config> =
        StorageDoubleMap<_, Identity, u16, Identity, u16, u64, ValueQuery>;

    // --- Rootnet ---

    /// Maximum allowed length for names
    #[pallet::storage]
    pub type MaxNameLength<T: Config> = StorageValue<_, u16, ValueQuery, ConstU16<32>>;

    /// Minimum allowed length for names
    #[pallet::storage]
    pub type MinNameLength<T: Config> = StorageValue<_, u16, ValueQuery, ConstU16<2>>;

    /// Maximum allowed modules globally
    #[pallet::storage]
    pub type MaxAllowedModules<T: Config> = StorageValue<_, u16, ValueQuery, ConstU16<10_000>>;

    /// Minimum stake weight
    #[pallet::storage]
    pub type MinWeightStake<T> = StorageValue<_, u64, ValueQuery>;

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

    /// Minimum share percentage for subnet founders
    #[pallet::storage]
    pub type FloorFounderShare<T: Config> = StorageValue<_, u8, ValueQuery, ConstU8<8>>;
    /// Global minimum allowed stake
    #[pallet::storage]
    pub type MinimumAllowedStake<T> = StorageValue<_, u64, ValueQuery, ConstU64<500000000>>;

    #[pallet::type_value]
    pub fn DefaultFounderShare<T: Config>() -> u16 {
        FloorFounderShare::<T>::get() as u16
    }

    /// Control delegation per account
    #[pallet::storage]
    pub type WeightSettingDelegation<T: Config> =
        StorageDoubleMap<_, Identity, u16, Identity, T::AccountId, T::AccountId>;

    // --- Module Fees ---

    /// Default values for fees used throughout the module
    pub struct FeeDefaults;
    impl FeeDefaults {
        pub const STAKE_DELEGATION: Percent = Percent::from_percent(5);
        pub const VALIDATOR_WEIGHT: Percent = Percent::from_percent(4);
    }

    /// Contains the minimum allowed values for delegation fees
    #[derive(Encode, Decode, Clone, PartialEq, TypeInfo, Debug, Eq)]
    pub struct MinimumFees {
        /// Minimum fee for stake delegation
        pub stake_delegation_fee: Percent,
        /// Minimum fee for validator weight delegation
        pub validator_weight_fee: Percent,
    }

    #[pallet::type_value]
    pub fn DefaultMinimumFees<T: Config>() -> MinimumFees {
        MinimumFees {
            stake_delegation_fee: FeeDefaults::STAKE_DELEGATION,
            validator_weight_fee: FeeDefaults::VALIDATOR_WEIGHT,
        }
    }

    /// Storage for minimum fees that can be updated via runtime
    #[pallet::storage]
    pub type MinFees<T> = StorageValue<_, MinimumFees, ValueQuery, DefaultMinimumFees<T>>;

    /// A fee structure containing delegation fees for both stake and validator weight
    #[derive(Encode, Decode, Clone, PartialEq, TypeInfo, Debug, Eq)]
    pub struct ModuleFees {
        /// Fee charged when delegators delegate their stake
        pub stake_delegation_fee: Percent,
        /// Fee charged when validators delegate their weight-setting authority
        pub validator_weight_fee: Percent,
    }

    impl Default for ModuleFees {
        fn default() -> Self {
            Self {
                stake_delegation_fee: FeeDefaults::STAKE_DELEGATION,
                validator_weight_fee: FeeDefaults::VALIDATOR_WEIGHT,
            }
        }
    }

    #[pallet::type_value]
    pub fn DefaultValidatorFees<T: Config>() -> ModuleFees {
        ModuleFees::default()
    }

    /// Maps validator accounts to their fee configuration
    #[pallet::storage]
    pub type ValidatorFeeConfig<T: Config> =
        StorageMap<_, Identity, T::AccountId, ModuleFees, ValueQuery, DefaultValidatorFees<T>>;

    impl ModuleFees {
        /// Creates a new ModuleFees instance with validation against minimum fees
        pub fn new<T: Config>(
            stake_delegation_fee: Percent,
            validator_weight_fee: Percent,
        ) -> Result<Self, &'static str> {
            let min_fees = MinFees::<T>::get();
            if stake_delegation_fee < min_fees.stake_delegation_fee {
                return Err("Stake delegation fee is below minimum threshold");
            }
            if validator_weight_fee < min_fees.validator_weight_fee {
                return Err("Validator weight fee is below minimum threshold");
            }

            Ok(Self {
                stake_delegation_fee,
                validator_weight_fee,
            })
        }

        /// Validates that the fees meet minimum requirements
        pub fn validate<T: Config>(&self) -> Result<(), &'static str> {
            let min_fees = MinFees::<T>::get();
            if self.stake_delegation_fee < min_fees.stake_delegation_fee {
                return Err("Stake delegation fee is below minimum threshold");
            }
            if self.validator_weight_fee < min_fees.validator_weight_fee {
                return Err("Validator weight fee is below minimum threshold");
            }
            Ok(())
        }
    }
}
