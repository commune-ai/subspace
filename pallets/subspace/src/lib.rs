#![allow(deprecated, non_camel_case_types, non_snake_case)]
#![cfg_attr(not(feature = "std"), no_std)]
#![recursion_limit = "512"]

use crate::subnet::SubnetChangeset;
use frame_system::{self as system, ensure_signed};
pub use pallet::*;
use scale_info::TypeInfo;
use sp_std::collections::{btree_map::BTreeMap, btree_set::BTreeSet};
// export the migrations here
pub mod migrations;

use frame_support::{
    dispatch,
    dispatch::{DispatchInfo, PostDispatchInfo},
    ensure,
    traits::{tokens::WithdrawReasons, Currency, ExistenceRequirement, IsSubType},
};

use codec::{Decode, Encode};
use frame_support::sp_runtime::transaction_validity::ValidTransaction;
use sp_runtime::{
    traits::{DispatchInfoOf, Dispatchable, PostDispatchInfoOf, SignedExtension},
    transaction_validity::{TransactionValidity, TransactionValidityError},
};
use sp_std::marker::PhantomData;

pub mod autogen_weights;
pub use autogen_weights::WeightInfo;

#[cfg(test)]
mod mock;

#[cfg(debug_assertions)]
pub use step::yuma;

// =========================
//	==== Pallet Imports =====
// =========================
mod global;
mod math;
pub mod module;
mod profit_share;
mod registration;
mod staking;
mod step;
pub mod subnet;
pub mod voting;
mod weights;

// TODO: better error handling in whole file

#[frame_support::pallet]
pub mod pallet {
    #![allow(
        deprecated,
        clippy::let_unit_value,
        clippy::too_many_arguments,
        clippy::type_complexity
    )]

    use self::voting::{Proposal, VoteMode};

    use super::*;
    use frame_support::{pallet_prelude::*, traits::Currency};
    use frame_system::pallet_prelude::*;

    use module::ModuleChangeset;
    use sp_arithmetic::per_things::Percent;
    pub use sp_std::{vec, vec::Vec};

    const STORAGE_VERSION: StorageVersion = StorageVersion::new(4);

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    #[pallet::storage_version(STORAGE_VERSION)]
    #[pallet::without_storage_info]
    pub struct Pallet<T>(_);

    // Configure the pallet by specifying the parameters and types on which it depends.
    #[pallet::config]
    pub trait Config: frame_system::Config {
        // Because this pallet emits events, it depends on the runtime's definition of an event.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        // --- Currency type that will be used to place deposits on modules
        type Currency: Currency<Self::AccountId> + Send + Sync;

        /// Type representing the weight of this pallet
        type WeightInfo: WeightInfo;
    }

    pub type AccountIdOf<T> = <T as frame_system::Config>::AccountId;

    // ============================
    // ==== Global Variables ====
    // ============================
    #[pallet::type_value]
    pub fn DefaultUnitEmission<T: Config>() -> u64 {
        23148148148
    }
    #[pallet::storage] // --- ITEM ( unit_emission )
    pub(super) type UnitEmission<T> = StorageValue<_, u64, ValueQuery, DefaultUnitEmission<T>>;

    #[pallet::storage] // --- MAP ( netuid ) --> min_allowed_weights
    pub type BurnRate<T> = StorageValue<_, u16, ValueQuery>;

    #[pallet::type_value]
    pub fn DefaultMinBurn<T: Config>() -> u64 {
        4_000_000_000 // 4 $COMAI
    }
    #[pallet::storage] // --- MinBurn
    pub type MinBurn<T> = StorageValue<_, u64, ValueQuery, DefaultMinBurn<T>>;

    #[pallet::type_value]
    pub fn DefaultMaxBurn<T: Config>() -> u64 {
        250_000_000_000 // 250 $COMAI
    }

    #[pallet::type_value]
    pub fn DefaultAdjustmentAlpha<T: Config>() -> u64 {
        u64::MAX / 2
    }

    #[pallet::storage] // --- adjusment alpha
    pub type AdjustmentAlpha<T> = StorageValue<_, u64, ValueQuery, DefaultAdjustmentAlpha<T>>;

    #[pallet::storage] // --- MaxBurn
    pub type MaxBurn<T> = StorageValue<_, u64, ValueQuery, DefaultMaxBurn<T>>;

    #[pallet::type_value]
    pub fn DefaultSubnetStakeThreshold<T: Config>() -> Percent {
        Percent::from_percent(5)
    }

    #[pallet::storage]
    pub type SubnetStakeThreshold<T> =
        StorageValue<_, Percent, ValueQuery, DefaultSubnetStakeThreshold<T>>;

    #[pallet::type_value]
    pub fn DefaultKappa<T: Config>() -> u16 {
        32_767 // This coresponds to 0,5 (majority of stake agreement)
    }

    #[pallet::storage]
    pub type Kappa<T> = StorageValue<_, u16, ValueQuery, DefaultKappa<T>>;

    #[pallet::storage] // --- DMAP ( netuid, uid ) --> bonds
    pub type Bonds<T: Config> =
        StorageDoubleMap<_, Identity, u16, Identity, u16, Vec<(u16, u16)>, ValueQuery>;

    #[pallet::type_value]
    pub fn DefaultBondsMovingAverage<T: Config>() -> u64 {
        900_000
    }

    #[pallet::storage] // --- MAP ( netuid ) --> bonds_moving_average
    pub type BondsMovingAverage<T> =
        StorageMap<_, Identity, u16, u64, ValueQuery, DefaultBondsMovingAverage<T>>;

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

    #[pallet::storage] // --- DMAP ( netuid ) --> consensus
    pub type Consensus<T: Config> = StorageMap<_, Identity, u16, Vec<u16>, ValueQuery>;

    #[pallet::storage] // --- DMAP ( netuid ) --> active
    pub type Active<T: Config> = StorageMap<_, Identity, u16, Vec<bool>, ValueQuery>;

    #[pallet::storage] // --- DMAP ( netuid ) --> rank
    pub type Rank<T: Config> = StorageMap<_, Identity, u16, Vec<u16>, ValueQuery>;

    #[pallet::type_value]
    pub fn DefaultMaxNameLength<T: Config>() -> u16 {
        32
    }
    #[pallet::storage] // --- ITEM ( max_name_length )
    pub(super) type MaxNameLength<T: Config> =
        StorageValue<_, u16, ValueQuery, DefaultMaxNameLength<T>>;

    #[pallet::type_value]
    pub fn DefaultMinNameLength<T: Config>() -> u16 {
        2
    }

    #[pallet::storage]
    pub(super) type MinNameLength<T: Config> =
        StorageValue<_, u16, ValueQuery, DefaultMinNameLength<T>>;

    #[pallet::type_value]
    pub fn DefaultMaxAllowedSubnets<T: Config>() -> u16 {
        256
    }
    #[pallet::storage] // --- ITEM ( max_allowed_subnets )
    pub(super) type MaxAllowedSubnets<T: Config> =
        StorageValue<_, u16, ValueQuery, DefaultMaxAllowedSubnets<T>>;

    #[pallet::storage]
    // --- MAP (netuid) --> registrations_this_interval
    pub(super) type RegistrationsThisInterval<T: Config> =
        StorageMap<_, Identity, u16, u16, ValueQuery>;

    #[pallet::storage]
    // --- MAP (netuid) --> burn
    pub type Burn<T: Config> = StorageMap<_, Identity, u16, u64, ValueQuery>;

    #[pallet::type_value]
    pub fn DefaultMaxAllowedModules<T: Config>() -> u16 {
        10_000
    }
    #[pallet::storage] // --- ITEM ( max_allowed_modules )
    pub type MaxAllowedModules<T: Config> =
        StorageValue<_, u16, ValueQuery, DefaultMaxAllowedModules<T>>;

    #[pallet::storage] // --- ITEM ( registrations_this block )
    pub type RegistrationsPerBlock<T> = StorageValue<_, u16, ValueQuery>;

    #[pallet::type_value]
    pub fn DefaultMaxRegistrationsPerBlock<T: Config>() -> u16 {
        10
    }
    #[pallet::storage] // --- ITEM( global_max_registrations_per_block )
    pub type MaxRegistrationsPerBlock<T> =
        StorageValue<_, u16, ValueQuery, DefaultMaxRegistrationsPerBlock<T>>;

    #[pallet::type_value]
    pub fn DefaultTargetRegistrationsPerInterval<T: Config>() -> u16 {
        DefaultTargetRegistrationsInterval::<T>::get() / 2
    }
    #[pallet::storage] // --- ITEM( global_target_registrations_interval )
    pub type TargetRegistrationsPerInterval<T> =
        StorageValue<_, u16, ValueQuery, DefaultTargetRegistrationsPerInterval<T>>;

    #[pallet::type_value] // --- ITEM( global_target_registrations_interval ) Measured in the number of blocks
    pub fn DefaultTargetRegistrationsInterval<T: Config>() -> u16 {
        DefaultTempo::<T>::get() * 2 // 2 times the epoch
    }
    #[pallet::storage] // --- ITEM( global_target_registrations_interval )
    pub type TargetRegistrationsInterval<T> =
        StorageValue<_, u16, ValueQuery, DefaultTargetRegistrationsInterval<T>>;

    #[pallet::storage] // --- MAP ( netuid ) --> min_allowed_weights
    pub type MinStakeGlobal<T> = StorageValue<_, u64, ValueQuery>;

    #[pallet::type_value]
    pub fn DefaultMinDelegationFeeGlobal<T: Config>() -> Percent {
        Percent::from_percent(5u8)
    }

    #[pallet::storage]
    pub type FloorDelegationFee<T> =
        StorageValue<_, Percent, ValueQuery, DefaultMinDelegationFeeGlobal<T>>;

    #[pallet::storage] // --- MAP ( netuid ) --> min_allowed_weights
    pub type MinWeightStake<T> = StorageValue<_, u64, ValueQuery>;

    #[pallet::type_value]
    pub fn DefaultMaxAllowedWeightsGlobal<T: Config>() -> u16 {
        512
    }
    #[pallet::storage] // --- MAP ( netuid ) --> min_allowed_weights
    pub type MaxAllowedWeightsGlobal<T> =
        StorageValue<_, u16, ValueQuery, DefaultMaxAllowedWeightsGlobal<T>>;

    #[derive(Decode, Encode, PartialEq, Eq, Clone, Debug, TypeInfo)]
    #[scale_info(skip_type_params(T))]
    pub struct ModuleParams<T: Config> {
        pub name: Vec<u8>,
        pub address: Vec<u8>,
        pub delegation_fee: Percent,
        pub metadata: Option<Vec<u8>>,
        pub controller: T::AccountId,
    }

    #[derive(Decode, Encode, PartialEq, Eq, Clone, TypeInfo)]
    #[scale_info(skip_type_params(T))]
    pub struct GlobalParams<T: Config> {
        pub burn_rate: u16,
        // max
        pub max_name_length: u16,             // max length of a network name
        pub min_name_length: u16,             // min length of a network name
        pub max_allowed_subnets: u16,         // max number of subnets allowed
        pub max_allowed_modules: u16,         // max number of modules allowed per subnet
        pub max_registrations_per_block: u16, // max number of registrations per block
        pub max_allowed_weights: u16,         // max number of weights per module

        // mins
        pub min_burn: u64,                 // min burn required
        pub max_burn: u64,                 // max burn allowed
        pub min_stake: u64,                // min stake required
        pub floor_delegation_fee: Percent, // min delegation fee
        pub min_weight_stake: u64,         // min weight stake required

        // other
        pub target_registrations_per_interval: u16, // desired number of registrations per interval
        pub target_registrations_interval: u16,     /* the number of blocks that defines the
                                                     * registration interval */
        pub adjustment_alpha: u64, // adjustment alpha
        pub unit_emission: u64,    // emission per block
        pub nominator: T::AccountId,

        pub subnet_stake_threshold: Percent,

        // porposals
        pub proposal_cost: u64,
        pub proposal_expiration: u32,
        pub proposal_participation_threshold: Percent,
    }

    impl<T: Config> core::fmt::Debug for GlobalParams<T>
    where
        T::AccountId: core::fmt::Debug,
    {
        fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
            f.debug_struct("GlobalParams")
                .field("burn_rate", &self.burn_rate)
                .field("max_name_length", &self.max_name_length)
                .field("max_allowed_subnets", &self.max_allowed_subnets)
                .field("max_allowed_modules", &self.max_allowed_modules)
                .field(
                    "max_registrations_per_block",
                    &self.max_registrations_per_block,
                )
                .field("max_allowed_weights", &self.max_allowed_weights)
                .field("min_burn", &self.min_burn)
                .field("max_burn", &self.max_burn)
                .field("min_stake", &self.min_stake)
                .field("floor_delegation_fee", &self.floor_delegation_fee)
                .field("min_weight_stake", &self.min_weight_stake)
                .field("subnet_stake_threshold", &self.subnet_stake_threshold)
                .field(
                    "target_registrations_per_interval",
                    &self.target_registrations_per_interval,
                )
                .field(
                    "target_registrations_interval",
                    &self.target_registrations_interval,
                )
                .field("adjustment_alpha", &self.adjustment_alpha)
                .field("unit_emission", &self.unit_emission)
                .field("nominator", &self.nominator)
                .finish()
        }
    }

    pub struct DefaultSubnetParams<T: Config>(sp_std::marker::PhantomData<((), T)>);

    impl<T: Config> DefaultSubnetParams<T> {
        pub fn get() -> SubnetParams<T> {
            SubnetParams {
                name: vec![],
                tempo: DefaultTempo::<T>::get(),
                immunity_period: DefaultImmunityPeriod::<T>::get(),
                min_allowed_weights: DefaultMinAllowedWeights::<T>::get(),
                max_allowed_weights: DefaultMaxAllowedWeights::<T>::get(),
                max_allowed_uids: DefaultMaxAllowedUids::<T>::get(),
                max_weight_age: DefaultMaxWeightAge::<T>::get(),
                max_stake: DefaultMaxStake::<T>::get(),
                trust_ratio: GetDefault::get(),
                founder_share: GetDefault::get(),
                incentive_ratio: DefaultIncentiveRatio::<T>::get(),
                min_stake: MinStakeGlobal::<T>::get(),
                founder: DefaultFounder::<T>::get(),
                vote_mode: DefaultVoteMode::<T>::get(),
            }
        }
    }

    // =========================
    // ==== Subnet PARAMS ====
    // =========================

    #[derive(Decode, Encode, PartialEq, Eq, Clone, Debug, TypeInfo)]
    #[scale_info(skip_type_params(T))]
    pub struct SubnetParams<T: Config> {
        // --- parameters
        pub founder: T::AccountId,
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
        pub name: Vec<u8>,
        pub tempo: u16, // how many blocks to wait before rewarding models
        pub trust_ratio: u16,
        pub vote_mode: VoteMode,
    }

    #[pallet::type_value]
    pub fn DefaultMaxAllowedUids<T: Config>() -> u16 {
        820
    }
    #[pallet::storage] // --- MAP ( netuid ) --> max_allowed_uids
    pub type MaxAllowedUids<T> =
        StorageMap<_, Identity, u16, u16, ValueQuery, DefaultMaxAllowedUids<T>>;

    #[pallet::type_value]
    pub fn DefaultImmunityPeriod<T: Config>() -> u16 {
        40
    }
    #[pallet::storage] // --- MAP ( netuid ) --> immunity_period
    pub type ImmunityPeriod<T> =
        StorageMap<_, Identity, u16, u16, ValueQuery, DefaultImmunityPeriod<T>>;

    #[pallet::type_value]
    pub fn DefaultMinAllowedWeights<T: Config>() -> u16 {
        1
    }
    #[pallet::storage] // --- MAP ( netuid ) --> min_allowed_weights
    pub type MinAllowedWeights<T> =
        StorageMap<_, Identity, u16, u16, ValueQuery, DefaultMinAllowedWeights<T>>;

    #[pallet::type_value]
    pub fn DefaultSelfVote<T: Config>() -> bool {
        true
    }
    #[pallet::storage] // --- MAP ( netuid ) --> min_allowed_weights
    pub type SelfVote<T> = StorageMap<_, Identity, u16, bool, ValueQuery, DefaultSelfVote<T>>;

    #[pallet::storage] // --- MAP ( netuid ) --> min_allowed_weights
    pub type MinStake<T> = StorageMap<_, Identity, u16, u64, ValueQuery>;

    #[pallet::type_value]
    pub fn DefaultMaxStake<T: Config>() -> u64 {
        u64::MAX
    }
    #[pallet::storage] // --- MAP ( netuid ) --> min_allowed_weights
    pub type MaxStake<T> = StorageMap<_, Identity, u16, u64, ValueQuery, DefaultMaxStake<T>>;

    #[pallet::type_value]
    pub fn DefaultMaxWeightAge<T: Config>() -> u64 {
        3600 // 3.6k blocks, that is 8 hours
    }
    #[pallet::storage] // --- MAP ( netuid ) --> min_allowed_weights
    pub type MaxWeightAge<T> =
        StorageMap<_, Identity, u16, u64, ValueQuery, DefaultMaxWeightAge<T>>;

    #[pallet::type_value]
    pub fn DefaultMaxAllowedWeights<T: Config>() -> u16 {
        420
    }
    #[pallet::storage] // --- MAP ( netuid ) --> min_allowed_weights
    pub type MaxAllowedWeights<T> =
        StorageMap<_, Identity, u16, u16, ValueQuery, DefaultMaxAllowedWeights<T>>;

    #[pallet::type_value]
    pub fn DefaultFounder<T: Config>() -> T::AccountId {
        T::AccountId::decode(&mut sp_runtime::traits::TrailingZeroInput::zeroes()).unwrap()
    }
    #[pallet::storage] // --- DMAP ( key, netuid ) --> bool
    pub type Founder<T: Config> =
        StorageMap<_, Identity, u16, T::AccountId, ValueQuery, DefaultFounder<T>>;

    #[pallet::storage] // --- DMAP ( key, netuid ) --> bool
    pub type FounderShare<T: Config> = StorageMap<_, Identity, u16, u16, ValueQuery>;

    #[pallet::type_value]
    pub fn DefaultIncentiveRatio<T: Config>() -> u16 {
        50
    }
    #[pallet::storage] // --- DMAP ( key, netuid ) --> bool
    pub type IncentiveRatio<T: Config> =
        StorageMap<_, Identity, u16, u16, ValueQuery, DefaultIncentiveRatio<T>>;

    #[pallet::type_value]
    pub fn DefaultTempo<T: Config>() -> u16 {
        100
    }
    #[pallet::storage] // --- MAP ( netuid ) --> epoch
    pub type Tempo<T> = StorageMap<_, Identity, u16, u16, ValueQuery, DefaultTempo<T>>;

    #[pallet::storage] // --- MAP ( netuid ) --> epoch
    pub type TrustRatio<T> = StorageMap<_, Identity, u16, u16, ValueQuery>;

    // =======================================
    // ==== Voting  ====
    // =======================================

    #[pallet::type_value]
    pub fn DefaultNominator<T: Config>() -> T::AccountId {
        T::AccountId::decode(&mut sp_runtime::traits::TrailingZeroInput::zeroes()).unwrap()
    }

    #[pallet::storage]
    pub type Nominator<T: Config> = StorageValue<_, T::AccountId, ValueQuery, DefaultNominator<T>>;

    // VOTING MODE
    #[pallet::type_value]
    pub fn DefaultVoteMode<T: Config>() -> VoteMode {
        VoteMode::Authority
    }

    #[pallet::storage] // --- MAP ( netuid ) --> epoch
    pub type VoteModeSubnet<T> =
        StorageMap<_, Identity, u16, VoteMode, ValueQuery, DefaultVoteMode<T>>;

    #[derive(Decode, Encode, PartialEq, Eq, Clone, Debug, TypeInfo)]
    pub struct SubnetInfo<T: Config> {
        // --- parameters
        pub params: SubnetParams<T>,
        pub netuid: u16, // --- unique id of the network
        pub n: u16,
        pub stake: u64,
        pub emission: u64,
        pub founder: T::AccountId,
    }

    #[pallet::storage] // --- ITEM( tota_number_of_existing_networks )
    pub type TotalSubnets<T> = StorageValue<_, u16, ValueQuery>;

    #[pallet::storage] // --- MAP( netuid ) --> subnet_emission
    pub type SubnetEmission<T> = StorageMap<_, Identity, u16, u64, ValueQuery>;

    #[pallet::storage] // --- MAP ( netuid ) --> subnetwork_n (Number of UIDs in the network).
    pub type N<T: Config> = StorageMap<_, Identity, u16, u16, ValueQuery>;

    #[pallet::storage] // --- MAP ( netuid ) --> pending_emission
    pub type PendingEmission<T> = StorageMap<_, Identity, u16, u64, ValueQuery>;

    #[pallet::storage] // --- MAP ( network_name ) --> netuid
    pub type SubnetNames<T: Config> = StorageMap<_, Identity, u16, Vec<u8>, ValueQuery>;

    // =======================================
    // ==== Module Variables  ====
    // =======================================

    #[pallet::storage] // --- DMAP ( netuid, module_key ) --> uid
    pub type Uids<T: Config> =
        StorageDoubleMap<_, Identity, u16, Blake2_128Concat, T::AccountId, u16, OptionQuery>;

    #[pallet::type_value]
    pub fn DefaultKey<T: Config>() -> T::AccountId {
        T::AccountId::decode(&mut sp_runtime::traits::TrailingZeroInput::zeroes()).unwrap()
    }
    #[pallet::storage] // --- DMAP ( netuid, uid ) --> module_key
    pub(super) type Keys<T: Config> =
        StorageDoubleMap<_, Identity, u16, Identity, u16, T::AccountId, ValueQuery, DefaultKey<T>>;

    #[pallet::storage] // --- DMAP ( netuid, uid ) --> module_name
    pub type Name<T: Config> =
        StorageDoubleMap<_, Twox64Concat, u16, Twox64Concat, u16, Vec<u8>, ValueQuery>;

    #[pallet::storage] // --- DMAP ( netuid, uid ) --> module_address
    pub type Address<T: Config> =
        StorageDoubleMap<_, Twox64Concat, u16, Twox64Concat, u16, Vec<u8>, ValueQuery>;

    #[pallet::storage] // --- DMAP ( netuid, module key ) --> metadata_uri
    pub type Metadata<T: Config> =
        StorageDoubleMap<_, Twox64Concat, u16, Twox64Concat, T::AccountId, Vec<u8>>;

    #[pallet::type_value]
    pub fn DefaultDelegationFee<T: Config>() -> Percent {
        Percent::from_percent(20u8)
    }
    #[pallet::storage] // -- DMAP(netuid, module_key) -> delegation_fee
    pub(super) type DelegationFee<T: Config> = StorageDoubleMap<
        _,
        Identity,
        u16,
        Blake2_128Concat,
        T::AccountId,
        Percent,
        ValueQuery,
        DefaultDelegationFee<T>,
    >;

    // STATE OF THE MODULE
    #[pallet::storage] // --- DMAP ( netuid, uid ) --> block number that the module is registered
    pub type RegistrationBlock<T: Config> =
        StorageDoubleMap<_, Identity, u16, Identity, u16, u64, ValueQuery>;

    // =======================================
    // ==== Module Staking Variables  ====
    // =======================================

    #[pallet::storage] // --- DMAP ( netuid, module_key ) --> stake | Returns the stake under a module.
    pub type Stake<T: Config> =
        StorageDoubleMap<_, Identity, u16, Identity, T::AccountId, u64, ValueQuery>;

    #[pallet::storage] // --- DMAP ( netuid, module_key ) --> Vec<(delegater, stake )> | Returns the list of delegates
                       // and their staked amount under a module
    pub type StakeFrom<T: Config> = StorageDoubleMap<
        _,
        Identity,
        u16,
        Identity,
        T::AccountId,
        BTreeMap<T::AccountId, u64>,
        ValueQuery,
    >;

    #[pallet::storage] // --- DMAP ( netuid, account_id ) --> Vec<(module_key, stake )> | Returns the list of the
    pub type StakeTo<T: Config> = StorageDoubleMap<
        _,
        Identity,
        u16,
        Identity,
        T::AccountId,
        BTreeMap<T::AccountId, u64>,
        ValueQuery,
    >;

    #[pallet::storage] // --- MAP( netuid ) --> lowest_subnet
    pub type RemovedSubnets<T> = StorageValue<_, BTreeSet<u16>, ValueQuery>;

    // TOTAL STAKE PER SUBNET
    #[pallet::storage] // --- MAP ( netuid ) --> subnet_total_stake
    pub type TotalStake<T> = StorageMap<_, Identity, u16, u64, ValueQuery>;

    // PROFIT SHARE VARIABLES
    #[pallet::storage] // --- DMAP ( netuid, account_id ) --> Vec<(module_key, stake )> | Returns the list of the
    pub type ProfitShares<T: Config> =
        StorageMap<_, Identity, T::AccountId, Vec<(T::AccountId, u16)>, ValueQuery>;

    #[pallet::type_value]
    pub fn DefaultProfitShareUnit<T: Config>() -> u16 {
        u16::MAX
    }
    #[pallet::storage] // --- DMAP ( netuid, account_id ) --> Vec<(module_key, stake )> | Returns the list of the
    pub type ProfitShareUnit<T: Config> =
        StorageValue<_, u16, ValueQuery, DefaultProfitShareUnit<T>>;

    // =======================================
    // ==== Module Consensus Variables  ====
    // =======================================
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

    #[pallet::storage] // --- DMAP ( netuid, uid ) --> weights
    pub type Weights<T: Config> =
        StorageDoubleMap<_, Identity, u16, Identity, u16, Vec<(u16, u16)>, ValueQuery>;

    // whitelist for the base subnet (netuid 0)
    #[pallet::storage]
    pub type LegitWhitelist<T: Config> = StorageMap<_, Identity, T::AccountId, u8, ValueQuery>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        NetworkAdded(u16, Vec<u8>), // --- Event created when a new network is added.
        NetworkRemoved(u16),        // --- Event created when a network is removed.
        StakeAdded(T::AccountId, T::AccountId, u64), /* --- Event created when stake has been
                                     * transfered from the a coldkey account
                                     * onto the key staking account. */
        StakeRemoved(T::AccountId, T::AccountId, u64), /* --- Event created when stake has been
                                                        * removed from the key staking account
                                                        * onto the coldkey account. */
        WeightsSet(u16, u16), /* ---- Event created when a caller successfully sets their
                               * weights on a subnetwork. */
        ModuleRegistered(u16, u16, T::AccountId), /* --- Event created when a new module
                                                   * account has been registered to the chain. */
        ModuleDeregistered(u16, u16, T::AccountId), /* --- Event created when a module account
                                                     * has been deregistered from the chain. */
        WhitelistModuleAdded(T::AccountId), /* --- Event created when a module account has been
                                             * added to the whitelist. */
        WhitelistModuleRemoved(T::AccountId), /* --- Event created when a module account has
                                               * been removed from the whitelist. */
        BulkModulesRegistered(u16, u16), /* --- Event created when multiple uids have been
                                          * concurrently registered. */
        BulkBalancesSet(u16, u16),
        MaxAllowedUidsSet(u16, u16), /* --- Event created when max allowed uids has been set
                                      * for a subnetwor. */
        MinAllowedWeightSet(u16, u16), /* --- Event created when minimun allowed weight is set
                                        * for a subnet. */
        ImmunityPeriodSet(u16, u16), /* --- Event created when immunity period is set for a
                                      * subnet. */
        ModuleUpdated(u16, T::AccountId), /* --- Event created when the module server
                                           * information is added to the network. */
        DelegateAdded(T::AccountId, T::AccountId, u16), /* --- Event created to signal a key
                                                         * has become a delegate. */
        UnitEmissionSet(u64), // --- Event created when setting the unit emission
        MaxNameLengthSet(u16), // --- Event created when setting the maximum network name length
        MinNameLenghtSet(u16), // --- Event created when setting the minimum network name length
        MaxAllowedSubnetsSet(u16), // --- Event created when setting the maximum allowed subnets
        MaxAllowedModulesSet(u16), // --- Event created when setting the maximum allowed modules
        MaxRegistrationsPerBlockSet(u16), // --- Event created when we set max registrations
        target_registrations_intervalSet(u16), // --- Event created when we set target registrations
        RegistrationBurnChanged(u64),

        //voting
        ProposalCreated(u64),                        // id of the proposal
        ProposalVoted(u64, T::AccountId, bool),      // (id, voter, vote)
        ProposalVoteUnregistered(u64, T::AccountId), // (id, voter)
        GlobalParamsUpdated(GlobalParams<T>),        /* --- Event created when global
                                                      * parameters are
                                                      * updated */
        SubnetParamsUpdated(u16), // --- Event created when subnet parameters are updated
        GlobalProposalAccepted(u64), // (id)
        CustomProposalAccepted(u64), // (id)
        SubnetProposalAccepted(u64, u16), // (id, netuid)
    }

    // Errors inform users that something went wrong.
    #[pallet::error]
    pub enum Error<T> {
        NetworkDoesNotExist, // --- Thrown when the network does not exist.
        TooFewVotesForNewProposal,
        NetworkExist, // --- Thrown when the network already exist.
        InvalidIpType, /* ---- Thrown when the user tries to serve an module which
                       * is not of type	4 (IPv4) or 6 (IPv6). */
        NotRegistered, // module which does not exist in the active set.
        NotEnoughStakeToWithdraw, /* ---- Thrown when the caller requests removing more stake
                        * then there exists in the staking account. See: fn
                        * remove_stake. */
        NotEnoughBalanceToStake, /*  ---- Thrown when the caller requests adding more stake
                                  * than there exists in the cold key account. See: fn
                                  * add_stake */
        BalanceWithdrawalError, /* ---- Thrown when the caller tries to add stake, but for some
                                 * reason the requested amount could not be withdrawn from the
                                 * coldkey account */
        WeightVecNotEqualSize, /* ---- Thrown when the caller attempts to set the weight keys
                                * and values but these vectors have different size. */
        DuplicateUids, /* ---- Thrown when the caller attempts to set weights with duplicate
                        * uids in the weight matrix. */
        InvalidUid, /* ---- Thrown when a caller attempts to set weight to at least one uid
                     * that does not exist in the metagraph. */
        InvalidUidsLength, /* ---- Thrown when the caller attempts to set weights with a
                            * different number of uids than allowed. */
        NotSettingEnoughWeights, /* ---- Thrown when the dispatch attempts to set weights on
                                  * chain with fewer elements than are allowed. */
        TooManyRegistrationsPerBlock, /* ---- Thrown when registrations this block exceeds
                                       * allowed number. */
        AlreadyRegistered, /* ---- Thrown when the caller requests registering a module which
                            * already exists in the active set. */
        MaxAllowedUIdsNotAllowed, // ---  Thrown if the vaule is invalid for MaxAllowedUids
        CouldNotConvertToBalance, /* ---- Thrown when the dispatch attempts to convert between
                                   * a u64 and T::balance but the call fails. */
        StakeAlreadyAdded, /* --- Thrown when the caller requests adding stake for a key to the
                            * total stake which already added */
        StorageValueOutOfRange, /* --- Thrown when the caller attempts to set a storage value
                                 * outside of its allowed range. */
        TempoHasNotSet, // --- Thrown when epoch has not set
        InvalidTempo,   // --- Thrown when epoch is not valid
        SettingWeightsTooFast, /* --- Thrown if the key attempts to set weights twice withing
                         * net_epoch/2 blocks. */
        BalanceSetError, // --- Thrown when an error occurs setting a balance
        MaxAllowedUidsExceeded, /* --- Thrown when number of accounts going to be registered
                          * exceed MaxAllowedUids for the network. */
        TooManyUids, /* ---- Thrown when the caller attempts to set weights with more uids than
                      * are allowed. */
        InvalidMaxAllowedUids, /* --- Thrown when the user tries to set max allowed uids to a
                                * value less than the current number of registered uids. */
        NetuidDoesNotExist,
        SubnetNameAlreadyExists,
        MissingSubnetName,
        SubnetNameTooShort,
        SubnetNameTooLong,
        InvalidSubnetName,
        BalanceNotAdded,
        StakeNotRemoved,
        KeyAlreadyRegistered,
        EmptyKeys,
        TooManyKeys,
        NotNominator, /* --- Thrown when the user tries to set the nominator and is not the
                       * nominator */
        AlreadyWhitelisted, /* --- Thrown when the user tries to whitelist an account that is
                             * already whitelisted. */
        NotWhitelisted, /* --- Thrown when the user tries to remove an account from the
                         * whitelist that is not whitelisted. */
        InvalidShares,
        ProfitSharesNotAdded,
        NotFounder,
        NameAlreadyRegistered,
        NotEnoughStakeToSetWeights,
        NotEnoughStakeToStartNetwork,
        NotEnoughStakePerWeight,
        NoSelfWeight,
        DifferentLengths,
        NotEnoughBalanceToRegister,
        StakeNotAdded,
        BalanceNotRemoved,
        BalanceCouldNotBeRemoved,
        NotEnoughStakeToRegister,
        StillRegistered,
        MaxAllowedModules, /* --- Thrown when the user tries to set max allowed modules to a
                            * value less than the current number of registered modules. */
        NotEnoughBalanceToTransfer,
        NotVoteMode,
        InvalidTrustRatio,
        InvalidMinAllowedWeights,
        InvalidMaxAllowedWeights,
        InvalidMinStake,
        InvalidMinDelegationFee,
        InvalidSubnetStakeThreshold,
        InvalidModuleMetadata,
        ModuleMetadataTooLong,

        InvalidMaxNameLength,
        InvalidMinNameLenght,
        InvalidMaxAllowedSubnets,
        InvalidMaxAllowedModules,
        InvalidMaxRegistrationsPerBlock,
        InvalidTargetRegistrationsInterval,
        InvalidVoteThreshold,
        InvalidUnitEmission,
        InvalidBurnRate,
        InvalidMinBurn,
        InvalidMaxBurn,
        InvalidTargetRegistrationsPerInterval,

        // Modules
        /// The module name is too long.
        ModuleNameTooLong,
        ModuleNameTooShort,
        /// The module name is invalid. It has to be a UTF-8 encoded string.
        InvalidModuleName,
        /// The address is too long.
        ModuleAddressTooLong,
        /// The module address is invalid.
        InvalidModuleAddress,
        /// The module name does not exist in the subnet.
        ModuleNameDoesNotExist,
        /// A module with this name already exists in the subnet.
        ModuleNameAlreadyExists,
        /// A module with this name already exists in the subnet.
        // VOTING
        ProposalNotFound,
        InvalidProposalStatus,
        InvalidProposalData,
        AlreadyVoted,
        InvalidVoteMode,
        InvalidImmunityPeriod,
        InvalidFounderShare,
        InvalidIncentiveRatio,

        InvalidProposalCost,
        InvalidProposalExpiration,
        InvalidProposalParticipationThreshold,
        InsufficientStake,
        VoteNotFound,
        InvalidProposalCustomData,
        ProposalCustomDataTooSmall,
        ProposalCustomDataTooLarge,
        NotEnoughBalanceToPropose,

        // Other
        InvalidMaxWeightAge,
        InvalidRecommendedWeight,
        InvalidMaxStake,
        ArithmeticError,
    }

    // ==================
    // ==== Genesis =====
    // ==================

    #[derive(frame_support::DefaultNoBound)]
    #[pallet::genesis_config]
    pub struct GenesisConfig<T: Config> {
        // key, name, address, weights
        pub modules: Vec<Vec<(T::AccountId, Vec<u8>, Vec<u8>, Vec<(u16, u16)>)>>,
        // name, tempo, immunity_period, min_allowed_weight, max_allowed_weight, max_allowed_uids,
        // immunity_ratio, founder
        pub subnets: Vec<(Vec<u8>, u16, u16, u16, u16, u16, u16, u64, T::AccountId)>,

        pub stake_to: Vec<Vec<(T::AccountId, Vec<(T::AccountId, u64)>)>>,

        pub block: u32,
    }

    #[pallet::genesis_build]
    impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
        fn build(&self) {
            // Set initial total issuance from balances
            // Subnet config values

            for (subnet_idx, subnet) in self.subnets.iter().enumerate() {
                let netuid: u16 = subnet_idx as u16;
                // --- Set subnet parameters

                let params: SubnetParams<T> = SubnetParams {
                    name: subnet.0.clone(),
                    tempo: subnet.1,
                    immunity_period: subnet.2,
                    min_allowed_weights: subnet.3,
                    max_allowed_weights: subnet.4,
                    max_allowed_uids: subnet.5,
                    min_stake: subnet.7,
                    founder: subnet.8.clone(),
                    ..DefaultSubnetParams::<T>::get()
                };

                let fee = DelegationFee::<T>::get(netuid, &params.founder);
                let changeset: SubnetChangeset<T> =
                    SubnetChangeset::new(params).expect("genesis subnets are valid");
                let _ = self::Pallet::<T>::add_subnet(changeset, Some(netuid))
                    .expect("Failed to register genesis subnet");
                for (uid_usize, (key, name, address, weights)) in
                    self.modules[subnet_idx].iter().enumerate()
                {
                    let changeset = ModuleChangeset::new(name.clone(), address.clone(), fee, None);
                    self::Pallet::<T>::append_module(netuid, key, changeset)
                        .expect("genesis modules are valid");
                    Weights::<T>::insert(netuid, uid_usize as u16, weights);
                }
            }
            // Now we can add the stake to the network
            for (subnet_idx, _subnet) in self.subnets.iter().enumerate() {
                let netuid: u16 = subnet_idx as u16;

                for (key, stake_to) in self.stake_to[netuid as usize].iter() {
                    for (module_key, stake_amount) in stake_to {
                        self::Pallet::<T>::increase_stake(netuid, key, module_key, *stake_amount);
                    }
                }
            }
        }
    }

    // ==================
    // ==== Proposals ===
    // ==================

    // Global Parameters of proposals

    #[pallet::type_value]
    pub fn DefaultProposalCost<T: Config>() -> u64 {
        10_000_000_000_000 // 10_000 $COMAI, the value is returned if the proosal passes
    }

    #[pallet::storage]
    pub type ProposalCost<T: Config> = StorageValue<_, u64, ValueQuery, DefaultProposalCost<T>>;

    #[pallet::type_value]
    pub fn DefaultProposalExpiration<T: Config>() -> u32 {
        130000 // Aprox 12 days
    }

    #[pallet::storage]
    pub type ProposalExpiration<T: Config> =
        StorageValue<_, u32, ValueQuery, DefaultProposalExpiration<T>>;

    #[pallet::type_value]
    pub fn DefaultProposalParticipationThreshold<T: Config>() -> Percent {
        Percent::from_percent(50)
    }

    #[pallet::storage]
    pub(super) type ProposalParticipationThreshold<T: Config> =
        StorageValue<_, Percent, ValueQuery, DefaultProposalParticipationThreshold<T>>;

    #[pallet::storage]
    pub type Proposals<T: Config> = StorageMap<_, Identity, u64, Proposal<T>>;

    // ================
    // ==== Hooks =====
    // ================

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        /// ---- Called on the initialization of this pallet. (the order of on_finalize calls is
        /// determined in the runtime)
        fn on_initialize(_block_number: BlockNumberFor<T>) -> Weight {
            Self::block_step();

            Weight::zero()
        }
    }

    // Dispatchable functions allow users to interact with the pallet and invoke state changes.
    // These functions materialize as "extrinsics", which are often compared to transactions.
    // Dispatchable functions must be annotated with a weight and must return a DispatchResult.
    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::weight((Weight::zero(), DispatchClass::Normal, Pays::No))]
        pub fn set_weights(
            origin: OriginFor<T>,
            netuid: u16,
            uids: Vec<u16>,
            weights: Vec<u16>,
        ) -> DispatchResult {
            Self::do_set_weights(origin, netuid, uids, weights)
        }

        #[pallet::weight((Weight::zero(), DispatchClass::Normal, Pays::No))]
        pub fn add_stake(
            origin: OriginFor<T>,
            netuid: u16,
            module_key: T::AccountId,
            amount: u64,
        ) -> DispatchResult {
            // do not allow zero stakes
            Self::do_add_stake(origin, netuid, module_key, amount)
        }

        #[pallet::weight((Weight::zero(), DispatchClass::Normal, Pays::No))]
        pub fn add_stake_multiple(
            origin: OriginFor<T>,
            netuid: u16,
            module_keys: Vec<T::AccountId>,
            amounts: Vec<u64>,
        ) -> DispatchResult {
            Self::do_add_stake_multiple(origin, netuid, module_keys, amounts)
        }

        #[pallet::weight((Weight::zero(), DispatchClass::Normal, Pays::No))]
        pub fn remove_stake(
            origin: OriginFor<T>,
            netuid: u16,
            module_key: T::AccountId,
            amount: u64,
        ) -> DispatchResult {
            Self::do_remove_stake(origin, netuid, module_key, amount)
        }

        #[pallet::weight((Weight::zero(), DispatchClass::Normal, Pays::No))]
        pub fn remove_stake_multiple(
            origin: OriginFor<T>,
            netuid: u16,
            module_keys: Vec<T::AccountId>,
            amounts: Vec<u64>,
        ) -> DispatchResult {
            Self::do_remove_stake_multiple(origin, netuid, module_keys, amounts)
        }

        #[pallet::weight((Weight::zero(), DispatchClass::Normal, Pays::No))]
        pub fn transfer_stake(
            origin: OriginFor<T>,         // --- The account that is calling this function.
            netuid: u16,                  // --- The network id.
            module_key: T::AccountId,     // --- The module key.
            new_module_key: T::AccountId, // --- The new module key.
            amount: u64,                  // --- The amount of stake to transfer.
        ) -> DispatchResult {
            Self::do_transfer_stake(origin, netuid, module_key, new_module_key, amount)
        }

        #[pallet::weight((Weight::zero(), DispatchClass::Normal, Pays::No))]
        pub fn transfer_multiple(
            origin: OriginFor<T>, // --- The account that is calling this function.
            destinations: Vec<T::AccountId>, // --- The module key.
            amounts: Vec<u64>,    // --- The amount of stake to transfer.
        ) -> DispatchResult {
            Self::do_transfer_multiple(origin, destinations, amounts)
        }

        #[pallet::weight((Weight::zero(), DispatchClass::Normal, Pays::No))]
        pub fn update_module(
            origin: OriginFor<T>,
            netuid: u16,
            name: Vec<u8>,
            address: Vec<u8>,
            delegation_fee: Option<Percent>,
            metadata: Option<Vec<u8>>,
        ) -> DispatchResult {
            let key = ensure_signed(origin.clone())?;
            ensure!(Self::is_registered(netuid, &key), Error::<T>::NotRegistered);

            let params = Self::module_params(netuid, &key);

            let changeset =
                ModuleChangeset::update(&params, name, address, delegation_fee, metadata);
            Self::do_update_module(origin, netuid, changeset)
        }

        #[pallet::weight((Weight::zero(), DispatchClass::Normal, Pays::No))]
        pub fn register(
            origin: OriginFor<T>,
            network: Vec<u8>,
            name: Vec<u8>,
            address: Vec<u8>,
            stake: u64,
            module_key: T::AccountId,
            metadata: Option<Vec<u8>>,
        ) -> DispatchResult {
            Self::do_register(origin, network, name, address, stake, module_key, metadata)
        }

        #[pallet::weight((Weight::zero(), DispatchClass::Normal, Pays::No))]
        pub fn deregister(origin: OriginFor<T>, netuid: u16) -> DispatchResult {
            Self::do_deregister(origin, netuid)
        }

        #[pallet::weight((Weight::zero(), DispatchClass::Normal, Pays::No))]
        pub fn add_profit_shares(
            origin: OriginFor<T>,
            keys: Vec<T::AccountId>,
            shares: Vec<u16>,
        ) -> DispatchResult {
            Self::do_add_profit_shares(origin, keys, shares)
        }

        #[pallet::weight((Weight::zero(), DispatchClass::Normal, Pays::No))]
        pub fn add_to_whitelist(
            origin: OriginFor<T>,
            module_key: T::AccountId,
            recommended_weight: u8,
        ) -> DispatchResult {
            Self::do_add_to_whitelist(origin, module_key, recommended_weight)
        }

        #[pallet::weight((Weight::zero(), DispatchClass::Normal, Pays::No))]
        pub fn remove_from_whitelist(
            origin: OriginFor<T>,
            module_key: T::AccountId,
        ) -> DispatchResult {
            Self::do_remove_from_whitelist(origin, module_key)
        }

        #[pallet::weight((Weight::zero(), DispatchClass::Normal, Pays::No))]
        pub fn update_subnet(
            origin: OriginFor<T>,
            netuid: u16,
            // params
            founder: T::AccountId,
            founder_share: u16,
            immunity_period: u16,
            incentive_ratio: u16,
            max_allowed_uids: u16,
            max_allowed_weights: u16,
            max_stake: u64,
            min_allowed_weights: u16,
            max_weight_age: u64,
            min_stake: u64,
            name: Vec<u8>,
            tempo: u16,
            trust_ratio: u16,
            vote_mode: VoteMode,
        ) -> DispatchResult {
            let params = SubnetParams {
                founder,
                founder_share,
                immunity_period,
                incentive_ratio,
                max_allowed_uids,
                max_allowed_weights,
                max_stake,
                min_allowed_weights,
                max_weight_age,
                min_stake,
                name,
                tempo,
                trust_ratio,
                vote_mode,
            };

            let changeset = SubnetChangeset::update(netuid, params)?;
            Self::do_update_subnet(origin, netuid, changeset)
        }

        // Proposal Calls
        #[pallet::weight((Weight::zero(), DispatchClass::Normal, Pays::No))]
        pub fn add_global_proposal(
            origin: OriginFor<T>,
            burn_rate: u16,                   // max
            max_name_length: u16,             // max length of a network name
            min_name_length: u16,             // min length of a network name
            max_allowed_subnets: u16,         // max number of subnets allowed
            max_allowed_modules: u16,         // max number of modules allowed per subnet
            max_registrations_per_block: u16, // max number of registrations per block
            max_allowed_weights: u16,         // max number of weights per module
            max_burn: u64,                    // max burn allowed to register
            min_burn: u64,                    // min burn required to register
            min_stake: u64,                   // min stake required
            floor_delegation_fee: Percent,    // min delegation fee
            min_weight_stake: u64,            // min weight stake required
            target_registrations_per_interval: u16, /* desired number of registrations per
                                               * interval */
            target_registrations_interval: u16, /* the number of blocks that defines the
                                                 * registration interval */
            adjustment_alpha: u64,           // adjustment alpha
            unit_emission: u64,              // emission per block
            nominator: T::AccountId,         // subnet 0 dao multisig
            subnet_stake_threshold: Percent, // stake needed to start subnet emission
            proposal_cost: u64,              /*amount of $COMAI to create a proposal
                                              * returned if proposal gets accepted */
            proposal_expiration: u32, // the block number, proposal expires at
            proposal_participation_threshold: Percent, /*  minimum stake of the overall network
                                       * stake,
                                       *  in order for proposal to get executed */
        ) -> DispatchResult {
            let mut params = Self::global_params();
            params.burn_rate = burn_rate;
            params.max_name_length = max_name_length;
            params.min_name_length = min_name_length;
            params.max_allowed_subnets = max_allowed_subnets;
            params.max_allowed_modules = max_allowed_modules;
            params.max_registrations_per_block = max_registrations_per_block;
            params.max_allowed_weights = max_allowed_weights;
            params.max_burn = max_burn;
            params.min_burn = min_burn;
            params.min_stake = min_stake;
            params.floor_delegation_fee = floor_delegation_fee;
            params.min_weight_stake = min_weight_stake;
            params.target_registrations_per_interval = target_registrations_per_interval;
            params.target_registrations_interval = target_registrations_interval;
            params.adjustment_alpha = adjustment_alpha;
            params.unit_emission = unit_emission;
            params.nominator = nominator;
            params.subnet_stake_threshold = subnet_stake_threshold;
            params.proposal_cost = proposal_cost;
            params.proposal_expiration = proposal_expiration;
            params.proposal_participation_threshold = proposal_participation_threshold;
            Self::do_add_global_proposal(origin, params)
        }

        #[pallet::weight((Weight::zero(), DispatchClass::Normal, Pays::No))]
        pub fn add_subnet_proposal(
            origin: OriginFor<T>,
            netuid: u16,           // subnet id
            founder: T::AccountId, // parameters
            name: Vec<u8>,         // parameters
            founder_share: u16,    // out of 100
            immunity_period: u16,  // immunity period
            incentive_ratio: u16,  // out of 100
            max_allowed_uids: u16, /* max number of weights allowed to be registered in this
                                    * subnet */
            max_allowed_weights: u16, /* max number of weights allowed to be registered in this
                                       * subnet */
            min_allowed_weights: u16, /* min number of weights allowed to be registered in this
                                       * subnet */
            max_stake: u64,      // max stake allowed
            min_stake: u64,      // min stake required
            max_weight_age: u64, // max age of a weight
            tempo: u16,          // how many blocks to wait before rewarding models
            trust_ratio: u16,    // missing comment
            vote_mode: VoteMode, // missing comment
        ) -> DispatchResult {
            let mut params = Self::subnet_params(netuid);
            params.founder = founder;
            params.name = name;
            params.founder_share = founder_share;
            params.immunity_period = immunity_period;
            params.incentive_ratio = incentive_ratio;
            params.max_allowed_uids = max_allowed_uids;
            params.max_allowed_weights = max_allowed_weights;
            params.min_allowed_weights = min_allowed_weights;
            params.max_stake = max_stake;
            params.min_stake = min_stake;
            params.max_weight_age = max_weight_age;
            params.tempo = tempo;
            params.trust_ratio = trust_ratio;
            params.vote_mode = vote_mode;
            Self::do_add_subnet_proposal(origin, netuid, params)
        }

        #[pallet::weight((Weight::zero(), DispatchClass::Normal, Pays::No))]
        pub fn add_custom_proposal(origin: OriginFor<T>, data: Vec<u8>) -> DispatchResult {
            Self::do_add_custom_proposal(origin, data)
        }

        #[pallet::weight((Weight::zero(), DispatchClass::Normal, Pays::No))]
        pub fn add_custom_subnet_proposal(
            origin: OriginFor<T>,
            netuid: u16,
            data: Vec<u8>,
        ) -> DispatchResult {
            Self::do_add_custom_subnet_proposal(origin, netuid, data)
        }

        #[pallet::weight((Weight::zero(), DispatchClass::Normal, Pays::No))]
        pub fn vote_proposal(
            origin: OriginFor<T>,
            proposal_id: u64,
            agree: bool,
        ) -> DispatchResult {
            Self::do_vote_proposal(origin, proposal_id, agree)
        }

        #[pallet::weight((Weight::zero(), DispatchClass::Normal, Pays::No))]
        pub fn unvote_proposal(origin: OriginFor<T>, proposal_id: u64) -> DispatchResult {
            Self::do_unregister_vote(origin, proposal_id)
        }
    }

    // ---- Subspace helper functions.
    impl<T: Config> Pallet<T> {
        // --- Returns the transaction priority for setting weights.
        pub fn get_priority_set_weights(key: &T::AccountId, netuid: u16) -> u64 {
            if Uids::<T>::contains_key(netuid, key) {
                let uid: u16 = Self::get_uid_for_key(netuid, &key.clone());
                let current_block_number: u64 = Self::get_current_block_number();
                return current_block_number - Self::get_last_update_for_uid(netuid, uid);
            }
            0
        }
        // --- Returns the transaction priority for setting weights.
        pub fn get_priority_stake(key: &T::AccountId, netuid: u16) -> u64 {
            if Uids::<T>::contains_key(netuid, key) {
                return Self::get_stake(netuid, key);
            }
            0
        }
    }

    /************************************************************
        CallType definition
    ************************************************************/
}

#[derive(Debug, PartialEq, Default)]
pub enum CallType {
    SetWeights,
    AddStake,
    TransferStakeMultiple,
    TransferMultiple,
    TransferStake,
    AddStakeMultiple,
    RemoveStakeMultiple,
    RemoveStake,
    AddDelegate,
    Register,
    AddNetwork,
    Serve,
    #[default]
    Other,
}

#[derive(Encode, Decode, Clone, Eq, PartialEq, TypeInfo)]
pub struct SubspaceSignedExtension<T: Config + Send + Sync + TypeInfo>(pub PhantomData<T>);

impl<T: Config + Send + Sync + TypeInfo> Default for SubspaceSignedExtension<T>
where
    T::RuntimeCall: Dispatchable<Info = DispatchInfo, PostInfo = PostDispatchInfo>,
    <T as frame_system::Config>::RuntimeCall: IsSubType<Call<T>>,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Config + Send + Sync + TypeInfo> SubspaceSignedExtension<T>
where
    T::RuntimeCall: Dispatchable<Info = DispatchInfo, PostInfo = PostDispatchInfo>,
    <T as frame_system::Config>::RuntimeCall: IsSubType<Call<T>>,
{
    pub fn new() -> Self {
        Self(Default::default())
    }

    pub fn get_priority_vanilla(who: &T::AccountId) -> u64 {
        // Return high priority so that every extrinsic except set_weights function will
        // have a higher priority than the set_weights call
        // get the current block number
        let current_block_number: u64 = Pallet::<T>::get_current_block_number();
        let balance = Pallet::<T>::get_balance_u64(who);

        // this is the current block number minus the last update block number
        current_block_number + balance
    }

    pub fn get_priority_set_weights(who: &T::AccountId, netuid: u16) -> u64 {
        // Return the non vanilla priority for a set weights call.

        Pallet::<T>::get_priority_set_weights(who, netuid)
    }

    pub fn u64_to_balance(
        input: u64,
    ) -> Option<
        <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance,
    > {
        input.try_into().ok()
    }
}

impl<T: Config + Send + Sync + TypeInfo> sp_std::fmt::Debug for SubspaceSignedExtension<T> {
    fn fmt(&self, f: &mut sp_std::fmt::Formatter) -> sp_std::fmt::Result {
        write!(f, "SubspaceSignedExtension")
    }
}

impl<T: Config + Send + Sync + TypeInfo> SignedExtension for SubspaceSignedExtension<T>
where
    T::RuntimeCall: Dispatchable<Info = DispatchInfo, PostInfo = PostDispatchInfo>,
    <T as frame_system::Config>::RuntimeCall: IsSubType<Call<T>>,
{
    const IDENTIFIER: &'static str = "SubspaceSignedExtension";

    type AccountId = T::AccountId;
    type Call = T::RuntimeCall;
    type AdditionalSigned = ();
    type Pre = (CallType, u64, Self::AccountId);

    fn additional_signed(&self) -> Result<Self::AdditionalSigned, TransactionValidityError> {
        Ok(())
    }

    fn validate(
        &self,
        who: &Self::AccountId,
        call: &Self::Call,
        _info: &DispatchInfoOf<Self::Call>,
        _len: usize,
    ) -> TransactionValidity {
        match call.is_sub_type() {
            Some(Call::set_weights { netuid, .. }) => {
                let priority: u64 = Self::get_priority_set_weights(who, *netuid);
                Ok(ValidTransaction {
                    priority,
                    longevity: 1,
                    ..Default::default()
                })
            }
            Some(Call::add_stake { .. }) => Ok(ValidTransaction {
                priority: Self::get_priority_vanilla(who),
                ..Default::default()
            }),
            Some(Call::remove_stake { .. }) => Ok(ValidTransaction {
                priority: Self::get_priority_vanilla(who),
                ..Default::default()
            }),
            Some(Call::update_subnet { .. }) => Ok(ValidTransaction {
                priority: Self::get_priority_vanilla(who),
                ..Default::default()
            }),
            Some(Call::add_profit_shares { .. }) => Ok(ValidTransaction {
                priority: Self::get_priority_vanilla(who),
                ..Default::default()
            }),

            Some(Call::register { .. }) => Ok(ValidTransaction {
                priority: Self::get_priority_vanilla(who),
                ..Default::default()
            }),
            _ => Ok(ValidTransaction {
                priority: Self::get_priority_vanilla(who),
                ..Default::default()
            }),
        }
    }

    // NOTE: Add later when we put in a pre and post dispatch step.
    fn pre_dispatch(
        self,
        who: &Self::AccountId,
        call: &Self::Call,
        _info: &DispatchInfoOf<Self::Call>,
        _len: usize,
    ) -> Result<Self::Pre, TransactionValidityError> {
        let who = who.clone();
        match call.is_sub_type() {
            Some(Call::add_stake { .. }) => Ok((CallType::AddStake, 0, who)),
            Some(Call::add_stake_multiple { .. }) => Ok((CallType::AddStakeMultiple, 0, who)),
            Some(Call::remove_stake { .. }) => Ok((CallType::RemoveStake, 0, who)),
            Some(Call::remove_stake_multiple { .. }) => Ok((CallType::RemoveStakeMultiple, 0, who)),
            Some(Call::transfer_stake { .. }) => Ok((CallType::TransferStake, 0, who)),
            Some(Call::transfer_multiple { .. }) => Ok((CallType::TransferMultiple, 0, who)),
            Some(Call::set_weights { .. }) => Ok((CallType::SetWeights, 0, who)),
            Some(Call::register { .. }) => Ok((CallType::Register, 0, who)),
            Some(Call::update_module { .. }) => Ok((CallType::Serve, 0, who)),
            _ => Ok((CallType::Other, 0, who)),
        }
    }

    fn post_dispatch(
        maybe_pre: Option<Self::Pre>,
        _info: &DispatchInfoOf<Self::Call>,
        _post_info: &PostDispatchInfoOf<Self::Call>,
        _len: usize,
        _result: &dispatch::DispatchResult,
    ) -> Result<(), TransactionValidityError> {
        if let Some((call_type, _transaction_fee, _who)) = maybe_pre {
            match call_type {
                CallType::SetWeights => {
                    log::debug!("Not Implemented!");
                }
                CallType::AddStake => {
                    log::debug!("Not Implemented! Need to add potential transaction fees here.");
                }

                CallType::AddStakeMultiple => {
                    log::debug!("Not Implemented! Need to add potential transaction fees here.");
                }
                CallType::RemoveStake => {
                    log::debug!("Not Implemented! Need to add potential transaction fees here.");
                }
                CallType::RemoveStakeMultiple => {
                    log::debug!("Not Implemented! Need to add potential transaction fees here.");
                }
                CallType::TransferStake => {
                    log::debug!("Not Implemented! Need to add potential transaction fees here.");
                }
                CallType::TransferStakeMultiple => {
                    log::debug!("Not Implemented! Need to add potential transaction fees here.");
                }
                CallType::TransferMultiple => {
                    log::debug!("Not Implemented! Need to add potential transaction fees here.");
                }
                CallType::AddNetwork => {
                    log::debug!("Not Implemented! Need to add potential transaction fees here.");
                }
                CallType::Register => {
                    log::debug!("Not Implemented!");
                }
                _ => {
                    log::debug!("Not Implemented!");
                }
            }
        }
        Ok(())
    }
}
