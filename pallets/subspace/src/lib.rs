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

pub use self::{network::subnet, params::global};
use frame_support::pallet_prelude::Weight;
use pallet_subspace_genesis_config::ConfigSubnet;
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
pub mod macros;
pub mod math;
pub mod network {
    pub mod module;
    pub mod registration;
    pub mod staking;
    pub mod subnet;
}
pub mod params {
    pub mod burn;
    pub mod global;
    pub mod module;
    pub mod subnet;
}

pub mod migrations;
pub mod rpc;
pub mod selections;
pub mod weights;

pub use crate::params::{
    burn::{BurnType, GeneralBurnConfiguration},
    global::GlobalParams,
    module::{ModuleChangeset, ModuleParams},
    subnet::{DefaultSubnetParams, SubnetChangeset, SubnetParams},
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
    pub use crate::weights::WeightInfo;
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

    // --- Subnet Storage ---
    define_subnet_includes!(
        double_maps: {
            Bonds,
            SetWeightCallsPerEpoch,
            Uids,
            Keys,
            Name,
            Address,
            Metadata,
            RegistrationBlock,
        },
        maps: {
            BondsMovingAverage: u64 = 900_000,
            ValidatorPermits,
            ValidatorTrust,
            PruningScores,
            MaxAllowedValidators,
            Consensus,
            Active,
            Rank,
            Burn,
            MaximumSetWeightCallsPerEpoch,
            SubnetNames,
            SubnetMetadata,
            N,
            Founder,
            IncentiveRatio: u16 = 50,
            ModuleBurnConfig,
            RegistrationsThisInterval,
            MaxEncryptionPeriod: Option<u64> = Some(10_800),
            CopierMargin: I64F64 = I64F64::from_num(0),
            UseWeightsEncryption,
            AlphaValues: (u16, u16) = (45875, 58982),
            MinValidatorStake,
            MaxAllowedUids: u16 = 420,
            ImmunityPeriod: u16 = 0,
            MinAllowedWeights: u16 = 1,
            MaxWeightAge: u64 = 3_600,
            MaxAllowedWeights: u16 = 420,
            Tempo: u16 = 100,
            FounderShare,
            Incentive,
            Trust,
            Dividends,
            Emission,
            LastUpdate,
            SubnetRegistrationBlock
        }
    );

    // --- Module Storage ---
    define_module_includes!(
        // Put here every module-related storage map that has netuid as a key and holds a vector of values. The vector has to be indexed by the module uid.
        vectors: {
            Active: bool = false,
            Consensus: u64 = 0,
            Emission: u64 = 0,
            Incentive: u64 = 0,
            Dividends: u64 = 0,
            LastUpdate: u64 = Pallet::<T>::get_current_block_number(),
            Rank: u64 = 0,
            Trust: u64 = 0,
            ValidatorPermits: bool = false,
            ValidatorTrust: u64 = 0,
            PruningScores: u16 = 0,
        },
        // Put here every module-related double map, where the first key is netuid, second key is module uid. These storages holds some value for each module ie. name, address, etc.
        swap_storages: {
            optional: {
            },
            required: {
                RegistrationBlock: u64 = Pallet::<T>::get_current_block_number(),
                Address: Vec<u8> = Vec::<u8>::new(),
                Name: Vec<u8> = Vec::<u8>::new(),
                Bonds: Vec<(u16, u16)> = Vec::<(u16, u16)>::new(),
            }
        },
        // Specifically for uids and keys
        key_storages: {
            uid_key: Uids,
            key_uid: Keys
        },
        // Put here every module-related double map, that has no uid association. first key is netuid, second key is key of module (not uid!)
        key_only_storages: {
            SetWeightCallsPerEpoch: u16,
            Metadata: Vec<u8>,
            WeightSettingDelegation: DelegationInfo<T::AccountId>
        }
    );

    #[pallet::storage]
    pub type Bonds<T: Config> =
        StorageDoubleMap<_, Identity, u16, Identity, u16, Vec<(u16, u16)>, ValueQuery>;

    #[pallet::storage]
    pub type BondsMovingAverage<T> =
        StorageMap<_, Identity, u16, u64, ValueQuery, BondsMovingAverageDefaultValue>;

    #[pallet::storage]
    pub type ValidatorPermits<T: Config> = StorageMap<_, Identity, u16, Vec<bool>, ValueQuery>;

    #[pallet::storage]
    pub type ValidatorTrust<T: Config> = StorageMap<_, Identity, u16, Vec<u16>, ValueQuery>;

    #[pallet::storage]
    pub type PruningScores<T: Config> = StorageMap<_, Identity, u16, Vec<u16>, ValueQuery>;

    #[pallet::type_value]
    pub fn DefaultMaxAllowedValidators<T: Config>() -> Option<u16> {
        None
    }

    #[pallet::storage]
    pub type MaxAllowedValidators<T> =
        StorageMap<_, Identity, u16, Option<u16>, ValueQuery, DefaultMaxAllowedValidators<T>>;

    #[pallet::storage]
    pub type Consensus<T: Config> = StorageMap<_, Identity, u16, Vec<u16>, ValueQuery>;

    #[pallet::storage]
    pub type Active<T: Config> = StorageMap<_, Identity, u16, Vec<bool>, ValueQuery>;

    #[pallet::storage]
    pub type Rank<T: Config> = StorageMap<_, Identity, u16, Vec<u16>, ValueQuery>;

    #[pallet::storage]
    pub type Burn<T: Config> = StorageMap<_, Identity, u16, u64, ValueQuery>;

    #[pallet::storage]
    pub type MaximumSetWeightCallsPerEpoch<T: Config> = StorageMap<_, Identity, u16, u16>;

    #[pallet::storage]
    pub type SetWeightCallsPerEpoch<T: Config> =
        StorageDoubleMap<_, Identity, u16, Identity, T::AccountId, u16, ValueQuery>;

    #[pallet::storage]
    pub type SubnetNames<T: Config> = StorageMap<_, Identity, u16, Vec<u8>, ValueQuery>;

    #[pallet::storage]
    pub type SubnetMetadata<T: Config> =
        StorageMap<_, Identity, u16, BoundedVec<u8, ConstU32<120>>>;

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
    pub type IncentiveRatio<T: Config> =
        StorageMap<_, Identity, u16, u16, ValueQuery, IncentiveRatioDefaultValue>;

    #[pallet::storage]
    pub type ModuleBurnConfig<T: Config> =
        StorageMap<_, Identity, u16, GeneralBurnConfiguration<T>, ValueQuery>;

    #[pallet::storage]
    pub type RegistrationsThisInterval<T: Config> = StorageMap<_, Identity, u16, u16, ValueQuery>;

    #[pallet::storage]
    pub type MaxEncryptionPeriod<T: Config> = StorageMap<_, Identity, u16, Option<u64>, ValueQuery>;

    #[pallet::storage]
    pub type CopierMargin<T: Config> =
        StorageMap<_, Identity, u16, I64F64, ValueQuery, CopierMarginDefaultValue>;

    #[pallet::storage]
    pub type UseWeightsEncryption<T: Config> = StorageMap<_, Identity, u16, bool, ValueQuery>;

    #[pallet::storage]
    pub type AlphaValues<T: Config> =
        StorageMap<_, Identity, u16, (u16, u16), ValueQuery, AlphaValuesDefaultValue>;

    #[pallet::storage]
    pub type MinValidatorStake<T: Config> =
        StorageMap<_, Identity, u16, u64, ValueQuery, T::DefaultMinValidatorStake>;

    #[pallet::storage]
    pub type MaxAllowedUids<T> =
        StorageMap<_, Identity, u16, u16, ValueQuery, MaxAllowedUidsDefaultValue>;

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

    #[pallet::storage]
    pub type FounderShare<T: Config> =
        StorageMap<_, Identity, u16, u16, ValueQuery, DefaultFounderShare<T>>;

    // --- Module ---

    #[pallet::storage]
    pub type Uids<T: Config> =
        StorageDoubleMap<_, Identity, u16, Blake2_128Concat, T::AccountId, u16>;

    #[pallet::storage]
    pub type Keys<T: Config> = StorageDoubleMap<_, Identity, u16, Identity, u16, T::AccountId>;

    #[pallet::storage]
    pub type Name<T: Config> =
        StorageDoubleMap<_, Twox64Concat, u16, Twox64Concat, u16, Vec<u8>, ValueQuery>;

    #[pallet::storage]
    pub type Address<T: Config> =
        StorageDoubleMap<_, Twox64Concat, u16, Twox64Concat, u16, Vec<u8>, ValueQuery>;

    #[pallet::storage]
    pub type Metadata<T: Config> =
        StorageDoubleMap<_, Twox64Concat, u16, Twox64Concat, T::AccountId, Vec<u8>>;

    #[pallet::storage]
    #[pallet::getter(fn get_incentive_for)]
    pub type Incentive<T: Config> = StorageMap<_, Identity, u16, Vec<u16>, ValueQuery>;

    #[pallet::storage]
    pub type Trust<T: Config> = StorageMap<_, Identity, u16, Vec<u16>, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn get_dividends_for)]
    pub type Dividends<T: Config> = StorageMap<_, Identity, u16, Vec<u16>, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn get_emission_for)]
    pub type Emission<T: Config> = StorageMap<_, Identity, u16, Vec<u64>, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn get_last_update_for)]
    pub type LastUpdate<T: Config> = StorageMap<_, Identity, u16, Vec<u64>, ValueQuery>;

    #[pallet::storage]
    pub type SubnetRegistrationBlock<T: Config> = StorageMap<_, Identity, u16, u64>;

    #[pallet::storage]
    pub type RegistrationBlock<T: Config> =
        StorageDoubleMap<_, Identity, u16, Identity, u16, u64, ValueQuery>;

    // --- Rootnet ---

    #[pallet::storage]
    pub type Rho<T: Config> = StorageValue<_, u16, ValueQuery, ConstU16<10>>;

    #[pallet::storage]
    pub type RootNetWeightCalls<T: Config> = StorageMap<_, Identity, u16, ()>;

    #[pallet::storage]
    pub type Kappa<T> = StorageValue<_, u16, ValueQuery, ConstU16<32_767>>;

    /// Maximum allowed length for names
    #[pallet::storage]
    pub type MaxNameLength<T: Config> = StorageValue<_, u16, ValueQuery, ConstU16<32>>;

    /// Minimum allowed length for names
    #[pallet::storage]
    pub type MinNameLength<T: Config> = StorageValue<_, u16, ValueQuery, ConstU16<2>>;

    /// Maximum number of allowed subnets
    #[pallet::storage]
    pub type MaxAllowedSubnets<T: Config> = StorageValue<_, u16, ValueQuery, ConstU16<256>>;

    /// Maximum allowed modules globally
    #[pallet::storage]
    pub type MaxAllowedModules<T: Config> = StorageValue<_, u16, ValueQuery, ConstU16<10_000>>;

    /// Minimum stake weight
    #[pallet::storage]
    pub type MinWeightStake<T> = StorageValue<_, u64, ValueQuery>;

    /// Global maximum allowed weights
    #[pallet::storage]
    pub type MaxAllowedWeightsGlobal<T> = StorageValue<_, u16, ValueQuery, ConstU16<512>>;

    // --- Registration Storage ---

    /// Number of registrations in the current block
    #[pallet::storage]
    pub type RegistrationsPerBlock<T> = StorageValue<_, u16, ValueQuery>;

    /// Maximum allowed registrations per block
    #[pallet::storage]
    pub type MaxRegistrationsPerBlock<T> = StorageValue<_, u16, ValueQuery, ConstU16<10>>;

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

    // --- Subnet Storage ---

    /// Available subnet IDs that can be reused
    #[pallet::storage]
    pub type SubnetGaps<T> = StorageValue<_, BTreeSet<u16>, ValueQuery>;

    /// Minimum share percentage for subnet founders
    #[pallet::storage]
    pub type FloorFounderShare<T: Config> = StorageValue<_, u8, ValueQuery, ConstU8<8>>;

    // --- Subnet Registration Configuration ---

    #[pallet::type_value]
    pub fn SubnetBurnConfigDefault<T: Config>() -> GeneralBurnConfiguration<T> {
        GeneralBurnConfiguration::<T>::default_for(BurnType::Subnet)
    }

    /// General burn configuration for subnet registration
    #[pallet::storage]
    pub type SubnetBurnConfig<T: Config> =
        StorageValue<_, GeneralBurnConfiguration<T>, ValueQuery, SubnetBurnConfigDefault<T>>;

    /// Subnet registrations in current interval
    #[pallet::storage]
    pub type SubnetRegistrationsThisInterval<T: Config> = StorageValue<_, u16, ValueQuery>;

    #[pallet::type_value]
    pub fn DefaultSubnetBurn<T: Config>() -> u64 {
        GeneralBurnConfiguration::<T>::default_for(BurnType::Subnet).min_burn
    }

    /// Minimum burn amount for subnet registration
    #[pallet::storage]
    pub type SubnetBurn<T: Config> = StorageValue<_, u64, ValueQuery, DefaultSubnetBurn<T>>;

    /// Global minimum allowed stake
    #[pallet::storage]
    pub type MinimumAllowedStake<T> = StorageValue<_, u64, ValueQuery, ConstU64<500000000>>;

    #[pallet::type_value]
    pub fn DefaultFounderShare<T: Config>() -> u16 {
        FloorFounderShare::<T>::get() as u16
    }

    /// Subnet immunity period
    #[pallet::storage]
    pub type SubnetImmunityPeriod<T: Config> = StorageValue<_, u64, ValueQuery, ConstU64<32400>>;

    /// Control delegation per account
    #[pallet::storage]
    pub type WeightSettingDelegation<T: Config> =
        StorageDoubleMap<_, Identity, u16, Identity, T::AccountId, T::AccountId>;

    #[pallet::storage]
    pub type Bridged<T: Config> = StorageMap<_, Identity, T::AccountId, u64, ValueQuery>;
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
    pub struct ValidatorFees {
        /// Fee charged when delegators delegate their stake
        pub stake_delegation_fee: Percent,
        /// Fee charged when validators delegate their weight-setting authority
        pub validator_weight_fee: Percent,
    }

    impl Default for ValidatorFees {
        fn default() -> Self {
            Self {
                stake_delegation_fee: FeeDefaults::STAKE_DELEGATION,
                validator_weight_fee: FeeDefaults::VALIDATOR_WEIGHT,
            }
        }
    }

    #[pallet::type_value]
    pub fn DefaultValidatorFees<T: Config>() -> ValidatorFees {
        ValidatorFees::default()
    }

    /// Maps validator accounts to their fee configuration
    #[pallet::storage]
    pub type ValidatorFeeConfig<T: Config> =
        StorageMap<_, Identity, T::AccountId, ValidatorFees, ValueQuery, DefaultValidatorFees<T>>;

    impl ValidatorFees {
        /// Creates a new ValidatorFees instance with validation against minimum fees
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
