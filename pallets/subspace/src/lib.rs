// disable all warnings
#![allow(warnings)]
#![cfg_attr(not(feature = "std"), no_std)]
#![recursion_limit = "512"]
use frame_system::{self as system, ensure_signed};
pub use pallet::*;

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
mod utils;
pub mod module;
mod subnet;
mod registration;
mod staking;
mod step;
mod weights;
mod voting;
mod profit_share;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::{pallet_prelude::*, traits::Currency};
	use frame_system::pallet_prelude::*;
	use scale_info::prelude::string::String;
	use serde::{Deserialize, Serialize};
	use serde_with::{serde_as, DisplayFromStr};
	use sp_arithmetic::per_things::Percent;
	pub use sp_std::{vec, vec::Vec};

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
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
	pub fn DefaultTxRateLimit<T: Config>() -> u64 {
		1
	}
	#[pallet::type_value]
	pub fn DefaultLastTxBlock<T: Config>() -> u64 {
		0
	}
	#[pallet::type_value]
	pub fn DefaultN<T: Config>() -> u16 {
		0
	}
	#[pallet::type_value]
	pub fn DefaultEmission<T: Config>() -> u64 {
		0
	}
	#[pallet::type_value]
	pub fn DefaultBlockAtRegistration<T: Config>() -> u64 {
		0
	}
	#[pallet::type_value]
	pub fn DefaultMaxAllowedUids<T: Config>() -> u16 {
		4096
	}
	#[pallet::type_value]
	pub fn DefaultImmunityPeriod<T: Config>() -> u16 {
		40
	}
	#[pallet::type_value]
	pub fn DefaultMinAllowedWeights<T: Config>() -> u16 {
		1
	}

	#[pallet::type_value]
	pub fn DefaultMaxNameLength<T: Config>() -> u16 {
		32
	}
	#[pallet::type_value]
	pub fn DefaultRegistrationsPerBlock<T: Config>() -> u16 {
		0
	}
	#[pallet::type_value]
	pub fn DefaultMaxRegistrationsPerBlock<T: Config>() -> u16 {
		100
	}
	#[pallet::type_value]
	pub fn DefaultMaxAllowedSubnets<T: Config>() -> u16 {
		64
	}
	#[pallet::type_value]
	pub fn DefaultMaxAllowedModules<T: Config>() -> u16 {
		10_000
	}
	#[pallet::type_value]
	pub fn DefaultPendingEmission<T: Config>() -> u64 {
		0
	}
	#[pallet::type_value]
	pub fn DefaultTempo<T: Config>() -> u16 {
		1
	}
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
	#[pallet::type_value]
	pub fn DefaultWeights<T: Config>() -> Vec<(u16, u16)> {
		vec![]
	}
	#[pallet::type_value]
	pub fn DefaultKey<T: Config>() -> T::AccountId {
		T::AccountId::decode(&mut sp_runtime::traits::TrailingZeroInput::zeroes()).unwrap()
	}
	#[pallet::type_value]
	pub fn DefaultStake<T: Config>() -> u64 {
		0
	}
	#[pallet::type_value]
	pub fn DefaultVotePeriod<T: Config>() -> u16 {
		100
	} // out of 100

	#[pallet::type_value]
	pub fn DefaultDelegationFee<T: Config>() -> Percent {
		Percent::from_percent(20u8)
	}

	// ============================
	// ==== Global Variables ====
	// ============================
	#[pallet::type_value]
	pub fn DefaultUnitEmission<T: Config>() -> u64 {
		23809523810
	}
	#[pallet::storage] // --- ITEM ( unit_emission )
	pub(super) type UnitEmission<T> = StorageValue<_, u64, ValueQuery, DefaultUnitEmission<T>>;
	#[pallet::storage] // --- ITEM ( tx_rate_limit )
	pub(super) type TxRateLimit<T> = StorageValue<_, u64, ValueQuery, DefaultTxRateLimit<T>>;
	// FIXME: NOT IN USE
	#[pallet::storage] // --- MAP ( key ) --> last_block
	pub(super) type LastTxBlock<T: Config> =
		StorageMap<_, Identity, T::AccountId, u64, ValueQuery, DefaultLastTxBlock<T>>;
	#[pallet::storage] // --- ITEM ( max_name_length )
	pub(super) type MaxNameLength<T: Config> =
		StorageValue<_, u16, ValueQuery, DefaultMaxNameLength<T>>;
	#[pallet::storage] // --- ITEM ( max_allowed_subnets )
	pub(super) type MaxAllowedSubnets<T: Config> =
		StorageValue<_, u16, ValueQuery, DefaultMaxAllowedSubnets<T>>;
	#[pallet::storage] // --- ITEM ( max_allowed_modules )
	pub(super) type MaxAllowedModules<T: Config> =
		StorageValue<_, u16, ValueQuery, DefaultMaxAllowedModules<T>>;
	#[pallet::storage] // --- ITEM ( registrations_this block )
	pub type RegistrationsPerBlock<T> =
		StorageValue<_, u16, ValueQuery, DefaultRegistrationsPerBlock<T>>;
	#[pallet::storage] // --- ITEM( global_max_registrations_per_block )
	pub type MaxRegistrationsPerBlock<T> =
		StorageValue<_, u16, ValueQuery, DefaultMaxRegistrationsPerBlock<T>>;
	#[pallet::storage] // --- MAP ( netuid ) --> subnet_total_stake
	pub type TotalStake<T> = StorageMap<_, Identity, u16, u64, ValueQuery>;

	// =========================
	// ==== Subnet PARAMS ====
	// =========================

	#[derive(Decode, Encode, PartialEq, Eq, Clone, Debug, TypeInfo)]
	pub struct SubnetParams {
		// --- parameters
		pub name: Vec<u8>,
		pub tempo: u16,           // how many blocks to wait before rewarding models
		pub immunity_period: u16, // immunity period
		pub min_allowed_weights: u16, /* min number of weights allowed to be registered in this
		                           * subnet */
		pub max_allowed_weights: u16, /* max number of weights allowed to be registered in this
		                               * subnet */
		pub max_allowed_uids: u16, // max number of uids allowed to be registered in this subnet
		pub burn_rate: u16,        // out of 100
		pub min_stake: u64,
		// pub democratic: bool
		pub vote_threshold: u16, // out of 100
		pub vote_mode: Vec<u8>,
	}

	#[pallet::storage] // --- MAP ( netuid ) --> max_allowed_uids
	pub type MaxAllowedUids<T> =
		StorageMap<_, Identity, u16, u16, ValueQuery, DefaultMaxAllowedUids<T>>;

	#[pallet::storage] // --- MAP ( netuid ) --> immunity_period
	pub type ImmunityPeriod<T> =
		StorageMap<_, Identity, u16, u16, ValueQuery, DefaultImmunityPeriod<T>>;
	#[pallet::storage] // --- MAP ( netuid ) --> min_allowed_weights
	pub type MinAllowedWeights<T> =
		StorageMap<_, Identity, u16, u16, ValueQuery, DefaultMinAllowedWeights<T>>;

	#[pallet::type_value]
	pub fn DefaultMinStake<T: Config>() -> u64 {0}	
	#[pallet::storage] // --- MAP ( netuid ) --> min_allowed_weights
	pub type MinStake<T> = StorageMap<_, Identity, u16, u64, ValueQuery, DefaultMinStake<T>>;

	#[pallet::type_value]
	pub fn DefaultMaxAllowedWeights<T: Config>() -> u16 {420}
	#[pallet::storage] // --- MAP ( netuid ) --> min_allowed_weights
	pub type MaxAllowedWeights<T> =StorageMap<_, Identity, u16, u16, ValueQuery, DefaultMaxAllowedWeights<T>>;
	

	#[pallet::type_value]
	pub fn DefaultBurnRate<T: Config>() -> u16 {0}



	#[pallet::storage] // --- MAP ( netuid ) --> min_allowed_weights
	pub type BurnRate<T> = StorageMap<_, Identity, u16, u16, ValueQuery, DefaultBurnRate<T>>;

	#[pallet::type_value]
	pub fn DefaultMinBurn<T: Config>() -> u64 {0}
	#[pallet::storage] // --- MAP ( netuid ) --> min_allowed_weights
	pub type MinBurn<T> = StorageMap<_, Identity, u16, u64, ValueQuery, DefaultMinBurn<T>>;


	#[pallet::type_value]
	pub fn DefaultPendingDeregisterUids<T: Config>() -> Vec<u16> {vec![]}
	#[pallet::storage] // --- MAP ( netuid ) --> min_allowed_weights
	pub type PendingDeregisterUids<T> = StorageMap<_, Identity, u16, Vec<u16>, ValueQuery, DefaultPendingDeregisterUids<T>>;


	#[pallet::type_value]
	pub fn DefaultAccount<T: Config>() -> T::AccountId {T::AccountId::decode(&mut sp_runtime::traits::TrailingZeroInput::zeroes()).unwrap()}
	#[pallet::storage] // --- DMAP ( key, netuid ) --> bool
	pub type Founder<T: Config> = StorageMap<_, Identity, u16, T::AccountId, ValueQuery, DefaultAccount<T>>;

	#[pallet::storage] // --- MAP ( netuid ) --> epoch
	pub type Tempo<T> = StorageMap<_, Identity, u16, u16, ValueQuery, DefaultTempo<T>>;

	#[pallet::type_value]
	pub fn DefaultTrustRatio<T: Config>() -> u16 {0}
	#[pallet::storage] // --- MAP ( netuid ) --> epoch
	pub type TrustRatio<T> = StorageMap<_, Identity, u16, u16, ValueQuery, DefaultTrustRatio<T>>;


	#[pallet::type_value]
	pub fn DefaultQuadraticVoting<T: Config>() -> bool {false}
	#[pallet::storage] // --- MAP ( netuid ) --> epoch
	pub type QuadraticVoting<T> = StorageMap<_, Identity, u16, bool, ValueQuery, DefaultQuadraticVoting<T>>;

	// =======================================
	// ==== Voting  ====
	// =======================================
	pub fn DefaultProposalId<T: Config>() -> u16 {u16::MAX} // out of 100
	#[pallet::storage] // --- MAP ( netuid ) --> epoch
	pub type Vote2ProposalId<T: Config> =StorageMap<_, Identity, T::AccountId, Vec<u8>, ValueQuery, DefaultVoteMode<T>>;
	
	// threshold
	#[pallet::type_value]
	pub fn DefaultVoteThreshold<T: Config>() -> u16 {50} // out of 100
	#[pallet::storage] // --- MAP ( netuid ) --> epoch
	pub type SubnetVoteThreshold<T> = StorageMap<_, Identity, u16, u16, ValueQuery, DefaultVoteThreshold<T>>;
	#[pallet::storage] // --- MAP ( netuid ) --> epoch
	pub type GlobalVoteThreshold<T> = StorageValue<_, u16, ValueQuery, DefaultVoteThreshold<T>>;
	
	// mode [stake, authority, quadratic]
	#[pallet::type_value]
	pub fn DefaultVoteMode<T: Config>() -> Vec<u8> {"authority".as_bytes().to_vec()}
	#[pallet::storage] // --- MAP ( netuid ) --> epoch
	pub type SubnetVoteMode<T> =StorageMap<_, Identity, u16, Vec<u8>, ValueQuery, DefaultVoteMode<T>>;
	#[pallet::storage] // --- MAP ( netuid ) --> epoch
	pub type GlobalVoteMode<T> =StorageValue<_, Vec<u8>, ValueQuery, DefaultVoteMode<T>>;

	#[derive(Decode, Encode, PartialEq, Eq, Clone, Debug, TypeInfo)]
	pub struct SubnetInfo<T: Config> {
		// --- parameters
		pub params: SubnetParams ,
		// pub mode: u8, // --- 0 for open, 1 for closed.
		// state variables
		pub netuid: u16, // --- unique id of the network
		pub n: u16,
		pub stake: u64,
		pub emission: u64,
		pub founder: T::AccountId,
	}

	#[pallet::storage] // --- ITEM( tota_number_of_existing_networks )
	pub type TotalSubnets<T> = StorageValue<_, u16, ValueQuery>;
	#[pallet::storage] // --- MAP( netuid ) --> subnet_emission
	pub type SubnetEmission<T> = StorageMap<_, Identity, u16, u64, ValueQuery, DefaultEmission<T>>;
	#[pallet::storage] // --- MAP ( netuid ) --> subnetwork_n (Number of UIDs in the network).
	pub type N<T: Config> = StorageMap<_, Identity, u16, u16, ValueQuery, DefaultN<T>>;
	#[pallet::storage] // --- MAP ( netuid ) --> pending_emission
	pub type PendingEmission<T> = StorageMap<_, Identity, u16, u64, ValueQuery, DefaultPendingEmission<T>>;
	#[pallet::storage] // --- MAP ( network_name ) --> netuid
	pub type Name2Subnet<T: Config> = StorageMap<_, Twox64Concat, Vec<u8>, u16, ValueQuery>;

	// =======================================
	// ==== Module Variables  ====
	// =======================================
	#[pallet::storage] // --- DMAP ( netuid, module_key ) --> uid
	pub(super) type Uids<T: Config> =
		StorageDoubleMap<_, Identity, u16, Blake2_128Concat, T::AccountId, u16, OptionQuery>;
	#[pallet::storage] // --- DMAP ( netuid, uid ) --> module_key
	pub(super) type Keys<T: Config> =
		StorageDoubleMap<_, Identity, u16, Identity, u16, T::AccountId, ValueQuery, DefaultKey<T>>;
	#[pallet::storage] // --- DMAP ( netuid, uid ) --> module_name
	pub type Names<T: Config> =
		StorageDoubleMap<_, Twox64Concat, u16, Twox64Concat, u16, Vec<u8>, ValueQuery>;
	#[pallet::storage] // --- DMAP ( netuid, uid ) --> module_address
	pub type Address<T: Config> =
		StorageDoubleMap<_, Twox64Concat, u16, Twox64Concat, u16, Vec<u8>, ValueQuery>;
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

	// =======================================
	// ==== Module Staking Variables  ====
	// =======================================

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

	// LOAN VARIABLES

	#[pallet::storage] // --- DMAP ( netuid, module_key ) --> Vec<(delegater, stake )> | Returns the list of delegates
	pub type LoanTo<T: Config> = StorageMap<
		_,
		Identity,
		T::AccountId,
		Vec<(T::AccountId, u64)>,
		ValueQuery,
	>;

	#[pallet::storage] // --- DMAP ( netuid, module_key ) --> Vec<(delegater, stake )> | Returns the list of delegates
	pub type LoanFrom<T: Config> = StorageMap<
		_,
		Identity,
		T::AccountId,
		Vec<(T::AccountId, u64)>,
		ValueQuery,
	>;

	// PROFIT SHARE VARIABLES
	
	#[pallet::type_value]
	pub fn DefaultProfitShares<T: Config>() -> Vec<(T::AccountId, u16)> {vec![]}
	
	#[pallet::storage] // --- DMAP ( netuid, account_id ) --> Vec<(module_key, stake )> | Returns the list of the
	pub type ProfitShares<T: Config> = StorageMap<_, 
			Identity, 
			T::AccountId, 
			Vec<(T::AccountId, u16)>, 
			ValueQuery, 
			DefaultProfitShares<T>>;

	#[pallet::type_value]
	pub fn DefaultProfitShareUnit<T: Config>() -> u16 {u16::MAX}
	#[pallet::storage] // --- DMAP ( netuid, account_id ) --> Vec<(module_key, stake )> | Returns the list of the
	pub type ProfitShareUnit<T: Config> = StorageValue<_, u16, ValueQuery, DefaultProfitShareUnit<T>>;
				   
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
		MaxRegistrationsPerBlockSet(u16), // --- Event created when we set max registrations per block
		GlobalUpdate(u16, u16, u16, u16, u64, u64),
		GlobalProposalAccepted(u64), // (id)
		CustomProposalAccepted(u64), // (id)
		SubnetProposalAccepted(u64, u16), // (id, netuid)
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		ModuleNameAlreadyExists, // --- Thrown when a module name already exists.
		NetworkDoesNotExist,     // --- Thrown when the network does not exist.
		NetworkExist,            // --- Thrown when the network already exist.
		InvalidIpType,           /* ---- Thrown when the user tries to serve an module which
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

				let default_params: SubnetParams = self::Pallet::<T>::default_subnet_params();

				let params = SubnetParams {
					name: subnet.0.clone(),
					tempo: subnet.1,
					immunity_period: subnet.2,
					min_allowed_weights: subnet.3,
					max_allowed_weights: subnet.4,
					max_allowed_uids: subnet.5,
					burn_rate: subnet.6,
					min_stake: subnet.7,
					vote_threshold: default_params.vote_threshold,
					vote_mode: default_params.vote_mode.clone(),
				};
				
				self::Pallet::<T>::add_network(params.clone());
				// --- Set subnet founder
				self::Pallet::<T>::set_subnet_founder(netuid, &subnet.8.clone());

				
				for (uid_usize, (key, name, address, weights)) in
					self.modules[subnet_idx].iter().enumerate()
				{
					self::Pallet::<T>::append_module(netuid, key, name.clone(), address.clone());
					Weights::<T>::insert(netuid, uid_usize as u16, weights);
				}
			}
			// Now we can add the stake to the network
			for (subnet_idx, subnet) in self.subnets.iter().enumerate() {
				let netuid: u16 = subnet_idx as u16;

				for (key, stake_to) in self.stake_to[netuid as usize].iter() {
					for (module_key, stake_amount) in stake_to {
						self::Pallet::<T>::increase_stake(netuid, key, module_key, *stake_amount);
					}
				}
			}
		}
	}

	#[derive(Decode, Encode, PartialEq, Eq, Clone, Debug, TypeInfo)]
	pub struct GlobalParams {
		pub max_name_length: u16,
		pub max_allowed_subnets: u16,
		pub max_allowed_modules: u16,
		pub max_registrations_per_block: u16,
		pub unit_emission: u64, 
		pub tx_rate_limit: u64,
		pub vote_threshold: u16,
		pub vote_mode: Vec<u8>,
		pub max_proposals: u64,
	}

	#[pallet::type_value]
	pub fn DefaultGlobalParams<T: Config>() -> GlobalParams {
		GlobalParams {
			max_name_length: DefaultMaxNameLength::<T>::get(),
			max_allowed_subnets: DefaultMaxAllowedSubnets::<T>::get(),
			max_allowed_modules: DefaultMaxAllowedModules::<T>::get(),
			max_registrations_per_block: DefaultMaxRegistrationsPerBlock::<T>::get(),
			unit_emission: DefaultUnitEmission::<T>::get(), 
			tx_rate_limit: DefaultTxRateLimit::<T>::get(),
			vote_threshold: DefaultVoteThreshold::<T>::get(),
			vote_mode: DefaultVoteMode::<T>::get(),
			max_proposals: DefaultMaxProposals::<T>::get(),
		}
	}

	// ========================================================
	// ==== Voting System to Update Global and Subnet  ====
	// ========================================================
	#[derive(Decode, Encode, PartialEq, Eq, Clone, Debug, TypeInfo)]
	#[scale_info(skip_type_params(T))]
	pub struct Proposal<T: Config> {
		// --- parameters
		pub subnet_params: SubnetParams,
		pub global_params: GlobalParams,
		pub netuid: u16, // FOR SUBNET PROPOSAL ONLY
		pub votes: u64,
		pub participants: Vec<T::AccountId>,
		pub accepted: bool,
		pub data: Vec<u8>, // for custom proposal
		pub mode: Vec<u8>, // "global", "subnet", "custom"
	}


	#[pallet::type_value]
	pub fn DefaultSubnetParams<T: Config>() -> SubnetParams {
		SubnetParams {
			name: vec![],
			tempo: DefaultTempo::<T>::get(),
			immunity_period: DefaultImmunityPeriod::<T>::get(),
			min_allowed_weights: DefaultMinAllowedWeights::<T>::get(),
			max_allowed_weights: DefaultMaxAllowedWeights::<T>::get(),
			max_allowed_uids: DefaultMaxAllowedUids::<T>::get(),
			burn_rate: DefaultBurnRate::<T>::get(),
			min_stake: 0, 
			vote_threshold: 50,
			vote_mode: DefaultVoteMode::<T>::get(),
		}
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
			data: vec![]
		}
	}

	#[pallet::storage] // --- MAP ( global_proposal_id ) --> global_update_proposal
	pub(super) type Proposals<T: Config> =
		StorageMap<_, Identity, u64, Proposal<T>, ValueQuery, DefaultProposal<T>>;

	#[pallet::type_value]
	pub fn DefaultMaxProposals<T: Config>() -> u64 {420}
	#[pallet::storage]
	pub(super) type MaxProposals<T: Config> = StorageValue<_, u64, ValueQuery>;
	
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
		fn on_initialize(_block_number: BlockNumberFor<T>) -> Weight {
			Self::block_step();

			return Weight::zero()
		}
	}

	// Dispatchable functions allow users to interact with the pallet and invoke state changes.
	// These functions materialize as "extrinsics", which are often compared to transactions.
	// Dispatchable functions must be annotated with a weight and must return a DispatchResult.
	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(T::WeightInfo::set_weights())]
		pub fn set_weights(
			origin: OriginFor<T>,
			netuid: u16,
			uids: Vec<u16>,
			weights: Vec<u16>,
		) -> DispatchResult {
			Self::do_set_weights(origin, netuid, uids, weights)
		}

		#[pallet::weight(T::WeightInfo::add_stake())]
		pub fn add_stake(
			origin: OriginFor<T>,
			netuid: u16,
			module_key: T::AccountId,
			amount: u64,
		) -> DispatchResult {
			Self::do_add_stake(origin, netuid, module_key, amount)
		}

		#[pallet::weight(T::WeightInfo::add_stake_multiple())]
		pub fn add_stake_multiple(
			origin: OriginFor<T>,
			netuid: u16,
			module_keys: Vec<T::AccountId>,
			amounts: Vec<u64>,
		) -> DispatchResult {
			Self::do_add_stake_multiple(origin, netuid, module_keys, amounts)
		}

		#[pallet::weight(T::WeightInfo::remove_stake())]
		pub fn remove_stake(
			origin: OriginFor<T>,
			netuid: u16,
			module_key: T::AccountId,
			amount: u64,
		) -> DispatchResult {
			Self::do_remove_stake(origin, netuid, module_key, amount)
		}

		#[pallet::weight(T::WeightInfo::remove_stake_multiple())]
		pub fn remove_stake_multiple(
			origin: OriginFor<T>,
			netuid: u16,
			module_keys: Vec<T::AccountId>,
			amounts: Vec<u64>,
		) -> DispatchResult {
			Self::do_remove_stake_multiple(origin, netuid, module_keys, amounts)
		}


		#[pallet::weight(T::WeightInfo::transfer_stake())]
		pub fn transfer_stake(
			origin: OriginFor<T>,         // --- The account that is calling this function.
			netuid: u16,                  // --- The network id.
			module_key: T::AccountId,     // --- The module key.
			new_module_key: T::AccountId, // --- The new module key.
			amount: u64,                  // --- The amount of stake to transfer.
		) -> DispatchResult {
			Self::do_transfer_stake(origin, netuid, module_key, new_module_key, amount)
		}

		#[pallet::weight(T::WeightInfo::transfer_multiple())]
		pub fn transfer_multiple(
			origin: OriginFor<T>, // --- The account that is calling this function.
			destinations: Vec<T::AccountId>, // --- The module key.
			amounts: Vec<u64>,    // --- The amount of stake to transfer.
		) -> DispatchResult {
			Self::do_transfer_multiple(origin, destinations, amounts)
		}

		#[pallet::weight(T::WeightInfo::update_network())]
		pub fn update_network(
			origin: OriginFor<T>,
			netuid: u16,
			name: Vec<u8>,
			tempo: u16,
			immunity_period: u16,
			min_allowed_weights: u16,
			max_allowed_weights: u16,
			max_allowed_uids: u16,
			burn_rate: u16,
			min_stake: u64,
			vote_threshold: u16,
			vote_mode: Vec<u8>,
			founder: T::AccountId,
		) -> DispatchResult {

			let params = SubnetParams {
				name: name.clone(),
				tempo: tempo,
				immunity_period: immunity_period,
				min_allowed_weights: min_allowed_weights,
				max_allowed_weights: max_allowed_weights,
				max_allowed_uids: max_allowed_uids,
				burn_rate: burn_rate,
				min_stake: min_stake,
				vote_threshold: vote_threshold,
				vote_mode: vote_mode.clone(),
			};
			Self::do_update_network(origin,netuid,params)
		}



		#[pallet::weight(T::WeightInfo::remove_network())]
		pub fn remove_network(origin: OriginFor<T>, netuid: u16) -> DispatchResult {
			Self::do_remove_network(origin, netuid)
		}

		#[pallet::weight(T::WeightInfo::update_module())]
		pub fn update_module(
			origin: OriginFor<T>,
			netuid: u16,
			name: Vec<u8>,
			address: Vec<u8>,
			delegation_fee: Option<Percent>,
		) -> DispatchResult {
			Self::do_update_module(origin, netuid, name, address, delegation_fee)
		}

		#[pallet::weight(T::WeightInfo::register())]
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

		#[pallet::weight(T::WeightInfo::deregister())]
		pub fn deregister(
			origin: OriginFor<T>,
			netuid : u16,
		) -> DispatchResult {
			Self::do_deregister(origin, netuid)
		}

		#[pallet::weight(T::WeightInfo::add_profit_shares())]
		pub fn add_profit_shares(
			origin: OriginFor<T>,
			keys: Vec<T::AccountId>,
			shares: Vec<u16>
		) -> DispatchResult {
			Self::do_add_profit_shares(origin, keys, shares);
			Ok(())
		}

		#[pallet::weight(T::WeightInfo::update_global())]
		pub fn update_global(
			origin: OriginFor<T>,
			max_name_length: u16,
			max_allowed_subnets: u16,
			max_allowed_modules: u16,
			max_registrations_per_block: u16,
			unit_emission: u64,
			tx_rate_limit: u64,
			vote_threshold: u16,
			vote_mode: Vec<u8>,
			max_proposals: u64,

		) -> DispatchResult {


			let params = GlobalParams {
				max_name_length: max_name_length,
				max_allowed_subnets: max_allowed_subnets,
				max_allowed_modules: max_allowed_modules,
				max_registrations_per_block: max_registrations_per_block,
				unit_emission: unit_emission,
				tx_rate_limit: tx_rate_limit,
				vote_threshold: vote_threshold,
				vote_mode: vote_mode,
				max_proposals: max_proposals,

			};
			Self::do_update_global(origin, params)
		}

		#[pallet::weight(T::WeightInfo::add_subnet_proposal())]
        pub fn add_subnet_proposal(
            origin: OriginFor<T>,
			netuid: u16, // FOR SUBNET PROPOSAL ONLY
			subnet_params: SubnetParams,
        ) -> DispatchResult {
			let mut proposal = DefaultProposal::<T>::get();

			proposal.subnet_params = subnet_params;
			proposal.mode = "subnet".as_bytes().to_vec();
			proposal.netuid = netuid;
			// proposal.data = data.unwrap_or(vec![]);

            Self::do_add_proposal(origin,  proposal)
        }

		#[pallet::weight(T::WeightInfo::add_global_proposal())]
        pub fn add_global_proposal(
            origin: OriginFor<T>,
            global_params: GlobalParams,
        ) -> DispatchResult {
			let mut proposal = DefaultProposal::<T>::get();
			proposal.global_params = global_params;
			proposal.mode = "global".as_bytes().to_vec();
			// proposal.data = data.unwrap_or(vec![]);
			
            Self::do_add_proposal(origin,  proposal)
        }


		#[pallet::weight(T::WeightInfo::vote_proposal())]
        pub fn vote_proposal(
            origin: OriginFor<T>,
            proposal_id: u64
        ) -> DispatchResult {
            Self::do_vote_proposal(
                origin,
                proposal_id
            )
        }

	}

	// ---- Subspace helper functions.
	impl<T: Config> Pallet<T> {
		// --- Returns the transaction priority for setting weights.
		pub fn get_priority_set_weights(key: &T::AccountId, netuid: u16) -> u64 {
			if Uids::<T>::contains_key(netuid, &key) {
				let uid: u16 = Self::get_uid_for_key(netuid, &key.clone());
				let current_block_number: u64 = Self::get_current_block_as_u64();
				return current_block_number - Self::get_last_update_for_uid(netuid, uid as u16)
			}
			return 0
		}
		// --- Returns the transaction priority for setting weights.
		pub fn get_priority_stake(key: &T::AccountId, netuid: u16) -> u64 {
			if Uids::<T>::contains_key(netuid, &key) {
				return Self::get_stake(netuid, key)
			}
			return 0
		}
		pub fn get_priority_balance(key: &T::AccountId) -> u64 {
			return Self::get_balance_u64(key)
		}
	}

	/************************************************************
		CallType definition
	************************************************************/
}

#[derive(Debug, PartialEq)]
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
	Other,
}
impl Default for CallType {
	fn default() -> Self {
		CallType::Other
	}
}

#[derive(Encode, Decode, Clone, Eq, PartialEq, TypeInfo)]
pub struct SubspaceSignedExtension<T: Config + Send + Sync + TypeInfo>(pub PhantomData<T>);

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
		return Pallet::<T>::get_priority_balance(who)
	}

	pub fn get_priority_set_weights(who: &T::AccountId, netuid: u16) -> u64 {
		// Return the non vanilla priority for a set weights call.

		return Pallet::<T>::get_priority_set_weights(who, netuid)
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
				Ok(ValidTransaction { priority, longevity: 1, ..Default::default() })
			},
			Some(Call::add_stake { .. }) => Ok(ValidTransaction {
				priority: Self::get_priority_vanilla(who),
				..Default::default()
			}),
			Some(Call::remove_stake { .. }) => Ok(ValidTransaction {
				priority: Self::get_priority_vanilla(who),
				..Default::default()
			}),
			Some(Call::remove_network { .. }) => Ok(ValidTransaction {
				priority: Self::get_priority_vanilla(who),
				..Default::default()
			}),
			Some(Call::update_network { .. }) => Ok(ValidTransaction {
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
			},
			Some(Call::add_stake_multiple { .. }) => {
				let transaction_fee = 0;
				Ok((CallType::AddStakeMultiple, transaction_fee, who.clone()))
			},
			Some(Call::remove_stake { .. }) => {
				let transaction_fee = 0;
				Ok((CallType::RemoveStake, transaction_fee, who.clone()))
			},
			Some(Call::remove_stake_multiple { .. }) => {
				let transaction_fee = 0;
				Ok((CallType::RemoveStakeMultiple, transaction_fee, who.clone()))
			},
			Some(Call::transfer_stake { .. }) => {
				let transaction_fee = 0;
				Ok((CallType::TransferStake, transaction_fee, who.clone()))
			},
			Some(Call::transfer_multiple { .. }) => {
				let transaction_fee = 0;
				Ok((CallType::TransferMultiple, transaction_fee, who.clone()))
			},
			Some(Call::set_weights { .. }) => {
				let transaction_fee = 0;
				Ok((CallType::SetWeights, transaction_fee, who.clone()))
			},
			Some(Call::register { .. }) => {
				let transaction_fee = 0;
				Ok((CallType::Register, transaction_fee, who.clone()))
			},
			Some(Call::update_module { .. }) => {
				let transaction_fee = 0;
				Ok((CallType::Serve, transaction_fee, who.clone()))
			},
			_ => {
				let transaction_fee = 0;
				Ok((CallType::Other, transaction_fee, who.clone()))
			},
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
				},
				CallType::AddStake => {
					log::debug!("Not Implemented! Need to add potential transaction fees here.");
				},

				CallType::AddStakeMultiple => {
					log::debug!("Not Implemented! Need to add potential transaction fees here.");
				},
				CallType::RemoveStake => {
					log::debug!("Not Implemented! Need to add potential transaction fees here.");
				},
				CallType::RemoveStakeMultiple => {
					log::debug!("Not Implemented! Need to add potential transaction fees here.");
				},
				CallType::TransferStake => {
					log::debug!("Not Implemented! Need to add potential transaction fees here.");
				},
				CallType::TransferStakeMultiple => {
					log::debug!("Not Implemented! Need to add potential transaction fees here.");
				},
				CallType::TransferMultiple => {
					log::debug!("Not Implemented! Need to add potential transaction fees here.");
				},
				CallType::AddNetwork => {
					log::debug!("Not Implemented! Need to add potential transaction fees here.");
				},
				CallType::Register => {
					log::debug!("Not Implemented!");
				},
				_ => {
					log::debug!("Not Implemented!");
				},
			}
		}
		Ok(())
	}
}
