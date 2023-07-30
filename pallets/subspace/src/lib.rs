

// disable all warnings
#![allow(warnings)]
#![cfg_attr(not(feature = "std"), no_std)]
#![recursion_limit = "512"]
// Edit this file to define custom logic or remove it if it is not needed.
// Learn more about FRAME and the core library of Substrate FRAME pallets:
// <https://docs.substrate.io/reference/frame-pallets/>
pub use pallet::*;

use frame_system::{
	self as system,
	ensure_signed
};

use frame_support::{
	dispatch,
	dispatch::{
		DispatchInfo,
		PostDispatchInfo
	}, ensure, 
	traits::{
		Currency, 
		ExistenceRequirement,
		tokens::{
			WithdrawReasons
		},
		IsSubType,
		}
};

use sp_std::marker::PhantomData;
use codec::{Decode, Encode};
use sp_runtime::{
	traits::{
		Dispatchable,
		DispatchInfoOf,
		SignedExtension,
		PostDispatchInfoOf
	},
	transaction_validity::{
		TransactionValidity,
		TransactionValidityError
	}
};
use scale_info::TypeInfo;
use frame_support::sp_runtime::transaction_validity::ValidTransaction;

// ============================
//	==== Benchmark Imports =====
// ============================
#[cfg(feature = "runtime-benchmarks")]
mod benchmarks;

// =========================
//	==== Pallet Imports =====
// =========================
mod step;
mod math;
mod network;
mod registration;
mod staking;
mod weights;
pub mod module;

#[frame_support::pallet]
pub mod pallet {
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;
	use frame_support::traits::Currency;
	use frame_support::sp_std::vec;
	use serde::{Serialize, Deserialize};
	use serde_with::{serde_as, DisplayFromStr};
	use frame_support::inherent::Vec;
	use scale_info::prelude::string::String;


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

	}
	

	pub type AccountIdOf<T> = <T as frame_system::Config>::AccountId;


	// =======================================
	// ==== Rate Limiting ====
	// =======================================	
	#[pallet::type_value]
	pub fn DefaultTxRateLimit<T: Config>() -> u64 { 1 }
	#[pallet::type_value]
	pub fn DefaultLastTxBlock<T: Config>() -> u64 { 0 }

	#[pallet::storage] // --- ITEM ( tx_rate_limit )
	pub(super) type TxRateLimit<T> = StorageValue<_, u64, ValueQuery, DefaultTxRateLimit<T>>;
	#[pallet::storage] // --- MAP ( key ) --> last_block
	pub(super) type LastTxBlock<T:Config> = StorageMap<_, Identity, T::AccountId, u64, ValueQuery, DefaultLastTxBlock<T>>;


	// ==============================
	// ==== Subnetwork Features =====
	// ==============================

	#[derive(Decode, Encode, PartialEq, Eq, Clone, Debug, TypeInfo)]
	pub struct SubnetInfo<T: Config> {

		// --- parameters
		pub name: Vec<u8>,
		pub tempo: u16, // how many blocks to wait before rewarding models
		pub immunity_period: u16, // how many blocks to wait before rewarding models
		pub min_allowed_weights: u16, // min number of weights allowed to be registered in this subnet
		pub max_allowed_weights: u16, // max number of weights allowed to be registered in this subnet
		pub max_allowed_uids: u16, // max number of uids allowed to be registered in this subnet
		// pub mode: u8, // --- 0 for open, 1 for closed.
		// state variables
		pub netuid: u16, // --- unique id of the network
		pub n : u16,
		pub stake: u64,
		pub emission: u64,
		pub founder: T::AccountId,
	}



	// ==============================
	// ==== Subnetworks Storage =====
	// ==============================
	#[pallet::type_value] 
	pub fn DefaultN<T:Config>() -> u16 { 0 }
	#[pallet::type_value] 
	pub fn DefaultEmission<T:Config>() -> u64 { 0 }
	#[pallet::type_value] 
	pub fn DefaultBlockAtRegistration<T: Config>() -> u64 { 0 }
	#[pallet::type_value]
	pub fn DefaultMaxAllowedUids<T: Config>() -> u16 { 4096 }
	#[pallet::type_value] 
	pub fn DefaultImmunityPeriod<T: Config>() -> u16 { 100 }
	#[pallet::type_value] 
	pub fn DefaultMinAllowedWeights<T: Config>() -> u16 { 1 }
	#[pallet::type_value] 
	pub fn DefaultMaxAllowedWeights<T: Config>() -> u16 { 420 }
	#[pallet::type_value] 
	pub fn DefaultMaxNameLength<T: Config>() -> u16 { 32 }
	#[pallet::type_value]
	pub fn DefaultRegistrationsThisBlock<T: Config>() ->  u16 { 0}
	#[pallet::type_value] 
	pub fn DefaultMaxRegistrationsPerBlock<T: Config>() -> u16 { 1 }
	#[pallet::type_value] 
	pub fn DefaultMaxAllowedSubnets<T: Config>() -> u16 { 100 }
	#[pallet::type_value]
	pub fn DefaultPendingEmission<T: Config>() ->  u64 { 0 }
	#[pallet::type_value]
	pub fn DefaultTempo<T: Config>() -> u16 { 1 }

	#[pallet::storage] // --- ITEM( tota_number_of_existing_networks )
	pub type TotalSubnets<T> = StorageValue<_, u16, ValueQuery>;
	#[pallet::storage] // --- ITEM( tota_number_of_existing_networks )
	pub type SubnetEmission<T> = StorageMap< _, Identity, u16, u64, ValueQuery, DefaultEmission<T> >;
	#[pallet::storage] // --- MAP ( netuid ) --> subnetwork_n (Number of UIDs in the network).
	pub type N<T:Config> = StorageMap< _, Identity, u16, u16, ValueQuery, DefaultN<T> >;
	#[pallet::storage] // --- DMAP ( key, netuid ) --> bool
	pub type Founder<T:Config> = StorageMap<_, Identity, u16, T::AccountId, ValueQuery, DefaultAccount<T>>;

	#[pallet::storage] // --- MAP ( netuid ) --> epoch
	pub type Tempo<T> = StorageMap<_, Identity, u16, u16, ValueQuery, DefaultTempo<T> >;
	#[pallet::storage] // --- MAP ( netuid ) --> pending_emission
	pub type PendingEmission<T> = StorageMap<_, Identity, u16, u64, ValueQuery, DefaultPendingEmission<T>>;
	#[pallet::storage] // --- DMAP ( netuid ) --> bonds
	pub(super) type MaxNameLength<T:Config> = StorageValue< _, u16, ValueQuery, DefaultMaxNameLength<T> >;
	
	#[pallet::storage] // --- MAP ( netuid ) --> weights_set_rate_limit
	pub type SubnetNamespace<T: Config> = StorageMap<_, Twox64Concat, Vec<u8>,  u16 , ValueQuery>;
	#[pallet::storage] // --- MAP ( netuid ) --> max_allowed_uids
	pub type MaxAllowedUids<T> = StorageMap<_, Identity, u16, u16, ValueQuery, DefaultMaxAllowedUids<T> >;
	#[pallet::storage] // --- MAP ( netuid ) --> immunity_period
	pub type ImmunityPeriod<T> = StorageMap<_, Identity, u16, u16, ValueQuery, DefaultImmunityPeriod<T> >;
	#[pallet::storage] // --- MAP ( netuid ) --> min_allowed_weights
	pub type MinAllowedWeights<T> = StorageMap< _, Identity, u16, u16, ValueQuery, DefaultMinAllowedWeights<T> >;
	#[pallet::storage] // --- MAP ( netuid ) --> min_allowed_weights
	pub type MaxAllowedWeights<T> = StorageMap< _, Identity, u16, u16, ValueQuery, DefaultMaxAllowedWeights<T> >;
	#[pallet::storage] // --- MAP ( netuid ) --> weights_set_rate_limit
	pub type RegistrationBlock<T:Config> = StorageDoubleMap<_, Identity, u16, Identity, u16, u64, ValueQuery, DefaultBlockAtRegistration<T> >;

	// =======================================
	// ==== Subnetwork Storage  ====
	// =======================================
	#[pallet::type_value] 
	pub fn EmptyU16Vec<T:Config>() -> Vec<u16> { vec![] }
	#[pallet::type_value] 
	pub fn EmptyU64Vec<T:Config>() -> Vec<u64> { vec![] }
	#[pallet::type_value] 
	pub fn EmptyBoolVec<T:Config>() -> Vec<bool> { vec![] }
	#[pallet::type_value] 
	pub fn DefaultWeights<T:Config>() -> Vec<(u16, u16)> { vec![] }
	#[pallet::type_value] 
	pub fn DefaultKey<T:Config>() -> T::AccountId { T::AccountId::decode(&mut sp_runtime::traits::TrailingZeroInput::zeroes()).unwrap() }

	
	#[pallet::storage] // --- DMAP ( netuid ) --> incentive
	pub(super) type Incentive<T:Config> = StorageMap< _, Identity, u16, Vec<u16>, ValueQuery, EmptyU16Vec<T>>;
	#[pallet::storage] // --- DMAP ( netuid ) --> dividends
	pub(super) type Dividends<T:Config> = StorageMap< _, Identity, u16, Vec<u16>, ValueQuery, EmptyU16Vec<T>>;
	#[pallet::storage] // --- DMAP ( netuid ) --> dividends
	pub(super) type Emission<T:Config> = StorageMap< _, Identity, u16, Vec<u64>, ValueQuery, EmptyU64Vec<T>>;
	#[pallet::storage] // --- DMAP ( netuid ) --> last_update
	pub(super) type LastUpdate<T:Config> = StorageMap< _, Identity, u16, Vec<u64>, ValueQuery, EmptyU64Vec<T>>;
	#[pallet::storage] // --- DMAP ( netuid, uid ) --> weights
    pub(super) type Weights<T:Config> = StorageDoubleMap<_, Identity, u16, Identity, u16, Vec<(u16, u16)>, ValueQuery, DefaultWeights<T> >;
	#[pallet::storage] // --- DMAP ( netuid, key ) --> uid
	pub(super) type Uids<T:Config> = StorageDoubleMap<_, Identity, u16, Blake2_128Concat, T::AccountId, u16, OptionQuery>;
	#[pallet::storage] // --- DMAP ( netuid, uid ) --> key
	pub(super) type Keys<T:Config> = StorageDoubleMap<_, Identity, u16, Identity, u16, T::AccountId, ValueQuery, DefaultKey<T> >;
	#[pallet::storage]
	pub type Namespace<T: Config> = StorageDoubleMap<_, Twox64Concat, u16, Twox64Concat, Vec<u8>, u16, ValueQuery>;
	#[pallet::storage]
	pub type Names<T: Config> = StorageDoubleMap<_, Twox64Concat, u16, Twox64Concat, u16, Vec<u8>, ValueQuery>;
	#[pallet::storage]
	pub type Address<T: Config> = StorageDoubleMap<_, Twox64Concat, u16, Twox64Concat, u16, Vec<u8>, ValueQuery>;
	#[pallet::storage] // --- DMAP ( hot, cold ) --> stake | Returns the stake under a key prefixed by key.
	pub type ProfitShareRatio<T:Config> = StorageDoubleMap<_,Identity, u16,  Identity, u16,  u16, ValueQuery>;
	
	// ============================
	// ==== Staking + Accounts ====
	// ============================
	#[pallet::type_value] 
	pub fn DefaultStake<T: Config>() -> u64 { 0 }
	
	#[pallet::type_value] 
	pub fn DefaultAccount<T: Config>() -> T::AccountId { T::AccountId::decode(&mut sp_runtime::traits::TrailingZeroInput::zeroes()).unwrap()}

	#[pallet::storage] // --- ITEM ( total_stake )
	pub type TotalStake<T> = StorageValue<_, u64, ValueQuery>;
	#[pallet::storage] // --- DMAP ( hot, cold ) --> stake | Returns the stake under a key prefixed by key.
	pub type Stake<T:Config> = StorageDoubleMap<_,Identity, u16,  Identity, T::AccountId, u64, ValueQuery, DefaultStake<T>>;
	#[pallet::storage] // --- DMAP ( netuid, key ) --> Vec<(delegater, stake )> | Returns the stake under a key prefixed by key.
	pub type StakeFrom<T:Config> = StorageDoubleMap<_,Identity, u16,  Identity, T::AccountId, Vec<(T::AccountId, u64)>, ValueQuery>;
	#[pallet::storage] // --- DMAP ( netuid, uid ) --> Vec<(uid, stake )> | Returns the stake under a key prefixed by key.
	pub type StakeTo<T:Config> = StorageDoubleMap<_,Identity, u16,  Identity, T::AccountId, Vec<(T::AccountId, u64)>, ValueQuery>;
	#[pallet::storage] // --- MAP ( netuid ) --> Registration this Block.
	pub type RegistrationsThisBlock<T> = StorageMap<_, Identity, u16, u16, ValueQuery, DefaultRegistrationsThisBlock<T>>;
	#[pallet::storage] // --- ITEM( global_max_registrations_per_block ) 
	pub type MaxRegistrationsPerBlock<T> = StorageMap<_, Identity, u16, u16, ValueQuery, DefaultMaxRegistrationsPerBlock<T> >;
	#[pallet::storage] // --- ITEM( global_max_registrations_per_block ) 
	pub type MaxAllowedSubnets<T> = StorageValue<_, u16, ValueQuery, DefaultMaxAllowedSubnets<T>>;
	#[pallet::storage] // --- ITEM ( total_stake )
	pub type SubnetTotalStake<T> = StorageMap<_, Identity,u16, u64, ValueQuery>;

	// Pallets use events to inform users when important changes are made.
	// https://docs.substrate.io/main-docs/build/events-errors/
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		// Event documentation should end with an array that provides descriptive names for event
		// parameters. [something, who]
		NetworkAdded( u16, Vec<u8> ),	// --- Event created when a new network is added.
		NetworkRemoved( u16 ), // --- Event created when a network is removed.
		StakeAdded( T::AccountId, T::AccountId, u64 ), // --- Event created when stake has been transfered from the a coldkey account onto the key staking account.
		StakeRemoved( T::AccountId, T::AccountId, u64 ), // --- Event created when stake has been removed from the key staking account onto the coldkey account.
		WeightsSet( u16, u16 ), // ---- Event created when a caller successfully set's their weights on a subnetwork.
		ModuleRegistered( u16, u16, T::AccountId ), // --- Event created when a new module account has been registered to the chain.
		BulkModulesRegistered( u16, u16 ), // --- Event created when multiple uids have been concurrently registered.
		BulkBalancesSet(u16, u16),
		MaxAllowedUidsSet( u16, u16 ), // --- Event created when max allowed uids has been set for a subnetwor.
		MaxRegistrationsPerBlockSet( u16, u16), // --- Event created when we set max registrations per block
		MinAllowedWeightSet( u16, u16 ), // --- Event created when minimun allowed weight is set for a subnet.
		ImmunityPeriodSet( u16, u16), // --- Event created when immunity period is set for a subnet.
		ModuleUpdated( u16, T::AccountId ), // --- Event created when the module server information is added to the network.
		DelegateAdded( T::AccountId, T::AccountId, u16 ), // --- Event created to signal a key has become a delegate.
		TxRateLimitSet( u64 ), // --- Event created when setting the transaction rate limit.
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		ModuleNameAlreadyExists, // --- Thrown when a module name already exists.
		NetworkDoesNotExist, // --- Thrown when the network does not exist.
		NetworkExist, // --- Thrown when the network already exist.
		InvalidIpType, // ---- Thrown when the user tries to serve an module which is not of type	4 (IPv4) or 6 (IPv6).
		InvalidIpAddress, // --- Thrown when an invalid IP address is passed to the serve function.
		NotRegistered, // ---- Thrown when the caller requests setting or removing data from a module which does not exist in the active set.
		NotEnoughStaketoWithdraw, // ---- Thrown when the caller requests removing more stake then there exists in the staking account. See: fn remove_stake.
		NotEnoughBalanceToStake, //  ---- Thrown when the caller requests adding more stake than there exists in the cold key account. See: fn add_stake
		BalanceWithdrawalError, // ---- Thrown when the caller tries to add stake, but for some reason the requested amount could not be withdrawn from the coldkey account
		WeightVecNotEqualSize, // ---- Thrown when the caller attempts to set the weight keys and values but these vectors have different size.
		DuplicateUids, // ---- Thrown when the caller attempts to set weights with duplicate uids in the weight matrix.
		InvalidUid, // ---- Thrown when a caller attempts to set weight to at least one uid that does not exist in the metagraph.
		NotSettingEnoughWeights, // ---- Thrown when the dispatch attempts to set weights on chain with fewer elements than are allowed.
		TooManyRegistrationsThisBlock, // ---- Thrown when registrations this block exceeds allowed number.
		AlreadyRegistered, // ---- Thrown when the caller requests registering a module which already exists in the active set.
		MaxAllowedUIdsNotAllowed, // ---  Thrown if the vaule is invalid for MaxAllowedUids
		CouldNotConvertToBalance, // ---- Thrown when the dispatch attempts to convert between a u64 and T::balance but the call fails.
		StakeAlreadyAdded, // --- Thrown when the caller requests adding stake for a key to the total stake which already added
		StorageValueOutOfRange, // --- Thrown when the caller attempts to set a storage value outside of its allowed range.
		TempoHasNotSet, // --- Thrown when epoch has not set
		InvalidTempo, // --- Thrown when epoch is not valid
		SettingWeightsTooFast, // --- Thrown if the key attempts to set weights twice withing net_epoch/2 blocks.
		BalanceSetError, // --- Thrown when an error occurs setting a balance
		MaxAllowedUidsExceeded, // --- Thrown when number of accounts going to be registered exceed MaxAllowedUids for the network.
		TooManyUids, // ---- Thrown when the caller attempts to set weights with more uids than allowed.
		TxRateLimitExceeded, // --- Thrown when a transactor exceeds the rate limit for transactions.
		InvalidMaxAllowedUids, // --- Thrown when the user tries to set max allowed uids to a value less than the current number of registered uids.
		SubnetNameAlreadyExists,
		ModuleNameTooLong,
		KeyAlreadyRegistered,
		ModuleNameDoesNotExist,
		KeyNameMismatch,
		NotSubnetFounder,
		NameAlreadyRegistered, 
		NotEnoughStaketoSetWeights,
		NotEnoughStakeToStartNetwork,
		NetworkRegistrationFailed,
		NetworkAlreadyRegistered,
		NoSelfWeight
	}

	// ==================
	// ==== Genesis =====
	// ==================

	#[pallet::genesis_config]
	#[cfg(feature = "std")]
	pub struct GenesisConfig<T: Config> {
		// key, name, address, stake, weights 
		pub modules: Vec<Vec<(T::AccountId, Vec<u8>, Vec<u8>, u64, Vec<(u16, u16)>)>>,
		// name, tempo, immunity_period, max_allowed_uids, min_allowed_weight, max_registrations_per_block, max_allowed_weights
		pub subnets: Vec<(Vec<u8>, u16, u16, u16, u16, T::AccountId)>,

		pub block: u64,
	}

	#[cfg(feature = "std")]
	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			Self { 
				modules: Default::default(),
				subnets: Default::default(),
				block: Default::default(),
			}
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
		fn build(&self) {
			// Set initial total issuance from balances
			// Subnet config values
			let block = self.block;
			// Module config values
			// set block to genesis block
			<frame_system::Pallet<T>>::set_block_number(T::BlockNumber::from(block as u32));
			
			for (i, subnet) in self.subnets.iter().enumerate() {
				let netuid: u16 = i as u16;
				SubnetNamespace::<T>::insert( subnet.0.clone(), netuid );
				Tempo::<T>::insert(netuid, subnet.1);
				ImmunityPeriod::<T>::insert(netuid, subnet.2);
				MinAllowedWeights::<T>::insert(netuid, subnet.3);
				MaxAllowedUids::<T>::insert(netuid, subnet.4);
				Founder::<T>::insert(netuid, subnet.5.clone());
				TotalSubnets::<T>::mutate( |n| *n += 1 );
				N::<T>::insert( netuid, 0 );

				let mut incentive : Vec<u16> = Incentive::<T>::get(netuid );
				let mut dividends : Vec<u16> = Dividends::<T>::get(netuid );
				let mut  emission : Vec<u64> = Emission::<T>::get(netuid);
				let mut last_update : Vec<u64> = LastUpdate::<T>::get(netuid);
				
				

				for (uid_usize, (key, name, address, stake, weights)) in self.modules[i].iter().enumerate() {
					let uid = uid_usize as u16;
					Keys::<T>::insert(netuid, uid , key.clone());
					Uids::<T>::insert(netuid, key.clone(), uid );
					Names::<T>::insert(netuid, uid , name);
					Namespace::<T>::insert(netuid, name, uid);
					Address::<T>::insert(netuid, uid , address);
					Weights::<T>::insert(netuid, uid , weights);

					incentive.push(0);
					dividends.push(0);
					emission.push(0);
					last_update.push(0);

					

					// increase  stake variables
					Stake::<T>::insert(netuid,key.clone(), stake);
					TotalStake::<T>::mutate( |n| *n += stake );
					SubnetTotalStake::<T>::insert(netuid , SubnetTotalStake::<T>::get(netuid).saturating_add( *stake ) );

					N::<T>::insert( netuid, N::<T>::get(netuid) + 1 );
					RegistrationBlock::<T>::insert(netuid, uid , 0);
					
				}

				Incentive::<T>::insert(netuid, incentive);
				Dividends::<T>::insert(netuid, dividends);
				Emission::<T>::insert(netuid, emission);
				LastUpdate::<T>::insert(netuid, last_update);
			}
			

		}
	}

	// ================
	// ==== Hooks =====
	// ================
  
	#[pallet::hooks] 
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> { 
		// ---- Called on the initialization of this pallet. (the order of on_finalize calls is determined in the runtime)
		//
		// # Args:
		// 	* 'n': (T::BlockNumber):
		// 		- The number of the block we are initializing.
		fn on_initialize( _block_number: BlockNumberFor<T> ) -> Weight {
			Self::block_step();
			
			return Weight::from_ref_time(110_634_229_000 as u64)
						.saturating_add(T::DbWeight::get().reads(8304 as u64))
						.saturating_add(T::DbWeight::get().writes(110 as u64));
		}
	}

	// Dispatchable functions allows users to interact with the pallet and invoke state changes.
	// These functions materialize as "extrinsics", which are often compared to transactions.
	// Dispatchable functions must be annotated with a weight and must return a DispatchResult.
	#[pallet::call]
	impl<T: Config> Pallet<T> {

        #[pallet::weight((Weight::from_ref_time(10_151_000_000)
		.saturating_add(T::DbWeight::get().reads(4104))
		.saturating_add(T::DbWeight::get().writes(2)), DispatchClass::Normal, Pays::No))]
		pub fn set_weights(
			origin:OriginFor<T>, 
			netuid: u16,
			uids: Vec<u16>, 
			weights: Vec<u16>,
		) -> DispatchResult {
			Self::do_set_weights( origin, netuid, uids, weights )
		}


		#[pallet::weight((Weight::from_ref_time(65_000_000)
		.saturating_add(T::DbWeight::get().reads(8))
		.saturating_add(T::DbWeight::get().writes(6)), DispatchClass::Normal, Pays::No))]
		pub fn add_stake(
			origin: OriginFor<T>, 
			netuid: u16,
			amount_staked: u64,
			module_key: T::AccountId,
			amount: u64
		) -> DispatchResult {
			Self::do_add_stake(origin,netuid,module_key, amount) 
		}



		#[pallet::weight((Weight::from_ref_time(66_000_000)
		.saturating_add(T::DbWeight::get().reads(8))
		.saturating_add(T::DbWeight::get().writes(6)), DispatchClass::Normal, Pays::No))]
		pub fn remove_stake(
			origin: OriginFor<T>, 
			netuid: u16,
			module_key: T::AccountId,
			amount: u64

		) -> DispatchResult {
			Self::do_remove_stake(origin, netuid, module_key, amount)
		}

		#[pallet::weight((Weight::from_ref_time(65_000_000)
		.saturating_add(T::DbWeight::get().reads(8))
		.saturating_add(T::DbWeight::get().writes(6)), DispatchClass::Normal, Pays::No))]
		pub fn update_network(
			origin: OriginFor<T>, 
			netuid: u16,
			name: Vec<u8>,
			immunity_period: u16,
			min_allowed_weights: u16,
			max_allowed_weights: u16,
			max_allowed_uids: u16,
			tempo: u16,
			founder: T::AccountId,
		) -> DispatchResult {
			Self::do_update_network(origin,netuid, 
									name.clone(), 
									immunity_period, 
									min_allowed_weights,
									max_allowed_weights,  
									max_allowed_uids, 
									tempo, 
									founder)
		}



		#[pallet::weight((Weight::from_ref_time(65_000_000)
		.saturating_add(T::DbWeight::get().reads(8))
		.saturating_add(T::DbWeight::get().writes(6)), DispatchClass::Normal, Pays::No))]
		pub fn remove_network(
			origin: OriginFor<T>, 
			netuid: u16,
		) -> DispatchResult {
			Self::do_remove_network(origin,netuid)
		}



		#[pallet::weight((Weight::from_ref_time(19_000_000)
		.saturating_add(T::DbWeight::get().reads(2))
		.saturating_add(T::DbWeight::get().writes(1)), DispatchClass::Normal, Pays::No))]
		pub fn update_module(
			origin:OriginFor<T>, 
			netuid: u16,
			name : Vec<u8>,
			address: Vec<u8>,

		) -> DispatchResult {
			Self::do_update_module( origin, netuid, name,  address ) 
		}


		#[pallet::weight((Weight::from_ref_time(91_000_000)
		.saturating_add(T::DbWeight::get().reads(27))
		.saturating_add(T::DbWeight::get().writes(22)), DispatchClass::Normal, Pays::No))]
		pub fn register( 
				origin:OriginFor<T>, 
				network: Vec<u8>,
				name: Vec<u8>,
				address: Vec<u8>,
				stake: u64, 
		) -> DispatchResult { 
			Self::do_registration(origin, network , name, address, stake,)
		}
		
	}	

	// ---- Subspace helper functions.
	impl<T: Config> Pallet<T> {
		// --- Returns the transaction priority for setting weights.
		pub fn get_priority_set_weights( key: &T::AccountId, netuid: u16 ) -> u64 {
			if Uids::<T>::contains_key( netuid, &key ) {
				let uid : u16 = Self::get_uid_for_key(netuid, &key.clone());
				let current_block_number: u64 = Self::get_current_block_as_u64();
				return current_block_number - Self::get_last_update_for_uid(netuid, uid as u16);
			}
			return 0;
		}
	}
}


/************************************************************
	CallType definition
************************************************************/
#[derive(Debug, PartialEq)]
pub enum CallType {
    SetWeights,
    AddStake,
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

impl<T: Config + Send + Sync + TypeInfo> SubspaceSignedExtension<T> where
	T::RuntimeCall: Dispatchable<Info=DispatchInfo, PostInfo=PostDispatchInfo>,
	<T as frame_system::Config>::RuntimeCall: IsSubType<Call<T>>,
{
	pub fn new() -> Self {
		Self(Default::default())
	}

	pub fn get_priority_vanilla() -> u64 {
		// Return high priority so that every extrinsic except set_weights function will 
		// have a higher priority than the set_weights call
		return u64::max_value();
	}

	pub fn get_priority_set_weights( who: &T::AccountId, netuid: u16 ) -> u64 {
		// Return the non vanilla priority for a set weights call.

		return Pallet::<T>::get_priority_set_weights( who, netuid );
	}

	pub fn u64_to_balance( input: u64 ) -> Option<<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance> { input.try_into().ok() }

}

impl <T:Config + Send + Sync + TypeInfo> sp_std::fmt::Debug for SubspaceSignedExtension<T> {
	fn fmt(&self, f: &mut sp_std::fmt::Formatter) -> sp_std::fmt::Result {
		write!(f, "SubspaceSignedExtension")
	}
}

impl<T: Config + Send + Sync + TypeInfo> SignedExtension for SubspaceSignedExtension<T>
    where
        T::RuntimeCall: Dispatchable<Info=DispatchInfo, PostInfo=PostDispatchInfo>,
        <T as frame_system::Config>::RuntimeCall: IsSubType<Call<T>>,
{
	const IDENTIFIER: &'static str = "SubspaceSignedExtension";

	type AccountId = T::AccountId;
	type Call = T::RuntimeCall;
	type AdditionalSigned = ();
	type Pre = (CallType, u64, Self::AccountId);
	
	fn additional_signed( &self ) -> Result<Self::AdditionalSigned, TransactionValidityError> { 
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
			Some(Call::set_weights{netuid, ..}) => {
				let priority: u64 = Self::get_priority_set_weights(who, *netuid);
                Ok(ValidTransaction {
                    priority: priority,
                    longevity: 1,
                    ..Default::default()
                })
            }
			Some(Call::add_stake{..}) => {
                Ok(ValidTransaction {
                    priority: Self::get_priority_vanilla(),
                    ..Default::default()
                })
            }
            Some(Call::remove_stake{..}) => {
                Ok(ValidTransaction {
                    priority: Self::get_priority_vanilla(),
                    ..Default::default()
                })
            }
            Some(Call::remove_network{..}) => {
                Ok(ValidTransaction {
                    priority: Self::get_priority_vanilla(),
                    ..Default::default()
                })
            }
            Some(Call::update_network{..}) => {
                Ok(ValidTransaction {
                    priority: Self::get_priority_vanilla(),
                    ..Default::default()
                })
            }
	
			Some(Call::register{..}) => {
                Ok(ValidTransaction {
                    priority: Self::get_priority_vanilla(),
                    ..Default::default()
                })
            }
			_ => {
                Ok(ValidTransaction {
                    priority: Self::get_priority_vanilla(),
                    ..Default::default()
                })
            }
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
            Some(Call::add_stake{..}) => {
				let transaction_fee = 0;
                Ok((CallType::AddStake, transaction_fee, who.clone()))
            }
            Some(Call::remove_stake{..}) => {
				let transaction_fee = 0;
                Ok((CallType::RemoveStake, transaction_fee, who.clone()))
            }
			Some(Call::set_weights{..}) => {
				let transaction_fee = 0;
                Ok((CallType::SetWeights, transaction_fee, who.clone())) 
            }
			Some(Call::register{..}) => {
                let transaction_fee = 0;
                Ok((CallType::Register, transaction_fee, who.clone()))
            }
            Some(Call::update_module{..}) => {
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
				CallType::RemoveStake => {
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
