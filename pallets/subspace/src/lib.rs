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
use scale_info::TypeInfo;
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
pub mod math;
pub mod migrations;
pub mod network {
    pub mod module;
    pub mod registration;
    pub mod staking;
    pub mod subnet;
}
pub mod params {
    pub mod global;
    pub mod params;
}
pub mod rpc;
pub mod selections;
pub mod signed_extension;
pub mod weights;

pub use crate::{
    network::{module::ModuleChangeset, subnet::SubnetChangeset},
    params::{
        global::{BurnType, GeneralBurnConfiguration},
        params::{DefaultSubnetParams, GlobalParams, ModuleParams, SubnetParams},
    },
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
    #![allow(
        deprecated,
        clippy::let_unit_value,
        clippy::too_many_arguments,
        clippy::type_complexity
    )]

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

    const STORAGE_VERSION: StorageVersion = StorageVersion::new(13);

    #[pallet::pallet]
    #[pallet::storage_version(STORAGE_VERSION)]
    #[pallet::without_storage_info]
    pub struct Pallet<T>(_);

    pub type BalanceOf<T> =
        <<T as Config>::Currency as Currency<<T as system::Config>::AccountId>>::Balance;

    pub type AccountIdOf<T> = <T as frame_system::Config>::AccountId;

    // ---------------------------------
    // Global Variables
    // ---------------------------------

    // Rootnet
    // =======
    #[pallet::storage]
    pub type Rho<T> = StorageValue<_, u16, ValueQuery, ConstU16<10>>;

    #[pallet::storage]
    pub type RootNetWeightCalls<T: Config> = StorageMap<_, Identity, u16, ()>;

    #[pallet::storage]
    pub type Kappa<T> = StorageValue<_, u16, ValueQuery, ConstU16<32_767>>;

    #[pallet::storage] // --- DMAP ( netuid, uid ) --> bonds
    pub type Bonds<T: Config> =
        StorageDoubleMap<_, Identity, u16, Identity, u16, Vec<(u16, u16)>, ValueQuery>;

    #[pallet::storage] // --- MAP ( netuid ) --> bonds_moving_average
    pub type BondsMovingAverage<T> =
        StorageMap<_, Identity, u16, u64, ValueQuery, ConstU64<900_000>>;

    #[pallet::storage] // --- DMAP ( netuid ) --> validator_permit
    pub type ValidatorPermits<T: Config> = StorageMap<_, Identity, u16, Vec<bool>, ValueQuery>;

    #[pallet::storage] // --- DMAP ( netuid ) --> validator_trust
    pub type ValidatorTrust<T: Config> = StorageMap<_, Identity, u16, Vec<u16>, ValueQuery>;

    #[pallet::storage] // --- DMAP ( netuid ) --> pruning_scores
    pub type PruningScores<T: Config> = StorageMap<_, Identity, u16, Vec<u16>, ValueQuery>;

    #[pallet::type_value]
    pub fn DefaultMaxAllowedValidators<T: Config>() -> Option<u16> {
        None // Some(128)
    }

    #[pallet::storage] // --- MAP ( netuid ) --> max_allowed_validators
    pub type MaxAllowedValidators<T> =
        StorageMap<_, Identity, u16, Option<u16>, ValueQuery, DefaultMaxAllowedValidators<T>>;

    #[pallet::storage] // --- MAP ( netuid ) --> consensus
    pub type Consensus<T: Config> = StorageMap<_, Identity, u16, Vec<u16>, ValueQuery>;

    #[pallet::storage] // --- MAP ( netuid ) --> active
    pub type Active<T: Config> = StorageMap<_, Identity, u16, Vec<bool>, ValueQuery>;

    #[pallet::storage] // --- DMAP ( netuid ) --> rank
    pub type Rank<T: Config> = StorageMap<_, Identity, u16, Vec<u16>, ValueQuery>;

    #[pallet::storage] // --- ITEM ( max_name_length )
    pub type MaxNameLength<T: Config> = StorageValue<_, u16, ValueQuery, ConstU16<32>>;

    #[pallet::storage] // --- ITEM ( min_name_length )
    pub type MinNameLength<T: Config> = StorageValue<_, u16, ValueQuery, ConstU16<2>>;

    #[pallet::storage] // --- ITEM ( max_allowed_subnets )
    pub type MaxAllowedSubnets<T: Config> = StorageValue<_, u16, ValueQuery, ConstU16<256>>;

    #[pallet::storage]
    // --- MAP (netuid) --> burn
    pub type Burn<T: Config> = StorageMap<_, Identity, u16, u64, ValueQuery>;

    #[pallet::storage] // --- ITEM ( max_allowed_modules )
    pub type MaxAllowedModules<T: Config> = StorageValue<_, u16, ValueQuery, ConstU16<10_000>>;

    #[pallet::type_value]
    pub fn DefaultFloorDelegationFee<T: Config>() -> Percent {
        Percent::from_percent(5)
    }

    #[pallet::storage] // --- ITEM ( floor_delegation_fee )
    pub type FloorDelegationFee<T> =
        StorageValue<_, Percent, ValueQuery, DefaultFloorDelegationFee<T>>;

    #[pallet::storage] // --- ITEM ( min_weight_stake )
    pub type MinWeightStake<T> = StorageValue<_, u64, ValueQuery>;

    #[pallet::storage] // --- ITEM ( max_allowed_weights_global )
    pub type MaxAllowedWeightsGlobal<T> = StorageValue<_, u16, ValueQuery, ConstU16<512>>;

    #[pallet::storage] // --- MAP ( netuid ) --> max_allowed_weights
    pub type MaximumSetWeightCallsPerEpoch<T: Config> = StorageMap<_, Identity, u16, u16>;

    #[pallet::storage] // DMAP ( netuid, account ) --> weight_calls
    pub type SetWeightCallsPerEpoch<T: Config> =
        StorageDoubleMap<_, Identity, u16, Identity, T::AccountId, u16, ValueQuery>;

    // ---------------------------------
    // Registrations
    // ---------------------------------

    #[pallet::storage] // --- ITEM ( registrations_this block )
    pub type RegistrationsPerBlock<T> = StorageValue<_, u16, ValueQuery>;

    #[pallet::storage] // --- ITEM ( global_max_registrations_per_block )
    pub type MaxRegistrationsPerBlock<T> = StorageValue<_, u16, ValueQuery, ConstU16<10>>;

    // ---------------------------------
    //  Module Staking Variables
    // ---------------------------------

    #[pallet::storage] // DMAP ( key, account ) --> stake
    pub type StakeFrom<T: Config> =
        StorageDoubleMap<_, Identity, T::AccountId, Identity, T::AccountId, u64, ValueQuery>;

    #[pallet::storage] // --- DMAP ( key, account ) --> stake
    pub type StakeTo<T: Config> =
        StorageDoubleMap<_, Identity, T::AccountId, Identity, T::AccountId, u64, ValueQuery>;

    #[pallet::storage] // --- ITEM  ( total_stake )
    pub type TotalStake<T> = StorageValue<_, u64, ValueQuery>;

    // ---------------------------------
    // Subnets
    // ---------------------------------

    #[pallet::storage] // --- ITEM ( subnet_gaps )
    pub type SubnetGaps<T> = StorageValue<_, BTreeSet<u16>, ValueQuery>;

    #[pallet::storage] // --- MAP ( network_name ) --> netuid
    pub type SubnetNames<T: Config> = StorageMap<_, Identity, u16, Vec<u8>, ValueQuery>;

    #[pallet::storage]
    pub type SubnetMetadata<T: Config> =
        StorageMap<_, Identity, u16, BoundedVec<u8, ConstU32<120>>>;

    #[pallet::storage] // --- ITEM ( floor_founder_share )
    pub type FloorFounderShare<T: Config> = StorageValue<_, u8, ValueQuery, ConstU8<8>>;

    #[pallet::storage] // --- MAP ( netuid ) --> subnetwork_n (Number of UIDs in the network).
    pub type N<T> = StorageMap<_, Identity, u16, u16, ValueQuery>;

    #[pallet::storage] // --- MAP ( netuid ) --> subnet_founder_key
    pub type Founder<T: Config> =
        StorageMap<_, Identity, u16, T::AccountId, ValueQuery, DefaultKey<T>>;

    #[pallet::storage] // --- DMAP ( key, netuid ) --> bool
    pub type IncentiveRatio<T: Config> =
        StorageMap<_, Identity, u16, u16, ValueQuery, ConstU16<50>>;

    // ---------------------------------
    // Subnet registration parameters
    // ---------------------------------

    #[pallet::type_value]
    pub fn SubnetBurnConfigDefault<T: Config>() -> GeneralBurnConfiguration<T> {
        GeneralBurnConfiguration::<T>::default_for(BurnType::Subnet)
    }

    #[pallet::storage]
    pub type SubnetBurnConfig<T: Config> =
        StorageValue<_, GeneralBurnConfiguration<T>, ValueQuery, SubnetBurnConfigDefault<T>>;

    #[pallet::storage] // --- MAP ( netuid ) -> module_burn_config
    pub type ModuleBurnConfig<T: Config> =
        StorageMap<_, Identity, u16, GeneralBurnConfiguration<T>, ValueQuery>;

    #[pallet::storage] // ITEM ( subnet_max_registrations_per_interval )
    pub type SubnetRegistrationsThisInterval<T: Config> = StorageValue<_, u16, ValueQuery>;

    #[pallet::storage] // --- MAP (netuid) --> registrations_this_interval
    pub type RegistrationsThisInterval<T: Config> = StorageMap<_, Identity, u16, u16, ValueQuery>;

    #[pallet::type_value]
    pub fn DefaultSubnetBurn<T: Config>() -> u64 {
        GeneralBurnConfiguration::<T>::default_for(BurnType::Subnet).min_burn
    }

    #[pallet::storage] // --- ITEM ( subnet_burn )
    pub type SubnetBurn<T: Config> = StorageValue<_, u64, ValueQuery, DefaultSubnetBurn<T>>;

    // TODO: make these a subnet params
    #[pallet::storage]
    pub type MaxEncryptionPeriod<T: Config> =
        StorageMap<_, Identity, u16, u64, ValueQuery, ConstU64<2_000>>;

    #[pallet::type_value]
    pub fn DefaultMinUnderperformanceThreshold() -> I64F64 {
        I64F64::from_num(0)
    }

    /// Allowed percentage profit margin of rationality,
    /// above full irrationality for the weight copying strategy.
    #[pallet::storage]
    pub type CopierMargin<T: Config> =
        StorageMap<_, Identity, u16, I64F64, ValueQuery, DefaultMinUnderperformanceThreshold>;

    #[pallet::storage]
    pub type UseWeightsEncrytyption<T: Config> = StorageMap<_, Identity, u16, bool, ValueQuery>;

    #[pallet::type_value]
    pub fn DefaultAlphaValues<T: Config>() -> (u16, u16) {
        (45875, 58982)
    }

    #[pallet::storage]
    pub type AlphaValues<T: Config> =
        StorageMap<_, Identity, u16, (u16, u16), ValueQuery, DefaultAlphaValues<T>>;

    #[pallet::storage]
    pub type MinValidatorStake<T: Config> =
        StorageMap<_, Identity, u16, u64, ValueQuery, T::DefaultMinValidatorStake>;

    #[pallet::storage] // --- MAP ( netuid ) --> max_allowed_uids
    pub type MaxAllowedUids<T> = StorageMap<_, Identity, u16, u16, ValueQuery, ConstU16<420>>;

    #[pallet::storage] // --- MAP ( netuid ) --> immunity_period
    pub type ImmunityPeriod<T> = StorageMap<_, Identity, u16, u16, ValueQuery, ConstU16<0>>;

    #[pallet::storage] // --- MAP ( netuid ) --> min_allowed_weights
    pub type MinAllowedWeights<T> = StorageMap<_, Identity, u16, u16, ValueQuery, ConstU16<1>>;

    #[pallet::storage] // ITEM ( minimum_allowed_stake )
    pub type MinimumAllowedStake<T> = StorageValue<_, u64, ValueQuery, ConstU64<500000000>>;

    #[pallet::storage] // --- MAP ( netuid ) --> min_allowed_weights
    pub type MaxWeightAge<T> = StorageMap<_, Identity, u16, u64, ValueQuery, ConstU64<3600>>;

    #[pallet::storage] // --- MAP ( netuid ) --> min_allowed_weights
    pub type MaxAllowedWeights<T> = StorageMap<_, Identity, u16, u16, ValueQuery, ConstU16<420>>;

    #[pallet::storage] // --- MAP ( netuid ) --> epoch
    pub type Tempo<T> = StorageMap<_, Identity, u16, u16, ValueQuery, ConstU16<100>>;

    #[pallet::storage] // --- MAP ( key, proportion )
    pub type FounderShare<T: Config> =
        StorageMap<_, Identity, u16, u16, ValueQuery, DefaultFounderShare<T>>;

    #[pallet::type_value]
    pub fn DefaultFounderShare<T: Config>() -> u16 {
        FloorFounderShare::<T>::get() as u16
    }

    // ---------------------------------
    // Module Variables
    // ---------------------------------

    #[pallet::storage] // --- DMAP ( netuid, module_key ) --> uid
    pub type Uids<T: Config> =
        StorageDoubleMap<_, Identity, u16, Blake2_128Concat, T::AccountId, u16>;

    #[pallet::type_value]
    pub fn DefaultKey<T: Config>() -> T::AccountId {
        T::AccountId::decode(&mut sp_runtime::traits::TrailingZeroInput::zeroes()).unwrap()
    }
    #[pallet::storage] // --- DMAP ( netuid, uid ) --> module_key
    pub type Keys<T: Config> = StorageDoubleMap<_, Identity, u16, Identity, u16, T::AccountId>;

    #[pallet::storage] // --- DMAP ( netuid, uid ) --> module_name
    pub type Name<T: Config> =
        StorageDoubleMap<_, Twox64Concat, u16, Twox64Concat, u16, Vec<u8>, ValueQuery>;

    #[pallet::storage] // --- DMAP ( netuid, uid ) --> module_address
    pub type Address<T: Config> =
        StorageDoubleMap<_, Twox64Concat, u16, Twox64Concat, u16, Vec<u8>, ValueQuery>;

    #[pallet::storage] // --- DMAP ( netuid, module key ) --> metadata_uri
    pub type Metadata<T: Config> =
        StorageDoubleMap<_, Twox64Concat, u16, Twox64Concat, T::AccountId, Vec<u8>>;

    // ---------------------------------
    // Module Consensus Variables
    // ---------------------------------

    #[pallet::storage] // --- MAP ( netuid ) --> incentive
    pub type Incentive<T: Config> = StorageMap<_, Identity, u16, Vec<u16>, ValueQuery>;
    #[pallet::storage] // --- MAP ( netuid ) --> trust
    pub type Trust<T: Config> = StorageMap<_, Identity, u16, Vec<u16>, ValueQuery>;
    #[pallet::storage] // --- MAP ( netuid ) --> dividends
    pub type Dividends<T: Config> = StorageMap<_, Identity, u16, Vec<u16>, ValueQuery>;
    #[pallet::storage] // --- MAP ( netuid ) --> emission
    pub type Emission<T: Config> = StorageMap<_, Identity, u16, Vec<u64>, ValueQuery>;
    #[pallet::storage] // --- MAP ( netuid ) --> last_update
    pub type LastUpdate<T: Config> = StorageMap<_, Identity, u16, Vec<u64>, ValueQuery>;

    #[pallet::storage] // ITEM ( max_allowed_weights_global )
    pub type SubnetImmunityPeriod<T: Config> = StorageValue<_, u64, ValueQuery, ConstU64<32400>>;

    #[pallet::storage] // --- MAP ( netuid, uid ) --> block number that the module is registered
    pub type SubnetRegistrationBlock<T: Config> = StorageMap<_, Identity, u16, u64>;

    #[pallet::storage] // --- DMAP ( netuid, uid ) --> block number that the module is registered
    pub type RegistrationBlock<T: Config> =
        StorageDoubleMap<_, Identity, u16, Identity, u16, u64, ValueQuery>;

    #[pallet::storage]
    pub type WeightSetAt<T: Config> = StorageDoubleMap<_, Identity, u16, Identity, u16, u64>;

    #[pallet::type_value]
    pub fn DefaultDelegationFee<T: Config>() -> Percent {
        Percent::from_percent(5u8)
    }

    #[pallet::storage] // -- DMAP (netuid, module_key) -> delegation_fee
    pub type DelegationFee<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, Percent, ValueQuery, DefaultDelegationFee<T>>;

    #[pallet::storage] // MAP (netuid, module_key) -> control_delegation
    pub type RootnetControlDelegation<T: Config> =
        StorageMap<_, Identity, T::AccountId, T::AccountId>;
}
impl<T: Config> Pallet<T> {
    /// Returns the total amount staked by the given key to other keys.
    #[inline]
    pub fn get_owned_stake(staker: &T::AccountId) -> u64 {
        StakeTo::<T>::iter_prefix_values(staker).sum()
    }

    /// Returns the total amount staked into the given key by other keys.
    #[inline]
    pub fn get_delegated_stake(staked: &T::AccountId) -> u64 {
        StakeFrom::<T>::iter_prefix_values(staked).sum()
    }
}
