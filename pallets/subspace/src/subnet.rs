use super::*;
use codec::Compact;
use frame_support::{
	pallet_prelude::{Decode, DispatchError, DispatchResult, Encode},
	storage::IterableStorageMap,
	traits::Currency,
	IterableStorageDoubleMap,
};
use sp_runtime::BoundedVec;
use crate::utils::is_vec_str;
use frame_system::ensure_root;
pub use sp_std::{vec, vec::Vec};
use substrate_fixed::types::{I32F32, I64F64};
extern crate alloc;

impl<T: Config> Pallet<T> {
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
		
		ensure!(is_vec_str(Self::get_vote_mode_subnet(netuid), "authority"), Error::<T>::NotAuthorityMode);
		ensure!(Self::if_subnet_netuid_exists(netuid), Error::<T>::SubnetNameAlreadyExists);
		ensure!(Self::is_subnet_founder(netuid, &key), Error::<T>::NotFounder);
		ensure!(Self::if_subnet_netuid_exists(netuid), Error::<T>::SubnetNameAlreadyExists);
		ensure!(Self::is_subnet_founder(netuid, &key), Error::<T>::NotFounder);

		let mut n = Self::subnet_n(netuid);

		while n > params.max_allowed_uids {
			Self::remove_module(netuid, Self::get_lowest_uid(netuid));

			n = Self::subnet_n(netuid);
		}
		
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
		Self::subnet_params(netuid).name.len() > 0
	}

	pub fn get_min_stake(netuid: u16) -> u64 {
		Self::subnet_params(netuid).min_stake
	}

	pub fn set_min_stake(netuid: u16, stake: u64) {
		let mut subnet_params = Self::subnet_params(netuid);;
		subnet_params.min_stake = stake;
		Self::set_subnet_params(netuid, subnet_params)

	}


	pub fn get_max_stake(netuid: u16) -> u64 {
		Self::subnet_params(netuid).max_stake
	}

	pub fn set_max_stake(netuid: u16, stake: u64) {
		let mut subnet_params = Self::subnet_params(netuid);;

		subnet_params.max_stake = stake;

		Self::set_subnet_params(netuid, subnet_params)
	}

	// get the least staked network
	pub fn least_staked_netuid() -> u16 {
		let mut min_stake: u64 = u64::MAX;
		let mut min_stake_netuid: u16 = u16::MAX;
		for (netuid, subnet_state) in <SubnetStateStorage<T> as IterableStorageMap<u16, SubnetState>>::iter() {
			let net_stake = subnet_state.total_stake;

			if net_stake <= min_stake && net_stake > 0 {
				min_stake = net_stake;
				min_stake_netuid = netuid;
			}
		}
		return min_stake_netuid
	}

	pub fn address_vector(netuid: u16) -> Vec<Vec<u8>>{
		let mut addresses: Vec<Vec<u8>> = Vec::new();
		for (uid, address) in <Address<T> as IterableStorageDoubleMap<u16, u16, Vec<u8>>>::iter_prefix(netuid) {
			addresses.push(address);
		}
		return addresses
	}

	pub fn name_vector(netuid: u16) -> Vec<Vec<u8>>{
		let mut names: Vec<Vec<u8>> = Vec::new();
		for (uid, name) in <Name<T> as IterableStorageDoubleMap<u16, u16, Vec<u8>>>::iter_prefix(netuid) {
			names.push(name);
		}
		return names
	}


	// get the least staked network
	pub fn min_subnet_stake() -> u64 {
		let mut min_stake: u64 = u64::MAX;
		for (netuid, subnet_state) in <SubnetStateStorage<T> as IterableStorageMap<u16, SubnetState>>::iter() {
			let net_stake = subnet_state.total_stake;
			
			if net_stake <= min_stake {
				min_stake = net_stake;
			}
		}
		return min_stake
	}

	pub fn get_network_stake(netuid: u16) -> u64 {
		Self::subnet_state(netuid).total_stake
	}

	pub fn add_pending_deregistration_uid(netuid: u16, uid: u16) {
		let mut subnet_state = Self::subnet_state(netuid);;

		subnet_state.pending_deregister_uids.push(uid);

		Self::set_subnet_state(netuid, subnet_state)
	}

	pub fn add_pending_deregistration_uids(netuid: u16, uids: Vec<u16>) {
		for uid in uids {
			Self::add_pending_deregistration_uid(netuid, uid);
		}
	}
	
	pub fn deregister_pending_uid(netuid: u16) {
		let mut subnet_state = Self::subnet_state(netuid);;

		if subnet_state.pending_deregister_uids.len() > 0 {
			let n = Self::subnet_n(netuid);
			let uid: u16 = subnet_state.pending_deregister_uids.remove(0);

			if uid < n {
				Self::remove_module(netuid, uid);
			}

			Self::set_subnet_state(netuid, subnet_state);
		}
	}

	pub fn set_max_allowed_uids(netuid: u16, mut max_allowed_uids: u16) {
		let n: u16 = Self::subnet_n(netuid);
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

		let mut subnet_params = Self::subnet_params(netuid);;

		subnet_params.max_allowed_uids = max_allowed_uids;

		Self::set_subnet_params(netuid, subnet_params)
	}



	pub fn set_name_subnet(netuid: u16, name: Vec<u8>) {
		// set the name if it doesnt exist
		let mut subnet_params = Self::subnet_params(netuid);;
		subnet_params.name = name;
		Self::set_subnet_params(netuid, subnet_params)
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
		return Self::subnet_params(default_netuid)
	}


	pub fn is_subnet_founder(netuid: u16, key: &T::AccountId) -> bool {
		return Self::get_subnet_founder(netuid) == *key
	}


	pub fn get_subnet_founder(netuid: u16) -> T::AccountId {
		return Self::subnet_params(netuid).founder
	}

	pub fn get_self_vote(netuid: u16) -> bool {
		Self::subnet_params(netuid).self_vote
	}


	// pub fn total_balance() -> u64 {
	//     let mut total_balance: u64 = 0;
	//     // iterate through all of the accounts with balance (noo stake)

	//     for ( key, stated_amount ) in <Stake<T> as IterableStorageDoubleMap<u16, T::AccountId,
	// u64> >::iter(){         total_balance = Self::get_balance_u64( &key ) + total_balance;
	//     }
	//     return total_balance;
	// }

	pub fn market_cap() -> u64 {
		let total_stake: u64 = Self::total_stake();
		return total_stake
	}

	pub fn get_unit_emission() -> u64 {
		let global_params = Self::global_params();
		return global_params.unit_emission
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

		let mut subnet_state = Self::subnet_state(netuid);;
		subnet_state.emission = token_emission;
		Self::set_subnet_state(netuid, subnet_state);

		return token_emission
	}

	pub fn set_subnet_state(netuid: u16, mut subnet_state: SubnetState) {
		SubnetStateStorage::<T>::insert(netuid, subnet_state)
	}
	pub fn get_subnet_emission(netuid: u16) -> u64 {
		return Self::calculate_network_emission(netuid)
	}

	pub fn add_subnet(params: SubnetParams<T>) -> u16 {
		let mut global_state = Self::global_state();
		// --- 1. Enfnsure that the network name does not already exist.
		let netuid =  global_state.total_subnets;
		// set stat once network is created
		global_state.total_subnets += 1;

		// set the subnet_params
		Self::set_subnet_params(netuid, params.clone());

		// set the subnet state
		Self::set_subnet_state(netuid, Self::subnet_state(netuid));

		// update the global_state
		Self::set_global_state(global_state);
		// --- 6. Emit the new network event.
		Self::deposit_event(Event::NetworkAdded(netuid, params.name.clone()));

		return netuid
	}
	
	// Initializes a new subnetwork under netuid with parameters.
	//
	pub fn if_subnet_name_exists(name: Vec<u8>) -> bool {
		for (netuid, subnet_params) in SubnetParamsStorage::<T>::iter() {
			if subnet_params.name == name {
				return true
			}
		}

		return false
	}

	pub fn subnet_name_exists(name: Vec<u8>) -> bool {
		return Self::if_subnet_name_exists(name.clone()).into()
	}

	pub fn if_subnet_netuid_exists(netuid: u16) -> bool {
		Self::subnet_params(netuid).name.len() > 0
	}

	pub fn get_netuid_for_name(name: Vec<u8>) -> u16 {
		for (netuid, subnet_params) in SubnetParamsStorage::<T>::iter() {
			if name == subnet_params.name {
				return netuid
			}
		}

		return u16::MAX
	}

	pub fn subnet_name(netuid: u16) -> Vec<u8> {
		Self::subnet_params(netuid).name
	}

	pub fn is_network_founder(netuid: u16, key: &T::AccountId) -> bool {
		// Returns true if the account is the founder of the network.
		let founder = Self::get_founder(netuid);
		return founder == *key
	}

	pub fn remote_subnet_for_name(name: Vec<u8>) -> u16 {
		let netuid = Self::get_netuid_for_name(name.clone());
		return Self::remove_subnet(netuid)
	}

	pub fn remove_netuid_stake_storage(netuid: u16) {
		// --- 1. Erase network stake, and remove network from list of networks.
		for (key, stated_amount) in
			<Stake<T> as IterableStorageDoubleMap<u16, T::AccountId, u64>>::iter_prefix(netuid)
		{
			Self::remove_stake_from_storage(netuid, &key);
		}
		// --- 4. Remove all stake.
		Stake::<T>::remove_prefix(netuid, None);

		let mut subnet_state = Self::subnet_state(netuid);;

		subnet_state.total_stake = 0;

		SubnetStateStorage::<T>::insert(netuid, subnet_state);
	}

	pub fn remove_subnet(netuid: u16) -> u16 {
		// --- 2. Ensure the network to be removed exists.
		if !Self::if_subnet_exist(netuid) {
			return 0
		}
		let name: Vec<u8> = Self::subnet_name(netuid);

		Self::remove_netuid_stake_storage(netuid);

		Uids::<T>::clear_prefix(netuid, u32::max_value(), None);
		Keys::<T>::clear_prefix(netuid, u32::max_value(), None);

		Name::<T>::clear_prefix(netuid, u32::max_value(), None);
		Address::<T>::clear_prefix(netuid, u32::max_value(), None);
		Weights::<T>::clear_prefix(netuid, u32::max_value(), None);
		Emission::<T>::remove(netuid);
		Incentive::<T>::remove(netuid);
		Dividends::<T>::remove(netuid);
		Trust::<T>::remove(netuid);
		LastUpdate::<T>::remove(netuid);
		DelegationFee::<T>::clear_prefix(netuid, u32::max_value(), None);
		RegistrationBlock::<T>::clear_prefix(netuid, u32::max_value(), None);
		
		
		// --- 2. Erase subnet parameters.
		SubnetStateStorage::<T>::remove(netuid);
		SubnetParamsStorage::<T>::remove(netuid);

		let mut global_state = Self::global_state();
		global_state.total_subnets = global_state.total_subnets.saturating_sub(1);
		GlobalStateStorage::<T>::put(global_state);

		// --- 4. Emit the event.
		log::info!("NetworkRemoved( netuid:{:?} )", netuid);
		Self::deposit_event(Event::NetworkRemoved(netuid));

		return netuid
	}

	// Returns the number of filled slots on a network.
	///
	pub fn subnet_n(netuid: u16) -> u16 {
		Self::subnet_state(netuid).n
	}

	// Returns true if the uid is set on the network.
	//
	pub fn is_uid_exist_on_network(netuid: u16, uid: u16) -> bool {
		return Keys::<T>::contains_key(netuid, uid)
	}

	// Returns true if the key holds a slot on the network.
	//
	pub fn is_key_registered_on_network(netuid: u16, key: &T::AccountId) -> bool {
		return Uids::<T>::contains_key(netuid, key)
	}

	pub fn is_key_registered(netuid: u16, key: &T::AccountId) -> bool {
		return Uids::<T>::contains_key(netuid, key)
	}




	pub fn is_key_registered_on_any_network( key: &T::AccountId) -> bool {
		for netuid in Self::netuids() {
			if Uids::<T>::contains_key(netuid, key) {
				return true
			}
		}
		return false
	}

	// Returs the key under the network uid as a Result. Ok if the uid is taken.
	//
	pub fn get_key_for_uid(netuid: u16, module_uid: u16) -> T::AccountId {
		Keys::<T>::try_get(netuid, module_uid).unwrap()
	}

	// Returns the uid of the key in the network as a Result. Ok if the key has a slot.
	//
	pub fn get_uid_for_key(netuid: u16, key: &T::AccountId) -> u16 {
		return Uids::<T>::get(netuid, key).unwrap_or(0)
	}


	pub fn get_trust_ratio(netuid: u16) -> u16 {
		Self::subnet_params(netuid).trust_ratio
	}

	pub fn set_trust_ratio(netuid: u16, trust_ratio: u16) {
		let mut subnet_params = Self::subnet_params(netuid);;
		subnet_params.trust_ratio = trust_ratio;
		Self::set_subnet_params(netuid, subnet_params)
	}

	pub fn get_quadratic_voting(netuid: u16) -> bool {
		Self::subnet_params(netuid).quadratic_voting
	}

	pub fn set_quadratic_voting(netuid: u16, quadratic_voting: bool) {
		let mut subnet_params = Self::subnet_params(netuid);;
		subnet_params.quadratic_voting = quadratic_voting;
		Self::set_subnet_params(netuid, subnet_params)
	}



	// Returns the stake of the uid on network or 0 if it doesnt exist.
	//
	pub fn get_stake_for_uid(netuid: u16, module_uid: u16) -> u64 {
		return Self::get_stake_for_key(netuid, &Self::get_key_for_uid(netuid, module_uid))
	}

	// we need to prefix the voting power by the network uid

	pub fn set_vote_threshold_subnet(netuid: u16, vote_threshold: u16) {
		let mut subnet_params = Self::subnet_params(netuid);;

		subnet_params.vote_threshold = vote_threshold;

		Self::set_subnet_params(netuid, subnet_params)
	}
	
	pub fn get_vote_mode_subnet(netuid: u16) -> Vec<u8> {
		Self::subnet_params(netuid).vote_mode
	}

	pub fn set_vote_mode_subnet(netuid: u16, vote_mode: Vec<u8>) {
		let mut subnet_params = Self::subnet_params(netuid);;

		subnet_params.vote_mode = vote_mode;

		Self::set_subnet_params(netuid, subnet_params)
	}
	
	pub fn get_subnet_vote_threshold(netuid: u16) -> u16 {
		Self::subnet_params(netuid).vote_threshold
	}

	pub fn get_stake_for_key(netuid: u16, key: &T::AccountId) -> u64 {
		return Stake::<T>::get(netuid, key)
	}

	// Return the total number of subnetworks available on the chain.
	//
	pub fn num_subnets() -> u16 {
		return Self::global_state().total_subnets
	}

	pub fn netuids() -> Vec<u16> {
		let mut netuids : Vec<u16> = Vec::new();

		for netuid in SubnetStateStorage::<T>::iter_keys() {
			netuids.push(netuid);
		}

		return netuids
	}

	
	pub fn random_netuid() -> u16{
		// get the number of subnets
		let netuids = Self::netuids();
		// get a random number between 0 and number_of_subnets
		let random_netuid_idx: usize = Self::random_idx(netuids.len() as u16) as usize;
		return netuids[random_netuid_idx]
	}

	// ========================
	// ==== Global Setters ====
	// ========================
	pub fn set_tempo(netuid: u16, tempo: u16) {
		let mut subnet_params = Self::subnet_params(netuid);;

		subnet_params.tempo = tempo;

		Self::set_subnet_params(netuid, subnet_params)
	}
	pub fn get_founder_share(netuid: u16) -> u16 {
		Self::subnet_params(netuid).founder_share
	}

	pub fn get_registration_block_for_uid(netuid: u16, uid: u16) -> u64 {
		return RegistrationBlock::<T>::get(netuid, uid)
	}
	
	pub fn get_incentive_ratio(netuid: u16) -> u16 {
		Self::subnet_params(netuid).incentive_ratio
	}

	pub fn get_founder(netuid: u16) -> T::AccountId {
		Self::subnet_params(netuid).founder
	}

	pub fn get_burn_emission_per_epoch(netuid: u16) -> u64 {
		let burn_rate: u16 = Self::get_burn_rate();
		let epoch_emission: u64 = Self::get_subnet_emission(netuid);
		let n: u16 = Self::subnet_n(netuid);
		// get the float and convert to u64
		if n == 0 {
			return 0
		}
		// get the float and convert to u64
		let burn_rate_float : I64F64 = I64F64::from_num(burn_rate) / I64F64::from_num(n * 100);
		let burn_emission_per_epoch: u64 = (I64F64::from_num(epoch_emission) * burn_rate_float).to_num::<u64>();

		return burn_emission_per_epoch
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
		let mut updated_last_update_vec = Self::get_last_update(netuid);
		if (uid as usize) < updated_last_update_vec.len() {
			updated_last_update_vec[uid as usize] = last_update;
			LastUpdate::<T>::insert(netuid, updated_last_update_vec);
		}
	}

	pub fn get_emission_for_key(netuid: u16, key: &T::AccountId) -> u64 {
		let uid = Self::get_uid_for_key(netuid, key);
		return Self::get_emission_for_uid(netuid, uid)
	}
	pub fn get_emission_for_uid(netuid: u16, uid: u16) -> u64 {
		let emissions = Self::get_emissions(netuid);

		if (uid as usize) < emissions.len() {
			return emissions[uid as usize]
		} else {
			return 0
		}
	}
	pub fn get_incentive_for_uid(netuid: u16, uid: u16) -> u16 {
		let incentives = Self::get_incentives(netuid);

		if (uid as usize) < incentives.len() {
			return incentives[uid as usize]
		} else {
			return 0
		}
	}
	
	pub fn get_dividends_for_uid(netuid: u16, uid: u16) -> u16 {
		let dividends = Self::get_dividends(netuid);

		if (uid as usize) < dividends.len() {
			return dividends[uid as usize]
		} else {
			return 0
		}
	}

	pub fn get_last_update_for_uid(netuid: u16, uid: u16) -> u64 {
		let last_updates = Self::get_last_update(netuid);

		if (uid as usize) < last_updates.len() {
			return last_updates[uid as usize]
		} else {
			return 0
		}
	}

	pub fn get_global_max_allowed_subnets() -> u16 {
		Self::global_params().max_allowed_subnets
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
		RegistrationBlock::<T>::get(netuid, uid)
	}

	pub fn get_module_age(netuid: u16, uid: u16) -> u64 {
		return Self::get_current_block_as_u64() - Self::get_module_registration_block(netuid, uid)
	}



	pub fn get_immunity_period(netuid: u16) -> u16 {
		Self::subnet_params(netuid).immunity_period
	}

	pub fn get_min_allowed_weights(netuid: u16) -> u16 {
		let min_allowed_weights = Self::subnet_params(netuid).min_allowed_weights;
		let n = Self::subnet_n(netuid);
		// if n < min_allowed_weights, then return n
		if (n < min_allowed_weights) {
			return n
		} else {
			return min_allowed_weights
		}
	}
	pub fn set_min_allowed_weights(netuid: u16, min_allowed_weights: u16) {
		let mut subnet_params = Self::subnet_params(netuid);;

		subnet_params.min_allowed_weights = min_allowed_weights;

		Self::set_subnet_params(netuid, subnet_params)
	}

	pub fn get_max_allowed_weights(netuid: u16) -> u16 {
		let max_allowed_weights = Self::subnet_params(netuid).max_allowed_weights;
		let n = Self::subnet_n(netuid);
		// if n < min_allowed_weights, then return n
		return max_allowed_weights.min(n)
	}
	pub fn set_max_allowed_weights(netuid: u16, mut max_allowed_weights: u16) {
		let global_params = Self::global_params();

		let mut subnet_params = Self::subnet_params(netuid);;

		subnet_params.max_allowed_weights = max_allowed_weights.min(global_params.max_allowed_weights);

		Self::set_subnet_params(netuid, subnet_params)
	}

	pub fn get_max_allowed_uids(netuid: u16) -> u16 {
		Self::subnet_params(netuid).max_allowed_uids
	}

	pub fn get_max_allowed_modules() -> u16 {
		Self::global_params().max_allowed_modules
	}

	pub fn total_n() -> u16 {
		let mut total_n: u16 = 0;
		for subnet_state in SubnetStateStorage::<T>::iter_values() {
			total_n += subnet_state.n;
		}
		return total_n
	}

	pub fn enough_space_for_n(n: u16) -> bool {
		let total_n: u16 = Self::total_n();
		let max_allowed_modules: u16 = Self::get_max_allowed_modules();
		return total_n + n <= max_allowed_modules
	}

	pub fn set_max_allowed_modules(max_allowed_modules: u16) {
		let mut global_params = Self::global_params();

		global_params.max_allowed_modules = max_allowed_modules;

		GlobalParamsStorage::<T>::put(global_params)
	}

	pub fn get_uids(netuid: u16) -> Vec<u16> {
		let n = Self::subnet_n(netuid);
		return (0..n).collect()
	}
	pub fn get_keys(netuid: u16) -> Vec<T::AccountId> {
		let uids: Vec<u16> = Self::get_uids(netuid);
		let keys: Vec<T::AccountId> =
			uids.iter().map(|uid| Self::get_key_for_uid(netuid, *uid)).collect();
		return keys
	}

	pub fn get_uid_key_tuples(netuid: u16) -> Vec<(u16, T::AccountId)> {
		let n = Self::subnet_n(netuid);
		let mut uid_key_tuples = Vec::<(u16, T::AccountId)>::new();
		for uid in 0..n{
			let key = Self::get_key_for_uid(netuid, uid);
			uid_key_tuples.push((uid, key));
		}
		return uid_key_tuples
	}

	pub fn get_names(netuid: u16) -> Vec<Vec<u8>> {
		let mut names = Vec::<Vec<u8>>::new();
		for (uid, name) in
			<Name<T> as IterableStorageDoubleMap<u16, u16, Vec<u8>>>::iter_prefix(netuid)
		{
			names.push(name);
		}
		return names
	}

	pub fn get_addresses(netuid: u16) -> Vec<T::AccountId> {
		let mut addresses = Vec::<T::AccountId>::new();
		for (key, uid) in
			<Uids<T> as IterableStorageDoubleMap<u16, T::AccountId, u16>>::iter_prefix(netuid)
		{
			addresses.push(key);
		}
		return addresses
	}

	pub fn is_subnet_removed(netuid: u16) -> bool {
		return Self::check_subnet_storage(netuid)
	}


	pub fn check_subnet_storage(netuid: u16) -> bool {
		let n = Self::subnet_n(netuid);
		let mut uids = Self::get_uids(netuid);
		let mut keys = Self::get_keys(netuid);
		let mut names = Self::get_names(netuid);
		let mut addresses = Self::get_addresses(netuid);
		let mut emissions = Self::get_emissions(netuid);
		let mut incentives = Self::get_incentives(netuid);
		let mut dividends = Self::get_dividends(netuid);
		let mut last_update = Self::get_last_update(netuid);
		

		if (n as usize) != uids.len() {
			return false
		}
		if (n as usize) != keys.len() {
			return false
		}
		if (n as usize) != names.len() {
			return false
		}
		if (n as usize) != addresses.len() {
			return false
		}
		if (n as usize) != emissions.len() {
			return false
		}
		if (n as usize) != incentives.len() {
			return false
		}
		if (n as usize) != dividends.len() {
			return false
		}
		if (n as usize) != last_update.len() {
			return false
		}
		
		// length of addresss
		let name_vector = Self::name_vector(netuid);
		if (n as usize) != name_vector.len() {
			return false
		}
		
		// length of addresss
		let address_vector = Self::address_vector(netuid);
		if (n as usize) != address_vector.len() {
			return false
		}
			
		return true
	}

	pub fn get_emissions(netuid: u16) -> Vec<u64> {
		Emission::<T>::get(netuid)
	}
	pub fn get_incentives(netuid: u16) -> Vec<u16> {
		Incentive::<T>::get(netuid)
	}
	pub fn get_trust(netuid: u16) -> Vec<u16> {
		Trust::<T>::get(netuid)
	}
	pub fn get_dividends(netuid: u16) -> Vec<u16> {
		Dividends::<T>::get(netuid)
	}
	pub fn get_last_update(netuid: u16) -> Vec<u64> {
		LastUpdate::<T>::get(netuid)
	}
	pub fn set_max_registrations_per_block(max_registrations_per_block: u16) {
		let mut global_params = Self::global_params();

		global_params.max_registrations_per_block = max_registrations_per_block;

		
	}

	pub fn is_registered(netuid: u16, key: &T::AccountId) -> bool {
		return Uids::<T>::contains_key(netuid, &key)
	}

	pub fn is_uid_registered(netuid: u16, uid: u16) -> bool {
		return Keys::<T>::contains_key(netuid, uid)
	}

	pub fn get_max_weight_age(netuid: u16) -> u64 {
		Self::subnet_params(netuid).max_weight_age
	}

	pub fn set_max_weight_age(netuid: u16, max_weight_age: u64) {
		let mut subnet_params = Self::subnet_params(netuid);;

		subnet_params.max_weight_age = max_weight_age;

		Self::set_subnet_params(netuid, subnet_params)
	}

	pub fn get_pending_deregister_uids(netuid: u16) -> Vec<u16> {
		Self::subnet_state(netuid).pending_deregister_uids
	}
}
