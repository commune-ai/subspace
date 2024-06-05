#![allow(deprecated, non_camel_case_types, non_snake_case)]
#![cfg_attr(not(feature = "std"), no_std)]
#![recursion_limit = "512"]

use crate::subnet::SubnetChangeset;
use frame_system::{self as system, ensure_signed};
pub use pallet::*;
use scale_info::TypeInfo;
use sp_std::collections::{btree_map::BTreeMap, btree_set::BTreeSet};

pub mod migrations;

use frame_support::{
    dispatch,
    dispatch::{DispatchInfo, PostDispatchInfo},
    ensure,
    traits::{tokens::WithdrawReasons, Currency, ExistenceRequirement, IsSubType},
    PalletId,
};

use frame_support::{pallet_prelude::Weight, sp_runtime::transaction_validity::ValidTransaction};
use parity_scale_codec::{Decode, Encode};
use sp_runtime::{
    traits::{DispatchInfoOf, Dispatchable, PostDispatchInfoOf, SignedExtension},
    transaction_validity::{TransactionValidity, TransactionValidityError},
};
use sp_std::marker::PhantomData;

// ---------------------------------
//	Benchmark Imports
// ---------------------------------

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

// ---------------------------------
// Pallet Imports
// ---------------------------------

pub mod global;
mod math;
pub mod module;
mod profit_share;
mod registration;
mod set_weights;
mod staking;
mod step;
pub mod subnet;
pub mod weights; // Weight benchmarks // Commune consensus weights

#[cfg(debug_assertions)]
pub use step::yuma;
// TODO: better error handling in whole file

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
    use frame_support::{pallet_prelude::*, traits::Currency, Identity};
    use frame_system::pallet_prelude::*;
    use global::BurnConfiguration;
    use module::ModuleChangeset;
    use pallet_governance_api::{GovernanceConfiguration, VoteMode};
    use sp_arithmetic::per_things::Percent;
    pub use sp_std::{vec, vec::Vec};

    const STORAGE_VERSION: StorageVersion = StorageVersion::new(11);

    #[pallet::pallet]
    #[pallet::storage_version(STORAGE_VERSION)]
    #[pallet::without_storage_info]
    pub struct Pallet<T>(_);

    // Configure the pallet by specifying the parameters and types on which it depends.
    #[pallet::config(with_default)]
    pub trait Config:
        frame_system::Config
        + pallet_governance_api::GovernanceApi<<Self as frame_system::Config>::AccountId>
    {
        /// This pallet's ID, used for generating the treasury account ID.
        #[pallet::constant]
        type PalletId: Get<PalletId>;

        /// Because this pallet emits events, it depends on the runtime's definition of an event.
        #[pallet::no_default_bounds]
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// Currency type that will be used to place deposits on modules.
        type Currency: Currency<Self::AccountId> + Send + Sync;

        /// The weight information of this pallet.
        type WeightInfo: WeightInfo;
    }

    pub type BalanceOf<T> =
        <<T as Config>::Currency as Currency<<T as system::Config>::AccountId>>::Balance;

    pub type AccountIdOf<T> = <T as frame_system::Config>::AccountId;

    // ---------------------------------
    // Global Variables
    // ---------------------------------

    #[pallet::storage]
    pub type BurnConfig<T: Config> = StorageValue<_, BurnConfiguration<T>, ValueQuery>;

    #[pallet::type_value]
    pub fn DefaultUnitEmission<T: Config>() -> u64 {
        23148148148
    }
    #[pallet::storage] // --- ITEM ( unit_emission )
    pub type UnitEmission<T> = StorageValue<_, u64, ValueQuery, DefaultUnitEmission<T>>;

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
    pub type MaxNameLength<T: Config> = StorageValue<_, u16, ValueQuery, DefaultMaxNameLength<T>>;

    #[pallet::type_value]
    pub fn DefaultMinNameLength<T: Config>() -> u16 {
        2
    }

    #[pallet::storage]
    pub type MinNameLength<T: Config> = StorageValue<_, u16, ValueQuery, DefaultMinNameLength<T>>;

    #[pallet::type_value]
    pub fn DefaultMaxAllowedSubnets<T: Config>() -> u16 {
        256
    }
    #[pallet::storage] // --- ITEM ( max_allowed_subnets )
    pub type MaxAllowedSubnets<T: Config> =
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

    #[pallet::storage]
    pub type MaximumSetWeightCallsPerEpoch<T: Config> =
        StorageMap<_, Identity, u16, u16, ValueQuery>;

    #[pallet::storage]
    pub type SetWeightCallsPerEpoch<T: Config> =
        StorageDoubleMap<_, Identity, u16, Identity, T::AccountId, u16, ValueQuery>;

    #[derive(Decode, Encode, PartialEq, Eq, Clone, Debug, TypeInfo)]
    #[scale_info(skip_type_params(T))]
    pub struct ModuleParams<T: Config> {
        pub name: Vec<u8>,
        pub address: Vec<u8>,
        pub delegation_fee: Percent,
        pub metadata: Option<Vec<u8>>,
        pub controller: T::AccountId,
    }

    #[derive(
        Decode, Encode, PartialEq, Eq, Clone, TypeInfo, frame_support::DebugNoBound, MaxEncodedLen,
    )]
    #[scale_info(skip_type_params(T))]
    pub struct GlobalParams<T: Config> {
        // max
        pub max_name_length: u16,             // max length of a network name
        pub min_name_length: u16,             // min length of a network name
        pub max_allowed_subnets: u16,         // max number of subnets allowed
        pub max_allowed_modules: u16,         // max number of modules allowed per subnet
        pub max_registrations_per_block: u16, // max number of registrations per block
        pub max_allowed_weights: u16,         // max number of weights per module

        // mins
        pub floor_delegation_fee: Percent, // min delegation fee
        pub floor_founder_share: u8,       // min founder share
        pub min_weight_stake: u64,         // min weight stake required

        // S0 governance
        pub curator: T::AccountId,
        pub general_subnet_application_cost: u64,

        // Other
        pub subnet_stake_threshold: Percent,
        pub burn_config: BurnConfiguration<T>,
        pub governance_config: GovernanceConfiguration,
    }

    // ---------------------------------
    // Subnet PARAMS
    // ---------------------------------

    pub struct DefaultSubnetParams<T: Config>(sp_std::marker::PhantomData<((), T)>);

    impl<T: Config> DefaultSubnetParams<T> {
        pub fn get() -> SubnetParams<T> {
            SubnetParams {
                name: BoundedVec::default(),
                tempo: DefaultTempo::<T>::get(),
                immunity_period: DefaultImmunityPeriod::<T>::get(),
                min_allowed_weights: DefaultMinAllowedWeights::<T>::get(),
                max_allowed_weights: DefaultMaxAllowedWeights::<T>::get(),
                max_allowed_uids: DefaultMaxAllowedUids::<T>::get(),
                max_weight_age: DefaultMaxWeightAge::<T>::get(),
                trust_ratio: GetDefault::get(),
                founder_share: FloorFounderShare::<T>::get() as u16,
                incentive_ratio: DefaultIncentiveRatio::<T>::get(),
                min_stake: 0,
                founder: DefaultKey::<T>::get(),
                maximum_set_weight_calls_per_epoch: 0,
                bonds_ma: DefaultBondsMovingAverage::<T>::get(),
                target_registrations_interval: DefaultTargetRegistrationsInterval::<T>::get(),
                target_registrations_per_interval: DefaultTargetRegistrationsPerInterval::<T>::get(
                ),
                max_registrations_per_interval: 42,
                adjustment_alpha: DefaultAdjustmentAlpha::<T>::get(),
                governance_config: GovernanceConfiguration {
                    vote_mode: VoteMode::Authority,
                    ..Default::default()
                },
            }
        }
    }

    #[derive(
        Decode, Encode, PartialEq, Eq, Clone, frame_support::DebugNoBound, TypeInfo, MaxEncodedLen,
    )]
    #[scale_info(skip_type_params(T))]
    pub struct SubnetParams<T: Config> {
        // --- parameters
        pub founder: T::AccountId,
        pub founder_share: u16,    // out of 100
        pub immunity_period: u16,  // immunity period
        pub incentive_ratio: u16,  // out of 100
        pub max_allowed_uids: u16, // Max allowed modules on a subnet
        pub max_allowed_weights: u16, /* max number of weights allowed to be registered in this
                                    * pub max_allowed_uids: u16, // max number of uids
                                    * allowed to be registered in this subnet */
        pub min_allowed_weights: u16, // min number of weights allowed to be registered in this
        pub max_weight_age: u64,      // max age of a weight
        pub min_stake: u64,           // min stake required
        pub name: BoundedVec<u8, ConstU32<256>>,
        pub tempo: u16, // how many blocks to wait before rewarding models
        pub trust_ratio: u16,
        pub maximum_set_weight_calls_per_epoch: u16,
        // consensus
        pub bonds_ma: u64,
        // registrations
        pub target_registrations_interval: u16,
        pub target_registrations_per_interval: u16,
        pub max_registrations_per_interval: u16,
        pub adjustment_alpha: u64,

        pub governance_config: GovernanceConfiguration,
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
        0
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

    #[pallet::storage] // --- MAP ( netuid ) --> min_allowed_weights
    pub type MinStake<T> = StorageMap<_, Identity, u16, u64, ValueQuery>;

    // Registration parameters
    // =======================

    #[pallet::type_value]
    pub fn DefaultTargetRegistrationsPerInterval<T: Config>() -> u16 {
        DefaultTargetRegistrationsInterval::<T>::get() / 2
    }
    #[pallet::storage] // MAP ( netuid ) --> trarget_registrations_per_interval
    pub type TargetRegistrationsPerInterval<T> =
        StorageMap<_, Identity, u16, u16, ValueQuery, DefaultTargetRegistrationsPerInterval<T>>;

    #[pallet::type_value]
    pub fn DefaultTargetRegistrationsInterval<T: Config>() -> u16 {
        DefaultTempo::<T>::get() * 2 // 2 times the epoch
    }
    #[pallet::storage] // --- MAP ( netuid ) --> trarget_registrations_interval
    pub type TargetRegistrationsInterval<T> =
        StorageMap<_, Identity, u16, u16, ValueQuery, DefaultTargetRegistrationsInterval<T>>;

    #[pallet::type_value]
    pub fn DefaultMaxRegistrationsPerInterval<T: Config>() -> u16 {
        42
    }
    #[pallet::storage] // --- MAP ( netuid ) --> trarget_registrations_interval
    pub type MaxRegistrationsPerInterval<T> =
        StorageMap<_, Identity, u16, u16, ValueQuery, DefaultMaxRegistrationsPerInterval<T>>;

    #[pallet::type_value]
    pub fn DefaultAdjustmentAlpha<T: Config>() -> u64 {
        u64::MAX / 2
    }
    #[pallet::storage] // --- MAP ( netuid ) --> adjustment_alpha
    pub type AdjustmentAlpha<T> =
        StorageMap<_, Identity, u16, u64, ValueQuery, DefaultAdjustmentAlpha<T>>;

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

    #[pallet::storage] // --- DMAP ( key, netuid ) --> bool
    pub type Founder<T: Config> =
        StorageMap<_, Identity, u16, T::AccountId, ValueQuery, DefaultKey<T>>;

    #[pallet::storage] // --- DMAP ( key, proportion )
    pub type FounderShare<T: Config> =
        StorageMap<_, Identity, u16, u16, ValueQuery, DefaultFounderShare<T>>;

    #[pallet::type_value]
    pub fn DefaultFounderShare<T: Config>() -> u16 {
        FloorFounderShare::<T>::get() as u16
    }

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

    // ---------------------------------
    // Voting
    // ---------------------------------

    #[pallet::type_value]
    pub fn DefaultCurator<T: Config>() -> T::AccountId {
        T::AccountId::decode(&mut sp_runtime::traits::TrailingZeroInput::zeroes()).unwrap()
    }

    #[pallet::type_value]
    pub fn DefaultFloorFounderShare<T: Config>() -> u8 {
        8
    }

    #[pallet::storage]
    pub type FloorFounderShare<T: Config> =
        StorageValue<_, u8, ValueQuery, DefaultFloorFounderShare<T>>;

    #[pallet::storage] // --- ITEM( tota_number_of_existing_networks )
    pub type TotalSubnets<T> = StorageValue<_, u16, ValueQuery>;

    #[pallet::storage] // --- MAP( netuid ) --> subnet_emission
    pub type SubnetEmission<T> = StorageMap<_, Identity, u16, u64, ValueQuery>;

    #[pallet::storage] // --- MAP ( netuid ) --> subnetwork_n (Number of UIDs in the network).
    pub type N<T> = StorageMap<_, Identity, u16, u16, ValueQuery>;

    #[pallet::storage] // --- MAP ( netuid ) --> pending_emission
    pub type PendingEmission<T> = StorageMap<_, Identity, u16, u64, ValueQuery>;

    #[pallet::storage] // --- MAP ( network_name ) --> netuid
    pub type SubnetNames<T: Config> = StorageMap<_, Identity, u16, Vec<u8>, ValueQuery>;

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

    #[pallet::storage] // --- DMAP ( netuid, uid ) --> block number that the module is registered
    pub type RegistrationBlock<T: Config> =
        StorageDoubleMap<_, Identity, u16, Identity, u16, u64, ValueQuery>;

    // ---------------------------------
    //  Module Staking Variables
    /// ---------------------------------

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
    pub type SubnetGaps<T> = StorageValue<_, BTreeSet<u16>, ValueQuery>;

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

    #[pallet::storage] // --- DMAP ( netuid, uid ) --> weights
    pub type Weights<T: Config> =
        StorageDoubleMap<_, Identity, u16, Identity, u16, Vec<(u16, u16)>, ValueQuery>;

    // ---------------------------------
    // Event Variables
    // ---------------------------------

    #[pallet::event]
    #[pallet::generate_deposit(pub fn deposit_event)]
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
        ModuleUpdated(u16, T::AccountId), /* --- Event created when the module got updated
                                           * information is added to the network. */

        // faucet
        Faucet(T::AccountId, BalanceOf<T>), // (id, balance_to_add)

        //voting
        ProposalVoted(u64, T::AccountId, bool), // (id, voter, vote)
        ProposalVoteUnregistered(u64, T::AccountId), // (id, voter)
        GlobalParamsUpdated(GlobalParams<T>),   /* --- Event created when global
                                                 * parameters are
                                                 * updated */
        SubnetParamsUpdated(u16), // --- Event created when subnet parameters are updated
        GlobalProposalAccepted(u64), // (id)
        CustomProposalAccepted(u64), // (id)
        SubnetProposalAccepted(u64, u16), // (id, netuid)
    }

    // ---------------------------------
    // Error Variables
    // ---------------------------------

    // Errors inform users that something went wrong.
    #[pallet::error]
    pub enum Error<T> {
        NetworkDoesNotExist, // --- Thrown when the network does not exist.
        NetworkIsImmuned,
        NotRegistered, // module which does not exist in the active set.
        NotEnoughStakeToWithdraw, /* ---- Thrown when the caller requests removing more stake
                        * then there exists in the staking account. See: fn
                        * remove_stake. */
        NotEnoughBalanceToStake, /*  ---- Thrown when the caller requests adding more stake
                                  * than there exists in the cold key account. See: fn
                                  * add_stake */
        WeightVecNotEqualSize, /* ---- Thrown when the caller attempts to set the weight keys
                                * and values but these vectors have different size. */
        DuplicateUids, /* ---- Thrown when the caller attempts to set weights with duplicate
                        * uids in the weight matrix. */
        InvalidUid, /* ---- Thrown when a caller attempts to set weight to at least one uid
                     * that does not exist in the metagraph. */
        InvalidUidsLength, /* ---- Thrown when the caller attempts to set weights with a
                            * different number of uids than allowed. */
        TooManyRegistrationsPerBlock, /* ---- Thrown when registrations this block exceeds
                                       * allowed number. */
        TooManyRegistrationsPerInterval, /* ---- Thrown when registrations this interval
                                          * exceeds
                                          * allowed number. */
        AlreadyRegistered, /* ---- Thrown when the caller requests registering a module which
                            * already exists in the active set. */
        CouldNotConvertToBalance, /* ---- Thrown when the dispatch attempts to convert between
                                   * a u64 and T::balance but the call fails. */
        InvalidTempo, // --- Thrown when epoch is not valid
        SettingWeightsTooFast, /* --- Thrown if the key attempts to set weights twice withing
                       * net_epoch/2 blocks. */
        InvalidMaxAllowedUids, /* --- Thrown when the user tries to set max allowed uids to a
                                * value less than the current number of registered uids. */
        NetuidDoesNotExist,
        SubnetNameAlreadyExists,
        SubnetNameTooShort,
        SubnetNameTooLong,
        InvalidSubnetName,
        BalanceNotAdded,
        StakeNotRemoved,
        KeyAlreadyRegistered,
        EmptyKeys,
        TooManyKeys,
        InvalidShares,
        ProfitSharesNotAdded,
        NotFounder,
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
        InvalidMinBurn,
        InvalidMaxBurn,

        // Faucet
        FaucetDisabled, // --- Thrown when the faucet is disabled.
        InvalidDifficulty,
        InvalidWorkBlock,
        InvalidSeal,

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
        /// A module with this name already exists in the subnet.
        ModuleNameAlreadyExists,

        // VOTING
        ProposalNotFound,
        InvalidProposalStatus,
        AlreadyVoted,
        InvalidVoteMode,
        InvalidFounderShare,
        InvalidIncentiveRatio,
        InvalidProposalCost,
        InvalidGeneralSubnetApplicationCost,
        InvalidProposalExpiration,
        InvalidProposalParticipationThreshold,
        InsufficientStake,
        VoteNotFound,
        InvalidProposalCustomData,
        ProposalCustomDataTooSmall,
        ProposalCustomDataTooLarge,

        // Other
        InvalidMaxWeightAge,
        MaximumSetWeightsPerEpochReached,
        InsufficientDaoTreasuryFunds,
        // Registrations
        InvalidTargetRegistrationsPerInterval,
        InvalidMaxRegistrationsPerInterval,
        InvalidAdjustmentAlpha,
        InvalidTargetRegistrationsInterval,
    }

    // ---------------------------------
    // Genesis
    // ---------------------------------

    #[derive(frame_support::DefaultNoBound)]
    #[pallet::genesis_config]
    pub struct GenesisConfig<T: Config> {
        // key, name, address, weights
        pub modules: Vec<Vec<(T::AccountId, Vec<u8>, Vec<u8>, Vec<(u16, u16)>)>>,
        // name, tempo, immunity_period, min_allowed_weight, max_allowed_weight, max_allowed_uids,
        // immunity_ratio, founder
        pub subnets: Vec<(Vec<u8>, u16, u16, u16, u16, u16, u64, T::AccountId)>,

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
                    name: subnet.0.clone().try_into().expect("subnet name is too long"),
                    tempo: subnet.1,
                    immunity_period: subnet.2,
                    min_allowed_weights: subnet.3,
                    max_allowed_weights: subnet.4,
                    max_allowed_uids: subnet.5,
                    min_stake: subnet.6,
                    founder: subnet.7.clone(),
                    ..DefaultSubnetParams::<T>::get()
                };

                let fee = DelegationFee::<T>::get(netuid, &params.founder);
                let changeset: SubnetChangeset<T> =
                    SubnetChangeset::new(params).expect("genesis subnets are valid");
                let _ = self::Pallet::<T>::add_subnet(changeset, Some(netuid))
                    .expect("Failed to register genesis subnet");

                if let Some(modules) = self.modules.get(subnet_idx) {
                    for (uid_usize, (key, name, address, weights)) in modules.iter().enumerate() {
                        let changeset =
                            ModuleChangeset::new(name.clone(), address.clone(), fee, None);
                        self::Pallet::<T>::append_module(netuid, key, changeset)
                            .expect("genesis modules are valid");
                        Weights::<T>::insert(netuid, uid_usize as u16, weights);
                    }
                }
            }
            // Now we can add the stake to the network

            let subnet_stakes = self
                .subnets
                .iter()
                .enumerate()
                .filter_map(|(subnet_id, _)| Some((subnet_id, self.stake_to.get(subnet_id)?)));
            for (subnet_id, stakes) in subnet_stakes {
                for (key, stake_to) in stakes {
                    for (module_key, stake_amount) in stake_to {
                        Pallet::<T>::increase_stake(
                            subnet_id as u16,
                            key,
                            module_key,
                            *stake_amount,
                        );
                    }
                }
            }
        }
    }

    // ---------------------------------
    // Hooks
    // ---------------------------------

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

    // ---------------------------------
    // Extrinsics
    // ---------------------------------

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        // ---------------------------------
        // Consensus operations
        // ---------------------------------

        #[pallet::call_index(0)]
        #[pallet::weight((T::WeightInfo::set_weights(), DispatchClass::Normal, Pays::No))]
        pub fn set_weights(
            origin: OriginFor<T>,
            netuid: u16,
            uids: Vec<u16>,
            weights: Vec<u16>,
        ) -> DispatchResult {
            Self::do_set_weights(origin, netuid, uids, weights)
        }

        // ---------------------------------
        // Stake operations
        // ---------------------------------

        #[pallet::call_index(1)]
        #[pallet::weight((T::WeightInfo::add_stake(), DispatchClass::Normal, Pays::No))]
        pub fn add_stake(
            origin: OriginFor<T>,
            netuid: u16,
            module_key: T::AccountId,
            amount: u64,
        ) -> DispatchResult {
            Self::do_add_stake(origin, netuid, module_key, amount)
        }

        #[pallet::call_index(2)]
        #[pallet::weight((T::WeightInfo::remove_stake(), DispatchClass::Normal, Pays::No))]
        pub fn remove_stake(
            origin: OriginFor<T>,
            netuid: u16,
            module_key: T::AccountId,
            amount: u64,
        ) -> DispatchResult {
            Self::do_remove_stake(origin, netuid, module_key, amount)
        }

        // ---------------------------------
        // Bulk stake operations
        // ---------------------------------

        #[pallet::call_index(3)]
        #[pallet::weight((T::WeightInfo::add_stake_multiple(), DispatchClass::Normal, Pays::No))]
        pub fn add_stake_multiple(
            origin: OriginFor<T>,
            netuid: u16,
            module_keys: Vec<T::AccountId>,
            amounts: Vec<u64>,
        ) -> DispatchResult {
            Self::do_add_stake_multiple(origin, netuid, module_keys, amounts)
        }

        #[pallet::call_index(4)]
        #[pallet::weight((T::WeightInfo::remove_stake_multiple(), DispatchClass::Normal, Pays::No))]
        pub fn remove_stake_multiple(
            origin: OriginFor<T>,
            netuid: u16,
            module_keys: Vec<T::AccountId>,
            amounts: Vec<u64>,
        ) -> DispatchResult {
            Self::do_remove_stake_multiple(origin, netuid, module_keys, amounts)
        }

        // ---------------------------------
        // Transfers
        // ---------------------------------

        #[pallet::call_index(5)]
        #[pallet::weight((T::WeightInfo::transfer_stake(), DispatchClass::Normal, Pays::No))]
        pub fn transfer_stake(
            origin: OriginFor<T>,         // --- The account that is calling this function.
            netuid: u16,                  // --- The network id.
            module_key: T::AccountId,     // --- The module key.
            new_module_key: T::AccountId, // --- The new module key.
            amount: u64,                  // --- The amount of stake to transfer.
        ) -> DispatchResult {
            Self::do_transfer_stake(origin, netuid, module_key, new_module_key, amount)
        }

        #[pallet::call_index(6)]
        #[pallet::weight((T::WeightInfo::transfer_multiple(), DispatchClass::Normal, Pays::No))]
        pub fn transfer_multiple(
            origin: OriginFor<T>, // --- The account that is calling this function.
            destinations: Vec<T::AccountId>, // --- The module key.
            amounts: Vec<u64>,    // --- The amount of stake to transfer.
        ) -> DispatchResult {
            Self::do_transfer_multiple(origin, destinations, amounts)
        }

        // ---------------------------------
        // Registereing / Deregistering
        // ---------------------------------

        #[pallet::call_index(7)]
        #[pallet::weight((T::WeightInfo::register(), DispatchClass::Normal, Pays::No))]
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

        #[pallet::call_index(8)]
        #[pallet::weight((T::WeightInfo::deregister(), DispatchClass::Normal, Pays::No))]
        pub fn deregister(origin: OriginFor<T>, netuid: u16) -> DispatchResult {
            Self::do_deregister(origin, netuid)
        }

        // ---------------------------------
        // Updating
        // ---------------------------------

        #[pallet::call_index(9)]
        #[pallet::weight((T::WeightInfo::deregister(), DispatchClass::Normal, Pays::No))]
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

        #[pallet::call_index(10)]
        #[pallet::weight((T::WeightInfo::update_subnet(), DispatchClass::Normal, Pays::No))]
        pub fn update_subnet(
            origin: OriginFor<T>,
            netuid: u16,
            founder: T::AccountId,
            founder_share: u16,
            immunity_period: u16,
            incentive_ratio: u16,
            max_allowed_uids: u16,
            max_allowed_weights: u16,
            min_allowed_weights: u16,
            max_weight_age: u64,
            min_stake: u64,
            name: BoundedVec<u8, ConstU32<256>>,
            tempo: u16,
            trust_ratio: u16,
            maximum_set_weight_calls_per_epoch: u16,
            vote_mode: VoteMode,
            bonds_ma: u64,
            target_registrations_interval: u16,
            target_registrations_per_interval: u16,
            max_registrations_per_interval: u16,
            adjustment_alpha: u64,
        ) -> DispatchResult {
            let params = SubnetParams {
                founder,
                founder_share,
                immunity_period,
                incentive_ratio,
                max_allowed_uids,
                max_allowed_weights,
                min_allowed_weights,
                max_weight_age,
                min_stake,
                name,
                tempo,
                trust_ratio,
                maximum_set_weight_calls_per_epoch,
                bonds_ma,
                target_registrations_interval,
                target_registrations_per_interval,
                max_registrations_per_interval,
                adjustment_alpha,
                governance_config: GovernanceConfiguration {
                    vote_mode,
                    ..T::get_subnet_governance_configuration(netuid)
                },
            };

            let changeset = SubnetChangeset::update(netuid, params)?;
            Self::do_update_subnet(origin, netuid, changeset)
        }

        // ---------------------------------
        // Profit sharing
        // ---------------------------------

        #[pallet::call_index(15)]
        #[pallet::weight((T::WeightInfo::add_profit_shares(), DispatchClass::Normal, Pays::No))]
        pub fn add_profit_shares(
            origin: OriginFor<T>,
            keys: Vec<T::AccountId>,
            shares: Vec<u16>,
        ) -> DispatchResult {
            Self::do_add_profit_shares(origin, keys, shares)
        }

        // ---------------------------------
        // Testnet
        // ---------------------------------

        #[pallet::call_index(16)]
        #[pallet::weight((Weight::from_parts(85_000_000, 0)
        .saturating_add(T::DbWeight::get().reads(16))
        .saturating_add(T::DbWeight::get().writes(28)), DispatchClass::Operational, Pays::No))]
        pub fn faucet(
            origin: OriginFor<T>,
            block_number: u64,
            nonce: u64,
            work: Vec<u8>,
        ) -> DispatchResult {
            if cfg!(feature = "testnet-faucet") {
                Self::do_faucet(origin, block_number, nonce, work)
            } else {
                Err(Error::<T>::FaucetDisabled.into())
            }
        }
    }

    // ---- Subspace helper functions.
    impl<T: Config> Pallet<T> {
        // --- Returns the transaction priority for setting weights.
        pub fn get_priority_set_weights(key: &T::AccountId, netuid: u16) -> u64 {
            if let Some(uid) = Uids::<T>::get(netuid, key) {
                let last_update = Self::get_last_update_for_uid(netuid, uid);
                Self::get_current_block_number().saturating_add(last_update)
            } else {
                0
            }
        }
        // --- Returns the transaction priority for setting weights.
        pub fn get_priority_stake(key: &T::AccountId, netuid: u16) -> u64 {
            if Uids::<T>::contains_key(netuid, key) {
                return Self::get_stake(netuid, key);
            }
            0
        }
    }
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
    Update,
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
        let current_block_number = Pallet::<T>::get_current_block_number();
        let balance = Pallet::<T>::get_balance_u64(who);

        // this is the current block number minus the last update block number
        current_block_number.saturating_add(balance)
    }

    pub fn get_priority_set_weights(who: &T::AccountId, netuid: u16) -> u64 {
        // Return the non vanilla priority for a set weights call.
        Pallet::<T>::get_priority_set_weights(who, netuid)
    }

    #[must_use]
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
            Some(Call::update_module { .. }) => Ok((CallType::Update, 0, who)),
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
