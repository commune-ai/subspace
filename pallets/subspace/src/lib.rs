#![allow(deprecated, non_camel_case_types, non_snake_case)]
#![cfg_attr(not(feature = "std"), no_std)]
#![recursion_limit = "512"]

use frame_system::{self as system, ensure_signed};
pub use pallet::*;

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
use scale_info::TypeInfo;
use sp_runtime::{
    traits::{DispatchInfoOf, Dispatchable, PostDispatchInfoOf, SignedExtension},
    transaction_validity::{TransactionValidity, TransactionValidityError},
};
use sp_std::marker::PhantomData;

// ============================
//	==== Benchmark Imports =====
// ============================
#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

pub mod autogen_weights;
pub use autogen_weights::WeightInfo;

#[cfg(test)]
mod mock;

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
mod subnet;
mod voting;
mod weights;

#[frame_support::pallet]
pub mod pallet {
    #![allow(
        deprecated,
        clippy::let_unit_value,
        clippy::too_many_arguments,
        clippy::type_complexity
    )]

    use super::*;
    use frame_support::{pallet_prelude::*, traits::Currency};
    use frame_system::pallet_prelude::*;

    use sp_arithmetic::per_things::Percent;
    pub use sp_std::{vec, vec::Vec};

    const STORAGE_VERSION: StorageVersion = StorageVersion::new(1);

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

    // =======================================
    // ==== Defaults ====
    // =======================================

    #[pallet::type_value]
    pub fn EmptyU16Vec<T: Config>() -> Vec<u16> {
        vec![]
    }
    #[pallet::type_value]
    pub fn EmptyU64Vec<T: Config>() -> Vec<u64> {
        vec![]
    }
    #[pallet::type_value]
    pub fn EmptyBoolVec<T: Config>() -> Vec<bool> {
        vec![]
    }

    // ============================
    // ==== Global Variables ====
    // ============================
    #[pallet::type_value]
    pub fn DefaultUnitEmission<T: Config>() -> u64 {
        23148148148
    }
    #[pallet::storage] // --- ITEM ( unit_emission )
    pub(super) type UnitEmission<T> = StorageValue<_, u64, ValueQuery, DefaultUnitEmission<T>>;

    #[pallet::type_value]
    pub fn DefaultTxRateLimit<T: Config>() -> u64 {
        1
    }
    #[pallet::storage] // --- ITEM ( tx_rate_limit )
    pub(super) type TxRateLimit<T> = StorageValue<_, u64, ValueQuery, DefaultTxRateLimit<T>>;
    // FIXME: NOT IN USE

    #[pallet::type_value]
    pub fn DefaultBurnRate<T: Config>() -> u16 {
        0
    }
    #[pallet::storage] // --- MAP ( netuid ) --> min_allowed_weights
    pub type BurnRate<T> = StorageValue<_, u16, ValueQuery, DefaultBurnRate<T>>;

    #[pallet::type_value]
    pub fn DefaultMinBurn<T: Config>() -> u64 {
        200_000_000 // 2 $COMAI
    }
    #[pallet::storage] // --- MinBurn
    pub type MinBurn<T> = StorageValue<_, u64, ValueQuery, DefaultMinBurn<T>>;

    #[pallet::type_value]
    pub fn DefaultMaxBurn<T: Config>() -> u64 {
        250_000_000_000 // 250 $COMAI
    }

    #[pallet::type_value]
    pub fn DefaultAdjustmentAlpha<T: Config>() -> u64 {
        0 // u64::MAX / 2
    }

    #[pallet::storage] // --- adjusment alpha
    pub type AdjustmentAlpha<T> = StorageValue<_, u64, ValueQuery, DefaultAdjustmentAlpha<T>>;

    #[pallet::storage] // --- MaxBurn
    pub type MaxBurn<T> = StorageValue<_, u64, ValueQuery, DefaultMaxBurn<T>>;

    #[pallet::type_value]
    pub fn DefaultLastTxBlock<T: Config>() -> u64 {
        0
    }
    #[pallet::storage] // --- MAP ( key ) --> last_block
    pub(super) type LastTxBlock<T: Config> =
        StorageMap<_, Identity, T::AccountId, u64, ValueQuery, DefaultLastTxBlock<T>>;

    #[pallet::type_value]
    pub fn DefaultMaxNameLength<T: Config>() -> u16 {
        32
    }
    #[pallet::storage] // --- ITEM ( max_name_length )
    pub(super) type MaxNameLength<T: Config> =
        StorageValue<_, u16, ValueQuery, DefaultMaxNameLength<T>>;

    #[pallet::type_value]
    pub fn DefaultMaxAllowedSubnets<T: Config>() -> u16 {
        256
    }
    #[pallet::storage] // --- ITEM ( max_allowed_subnets )
    pub(super) type MaxAllowedSubnets<T: Config> =
        StorageValue<_, u16, ValueQuery, DefaultMaxAllowedSubnets<T>>;

    #[pallet::type_value]
    pub fn DefaultRegistrationsThisInterval<T: Config>() -> u16 {
        0
    }

    #[pallet::storage] // --- ITEM ( registrations_this_interval )
    pub(super) type RegistrationsThisInterval<T: Config> =
        StorageValue<_, u16, ValueQuery, DefaultRegistrationsThisInterval<T>>;

    #[pallet::type_value]
    pub fn DefaultBurn<T: Config>() -> u64 {
        0
    }

    #[pallet::storage] // --- ITEM ( burn )
    pub(super) type Burn<T: Config> = StorageValue<_, u64, ValueQuery, DefaultBurn<T>>;

    #[pallet::type_value]
    pub fn DefaultMaxAllowedModules<T: Config>() -> u16 {
        10_000
    }
    #[pallet::storage] // --- ITEM ( max_allowed_modules )
    pub(super) type MaxAllowedModules<T: Config> =
        StorageValue<_, u16, ValueQuery, DefaultMaxAllowedModules<T>>;

    #[pallet::type_value]
    pub fn DefaultRegistrationsPerBlock<T: Config>() -> u16 {
        0
    }
    #[pallet::storage] // --- ITEM ( registrations_this block )
    pub type RegistrationsPerBlock<T> =
        StorageValue<_, u16, ValueQuery, DefaultRegistrationsPerBlock<T>>;

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

    #[pallet::type_value]
    pub fn DefaultMinStakeGlobal<T: Config>() -> u64 {
        100
    }
    #[pallet::storage] // --- MAP ( netuid ) --> min_allowed_weights
    pub type MinStakeGlobal<T> = StorageValue<_, u64, ValueQuery, DefaultMinStake<T>>;

    #[pallet::type_value]
    pub fn DefaultMinDelegationFeeGlobal<T: Config>() -> Percent {
        Percent::from_percent(5u8)
    }

    #[pallet::storage]
    pub type FloorDelegationFee<T> =
        StorageValue<_, Percent, ValueQuery, DefaultMinDelegationFeeGlobal<T>>;

    #[pallet::type_value]
    pub fn DefaultMinWeightStake<T: Config>() -> u64 {
        0
    }
    #[pallet::storage] // --- MAP ( netuid ) --> min_allowed_weights
    pub type MinWeightStake<T> = StorageValue<_, u64, ValueQuery, DefaultMinWeightStake<T>>;

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
        pub delegation_fee: Percent, // delegate_fee
        pub controller: T::AccountId,
    }

    #[derive(Decode, Encode, PartialEq, Eq, Clone, Debug, TypeInfo)]
    // skip
    pub struct GlobalParams {
        pub burn_rate: u16,
        // max
        pub max_name_length: u16,             // max length of a network name
        pub max_allowed_subnets: u16,         // max number of subnets allowed
        pub max_allowed_modules: u16,         // max number of modules allowed per subnet
        pub max_registrations_per_block: u16, // max number of registrations per block
        pub max_allowed_weights: u16,         // max number of weights per module
        pub max_proposals: u64,               // max number of proposals per block

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
        pub tx_rate_limit: u64,    // tx rate limit
        pub vote_threshold: u16,   // out of 100
        pub vote_mode: Vec<u8>,    // out of 100
    }

    #[pallet::type_value]
    pub fn DefaultGlobalParams<T: Config>() -> GlobalParams {
        GlobalParams {
            burn_rate: DefaultBurnRate::<T>::get(),
            max_allowed_subnets: DefaultMaxAllowedSubnets::<T>::get(),
            max_allowed_modules: DefaultMaxAllowedModules::<T>::get(),
            max_allowed_weights: DefaultMaxAllowedWeightsGlobal::<T>::get(),
            max_registrations_per_block: DefaultMaxRegistrationsPerBlock::<T>::get(),
            target_registrations_per_interval: DefaultTargetRegistrationsPerInterval::<T>::get(),
            target_registrations_interval: DefaultTargetRegistrationsInterval::<T>::get(),
            max_name_length: DefaultMaxNameLength::<T>::get(),
            max_proposals: DefaultMaxProposals::<T>::get(),
            min_burn: DefaultMinBurn::<T>::get(),
            max_burn: DefaultMaxBurn::<T>::get(),
            min_stake: DefaultMinStakeGlobal::<T>::get(),
            floor_delegation_fee: DefaultMinDelegationFeeGlobal::<T>::get(),
            min_weight_stake: DefaultMinWeightStake::<T>::get(),
            adjustment_alpha: DefaultAdjustmentAlpha::<T>::get(),
            unit_emission: DefaultUnitEmission::<T>::get(),
            tx_rate_limit: DefaultTxRateLimit::<T>::get(),
            vote_threshold: DefaultVoteThreshold::<T>::get(),
            vote_mode: DefaultVoteMode::<T>::get(),
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
        pub self_vote: bool, //
        pub tempo: u16,      // how many blocks to wait before rewarding models
        pub trust_ratio: u16,
        pub vote_threshold: u16, // out of 100
        pub vote_mode: Vec<u8>,
    }

    #[pallet::type_value]
    pub fn DefaultSubnetParams<T: Config>() -> SubnetParams<T> {
        SubnetParams {
            name: vec![],
            tempo: DefaultTempo::<T>::get(),
            immunity_period: DefaultImmunityPeriod::<T>::get(),
            min_allowed_weights: DefaultMinAllowedWeights::<T>::get(),
            max_allowed_weights: DefaultMaxAllowedWeights::<T>::get(),
            max_allowed_uids: DefaultMaxAllowedUids::<T>::get(),
            max_weight_age: DefaultMaxWeightAge::<T>::get(),
            max_stake: DefaultMaxStake::<T>::get(),
            vote_threshold: DefaultVoteThreshold::<T>::get(),
            vote_mode: DefaultVoteMode::<T>::get(),
            trust_ratio: DefaultTrustRatio::<T>::get(),
            self_vote: DefaultSelfVote::<T>::get(),
            founder_share: DefaultFounderShare::<T>::get(),
            incentive_ratio: DefaultIncentiveRatio::<T>::get(),
            min_stake: DefaultMinStake::<T>::get(),
            founder: DefaultFounder::<T>::get(),
        }
    }

    #[pallet::type_value]
    pub fn DefaultMaxAllowedUids<T: Config>() -> u16 {
        4096
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

    #[pallet::type_value]
    pub fn DefaultMinStake<T: Config>() -> u64 {
        0
    }
    #[pallet::storage] // --- MAP ( netuid ) --> min_allowed_weights
    pub type MinStake<T> = StorageMap<_, Identity, u16, u64, ValueQuery, DefaultMinStake<T>>;

    #[pallet::type_value]
    pub fn DefaultMaxStake<T: Config>() -> u64 {
        u64::MAX
    }
    #[pallet::storage] // --- MAP ( netuid ) --> min_allowed_weights
    pub type MaxStake<T> = StorageMap<_, Identity, u16, u64, ValueQuery, DefaultMaxStake<T>>;

    #[pallet::type_value]
    pub fn DefaultMaxWeightAge<T: Config>() -> u64 {
        u64::MAX
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

    #[pallet::type_value]
    pub fn DefaultFounderShare<T: Config>() -> u16 {
        0
    }
    #[pallet::storage] // --- DMAP ( key, netuid ) --> bool
    pub type FounderShare<T: Config> =
        StorageMap<_, Identity, u16, u16, ValueQuery, DefaultFounderShare<T>>;

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

    #[pallet::type_value]
    pub fn DefaultTrustRatio<T: Config>() -> u16 {
        0
    }
    #[pallet::storage] // --- MAP ( netuid ) --> epoch
    pub type TrustRatio<T> = StorageMap<_, Identity, u16, u16, ValueQuery, DefaultTrustRatio<T>>;

    #[pallet::type_value]
    pub fn DefaultQuadraticVoting<T: Config>() -> bool {
        false
    }
    #[pallet::storage] // --- MAP ( netuid ) --> epoch
    pub type QuadraticVoting<T> =
        StorageMap<_, Identity, u16, bool, ValueQuery, DefaultQuadraticVoting<T>>;

    // =======================================
    // ==== Voting  ====
    // =======================================

    #[derive(Decode, Encode, PartialEq, Eq, Clone, Debug, TypeInfo)]
    pub struct VoterInfo {
        // --- parameters
        pub proposal_id: u64,
        pub votes: u64,
        pub participant_index: u16,
    }

    #[pallet::type_value]
    pub fn DefaultVoterInfo<T: Config>() -> VoterInfo {
        VoterInfo {
            proposal_id: u64::MAX,
            votes: 0,
            participant_index: u16::MAX,
        }
    }
    #[pallet::storage] // --- MAP ( netuid ) --> epoch
    pub type Voter2Info<T: Config> =
        StorageMap<_, Identity, T::AccountId, VoterInfo, ValueQuery, DefaultVoterInfo<T>>;

    pub fn DefaultVoteStake<T: Config>() -> u64 {
        0
    } // out of 100

    // VOTING THRESHOOLD
    #[pallet::type_value]
    pub fn DefaultVoteThreshold<T: Config>() -> u16 {
        50
    } // out of 100
    #[pallet::storage] // --- MAP ( netuid ) --> epoch
    pub type VoteThresholdSubnet<T> =
        StorageMap<_, Identity, u16, u16, ValueQuery, DefaultVoteThreshold<T>>;
    #[pallet::storage] // --- MAP ( netuid ) --> epoch
    pub type GlobalVoteThreshold<T> = StorageValue<_, u16, ValueQuery, DefaultVoteThreshold<T>>;

    // VOTING MODE
    // OPTIONS -> [stake, authority, quadratic]
    #[pallet::type_value]
    pub fn DefaultVoteMode<T: Config>() -> Vec<u8> {
        "authority".as_bytes().to_vec()
    }
    #[pallet::storage] // --- MAP ( netuid ) --> epoch
    pub type VoteModeSubnet<T> =
        StorageMap<_, Identity, u16, Vec<u8>, ValueQuery, DefaultVoteMode<T>>;
    #[pallet::storage] // --- MAP ( netuid ) --> epoch
    pub type VoteModeGlobal<T> = StorageValue<_, Vec<u8>, ValueQuery, DefaultVoteMode<T>>;

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

    #[pallet::type_value]
    pub fn DefaultEmission<T: Config>() -> u64 {
        0
    }
    #[pallet::storage] // --- MAP( netuid ) --> subnet_emission
    pub type SubnetEmission<T> = StorageMap<_, Identity, u16, u64, ValueQuery, DefaultEmission<T>>;

    #[pallet::type_value]
    pub fn DefaultN<T: Config>() -> u16 {
        0
    }
    #[pallet::storage] // --- MAP ( netuid ) --> subnetwork_n (Number of UIDs in the network).
    pub type N<T: Config> = StorageMap<_, Identity, u16, u16, ValueQuery, DefaultN<T>>;

    #[pallet::type_value]
    pub fn DefaultPendingEmission<T: Config>() -> u64 {
        0
    }
    #[pallet::storage] // --- MAP ( netuid ) --> pending_emission
    pub type PendingEmission<T> =
        StorageMap<_, Identity, u16, u64, ValueQuery, DefaultPendingEmission<T>>;

    #[pallet::storage] // --- MAP ( network_name ) --> netuid
    pub type SubnetNames<T: Config> = StorageMap<_, Identity, u16, Vec<u8>, ValueQuery>;

    // =======================================
    // ==== Module Variables  ====
    // =======================================

    #[pallet::storage] // --- DMAP ( netuid, module_key ) --> uid
    pub(super) type Uids<T: Config> =
        StorageDoubleMap<_, Identity, u16, Blake2_128Concat, T::AccountId, u16, OptionQuery>;

    #[pallet::storage] // --- DMAP ( netuid, module_key ) --> uid
    pub(super) type Key2Controller<T: Config> = StorageDoubleMap<
        _,
        Identity,
        T::AccountId,
        Blake2_128Concat,
        T::AccountId,
        u16,
        OptionQuery,
    >;

    #[pallet::storage] // --- DMAP ( netuid, module_key ) --> uid
    pub(super) type Controller2Keys<T: Config> = StorageDoubleMap<
        _,
        Identity,
        T::AccountId,
        Blake2_128Concat,
        Vec<T::AccountId>,
        u16,
        OptionQuery,
    >;

    #[pallet::type_value]
    pub fn DefaultKey<T: Config>() -> T::AccountId {
        T::AccountId::decode(&mut sp_runtime::traits::TrailingZeroInput::zeroes()).unwrap()
    }
    #[pallet::storage] // --- DMAP ( netuid, uid ) --> module_key
    pub(super) type Keys<T: Config> =
        StorageDoubleMap<_, Identity, u16, Identity, u16, T::AccountId, ValueQuery, DefaultKey<T>>;

    #[pallet::type_value]
    pub fn DefaultName<T: Config>() -> Vec<u8> {
        vec![]
    }
    #[pallet::storage] // --- DMAP ( netuid, uid ) --> module_name
    pub type Name<T: Config> =
        StorageDoubleMap<_, Twox64Concat, u16, Twox64Concat, u16, Vec<u8>, ValueQuery>;

    #[pallet::type_value]
    pub fn DefaultAddress<T: Config>() -> Vec<u8> {
        vec![]
    }
    #[pallet::storage] // --- DMAP ( netuid, uid ) --> module_address
    pub type Address<T: Config> =
        StorageDoubleMap<_, Twox64Concat, u16, Twox64Concat, u16, Vec<u8>, ValueQuery>;

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

    #[pallet::type_value]
    pub fn DefaultBlockAtRegistration<T: Config>() -> u64 {
        0
    }
    #[pallet::storage] // --- DMAP ( netuid, uid ) --> block number that the module is registered
    pub type RegistrationBlock<T: Config> = StorageDoubleMap<
        _,
        Identity,
        u16,
        Identity,
        u16,
        u64,
        ValueQuery,
        DefaultBlockAtRegistration<T>,
    >;

    // =======================================
    // ==== Module Staking Variables  ====
    // =======================================

    #[pallet::type_value]
    pub fn DefaultStake<T: Config>() -> u64 {
        0
    }

    #[pallet::storage] // --- DMAP ( netuid, module_key ) --> stake | Returns the stake under a module.
    pub type Stake<T: Config> = StorageDoubleMap<
        _,
        Identity,
        u16,
        Identity,
        T::AccountId,
        u64,
        ValueQuery,
        DefaultStake<T>,
    >;
    #[pallet::storage] // --- DMAP ( netuid, module_key ) --> Vec<(delegater, stake )> | Returns the list of delegates
                       // and their staked amount under a module
    pub type StakeFrom<T: Config> = StorageDoubleMap<
        _,
        Identity,
        u16,
        Identity,
        T::AccountId,
        Vec<(T::AccountId, u64)>,
        ValueQuery,
    >;

    #[pallet::storage] // --- DMAP ( netuid, account_id ) --> Vec<(module_key, stake )> | Returns the list of the
    pub type StakeTo<T: Config> = StorageDoubleMap<
        _,
        Identity,
        u16,
        Identity,
        T::AccountId,
        Vec<(T::AccountId, u64)>,
        ValueQuery,
    >;

    // TOTAL STAKE PER SUBNET
    #[pallet::storage] // --- MAP ( netuid ) --> subnet_total_stake
    pub type TotalStake<T> = StorageMap<_, Identity, u16, u64, ValueQuery>;

    // LOAN VARIABLES

    #[pallet::storage] // --- DMAP ( netuid, module_key ) --> Vec<(delegater, stake )> | Returns the list of delegates
    pub type LoanTo<T: Config> =
        StorageMap<_, Identity, T::AccountId, Vec<(T::AccountId, u64)>, ValueQuery>;

    #[pallet::storage] // --- DMAP ( netuid, module_key ) --> Vec<(delegater, stake )> | Returns the list of delegates
    pub type LoanFrom<T: Config> =
        StorageMap<_, Identity, T::AccountId, Vec<(T::AccountId, u64)>, ValueQuery>;

    // PROFIT SHARE VARIABLES

    #[pallet::type_value]
    pub fn DefaultProfitShares<T: Config>() -> Vec<(T::AccountId, u16)> {
        vec![]
    }

    #[pallet::storage] // --- DMAP ( netuid, account_id ) --> Vec<(module_key, stake )> | Returns the list of the
    pub type ProfitShares<T: Config> = StorageMap<
        _,
        Identity,
        T::AccountId,
        Vec<(T::AccountId, u16)>,
        ValueQuery,
        DefaultProfitShares<T>,
    >;

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
    pub(super) type Incentive<T: Config> =
        StorageMap<_, Identity, u16, Vec<u16>, ValueQuery, EmptyU16Vec<T>>;
    #[pallet::storage] // --- MAP ( netuid ) --> trust
    pub(super) type Trust<T: Config> =
        StorageMap<_, Identity, u16, Vec<u16>, ValueQuery, EmptyU16Vec<T>>;
    #[pallet::storage] // --- MAP ( netuid ) --> dividends
    pub(super) type Dividends<T: Config> =
        StorageMap<_, Identity, u16, Vec<u16>, ValueQuery, EmptyU16Vec<T>>;
    #[pallet::storage] // --- MAP ( netuid ) --> emission
    pub(super) type Emission<T: Config> =
        StorageMap<_, Identity, u16, Vec<u64>, ValueQuery, EmptyU64Vec<T>>;
    #[pallet::storage] // --- MAP ( netuid ) --> last_update
    pub(super) type LastUpdate<T: Config> =
        StorageMap<_, Identity, u16, Vec<u64>, ValueQuery, EmptyU64Vec<T>>;

    #[pallet::type_value]
    pub fn DefaultWeights<T: Config>() -> Vec<(u16, u16)> {
        vec![]
    }
    #[pallet::storage] // --- DMAP ( netuid, uid ) --> weights
    pub(super) type Weights<T: Config> = StorageDoubleMap<
        _,
        Identity,
        u16,
        Identity,
        u16,
        Vec<(u16, u16)>,
        ValueQuery,
        DefaultWeights<T>,
    >;

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
        TxRateLimitSet(u64), // --- Event created when setting the transaction rate limit.
        UnitEmissionSet(u64), // --- Event created when setting the unit emission
        MaxNameLengthSet(u16), // --- Event created when setting the maximum network name length
        MaxAllowedSubnetsSet(u16), // --- Event created when setting the maximum allowed subnets
        MaxAllowedModulesSet(u16), // --- Event created when setting the maximum allowed modules
        MaxRegistrationsPerBlockSet(u16), // --- Event created when we set max registrations
        target_registrations_intervalSet(u16), // --- Event created when we set target registrations
        GlobalParamsUpdated(GlobalParams), // --- Event created when global parameters are updated
        SubnetParamsUpdated(u16), // --- Event created when subnet parameters are updated
        GlobalProposalAccepted(u64), // (id)
        CustomProposalAccepted(u64), // (id)
        SubnetProposalAccepted(u64, u16), // (id, netuid)
    }

    // Errors inform users that something went wrong.
    #[pallet::error]
    pub enum Error<T> {
        ModuleNameAlreadyExists, // --- Thrown when a module name already exists.
        NetworkDoesNotExist,     // --- Thrown when the network does not exist.
        TooFewVotesForNewProposal,
        ModuleAddressTooLong,
        NetworkExist, // --- Thrown when the network already exist.
        InvalidIpType, /* ---- Thrown when the user tries to serve an module which
                       * is not of type	4 (IPv4) or 6 (IPv6). */
        InvalidIpAddress, /* --- Thrown when an invalid IP address is passed to the serve
                           * function. */
        NotRegistered, /* ---- Thrown when the caller requests setting or removing data from a
                        * module which does not exist in the active set. */
        NotEnoughStaketoWithdraw, /* ---- Thrown when the caller requests removing more stake
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
                      * allowed. */
        TxRateLimitExceeded, /* --- Thrown when a transactor exceeds the rate limit for
                              * transactions. */
        InvalidMaxAllowedUids, /* --- Thrown when the user tries to set max allowed uids to a
                                * value less than the current number of registered uids. */
        SubnetNameAlreadyExists,
        BalanceNotAdded,
        StakeNotRemoved,
        SubnetNameNotExists,
        ModuleNameTooLong, /* --- Thrown when the user tries to register a module name that is
                            * too long. */
        KeyAlreadyRegistered, //
        ModuleNameDoesNotExist, /* --- Thrown when the user tries to remove a module name that
                               * does not exist. */
        KeyNameMismatch,
        NotFounder,
        NameAlreadyRegistered,
        NotEnoughStaketoSetWeights,
        NotEnoughStakeToStartNetwork,
        NetworkRegistrationFailed,
        NetworkAlreadyRegistered,
        NotEnoughtStakePerWeight,
        NoSelfWeight,
        DifferentLengths,
        NotEnoughBalanceToRegister,
        StakeNotAdded,
        BalanceNotRemoved,
        NotEnoughStakeToRegister,
        StillRegistered,
        MaxAllowedModules, /* --- Thrown when the user tries to set max allowed modules to a
                            * value less than the current number of registered modules. */
        TooMuchUpdateProposals,
        InvalidProposalId,
        UpdateProposalAlreadyVoted,
        UpdateProposalVoteNotAvailable,
        NotEnoughVotesToAccept,
        NotEnoughBalanceToTransfer,
        NotAuthorityMode,
        InvalidTrustRatio,
        InvalidMinAllowedWeights,
        InvalidMaxAllowedWeights,
        InvalidMinStake,
        InvalidMinDelegationFee,

        InvalidGlobalParams,
        InvalidMaxNameLength,
        InvalidMaxAllowedSubnets,
        InvalidMaxAllowedModules,
        InvalidMaxRegistrationsPerBlock,
        InvalidTargetRegistrationsInterval,
        InvalidVoteThreshold,
        InvalidMaxProposals,
        InvalidUnitEmission,
        InvalidTxRateLimit,
        InvalidBurnRate,
        InvalidMinBurn,
        InvalidMaxBurn,

        // VOTING
        ProposalDoesNotExist,
        VotingPowerIsZero,
        InvalidProposalData,
        ProposalDataTooLarge,
        VoterIsNotRegistered,
        VoterIsRegistered,
        InvalidVoteMode,
        InvalidMaxWeightAge,
        InvalidMaxStake,

        AlreadyControlled,
        AlreadyController,
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
            // let block: u32 = self.block;
            // frame_system::Pallet::<T>::set_block_number(T::BlockNumber::from(block));

            for (subnet_idx, subnet) in self.subnets.iter().enumerate() {
                let netuid: u16 = subnet_idx as u16;
                // --- Set subnet parameters

                let default_params = self::Pallet::<T>::default_subnet_params();

                let params = SubnetParams {
                    name: subnet.0.clone(),
                    tempo: subnet.1,
                    immunity_period: subnet.2,
                    min_allowed_weights: subnet.3,
                    max_allowed_weights: subnet.4,
                    max_allowed_uids: subnet.5,
                    // subnet.6
                    min_stake: subnet.7,
                    founder: subnet.8.clone(),
                    max_stake: default_params.max_stake,
                    vote_threshold: default_params.vote_threshold,
                    vote_mode: default_params.vote_mode.clone(),
                    trust_ratio: default_params.trust_ratio,
                    self_vote: default_params.self_vote,
                    founder_share: default_params.founder_share,
                    incentive_ratio: default_params.incentive_ratio,
                    max_weight_age: default_params.max_weight_age,
                };

                self::Pallet::<T>::add_subnet(params);

                for (uid_usize, (key, name, address, weights)) in
                    self.modules[subnet_idx].iter().enumerate()
                {
                    self::Pallet::<T>::append_module(netuid, key, name.clone(), address.clone());
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

    // ========================================================
    // ==== Voting System to Update Global and Subnet  ====
    // ========================================================
    #[derive(Decode, Encode, PartialEq, Eq, Clone, Debug, TypeInfo)]
    #[scale_info(skip_type_params(T))]
    pub struct Proposal<T: Config> {
        // --- parameters
        pub subnet_params: SubnetParams<T>,
        pub global_params: GlobalParams,
        pub netuid: u16, // FOR SUBNET PROPOSAL ONLY
        pub votes: u64,
        pub participants: Vec<T::AccountId>,
        pub accepted: bool,
        pub data: Vec<u8>, // for custom proposal
        pub mode: Vec<u8>, // "global", "subnet", "custom"
    }

    #[pallet::type_value]
    pub fn DefaultProposal<T: Config>() -> Proposal<T> {
        Proposal {
            global_params: DefaultGlobalParams::<T>::get(),
            subnet_params: DefaultSubnetParams::<T>::get(),
            votes: 0,
            netuid: 0,
            participants: vec![],
            accepted: false,
            mode: vec![],
            data: vec![],
        }
    }

    #[pallet::storage] // --- MAP ( global_proposal_id ) --> global_update_proposal
    pub(super) type Proposals<T: Config> =
        StorageMap<_, Identity, u64, Proposal<T>, ValueQuery, DefaultProposal<T>>;

    #[pallet::type_value]
    pub fn DefaultMaxProposals<T: Config>() -> u64 {
        128
    }
    #[pallet::storage]
    pub(super) type MaxProposals<T: Config> =
        StorageValue<_, u64, ValueQuery, DefaultMaxProposals<T>>;

    // ================
    // ==== Hooks =====
    // ================

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        // ---- Called on the initialization of this pallet. (the order of on_finalize calls is
        // determined in the runtime)
        //
        // # Args:
        // 	* 'n': (T::BlockNumber):
        // 		- The number of the block we are initializing.
        // fn on_runtime_upgrade() -> frame_support::weights::Weight {
        // 	migration::on_runtime_upgrade::<T>()
        // }

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
        ) -> DispatchResult {
            let key = ensure_signed(origin.clone())?;
            ensure!(Self::is_registered(netuid, &key), Error::<T>::NotRegistered);
            let uid: u16 = Self::get_uid_for_key(netuid, &key);
            let mut params = Self::module_params(netuid, uid);
            params.name = name;
            params.address = address;
            if let Some(delegation_fee) = delegation_fee {
                ensure!(
                    delegation_fee >= Self::get_floor_delegation_fee(),
                    Error::<T>::InvalidMinDelegationFee
                );
                params.delegation_fee = delegation_fee;
            }
            Self::do_update_module(origin, netuid, params)
        }

        #[pallet::weight((Weight::zero(), DispatchClass::Normal, Pays::No))]
        pub fn register(
            origin: OriginFor<T>,
            network: Vec<u8>,
            name: Vec<u8>,
            address: Vec<u8>,
            stake: u64,
            module_key: T::AccountId,
        ) -> DispatchResult {
            Self::do_register(origin, network, name, address, stake, module_key)
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
        pub fn update_global(
            origin: OriginFor<T>,
            burn_rate: u16,
            max_allowed_modules: u16,
            max_allowed_subnets: u16,
            max_name_length: u16,
            max_proposals: u64,
            max_registrations_per_block: u16,
            min_burn: u64,
            max_burn: u64,
            min_stake: u64,
            min_weight_stake: u64,
            tx_rate_limit: u64,
            unit_emission: u64,
            vote_mode: Vec<u8>,
            vote_threshold: u16,
            adjustment_alpha: u64,
            floor_delegation_fee: Percent,
            target_registrations_per_interval: u16,
            target_registrations_interval: u16,
        ) -> DispatchResult {
            let mut params = Self::global_params();

            params.burn_rate = burn_rate;
            params.max_allowed_modules = max_allowed_modules;
            params.max_allowed_subnets = max_allowed_subnets;
            params.max_name_length = max_name_length;
            params.max_proposals = max_proposals;
            params.max_registrations_per_block = max_registrations_per_block;
            params.min_burn = min_burn;
            params.max_burn = max_burn;
            params.min_stake = min_stake;
            params.min_weight_stake = min_weight_stake;
            params.tx_rate_limit = tx_rate_limit;
            params.unit_emission = unit_emission;
            params.vote_mode = vote_mode;
            params.vote_threshold = vote_threshold;
            params.adjustment_alpha = adjustment_alpha;
            params.floor_delegation_fee = floor_delegation_fee;
            params.target_registrations_per_interval = target_registrations_per_interval;
            params.target_registrations_interval = target_registrations_interval;

            // Check if the parameters are valid
            Self::check_global_params(params.clone())?;

            // if so update them
            Self::do_update_global(origin, params)
        }

        #[pallet::weight((Weight::zero(), DispatchClass::Normal, Pays::No))]
        pub fn add_global_proposal(
            origin: OriginFor<T>,
            // params
            burn_rate: u16,
            max_name_length: u16,
            max_allowed_subnets: u16,
            max_allowed_modules: u16,
            max_proposals: u64,
            max_registrations_per_block: u16,
            min_burn: u64,
            min_stake: u64,
            min_weight_stake: u64,
            unit_emission: u64,
            tx_rate_limit: u64,
            vote_threshold: u16,
            vote_mode: Vec<u8>,
        ) -> DispatchResult {
            let mut params = Self::global_params();

            params.burn_rate = burn_rate;
            params.max_allowed_modules = max_allowed_modules;
            params.max_allowed_subnets = max_allowed_subnets;
            params.max_name_length = max_name_length;
            params.max_proposals = max_proposals;
            params.max_registrations_per_block = max_registrations_per_block;
            params.min_burn = min_burn;
            params.min_stake = min_stake;
            params.min_weight_stake = min_weight_stake;
            params.tx_rate_limit = tx_rate_limit;
            params.unit_emission = unit_emission;
            params.vote_mode = vote_mode;
            params.vote_threshold = vote_threshold;

            Self::do_add_global_proposal(origin, params)
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
            self_vote: bool,
            tempo: u16,
            trust_ratio: u16,
            vote_mode: Vec<u8>,
            vote_threshold: u16,
        ) -> DispatchResult {
            let mut params = Self::subnet_params(netuid);
            params.founder = founder;
            params.founder_share = founder_share;
            params.immunity_period = immunity_period;
            params.incentive_ratio = incentive_ratio;
            params.max_allowed_uids = max_allowed_uids;
            params.max_allowed_weights = max_allowed_weights;
            params.max_stake = max_stake;
            params.min_allowed_weights = min_allowed_weights;
            params.max_weight_age = max_weight_age;
            params.min_stake = min_stake;
            params.name = name;
            params.self_vote = self_vote;
            params.tempo = tempo;
            params.trust_ratio = trust_ratio;
            params.vote_mode = vote_mode;
            params.vote_threshold = vote_threshold;

            // Check if subnet parameters are valid
            Self::check_subnet_params(params.clone())?;

            // if so update them
            Self::do_update_subnet(origin, netuid, params)
        }

        #[pallet::weight((Weight::zero(), DispatchClass::Normal, Pays::No))]
        pub fn add_subnet_proposal(
            origin: OriginFor<T>,
            netuid: u16, // FOR SUBNET PROPOSAL ONLY
            founder: T::AccountId,
            founder_share: u16,
            immunity_period: u16,
            incentive_ratio: u16,
            max_allowed_uids: u16,
            max_allowed_weights: u16,
            max_stake: u64,
            max_weight_age: u64,
            min_allowed_weights: u16,
            min_stake: u64,
            name: Vec<u8>,
            self_vote: bool,
            tempo: u16,
            trust_ratio: u16,
            vote_mode: Vec<u8>,
            vote_threshold: u16,
        ) -> DispatchResult {
            let mut params = Self::subnet_params(netuid);
            params.founder = founder;
            params.founder_share = founder_share;
            params.immunity_period = immunity_period;
            params.incentive_ratio = incentive_ratio;
            params.max_allowed_uids = max_allowed_uids;
            params.max_allowed_weights = max_allowed_weights;
            params.max_stake = max_stake;
            params.max_weight_age = max_weight_age;
            params.min_allowed_weights = min_allowed_weights;
            params.min_stake = min_stake;
            params.name = name;
            params.self_vote = self_vote;
            params.tempo = tempo;
            params.trust_ratio = trust_ratio;
            params.vote_mode = vote_mode;
            params.vote_threshold = vote_threshold;

            Self::do_add_subnet_proposal(origin, netuid, params)
        }

        #[pallet::weight((Weight::zero(), DispatchClass::Normal, Pays::No))]
        pub fn add_custom_proposal(origin: OriginFor<T>, data: Vec<u8>) -> DispatchResult {
            Self::do_add_custom_proposal(origin, data)
        }

        #[pallet::weight((Weight::zero(), DispatchClass::Normal, Pays::No))]
        pub fn vote_proposal(origin: OriginFor<T>, proposal_id: u64) -> DispatchResult {
            Self::do_vote_proposal(origin, proposal_id)
        }

        #[pallet::weight((Weight::zero(), DispatchClass::Normal, Pays::No))]
        pub fn unvote_proposal(origin: OriginFor<T>) -> DispatchResult {
            Self::do_unregister_voter(origin)
        }
    }

    // ---- Subspace helper functions.
    impl<T: Config> Pallet<T> {
        // --- Returns the transaction priority for setting weights.
        pub fn get_priority_set_weights(key: &T::AccountId, netuid: u16) -> u64 {
            if Uids::<T>::contains_key(netuid, key) {
                let uid: u16 = Self::get_uid_for_key(netuid, &key.clone());
                let current_block_number: u64 = Self::get_current_block_as_u64();
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
        pub fn get_priority_balance(key: &T::AccountId) -> u64 {
            Self::get_balance_u64(key)
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
        let current_block_number: u64 = Pallet::<T>::get_current_block_as_u64();
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
        match call.is_sub_type() {
            Some(Call::add_stake { .. }) => {
                let transaction_fee = 0;
                Ok((CallType::AddStake, transaction_fee, who.clone()))
            }
            Some(Call::add_stake_multiple { .. }) => {
                let transaction_fee = 0;
                Ok((CallType::AddStakeMultiple, transaction_fee, who.clone()))
            }
            Some(Call::remove_stake { .. }) => {
                let transaction_fee = 0;
                Ok((CallType::RemoveStake, transaction_fee, who.clone()))
            }
            Some(Call::remove_stake_multiple { .. }) => {
                let transaction_fee = 0;
                Ok((CallType::RemoveStakeMultiple, transaction_fee, who.clone()))
            }
            Some(Call::transfer_stake { .. }) => {
                let transaction_fee = 0;
                Ok((CallType::TransferStake, transaction_fee, who.clone()))
            }
            Some(Call::transfer_multiple { .. }) => {
                let transaction_fee = 0;
                Ok((CallType::TransferMultiple, transaction_fee, who.clone()))
            }
            Some(Call::set_weights { .. }) => {
                let transaction_fee = 0;
                Ok((CallType::SetWeights, transaction_fee, who.clone()))
            }
            Some(Call::register { .. }) => {
                let transaction_fee = 0;
                Ok((CallType::Register, transaction_fee, who.clone()))
            }
            Some(Call::update_module { .. }) => {
                let transaction_fee = 0;
                Ok((CallType::Serve, transaction_fee, who.clone()))
            }
            _ => {
                let transaction_fee = 0;
                Ok((CallType::Other, transaction_fee, who.clone()))
            }
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
