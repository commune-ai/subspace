use super::*;
use codec::Compact;
use frame_support::{
	pallet_prelude::{Decode, DispatchError, DispatchResult, Encode},
	storage::IterableStorageMap,
	traits::Currency,
	IterableStorageDoubleMap,
};
use crate::utils::is_vec_str;
use frame_system::ensure_root;
pub use sp_std::{vec, vec::Vec};
use substrate_fixed::types::{I32F32, I64F64};
extern crate alloc;

impl<T: Config> Pallet<T> {
	// Returns true if the subnetwork exists.
	//


	pub fn do_remote_subnet(origin: T::RuntimeOrigin, netuid: u16) -> DispatchResult {
		let key = ensure_signed(origin)?;
		// --- 1. Ensure the network name does not already exist.

		ensure!(Self::if_subnet_netuid_exists(netuid), Error::<T>::SubnetNameAlreadyExists);
		ensure!(Self::is_subnet_founder(netuid, &key), Error::<T>::NotFounder);

		Self::remove_subnet(netuid);
		// --- 16. Ok and done.
		Ok(())
	}

	pub fn do_update_subnet(
		origin: T::RuntimeOrigin,
		netuid: u16,
		params: SubnetParams<T>,
	) -> DispatchResult {
		let key = ensure_signed(origin)?;
		// only the founder can update the network on authority mode
		
		ensure!(is_vec_str(Self::get_vote_mode_subnet(netuid), "authority"), Error::<T>::NotAuthorityMode);
		ensure!(Self::if_subnet_netuid_exists(netuid), Error::<T>::SubnetNameAlreadyExists);
		ensure!(Self::is_subnet_founder(netuid, &key), Error::<T>::NotFounder);
		ensure!(Self::if_subnet_netuid_exists(netuid), Error::<T>::SubnetNameAlreadyExists);
		ensure!(Self::is_subnet_founder(netuid, &key), Error::<T>::NotFounder);
		Self::check_subnet_params(params.clone())?;
		Self::set_subnet_params(netuid, params);
		// --- 16. Ok and done.
		Ok(())
	}



    pub fn check_subnet_params(params: SubnetParams<T>) -> DispatchResult{
        // checks if params are valid

		let global_params = Self::global_params();

        // check valid tempo		
		ensure!(params.min_allowed_weights <= params.max_allowed_weights, Error::<T>::InvalidMinAllowedWeights);
		
		ensure!(params.min_allowed_weights >= 1, Error::<T>::InvalidMinAllowedWeights);

		ensure!(params.max_allowed_weights <= global_params.max_allowed_weights, Error::<T>::InvalidMaxAllowedWeights);

		// the  global params must be larger than the global min_stake
		ensure!(params.min_stake >= global_params.min_stake, Error::<T>::InvalidMinStake);

		ensure!(params.max_stake > params.min_stake, Error::<T>::InvalidMaxStake);

		ensure!(params.tempo > 0, Error::<T>::InvalidTempo);

		ensure!(params.max_weight_age > params.tempo as u64,  Error::<T>::InvalidMaxWeightAge);
                		
		// ensure the trust_ratio is between 0 and 100
		ensure!(params.trust_ratio <= 100, Error::<T>::InvalidTrustRatio);

		// ensure the vode_mode is in "authority", "stake"
		ensure!(
			is_vec_str(params.vote_mode.clone(),"authority") ||
			is_vec_str(params.vote_mode.clone(),"stake"),
		 Error::<T>::InvalidVoteMode);
        Ok(())


    }

	pub fn set_subnet_params(netuid: u16, mut params: SubnetParams<T>) {
		SubnetParamsStorage::<T>::insert(netuid, params)
	}

	pub fn if_subnet_exist(netuid: u16) -> bool {
		SubnetStateStorage::<T>::contains_key(netuid)
	}

	pub fn get_subnet_min_stake(netuid: u16) -> u64 {
		Self::subnet_params(netuid).min_stake
	}

	pub fn set_min_stake(netuid: u16, stake: u64) {
		SubnetParamsStorage::<T>::mutate(netuid, |subnet_params| {
			subnet_params.min_stake = stake;
		});
	}

	// get the least staked network
	pub fn least_staked_netuid() -> u16 {
		let mut min_stake: u64 = u64::MAX;
		let mut min_stake_netuid: u16 = u16::MAX;
		
		for (netuid, subnet_state) in <SubnetStateStorage<T> as IterableStorageMap<u16, SubnetState>>::iter() {
			if subnet_state.total_stake <= min_stake {
				min_stake = subnet_state.total_stake;
				min_stake_netuid = netuid;
			}
		}
		
		min_stake_netuid
	}

	pub fn address_vector(netuid: u16) -> Vec<Vec<u8>>{
		let mut addresses: Vec<Vec<u8>> = Vec::new();
		
		for (_uid, module_params) in <ModuleParamsStorage<T> as IterableStorageDoubleMap<u16, u16, ModuleParams<T>>>::iter_prefix(netuid) {
			addresses.push(module_params.address);
		}

		return addresses
	}

	pub fn name_vector(netuid: u16) -> Vec<Vec<u8>>{
		let mut names: Vec<Vec<u8>> = Vec::new();

		for (_uid, module_params) in <ModuleParamsStorage<T> as IterableStorageDoubleMap<u16, u16, ModuleParams<T>>>::iter_prefix(netuid) {
			names.push(module_params.name);
		}
		
		return names
	}


	pub fn add_pending_deregistration_uid(netuid: u16, uid: u16) {
		SubnetStateStorage::<T>::mutate(netuid, |subnet_state| {
			subnet_state.pending_deregister_uids.push(uid);
		});
	}

	pub fn add_pending_deregistration_uids(netuid: u16, uids: Vec<u16>) {
		for uid in uids {
			Self::add_pending_deregistration_uid(netuid, uid);
		}
	}
	
	pub fn deregister_pending_uid(netuid: u16) {
		let mut pending_deregister_uids:  Vec<u16> = Self::subnet_state(netuid).pending_deregister_uids;

		if pending_deregister_uids.len() > 0 {
			let n = Self::get_subnet_n_uids(netuid);
			let uid: u16 = pending_deregister_uids.remove(0);

			if uid < n {
				Self::remove_module(netuid, uid);

				SubnetStateStorage::<T>::mutate(netuid, |subnet_state| {
					subnet_state.pending_deregister_uids = pending_deregister_uids;
				});
			}
		}
	}

	pub fn set_max_allowed_uids(netuid: u16, mut max_allowed_uids: u16) {
		let n: u16 = Self::get_subnet_n_uids(netuid);
		if max_allowed_uids < n {
			// limit it at 256 at a time

			let mut remainder_n: u16 = (n - max_allowed_uids);
			let max_remainder = 256;
			if  remainder_n > max_remainder {
				// remove the modules in small amounts, as this can be a heavy load on the chain
				remainder_n = max_remainder;
				max_allowed_uids = n - remainder_n;
			}
			// remove the modules by adding the to the deregister queue
			for i in 0..remainder_n {
				let next_uid: u16= n - 1 - i;
				Self::remove_module(netuid, next_uid);
			}
		}

		SubnetParamsStorage::<T>::mutate(netuid, |subnet_params| {
			subnet_params.max_allowed_uids = max_allowed_uids;
		});
	}

	pub fn uid_in_immunity(netuid: u16, uid: u16) -> bool {
		let block_at_registration: u64 = Self::get_module_registration_block(netuid, uid);
		let immunity_period: u64 = Self::get_immunity_period(netuid) as u64;
		let current_block: u64 = Self::get_current_block_as_u64();
		return current_block - block_at_registration < immunity_period
	}

	pub fn default_subnet_params() -> SubnetParams<T> {
		// get an invalid 
		let default_netuid: u16 = Self::num_subnets() + 1;

		Self::subnet_params(default_netuid)
	}

	pub fn is_subnet_founder(netuid: u16, key: &T::AccountId) -> bool {
		Self::subnet_params(netuid).founder == *key
	}

	pub fn get_subnet_founder(netuid: u16) -> T::AccountId {
		Self::subnet_params(netuid).founder
	}

	pub fn get_self_vote(netuid: u16) -> bool {
		Self::subnet_params(netuid).self_vote
	}

	pub fn market_cap() -> u64 {
		let total_stake: u64 = Self::total_stake();
		
		total_stake
	}

	pub fn get_unit_emission() -> u64 {
		Self::global_params().unit_emission
	}

	// Returns the total amount of stake in the staking table.
	pub fn get_total_emission_per_block() -> u64 {
		let market_cap: u64 = Self::market_cap();
		let mut unit_emission: u64 = Self::get_unit_emission();
		let mut emission_per_block: u64 = unit_emission; // assuming 8 second block times
		let halving_total_stake_checkpoints: Vec<u64> =
			vec![10_000_000, 20_000_000, 30_000_000, 40_000_000]
				.iter()
				.map(|x| x * unit_emission)
				.collect();
		for (i, having_stake) in halving_total_stake_checkpoints.iter().enumerate() {
			let halving_factor = 2u64.pow((i) as u32);
			if market_cap < *having_stake {
				emission_per_block = emission_per_block / halving_factor;
				break
			}
		}

		return emission_per_block
	}

	pub fn get_total_subnet_balance(netuid: u16) -> u64 {
		let keys = Self::get_keys(netuid);
		return keys.iter().map(|x| Self::get_balance_u64(x)).sum()
	}

	pub fn calculate_network_emission(netuid: u16) -> u64 {
		let subnet_stake: I64F64 = I64F64::from_num(Self::get_total_subnet_stake(netuid));
		let total_stake: I64F64 = I64F64::from_num(Self::total_stake());

		let mut subnet_ratio: I64F64 = I64F64::from_num(0);

		if total_stake > I64F64::from_num(0) {
			subnet_ratio = subnet_stake / total_stake;
		} else {
			let n = Self::global_state().total_subnets;
			if n > 1 {
				subnet_ratio = I64F64::from_num(1) / I64F64::from_num(n);
			} else {
				// n == 1
				subnet_ratio = I64F64::from_num(1);
			}
		}

		let total_emission_per_block: u64 = Self::get_total_emission_per_block();
		let token_emission: u64 =
			(subnet_ratio * I64F64::from_num(total_emission_per_block)).to_num::<u64>();

		SubnetStateStorage::<T>::mutate(netuid, |subnet_state| {
			subnet_state.emission = token_emission;
		});

		token_emission
	}

	pub fn get_subnet_emission(netuid: u16) -> u64 {
		return Self::calculate_network_emission(netuid)
	}

	pub fn add_subnet(params: SubnetParams<T>) -> u16 {

		// --- 1. Enfnsure that the network name does not already exist.
		let total_networks: u16 = Self::global_state().total_subnets;
		let max_networks = Self::get_global_max_allowed_subnets();
		let netuid = total_networks;

		Self::set_subnet_params(netuid, params.clone());
		// set stat once network is created
		GlobalStateStorage::<T>::mutate(|global_state| {
			global_state.total_subnets +=  1;
		});

		SubnetStateStorage::<T>::mutate(netuid, |subnet_state| {
			subnet_state.n_uids =  0;
		});

		// --- 6. Emit the new network event.
		Self::deposit_event(Event::NetworkAdded(netuid, params.name.clone()));

		netuid
	}
	
	// Initializes a new subnetwork under netuid with parameters.
	//
	pub fn subnet_name_exists(name: Vec<u8>) -> bool {
		for (netuid, subnet_params) in <SubnetParamsStorage<T> as IterableStorageMap<u16, SubnetParams<T>>>::iter() {
			if subnet_params.name == name {
				return true
			}
		}

		false
	}

	pub fn if_subnet_netuid_exists(netuid: u16) -> bool {
		SubnetStateStorage::<T>::contains_key(netuid)
	}

	pub fn get_netuid_for_name(name: Vec<u8>) -> u16 {
		for (netuid, subnet_params) in <SubnetParamsStorage<T> as IterableStorageMap<u16, SubnetParams<T>>>::iter() {
			if name == subnet_params.name {
				return netuid
			}
		}

		u16::MAX
	}

	pub fn get_subnet_name(netuid: u16) -> Vec<u8> {
		Self::subnet_params(netuid).name
	}

	pub fn remove_subnet(netuid: u16) -> u16 {
		// --- 2. Ensure the network to be removed exists.
		if !Self::if_subnet_exist(netuid) {
			return 0
		}

		let name: Vec<u8> = Self::get_subnet_name(netuid);

		for (uid, stated_amount) in
			<ModuleStateStorage<T> as IterableStorageDoubleMap<u16, u16, ModuleState<T>>>::iter_prefix(netuid)
		{
			Self::remove_stake_from(netuid, uid);
		}

		ModuleStateStorage::<T>::clear_prefix(netuid, u32::MAX, None);
		ModuleParamsStorage::<T>::clear_prefix(netuid, u32::MAX, None);

		SubnetStateStorage::<T>::remove(netuid);
		SubnetParamsStorage::<T>::remove(netuid);

		GlobalStateStorage::<T>::mutate(|global_state| {
			global_state.total_subnets.saturating_sub(1);
		});
		
		// --- 4. Emit the event.
		log::info!("NetworkRemoved( netuid:{:?} )", netuid);
		
		Self::deposit_event(Event::NetworkRemoved(netuid));

		return netuid
	}

	// Returns the number of filled slots on a network.
	///
	pub fn get_subnet_n_uids(netuid: u16) -> u16 {
		Self::subnet_state(netuid).n_uids
	}

	// Returns true if the uid is set on the network.
	//
	pub fn is_uid_exist_on_network(netuid: u16, uid: u16) -> bool {
		ModuleStateStorage::<T>::contains_key(netuid, uid)
	}

	// Returns true if the key holds a slot on the network.
	//
	pub fn is_key_registered(netuid: u16, key: &T::AccountId) -> bool {
		let uid = Self::get_uid_for_key(netuid, key);

		uid != u16::MAX
	}

	// Returs the key under the network uid as a Result. Ok if the uid is taken.
	//
	pub fn get_key_for_uid(netuid: u16, module_uid: u16) -> T::AccountId {
		Self::module_state(netuid, module_uid).module_key
	}

	pub fn get_trust_ratio(netuid: u16) -> u16 {
		Self::subnet_params(netuid).trust_ratio
	}

	pub fn get_quadradic_voting(netuid: u16) -> bool {
		Self::subnet_params(netuid).quadratic_voting
	}

	pub fn if_module_name_exists(netuid: u16, name: Vec<u8>) -> bool {
		for (uid, module_params) in
			<ModuleParamsStorage<T> as IterableStorageDoubleMap<u16, u16, ModuleParams<T>>>::iter_prefix(netuid)
		{
			if module_params.name == name {
				return true
			}
		}

		false
	}

	// Returns the stake of the uid on network or 0 if it doesnt exist.
	//
	pub fn get_stake_for_uid(netuid: u16, module_uid: u16) -> u64 {
		Self::module_state(netuid, module_uid).stake
	}

	// we need to prefix the voting power by the network uid

	pub fn get_vote_mode_subnet(netuid: u16) -> Vec<u8> {
		Self::subnet_params(netuid).vote_mode
	}

	pub fn get_stake_for_key(netuid: u16, key: &T::AccountId) -> u64 {
		for (_uid, module_state) in  <ModuleStateStorage<T> as IterableStorageDoubleMap<u16, u16, ModuleState<T>>>::iter_prefix(netuid)
		{
			if module_state.module_key == *key {
				return module_state.stake;
			}
		}

		0
	}

	// Return the total number of subnetworks available on the chain.
	//
	pub fn num_subnets() -> u16 {
		let mut number_of_subnets: u16 = 0;

		for (_, _) in <SubnetStateStorage<T> as IterableStorageMap<u16, SubnetState>>::iter() {
			number_of_subnets += 1;
		}

		number_of_subnets
	}

	pub fn netuids() -> Vec<u16> {
		let mut netuids : Vec<u16> = Vec::new();
		
		for (netuid, _subnet_state) in <SubnetStateStorage<T> as IterableStorageMap<u16, SubnetState>>::iter() {
			netuids.push(netuid);
		}

		netuids
	}

	pub fn random_netuid() -> u16{
		// get the number of subnets
		let netuids = Self::netuids();
		// get a random number between 0 and number_of_subnets
		let random_netuid_idx: usize = Self::random_idx(netuids.len() as u16) as usize;
		
		netuids[random_netuid_idx]
	}

	// ========================
	// ==== Global Setters ====
	// ========================
	pub fn set_tempo(netuid: u16, tempo: u16) {
		SubnetParamsStorage::<T>::mutate(netuid, |subnet_params| {
			subnet_params.tempo = tempo;
		});
	}

	pub fn set_founder_share(netuid: u16, mut founder_share: u16) {
		if founder_share > 100 {
			founder_share = 100;
		}

		SubnetParamsStorage::<T>::mutate(netuid, |subnet_params| {
			subnet_params.founder_share = founder_share;
		});
	}
	pub fn get_founder_share(netuid: u16) -> u16 {
		Self::subnet_params(netuid).founder_share
	}

	pub fn get_incentive_ratio(netuid: u16) -> u16 {
		Self::subnet_params(netuid).incentive_ratio
	}

	pub fn get_founder(netuid: u16) -> T::AccountId {
		Self::subnet_params(netuid).founder
	}

	pub fn set_founder(netuid: u16, founder: T::AccountId) {
		SubnetParamsStorage::<T>::mutate(netuid, |subnet_params| {
			subnet_params.founder = founder;
		});
	}

	pub fn get_burn_emission_per_epoch(netuid: u16) -> u64 {
		let burn_rate: u16 = Self::get_burn_rate();
		let epoch_emission: u64 = Self::get_subnet_emission(netuid);
		let n: u16 = Self::get_subnet_n_uids(netuid);
		// get the float and convert to u64
		if n == 0 {
			return 0
		}
		let burn_rate_float : I64F64 = I64F64::from_num(burn_rate) / I64F64::from_num(n * 100);
		let burn_emission_per_epoch: u64 = (I64F64::from_num(epoch_emission) * burn_rate_float).to_num::<u64>();

		burn_emission_per_epoch
	}


	// ========================
	// ==== Global Getters ====
	// ========================
	pub fn get_current_block_as_u64() -> u64 {
		TryInto::try_into(<frame_system::Pallet<T>>::block_number())
			.ok()
			.expect("blockchain will not exceed 2^64 blocks; QED.")
	}

	// Emission is the same as the Yomama params

	pub fn set_last_update_for_uid(netuid: u16, uid: u16, last_update: u64) {
		ModuleStateStorage::<T>::mutate(netuid, uid, |module_state| {
			module_state.last_update = last_update;
		});
	}

	pub fn get_emission_for_key(netuid: u16, key: &T::AccountId) -> u64 {
		let uid = Self::get_uid_for_key(netuid, key);
		
		Self::get_emission_for_uid(netuid, uid)
	}

	pub fn get_emission_for_uid(netuid: u16, uid: u16) -> u64 {
		Self::module_state(netuid, uid).emission
	}
	
	pub fn get_last_update_for_uid(netuid: u16, uid: u16) -> u64 {
		Self::module_state(netuid, uid).last_update
	}

	// ============================
	// ==== Subnetwork Getters ====
	// ============================
	pub fn get_tempo(netuid: u16) -> u16 {
		Self::subnet_params(netuid).tempo
	}

	pub fn get_pending_emission(netuid: u16) -> u64 {
		Self::subnet_state(netuid).pending_emission
	}

	pub fn get_registrations_this_block() -> u16 {
		Self::global_state().registrations_per_block
	}

	pub fn get_module_registration_block(netuid: u16, uid: u16) -> u64 {
		Self::module_state(netuid, uid).registration_block
	}

	pub fn get_immunity_period(netuid: u16) -> u16 {
		Self::subnet_params(netuid).immunity_period
	}

	pub fn get_min_allowed_weights(netuid: u16) -> u16 {
		let min_allowed_weights = Self::subnet_params(netuid).min_allowed_weights;
		let n = Self::get_subnet_n_uids(netuid);

		// if n < min_allowed_weights, then return n
		if (n < min_allowed_weights) {
			return n;
		}
		
		min_allowed_weights
	}

	pub fn set_min_allowed_weights(netuid: u16, min_allowed_weights: u16) {
		SubnetParamsStorage::<T>::mutate(netuid, |subnet_params| {
			subnet_params.min_allowed_weights = min_allowed_weights;
		});
	}

	pub fn get_max_allowed_weights(netuid: u16) -> u16 {
		let max_allowed_weights = Self::subnet_params(netuid).max_allowed_weights;
		let n = Self::get_subnet_n_uids(netuid);

		// if n < min_allowed_weights, then return n
		max_allowed_weights.min(n)
	}

	pub fn set_max_allowed_weights(netuid: u16, mut max_allowed_weights: u16) {
		let global_params = Self::global_params();

		SubnetParamsStorage::<T>::mutate(netuid, |subnet_params| {
			subnet_params.max_allowed_weights = max_allowed_weights.min(global_params.max_allowed_weights);
		});
	}

	pub fn get_max_allowed_uids(netuid: u16) -> u16 {
		Self::subnet_params(netuid).max_allowed_uids
	}

	pub fn get_max_allowed_modules() -> u16 {
		Self::global_params().max_allowed_modules
	}

	pub fn total_n() -> u16 {
		let mut total_n: u16 = 0;

		for (netuid, subnet_state) in <SubnetStateStorage<T> as IterableStorageMap<u16, SubnetState>>::iter() {
			total_n += subnet_state.n_uids;
		}
		
		total_n
	}

	pub fn enough_space_for_n(n: u16) -> bool {
		let total_n: u16 = Self::total_n();
		let max_allowed_modules: u16 = Self::get_max_allowed_modules();
		
		total_n + n <= max_allowed_modules
	}

	pub fn set_max_allowed_modules(max_allowed_modules: u16) {
		GlobalParamsStorage::<T>::mutate(|global_params| {
			global_params.max_allowed_modules = max_allowed_modules;
		});
	}

	pub fn get_uids(netuid: u16) -> Vec<u16> {
		let n = Self::get_subnet_n_uids(netuid);
		(0..n).collect()
	}
	pub fn get_keys(netuid: u16) -> Vec<T::AccountId> {
		let uids: Vec<u16> = Self::get_uids(netuid);
		let keys: Vec<T::AccountId> =
			uids.iter().map(|uid| Self::get_key_for_uid(netuid, *uid)).collect();
		return keys
	}

	pub fn get_uid_key_tuples(netuid: u16) -> Vec<(u16, T::AccountId)> {
		let n = Self::get_subnet_n_uids(netuid);
		let mut uid_key_tuples = Vec::<(u16, T::AccountId)>::new();
		for uid in 0..n{
			let key = Self::get_key_for_uid(netuid, uid);
			uid_key_tuples.push((uid, key));
		}
		return uid_key_tuples
	}

	pub fn get_names(netuid: u16) -> Vec<Vec<u8>> {
		let mut names = Vec::<Vec<u8>>::new();
		for (uid, module_params) in
			<ModuleParamsStorage<T> as IterableStorageDoubleMap<u16, u16, ModuleParams<T>>>::iter_prefix(netuid)
		{
			names.push(module_params.name);
		}

		names
	}

	pub fn get_addresses(netuid: u16) -> Vec<Vec<u8>> {
		let mut addresses = Vec::<Vec<u8>>::new();
		
		for (uid, module_params) in
			<ModuleParamsStorage<T> as IterableStorageDoubleMap<u16, u16, ModuleParams<T>>>::iter_prefix(netuid)
		{
			addresses.push(module_params.address);
		}
		
		addresses
	}

	pub fn check_subnet_storage(netuid: u16) -> bool {
		let n = Self::get_subnet_n_uids(netuid);

		let mut module_params_n = 0;
		let mut module_state_n = 0;

		for (_uid, _module_params) in
			<ModuleParamsStorage<T> as IterableStorageDoubleMap<u16, u16, ModuleParams<T>>>::iter_prefix(netuid)
		{
			module_params_n += 1;
		}

		for (_uid, _module_state) in
			<ModuleStateStorage<T> as IterableStorageDoubleMap<u16, u16, ModuleState<T>>>::iter_prefix(netuid)
		{
			module_state_n += 1;
		}
		
		if module_params_n != n || module_state_n != n {
			return false;
		}
	
		true
	}

	pub fn get_emissions(netuid: u16) -> Vec<u64> {
		let mut emissions: Vec<u64> = Vec::new();

		for (_uid, module_state) in
			<ModuleStateStorage<T> as IterableStorageDoubleMap<u16, u16, ModuleState<T>>>::iter_prefix(netuid)
		{
			emissions.push(module_state.emission);
		}

		emissions
	}
	pub fn get_incentives(netuid: u16) -> Vec<u16> {
		let mut incentives: Vec<u16> = Vec::new();

		for (_uid, module_state) in
			<ModuleStateStorage<T> as IterableStorageDoubleMap<u16, u16, ModuleState<T>>>::iter_prefix(netuid)
		{
			incentives.push(module_state.incentive);
		}

		incentives
	}
	pub fn get_trust(netuid: u16) -> Vec<u16> {
		let mut trusts: Vec<u16> = Vec::new();

		for (_uid, module_state) in
			<ModuleStateStorage<T> as IterableStorageDoubleMap<u16, u16, ModuleState<T>>>::iter_prefix(netuid)
		{
			trusts.push(module_state.trust);
		}

		trusts
	}
	pub fn get_dividends(netuid: u16) -> Vec<u16> {
		let mut dividends: Vec<u16> = Vec::new();

		for (_uid, module_state) in
			<ModuleStateStorage<T> as IterableStorageDoubleMap<u16, u16, ModuleState<T>>>::iter_prefix(netuid)
		{
			dividends.push(module_state.dividend);
		}

		dividends
	}

	pub fn get_last_updates(netuid: u16) -> Vec<u64> {
		let mut last_updates: Vec<u64> = Vec::new();

		for (_uid, module_state) in
			<ModuleStateStorage<T> as IterableStorageDoubleMap<u16, u16, ModuleState<T>>>::iter_prefix(netuid)
		{
			last_updates.push(module_state.last_update);
		}

		last_updates
	}

	pub fn set_max_registrations_per_block(max_registrations_per_block: u16) {
		GlobalParamsStorage::<T>::mutate(|global_params| {
			global_params.max_registrations_per_block = max_registrations_per_block;
		});
	}

	pub fn is_registered(netuid: u16, key: &T::AccountId) -> bool {
		let uid = Self::get_uid_for_key(netuid, key);

		uid != u16::MAX
	}

	pub fn get_pending_deregister_uids(netuid: u16) -> Vec<u16> {
		Self::subnet_state(netuid).pending_deregister_uids
	}
}
