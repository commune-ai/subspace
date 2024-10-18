#![allow(deprecated, non_camel_case_types, non_snake_case)]
#![cfg_attr(not(feature = "std"), no_std)]
#![recursion_limit = "512"]

use crate::subnet::SubnetChangeset;
use frame_system::{self as system, ensure_signed};
pub use pallet::*;
use scale_info::TypeInfo;
use sp_std::collections::btree_set::BTreeSet;
pub mod migrations;

use frame_support::{
    dispatch, ensure,
    traits::{tokens::WithdrawReasons, ConstU32, Currency, ExistenceRequirement},
    PalletId,
};

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

pub mod global;
pub mod math;
pub mod module;
mod registration;
pub mod rpc;
mod set_weights;
mod staking;
pub mod subnet;
pub mod weights;

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
    use global::{BurnType, GeneralBurnConfiguration};
    use module::ModuleChangeset;
    use pallet_governance_api::{GovernanceConfiguration, VoteMode};
    use sp_arithmetic::per_things::Percent;
    use sp_core::{ConstU16, ConstU64, ConstU8};
    pub use sp_std::{vec, vec::Vec};

    const STORAGE_VERSION: StorageVersion = StorageVersion::new(14);

    #[pallet::pallet]
    #[pallet::storage_version(STORAGE_VERSION)]
    #[pallet::without_storage_info]
    pub struct Pallet<T>(_);

    // Configure the pallet by specifying the parameters and types on which it depends.
    #[pallet::config(with_default)]
    pub trait Config:
        frame_system::Config
        + pallet_governance_api::GovernanceApi<<Self as frame_system::Config>::AccountId>
        + pallet_subnet_emission_api::SubnetEmissionApi
    {
        /// This pallet's ID, used for generating the treasury account ID.
        #[pallet::constant]
        type PalletId: Get<PalletId>;

        /// Because this pallet emits events, it depends on the runtime's definition of an event.
        #[pallet::no_default_bounds]
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// Currency type that will be used to place deposits on modules.
        type Currency: Currency<Self::AccountId> + Send + Sync;

        /// The default number of modules that can be registered per interval.
        type DefaultMaxRegistrationsPerInterval: Get<u16>;
        /// The default number of subnets that can be registered per interval.
        type DefaultMaxSubnetRegistrationsPerInterval: Get<u16>;
        /// The default minimum burn amount required for module registration.
        type DefaultModuleMinBurn: Get<u64>;
        /// The default minimum burn amount required for module registration.
        type DefaultSubnetMinBurn: Get<u64>;
        type DefaultMinValidatorStake: Get<u64>;

        /// The weight information of this pallet.
        type WeightInfo: WeightInfo;
        /// Should enforce legit whitelist on general subnet
        type EnforceWhitelist: Get<bool>;
    }

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
        pub subnet_immunity_period: u64,
        pub governance_config: GovernanceConfiguration,

        pub kappa: u16,
        pub rho: u16,
    }

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

    // ---------------------------------
    // Subnet PARAMS
    // ---------------------------------

    #[pallet::storage]
    pub type MinValidatorStake<T: Config> =
        StorageMap<_, Identity, u16, u64, ValueQuery, T::DefaultMinValidatorStake>;

    pub struct DefaultSubnetParams<T: Config>(sp_std::marker::PhantomData<((), T)>);

    impl<T: Config> DefaultSubnetParams<T> {
        // TODO: not hardcode values here, get them from the storages instead,
        // if they implement default already.
        pub fn get() -> SubnetParams<T> {
            SubnetParams {
                name: BoundedVec::default(),
                tempo: 100,
                immunity_period: 0,
                min_allowed_weights: 1,
                max_allowed_weights: 420,
                max_allowed_uids: 420,
                max_weight_age: 3_600,
                trust_ratio: GetDefault::get(),
                founder_share: FloorFounderShare::<T>::get() as u16,
                incentive_ratio: 50,
                founder: DefaultKey::<T>::get(),
                maximum_set_weight_calls_per_epoch: 0,
                bonds_ma: 900_000,

                // registrations
                module_burn_config: GeneralBurnConfiguration::<T>::default_for(BurnType::Module),
                min_validator_stake: T::DefaultMinValidatorStake::get(),
                max_allowed_validators: None,
                governance_config: GovernanceConfiguration {
                    vote_mode: VoteMode::Authority,
                    ..Default::default()
                },
                metadata: None,
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
        pub name: BoundedVec<u8, ConstU32<256>>,
        pub metadata: Option<BoundedVec<u8, ConstU32<120>>>,
        pub tempo: u16, // how many blocks to wait before rewarding models
        pub trust_ratio: u16,
        pub maximum_set_weight_calls_per_epoch: u16,
        // consensus
        pub bonds_ma: u64,
        pub module_burn_config: GeneralBurnConfiguration<T>,
        pub min_validator_stake: u64,
        pub max_allowed_validators: Option<u16>,
        pub governance_config: GovernanceConfiguration,
    }

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
    pub type TrustRatio<T> = StorageMap<_, Identity, u16, u16, ValueQuery>;

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

    #[derive(Decode, Encode, PartialEq, Eq, Clone, Debug, TypeInfo)]
    #[scale_info(skip_type_params(T))]
    pub struct ModuleParams<T: Config> {
        pub name: Vec<u8>,
        pub address: Vec<u8>,
        pub delegation_fee: Percent,
        pub metadata: Option<Vec<u8>>,
        pub controller: T::AccountId,
    }

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

    #[pallet::storage] // --- DMAP ( netuid, uid ) --> (u64, weights)
    pub type Weights<T: Config> =
        StorageDoubleMap<_, Identity, u16, Identity, u16, Vec<(u16, u16)>, ValueQuery>;

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

    // ---------------------------------
    // Event Variables
    // ---------------------------------

    #[pallet::event]
    #[pallet::generate_deposit(pub fn deposit_event)]
    pub enum Event<T: Config> {
        /// Event created when a new network is added
        NetworkAdded(u16, Vec<u8>),
        /// Event created when a network is removed
        NetworkRemoved(u16),
        /// Event created when stake has been transferred from the coldkey account onto the key
        /// staking account
        StakeAdded(T::AccountId, T::AccountId, u64),
        /// Event created when stake has been removed from the key staking account onto the coldkey
        /// account
        StakeRemoved(T::AccountId, T::AccountId, u64),
        /// Event created when a caller successfully sets their weights on a subnetwork
        WeightsSet(u16, u16),
        /// Event created when a new module account has been registered to the chain
        ModuleRegistered(u16, u16, T::AccountId),
        /// Event created when a module account has been deregistered from the chain
        ModuleDeregistered(u16, u16, T::AccountId),
        /// Event created when the module's updated information is added to the network
        ModuleUpdated(u16, T::AccountId),
        // Parameter Updates
        /// Event created when global parameters are updated
        GlobalParamsUpdated(GlobalParams<T>),
        /// Event created when subnet parameters are updated
        SubnetParamsUpdated(u16),
    }

    // ---------------------------------
    // Error Variables
    // ---------------------------------

    // Errors inform users about failures
    #[pallet::error]
    pub enum Error<T> {
        /// The specified network does not exist.
        NetworkDoesNotExist,
        /// The specified module does not exist.
        ModuleDoesNotExist,
        /// The network is immune to changes.
        NetworkIsImmuned,
        /// Insufficient balance to register a subnet.
        NotEnoughBalanceToRegisterSubnet,
        /// Insufficient stake to withdraw the requested amount.
        NotEnoughStakeToWithdraw,
        /// Insufficient balance in the cold key account to stake the requested amount.
        NotEnoughBalanceToStake,
        /// The weight vectors for keys and values have different sizes.
        WeightVecNotEqualSize,
        /// Duplicate UIDs detected in the weight matrix.
        DuplicateUids,
        /// At least one UID in the weight matrix does not exist in the metagraph.
        InvalidUid,
        /// The number of UIDs in the weight matrix is different from the allowed amount.
        InvalidUidsLength,
        /// The number of registrations in this block exceeds the allowed limit.
        TooManyRegistrationsPerBlock,
        /// The number of registrations in this interval exceeds the allowed limit.
        TooManyRegistrationsPerInterval,
        /// The number of subnet registrations in this interval exceeds the allowed limit.
        TooManySubnetRegistrationsPerInterval,
        /// The module is already registered in the active set.
        AlreadyRegistered,
        /// Failed to convert between u64 and T::Balance.
        CouldNotConvertToBalance,
        /// The specified tempo (epoch) is not valid.
        InvalidTempo,
        /// Attempted to set weights twice within net_epoch/2 blocks.
        SettingWeightsTooFast,
        /// Attempted to set max allowed UIDs to a value less than the current number of registered
        /// UIDs.
        InvalidMaxAllowedUids,
        /// The specified netuid does not exist.
        NetuidDoesNotExist,
        /// A subnet with the given name already exists.
        SubnetNameAlreadyExists,
        /// The subnet name is too short.
        SubnetNameTooShort,
        /// The subnet name is too long.
        SubnetNameTooLong,
        /// The subnet name contains invalid characters.
        InvalidSubnetName,
        /// Failed to add balance to the account.
        BalanceNotAdded,
        /// Failed to remove stake from the account.
        StakeNotRemoved,
        /// The key is already registered.
        KeyAlreadyRegistered,
        /// No keys provided (empty key set).
        EmptyKeys,
        /// Too many keys provided.
        TooManyKeys,
        /// Invalid shares distribution.
        InvalidShares,
        /// The caller is not the founder of the subnet.
        NotFounder,
        /// Insufficient stake to set weights.
        NotEnoughStakeToSetWeights,
        /// Insufficient stake to start a network.
        NotEnoughStakeToStartNetwork,
        /// Insufficient stake per weight.
        NotEnoughStakePerWeight,
        /// No self-weight provided.
        NoSelfWeight,
        /// Vectors have different lengths.
        DifferentLengths,
        /// Insufficient balance to register.
        NotEnoughBalanceToRegister,
        /// Failed to add stake to the account.
        StakeNotAdded,
        /// Failed to remove balance from the account.
        BalanceNotRemoved,
        /// Balance could not be removed from the account.
        BalanceCouldNotBeRemoved,
        /// Insufficient stake to register.
        NotEnoughStakeToRegister,
        /// The entity is still registered and cannot be modified.
        StillRegistered,
        /// Attempted to set max allowed modules to a value less than the current number of
        /// registered modules.
        MaxAllowedModules,
        /// Insufficient balance to transfer.
        NotEnoughBalanceToTransfer,
        /// The system is not in vote mode.
        NotVoteMode,
        /// The trust ratio is invalid.
        InvalidTrustRatio,
        /// The minimum allowed weights value is invalid.
        InvalidMinAllowedWeights,
        /// The maximum allowed weights value is invalid.
        InvalidMaxAllowedWeights,
        /// The minimum delegation fee is invalid.
        InvalidMinDelegationFee,
        /// The module metadata is invalid.
        InvalidModuleMetadata,
        /// The module metadata is too long.
        ModuleMetadataTooLong,
        /// The module metadata is invalid.
        InvalidSubnetMetadata,
        /// The module metadata is too long.
        SubnetMetadataTooLong,
        /// The maximum name length is invalid.
        InvalidMaxNameLength,
        /// The minimum name length is invalid.
        InvalidMinNameLenght,
        /// The maximum allowed subnets value is invalid.
        InvalidMaxAllowedSubnets,
        /// The maximum allowed modules value is invalid.
        InvalidMaxAllowedModules,
        /// The maximum registrations per block value is invalid.
        InvalidMaxRegistrationsPerBlock,
        /// The minimum burn value is invalid, likely too small.
        InvalidMinBurn,
        /// The maximum burn value is invalid.
        InvalidMaxBurn,
        /// The module name is too long.
        ModuleNameTooLong,
        /// The module name is too short.
        ModuleNameTooShort,
        /// The module name is invalid. It must be a UTF-8 encoded string.
        InvalidModuleName,
        /// The module address is too long.
        ModuleAddressTooLong,
        /// The module address is invalid.
        InvalidModuleAddress,
        /// A module with this name already exists in the subnet.
        ModuleNameAlreadyExists,
        /// The founder share is invalid.
        InvalidFounderShare,
        /// The incentive ratio is invalid.
        InvalidIncentiveRatio,
        /// The general subnet application cost is invalid.
        InvalidGeneralSubnetApplicationCost,
        /// The proposal expiration is invalid.
        InvalidProposalExpiration,
        /// The maximum weight age is invalid.
        InvalidMaxWeightAge,
        /// The maximum number of set weights per epoch has been reached.
        MaxSetWeightsPerEpochReached,
        /// An arithmetic error occurred during calculation.
        ArithmeticError,
        /// The target registrations per interval is invalid.
        InvalidTargetRegistrationsPerInterval,
        /// The maximum registrations per interval is invalid.
        InvalidMaxRegistrationsPerInterval,
        /// The adjustment alpha value is invalid.
        InvalidAdjustmentAlpha,
        /// The target registrations interval is invalid.
        InvalidTargetRegistrationsInterval,
        /// The minimum immunity stake is invalid.
        InvalidMinImmunityStake,
        /// The extrinsic panicked during execution.
        ExtrinsicPanicked,
        /// A step in the process panicked.
        StepPanicked,
        /// The stake amount to add or remove is too small. Minimum is 0.5 unit.
        StakeTooSmall,
        /// The target rootnet validator is delegating weights to another validator
        TargetIsDelegatingControl,
        /// There is no subnet that is running with the Rootnet consensus
        RootnetSubnetNotFound,
        /// MinValidatorStake must be lower than 250k
        InvalidMinValidatorStake,
        /// The maximum allowed validators value is invalid, minimum is 10.
        InvalidMaxAllowedValidators,
        /// Uid is not in general subnet legit whitelist
        UidNotWhitelisted,
    }

    // ---------------------------------
    // Genesis
    // ---------------------------------

    #[derive(frame_support::DefaultNoBound)]
    #[pallet::genesis_config]
    pub struct GenesisConfig<T: Config> {
        pub subnets: Vec<pallet_subspace_genesis_config::ConfigSubnet<Vec<u8>, T::AccountId>>,
        pub block: u32,
    }

    #[pallet::genesis_build]
    impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
        fn build(&self) {
            let def = DefaultSubnetParams::<T>::get();

            for (netuid, subnet) in self.subnets.iter().enumerate() {
                let netuid = netuid as u16;

                let params: SubnetParams<T> = SubnetParams {
                    name: subnet.name.clone().try_into().expect("subnet name is too long"),
                    founder: subnet.founder.clone(),
                    tempo: subnet.tempo.unwrap_or(def.tempo),
                    immunity_period: subnet.immunity_period.unwrap_or(def.immunity_period),
                    min_allowed_weights: subnet
                        .min_allowed_weights
                        .unwrap_or(def.min_allowed_weights),
                    max_allowed_weights: subnet
                        .max_allowed_weights
                        .unwrap_or(def.max_allowed_weights),
                    max_allowed_uids: subnet.max_allowed_uids.unwrap_or(def.max_allowed_uids),
                    ..def.clone()
                };

                log::info!("registering subnet {netuid} with params: {params:?}");

                let fee = DelegationFee::<T>::get(&params.founder);
                let changeset: SubnetChangeset<T> =
                    SubnetChangeset::new(params).expect("genesis subnets are valid");
                let _ = self::Pallet::<T>::add_subnet(changeset, Some(netuid))
                    .expect("Failed to register genesis subnet");

                for (module_uid, module) in subnet.modules.iter().enumerate() {
                    let module_uid = module_uid as u16;

                    let changeset = ModuleChangeset::new(
                        module.name.clone(),
                        module.address.clone(),
                        fee,
                        None,
                    );
                    self::Pallet::<T>::append_module(netuid, &module.key, changeset)
                        .expect("genesis modules are valid");
                    Weights::<T>::insert(
                        netuid,
                        module_uid,
                        module.weights.clone().unwrap_or_default(),
                    );

                    for (staker, stake) in module.stake_from.iter().flatten() {
                        Pallet::<T>::increase_stake(staker, &module.key, *stake);
                    }
                }
            }
            log::info!("{:?}", SubnetGaps::<T>::get());
        }
    }

    // ---------------------------------
    // Hooks
    // ---------------------------------

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        /// ---- Called on the initialization of this pallet. (the order of on_finalize calls is
        /// determined in the runtime)
        fn on_initialize(block_number: BlockNumberFor<T>) -> Weight {
            let block_number: u64 =
                block_number.try_into().ok().expect("blockchain won't pass 2 ^ 64 blocks");

            // Adjust costs to reflect the demand
            Self::adjust_registration_parameters(block_number);

            // Clears the root net weights daily quota
            Self::clear_rootnet_daily_weight_calls(block_number);

            Self::copy_delegated_weights(block_number);

            for netuid in N::<T>::iter_keys() {
                if Self::blocks_until_next_epoch(netuid, block_number) > 0 {
                    continue;
                }

                // Clear weights for normal subnets
                Self::clear_set_weight_rate_limiter(netuid);
            }

            // TODO: fix latr
            Weight::default()
        }

        fn on_idle(_n: BlockNumberFor<T>, _remaining: Weight) -> Weight {
            log::info!("running on_idle");
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
            module_key: T::AccountId,
            amount: u64,
        ) -> DispatchResult {
            Self::do_add_stake(origin, module_key, amount)
        }

        #[pallet::call_index(2)]
        #[pallet::weight((T::WeightInfo::remove_stake(), DispatchClass::Normal, Pays::No))]
        pub fn remove_stake(
            origin: OriginFor<T>,
            module_key: T::AccountId,
            amount: u64,
        ) -> DispatchResult {
            Self::do_remove_stake(origin, module_key, amount)
        }

        // ---------------------------------
        // Bulk stake operations
        // ---------------------------------

        #[pallet::call_index(3)]
        #[pallet::weight((T::WeightInfo::add_stake_multiple(), DispatchClass::Normal, Pays::No))]
        pub fn add_stake_multiple(
            origin: OriginFor<T>,
            module_keys: Vec<T::AccountId>,
            amounts: Vec<u64>,
        ) -> DispatchResult {
            Self::do_add_stake_multiple(origin, module_keys, amounts)
        }

        #[pallet::call_index(4)]
        #[pallet::weight((T::WeightInfo::remove_stake_multiple(), DispatchClass::Normal, Pays::No))]
        pub fn remove_stake_multiple(
            origin: OriginFor<T>,
            module_keys: Vec<T::AccountId>,
            amounts: Vec<u64>,
        ) -> DispatchResult {
            Self::do_remove_stake_multiple(origin, module_keys, amounts)
        }

        // ---------------------------------
        // Transfers
        // ---------------------------------

        #[pallet::call_index(5)]
        #[pallet::weight((T::WeightInfo::transfer_stake(), DispatchClass::Normal, Pays::No))]
        pub fn transfer_stake(
            origin: OriginFor<T>,         // --- The account that is calling this function.
            module_key: T::AccountId,     // --- The module key.
            new_module_key: T::AccountId, // --- The new module key.
            amount: u64,                  // --- The amount of stake to transfer.
        ) -> DispatchResult {
            Self::do_transfer_stake(origin, module_key, new_module_key, amount)
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
            network_name: Vec<u8>,
            name: Vec<u8>,
            address: Vec<u8>,
            module_key: T::AccountId,
            metadata: Option<Vec<u8>>,
        ) -> DispatchResult {
            Self::do_register(origin, network_name, name, address, module_key, metadata)
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
            ensure!(
                Self::is_registered(Some(netuid), &key),
                Error::<T>::ModuleDoesNotExist
            );

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
            name: BoundedVec<u8, ConstU32<256>>,
            metadata: Option<BoundedVec<u8, ConstU32<120>>>,
            immunity_period: u16,
            incentive_ratio: u16,
            max_allowed_uids: u16,
            max_allowed_weights: u16,
            min_allowed_weights: u16,
            max_weight_age: u64,
            tempo: u16,
            trust_ratio: u16,
            maximum_set_weight_calls_per_epoch: u16,
            vote_mode: VoteMode,
            bonds_ma: u64,
            module_burn_config: GeneralBurnConfiguration<T>,
            min_validator_stake: u64,
            max_allowed_validators: Option<u16>,
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
                name,
                tempo,
                trust_ratio,
                maximum_set_weight_calls_per_epoch,
                bonds_ma,
                module_burn_config,
                min_validator_stake,
                max_allowed_validators,
                governance_config: GovernanceConfiguration {
                    vote_mode,
                    ..T::get_subnet_governance_configuration(netuid)
                },
                metadata,
            };

            let changeset = SubnetChangeset::update(netuid, params)?;
            Self::do_update_subnet(origin, netuid, changeset)
        }

        #[pallet::call_index(11)]
        #[pallet::weight((T::WeightInfo::delegate_rootnet_control(), DispatchClass::Normal, Pays::No))]
        pub fn delegate_rootnet_control(
            origin: OriginFor<T>,
            target: T::AccountId,
        ) -> DispatchResult {
            Self::do_delegate_rootnet_control(origin, target)
        }

        #[pallet::call_index(12)]
        #[pallet::weight((T::WeightInfo::register(), DispatchClass::Normal, Pays::No))]
        pub fn register_subnet(
            origin: OriginFor<T>,
            name: Vec<u8>,
            metadata: Option<Vec<u8>>,
        ) -> DispatchResult {
            Self::do_register_subnet(origin, name, metadata)
        }
    }
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
