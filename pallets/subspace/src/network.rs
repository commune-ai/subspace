use super::*;
use codec::Compact;
use frame_support::{
	pallet_prelude::{Decode, DispatchError, DispatchResult, Encode},
	storage::IterableStorageMap,
	traits::Currency,
	IterableStorageDoubleMap,
};
use frame_system::ensure_root;
pub use sp_std::{vec, vec::Vec};
use substrate_fixed::types::{I32F32, I64F64};
extern crate alloc;

impl<T: Config> Pallet<T> {
	// Returns true if the subnetwork exists.
	//
	pub fn if_subnet_exist(netuid: u16) -> bool {
		return N::<T>::contains_key(netuid)
	}

	// Returns true if the subnetwork exists.
	pub fn subnet_exists(netuid: u16) -> bool {
		return N::<T>::contains_key(netuid)
	}

	pub fn get_min_stake(netuid: u16) -> u64 {
		return MinStake::<T>::get(netuid)
	}

	pub fn set_min_stake(netuid: u16, stake: u64) {
		MinStake::<T>::insert(netuid, stake)
	}

	// get the least staked network
	pub fn least_staked_netuid() -> u16 {
		let mut min_stake: u64 = u64::MAX;
		let mut min_stake_netuid: u16 = u16::MAX;
		for (netuid, net_stake) in <TotalStake<T> as IterableStorageMap<u16, u64>>::iter() {
			if net_stake <= min_stake {
				min_stake = net_stake;
				min_stake_netuid = netuid;
			}
		}
		return min_stake_netuid
	}

	pub fn enough_stake_to_start_network(stake: u64) -> bool {
		let num_subnets: u16 = Self::get_number_of_subnets();
		let max_subnets: u16 = MaxAllowedSubnets::<T>::get();
		// if we have not reached the max number of subnets, then we can start a new one
		if num_subnets < max_subnets {
			return true
		}
		// if we have reached the max number of subnets, then we can start a new one if the stake is
		// greater than the least staked network
		if Self::get_number_of_subnets() == 0 {
			return true
		}
		// if we have reached the max number of subnets, then we can start a new one if the stake is
		// greater than the least staked network
		return stake > Self::min_subnet_stake()
	}

	// get the least staked network
	pub fn min_subnet_stake() -> u64 {
		let mut min_stake: u64 = u64::MAX;
		for (netuid, net_stake) in <TotalStake<T> as IterableStorageMap<u16, u64>>::iter() {
			if net_stake <= min_stake {
				min_stake = net_stake;
			}
		}
		return min_stake
	}

	pub fn get_network_stake(netuid: u16) -> u64 {
		return TotalStake::<T>::get(netuid)
	}

	pub fn do_add_network(origin: T::RuntimeOrigin, name: Vec<u8>, stake: u64) -> DispatchResult {
		let key = ensure_signed(origin)?;
		// --- 1. Ensure the network name does not already exist.
		if Self::get_number_of_subnets() > 0 {
			ensure!(
				!Self::if_subnet_name_exists(name.clone()),
				Error::<T>::SubnetNameAlreadyExists
			);
			ensure!(
				Self::enough_stake_to_start_network(stake),
				Error::<T>::NotEnoughStakeToStartNetwork
			);
		}

		let subnet_params: SubnetParams<T> = Self::default_subnet_params();
		Self::add_network(
			name.clone(),
			subnet_params.tempo,
			subnet_params.immunity_period,
			subnet_params.min_allowed_weights,
			subnet_params.max_allowed_weights,
			subnet_params.max_allowed_uids,
			subnet_params.min_stake,
			&key.clone(), // founder
			stake,        //stake
		);
		// --- 16. Ok and done.
		Ok(())
	}

	pub fn do_remove_network(origin: T::RuntimeOrigin, netuid: u16) -> DispatchResult {
		let key = ensure_signed(origin)?;
		// --- 1. Ensure the network name does not already exist.

		ensure!(Self::if_subnet_netuid_exists(netuid), Error::<T>::SubnetNameAlreadyExists);
		ensure!(Self::is_subnet_founder(netuid, &key), Error::<T>::NotSubnetFounder);

		Self::remove_network_for_netuid(netuid);
		// --- 16. Ok and done.
		Ok(())
	}

	pub fn do_update_network(
		origin: T::RuntimeOrigin,
		netuid: u16,
		name: Vec<u8>,
		immunity_period: u16,
		min_allowed_weights: u16,
		max_allowed_weights: u16,
		max_allowed_uids: u16,
		min_stake: u64,
		tempo: u16,
		founder: T::AccountId,
	) -> DispatchResult {
		let key = ensure_signed(origin)?;

		ensure!(Self::if_subnet_netuid_exists(netuid), Error::<T>::SubnetNameAlreadyExists);
		ensure!(Self::is_subnet_founder(netuid, &key), Error::<T>::NotSubnetFounder);

		Self::update_network_for_netuid(
			netuid,
			name.clone(),
			immunity_period,
			min_allowed_weights,
			max_allowed_weights,
			max_allowed_uids,
			min_stake,
			tempo,
			founder,
		);
		// --- 16. Ok and done.
		Ok(())
	}

	pub fn do_propose_network_update(
		origin: T::RuntimeOrigin,
		netuid: u16,
		name: Vec<u8>,
		immunity_period: u16,
		min_allowed_weights: u16,
		max_allowed_weights: u16,
		max_allowed_uids: u16,
		min_stake: u64,
		tempo: u16,
		vote_period: u16,
		vote_threshold: u16,
		founder: T::AccountId,
	) -> DispatchResult {
		let key = ensure_signed(origin)?;

		ensure!(Self::if_subnet_netuid_exists(netuid), Error::<T>::SubnetNameAlreadyExists);
		ensure!(Self::is_subnet_founder(netuid, &key), Error::<T>::NotSubnetFounder);

		let params: SubnetParams<T> = SubnetParams {
			name: name.clone(),
			immunity_period,
			min_allowed_weights,
			max_allowed_weights,
			max_allowed_uids,
			min_stake,
			tempo,
			founder: founder.clone(),
			vote_period,
			vote_threshold,
		};
		let proposal = SubnetProposal {
			params,
			votes: Self::get_stake_for_key(netuid, &key),
			proposer: key.clone(),
		};

		// --- 16. Ok and done.
		Ok(())
	}

	pub fn review_proposal(netuid: u16, proposal: SubnetProposal<T>) {
		let mut total_subnet_stake: u64 = TotalStake::<T>::get(netuid);
		let mut vote_threshold: u64 =
			total_subnet_stake * proposal.params.vote_threshold as u64 / 100;
		if (proposal.votes > vote_threshold) {
			Self::update_network_from_params(netuid, proposal.params);
		}
	}

	pub fn update_network_from_params(netuid: u16, params: SubnetParams<T>) {
		return Self::update_network_for_netuid(
			netuid,
			params.name,
			params.immunity_period,
			params.min_allowed_weights,
			params.max_allowed_weights,
			params.max_allowed_uids,
			params.min_stake,
			params.tempo,
			params.founder,
		)
	}

	pub fn update_network_for_netuid(
		netuid: u16,
		name: Vec<u8>,
		mut immunity_period: u16,
		mut min_allowed_weights: u16,
		mut max_allowed_weights: u16,
		mut max_allowed_uids: u16,
		mut min_stake: u64,
		tempo: u16,
		founder: T::AccountId,
	) {
		let n: u16 = Self::get_subnet_n(netuid);

		// TEMPO, IMMUNITY_PERIOD, MIN_ALLOWED_WEIGHTS, MAX_ALLOWED_WEIGHTS, MAX_ALLOWED_UIDS,
		// MAX_IMMUNITY_RATIO
		Tempo::<T>::insert(netuid, tempo);
		MaxAllowedUids::<T>::insert(netuid, max_allowed_uids);

		// IMMUNITY PERIOD AND MAX IMMUNITY RATIO

		ImmunityPeriod::<T>::insert(netuid, immunity_period);

		// MAX_ALLOWED_WEIGHTS
		if max_allowed_weights > max_allowed_uids {
			max_allowed_weights = max_allowed_uids;
		}
		MaxAllowedWeights::<T>::insert(netuid, max_allowed_weights);

		// MIN_ALLOWED_WEIGHTS
		// make sure the min_allowed_weights is less than the max_allowed_weights
		if min_allowed_weights > max_allowed_weights {
			min_allowed_weights = max_allowed_weights;
		}
		MinAllowedWeights::<T>::insert(netuid, min_allowed_weights);
		Founder::<T>::insert(netuid, founder);
		// remove the modules if the max_allowed_uids is less than the current number of modules

		if max_allowed_uids < n {
			let remainder_n: u16 = n - max_allowed_uids;
			for i in 0..remainder_n {
				Self::remove_module(netuid, Self::get_lowest_uid(netuid));
			}
		}

		MinStake::<T>::insert(netuid, min_stake);

		if name.len() > 0 {
			// update the name if it is not empty
			let old_name: Vec<u8> = Self::get_name_for_netuid(netuid);
			SubnetNamespace::<T>::remove(old_name.clone());
			SubnetNamespace::<T>::insert(name.clone(), netuid)
		}
	}

	pub fn uid_in_immunity(netuid: u16, uid: u16) -> bool {
		let block_at_registration: u64 = Self::get_module_registration_block(netuid, uid);
		let immunity_period: u64 = Self::get_immunity_period(netuid) as u64;
		let current_block: u64 = Self::get_current_block_as_u64();
		return current_block - block_at_registration < immunity_period
	}

	pub fn get_subnet_params(netuid: u16) -> SubnetParams<T> {
		SubnetParams {
			immunity_period: ImmunityPeriod::<T>::get(netuid),
			min_allowed_weights: MinAllowedWeights::<T>::get(netuid),
			max_allowed_weights: MaxAllowedWeights::<T>::get(netuid),
			max_allowed_uids: MaxAllowedUids::<T>::get(netuid),
			min_stake: MinStake::<T>::get(netuid),
			tempo: Tempo::<T>::get(netuid),
			founder: Founder::<T>::get(netuid),
			name: <Vec<u8>>::new(),
			vote_period: VotePeriod::<T>::get(netuid),
			vote_threshold: VoteThreshold::<T>::get(netuid),
		}
	}
	pub fn default_subnet_params() -> SubnetParams<T> {
		let default_netuid: u16 = Self::get_number_of_subnets() + 1;
		return Self::get_subnet_params(default_netuid)
	}

	pub fn get_subnet(netuid: u16) -> SubnetInfo<T> {
		let subnet_params: SubnetParams<T> = Self::get_subnet_params(netuid);
		return SubnetInfo {
			params: subnet_params,
			netuid,
			stake: TotalStake::<T>::get(netuid),
			emission: SubnetEmission::<T>::get(netuid),
			n: N::<T>::get(netuid),
		}
	}

	pub fn default_subnet() -> SubnetInfo<T> {
		let netuid: u16 = Self::get_number_of_subnets() + 1;
		return Self::get_subnet(netuid)
	}

	pub fn is_subnet_founder(netuid: u16, key: &T::AccountId) -> bool {
		return Founder::<T>::get(netuid) == *key
	}

	pub fn add_network_from_registration(
		name: Vec<u8>,
		stake: u64,
		founder_key: &T::AccountId,
	) -> u16 {
		// use default parameters

		let params = Self::default_subnet_params();

		let netuid = Self::add_network(
			name.clone(),
			params.tempo,
			params.immunity_period,
			params.min_allowed_weights,
			params.max_allowed_weights,
			params.max_allowed_uids,
			params.min_stake,
			&founder_key, // founder,
			stake,
		);

		// --- 16. Ok and done.
		return netuid
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
		return UnitEmission::<T>::get()
	}

	pub fn set_unit_emission(unit_emission: u64) {
		UnitEmission::<T>::put(unit_emission)
	}

	pub fn get_unit_emission_per_block() -> u64 {
		return UnitEmission::<T>::get() * 4
	}

	// Returns the total amount of stake in the staking table.
	pub fn get_total_emission_per_block() -> u64 {
		let market_cap: u64 = Self::market_cap();
		let mut unit_emission: u64 = Self::get_unit_emission();
		let mut emission_per_block: u64 = 4 * unit_emission; // assuming 8 second block times
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
			let n = TotalSubnets::<T>::get();
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

		SubnetEmission::<T>::insert(netuid, token_emission);

		return token_emission
	}

	pub fn get_subnet_emission(netuid: u16) -> u64 {
		return Self::calculate_network_emission(netuid)
	}

	pub fn add_network(
		name: Vec<u8>,
		tempo: u16,
		immunity_period: u16,
		min_allowed_weights: u16,
		max_allowed_weights: u16,
		max_allowed_uids: u16,
		min_stake: u64,
		founder: &T::AccountId,
		stake: u64,
	) -> u16 {
		// --- 1. Enfnsure that the network name does not already exist.
		let total_networks: u16 = TotalSubnets::<T>::get();
		let max_networks = MaxAllowedSubnets::<T>::get();
		let netuid = total_networks;

		Tempo::<T>::insert(netuid, tempo);
		MaxAllowedUids::<T>::insert(netuid, max_allowed_uids);
		ImmunityPeriod::<T>::insert(netuid, immunity_period);
		MinAllowedWeights::<T>::insert(netuid, min_allowed_weights);
		MaxAllowedWeights::<T>::insert(netuid, max_allowed_weights);
		MinStake::<T>::insert(netuid, min_stake);
		SubnetNamespace::<T>::insert(name.clone(), netuid);
		Founder::<T>::insert(netuid, founder);
		// set stat once network is created
		TotalSubnets::<T>::mutate(|n| *n += 1);
		N::<T>::insert(netuid, 0);

		// --- 6. Emit the new network event.
		log::info!("NetworkAdded( netuid:{:?}, name:{:?} )", netuid, name.clone());
		Self::deposit_event(Event::NetworkAdded(netuid, name.clone()));

		return netuid
	}

	// Initializes a new subnetwork under netuid with parameters.
	//
	pub fn if_subnet_name_exists(name: Vec<u8>) -> bool {
		return SubnetNamespace::<T>::contains_key(name.clone()).into()
	}

	pub fn subnet_name_exists(name: Vec<u8>) -> bool {
		return SubnetNamespace::<T>::contains_key(name.clone()).into()
	}

	pub fn if_subnet_netuid_exists(netuid: u16) -> bool {
		return SubnetNamespace::<T>::contains_key(Self::get_name_for_netuid(netuid)).into()
	}

	pub fn get_netuid_for_name(name: Vec<u8>) -> u16 {
		let netuid: u16 = SubnetNamespace::<T>::get(name.clone());
		return netuid
	}

	pub fn get_name_for_netuid(netuid: u16) -> Vec<u8> {
		for (name, _netuid) in <SubnetNamespace<T> as IterableStorageMap<Vec<u8>, u16>>::iter() {
			if _netuid == netuid {
				return name
			}
		}
		return Vec::new()
	}

	// Removes the network (netuid) and all of its parameters.
	//

	pub fn remove_least_staked_netuid() -> u16 {
		let netuid: u16 = Self::least_staked_netuid();
		return Self::remove_network_for_netuid(netuid)
	}

	// Returns true if the account is the founder of the network.
	pub fn is_network_founder(netuid: u16, key: &T::AccountId) -> bool {
		let founder = Founder::<T>::get(netuid);
		return founder == key.clone()
	}

	pub fn remove_network_for_name(name: Vec<u8>) -> u16 {
		let netuid = Self::get_netuid_for_name(name.clone());
		return Self::remove_network_for_netuid(netuid)
	}

	pub fn remove_network_for_netuid(netuid: u16) -> u16 {
		// --- 2. Ensure the network to be removed exists.
		if !Self::if_subnet_exist(netuid) {
			return 0
		}
		let name: Vec<u8> = Self::get_name_for_netuid(netuid);

		// --- 1. Erase network stake, and remove network from list of networks.
		for (key, stated_amount) in
			<Stake<T> as IterableStorageDoubleMap<u16, T::AccountId, u64>>::iter_prefix(netuid)
		{
			Self::remove_stake_from_storage(netuid, &key);
		}
		// --- 4. Remove all stake.
		Stake::<T>::remove_prefix(netuid, None);
		TotalStake::<T>::remove(netuid);

		SubnetNamespace::<T>::remove(name.clone());
		Names::<T>::clear_prefix(netuid, u32::max_value(), None);
		Address::<T>::clear_prefix(netuid, u32::max_value(), None);
		Uids::<T>::clear_prefix(netuid, u32::max_value(), None);
		Keys::<T>::clear_prefix(netuid, u32::max_value(), None);

		// Remove consnesus vectors
		Weights::<T>::clear_prefix(netuid, u32::max_value(), None);
		Emission::<T>::remove(netuid);
		Incentive::<T>::remove(netuid);
		Dividends::<T>::remove(netuid);
		Trust::<T>::remove(netuid);
		LastUpdate::<T>::remove(netuid);

		// --- 2. Erase network parameters.
		Founder::<T>::remove(netuid);
		MinStake::<T>::remove(netuid);
		Tempo::<T>::remove(netuid);
		MaxAllowedUids::<T>::remove(netuid);
		ImmunityPeriod::<T>::remove(netuid);
		MinAllowedWeights::<T>::remove(netuid);
		MaxAllowedWeights::<T>::remove(netuid);

		// Adjust the total number of subnets. and remove the subnet from the list of subnets.
		N::<T>::remove(netuid);
		TotalSubnets::<T>::mutate(|val| *val -= 1);
		// --- 4. Emit the event.
		log::info!("NetworkRemoved( netuid:{:?} )", netuid);
		Self::deposit_event(Event::NetworkRemoved(netuid));

		return netuid
	}

	pub fn get_subnets() -> Vec<SubnetInfo<T>> {
		let mut subnets_info = Vec::<SubnetInfo<T>>::new();
		for (netuid, net_n) in <N<T> as IterableStorageMap<u16, u16>>::iter() {
			subnets_info.push(Self::get_subnet(netuid));
		}
		return subnets_info
	}

	// Returns the number of filled slots on a network.
	///
	pub fn get_subnet_n(netuid: u16) -> u16 {
		return N::<T>::get(netuid)
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

	pub fn get_name_for_uid(netuid: u16, uid: u16) -> Vec<u8> {
		return Names::<T>::get(netuid, uid)
	}

	pub fn if_module_name_exists(netuid: u16, name: Vec<u8>) -> bool {
		for (uid, _name) in
			<Names<T> as IterableStorageDoubleMap<u16, u16, Vec<u8>>>::iter_prefix(netuid)
		{
			if _name == name {
				return true
			}
		}
		return false
	}

	// Returns the stake of the uid on network or 0 if it doesnt exist.
	//
	pub fn get_stake_for_uid(netuid: u16, module_uid: u16) -> u64 {
		return Self::get_stake_for_key(netuid, &Self::get_key_for_uid(netuid, module_uid))
	}

	pub fn get_stake_for_key(netuid: u16, key: &T::AccountId) -> u64 {
		if Self::is_key_registered_on_network(netuid, &key) {
			return Stake::<T>::get(netuid, key)
		} else {
			return 0
		}
	}

	// Return the total number of subnetworks available on the chain.
	//
	pub fn get_number_of_subnets() -> u16 {
		let mut number_of_subnets: u16 = 0;
		for (_, _) in <N<T> as IterableStorageMap<u16, u16>>::iter() {
			number_of_subnets = number_of_subnets + 1;
		}
		return number_of_subnets
	}

	// ========================
	// ==== Global Setters ====
	// ========================
	pub fn set_tempo(netuid: u16, tempo: u16) {
		Tempo::<T>::insert(netuid, tempo);
	}

	pub fn set_registrations_this_block(registrations_this_block: u16) {
		RegistrationsPerBlock::<T>::set(registrations_this_block);
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

	pub fn get_emission_for_uid(netuid: u16, uid: u16) -> u64 {
		let vec = Emission::<T>::get(netuid);
		if (uid as usize) < vec.len() {
			return vec[uid as usize]
		} else {
			return 0
		}
	}
	pub fn get_incentive_for_uid(netuid: u16, uid: u16) -> u16 {
		let vec = Incentive::<T>::get(netuid);
		if (uid as usize) < vec.len() {
			return vec[uid as usize]
		} else {
			return 0
		}
	}
	pub fn get_dividends_for_uid(netuid: u16, uid: u16) -> u16 {
		let vec = Dividends::<T>::get(netuid);
		if (uid as usize) < vec.len() {
			return vec[uid as usize]
		} else {
			return 0
		}
	}
	pub fn get_last_update_for_uid(netuid: u16, uid: u16) -> u64 {
		let vec = LastUpdate::<T>::get(netuid);
		if (uid as usize) < vec.len() {
			return vec[uid as usize]
		} else {
			return 0
		}
	}

	pub fn get_max_allowed_subnets() -> u16 {
		MaxAllowedSubnets::<T>::get()
	}
	pub fn set_max_allowed_subnets(max_allowed_subnets: u16) {
		MaxAllowedSubnets::<T>::set(max_allowed_subnets)
	}

	// ============================
	// ==== Subnetwork Getters ====
	// ============================
	pub fn get_tempo(netuid: u16) -> u16 {
		Tempo::<T>::get(netuid)
	}
	pub fn get_pending_emission(netuid: u16) -> u64 {
		PendingEmission::<T>::get(netuid)
	}
	pub fn get_registrations_this_block() -> u16 {
		RegistrationsPerBlock::<T>::get()
	}

	pub fn get_module_registration_block(netuid: u16, uid: u16) -> u64 {
		RegistrationBlock::<T>::get(netuid, uid)
	}

	pub fn get_module_age(netuid: u16, uid: u16) -> u64 {
		return Self::get_current_block_as_u64() - Self::get_module_registration_block(netuid, uid)
	}
	// ========================
	// ==== Rate Limiting =====
	// ========================
	pub fn get_last_tx_block(key: &T::AccountId) -> u64 {
		LastTxBlock::<T>::get(key)
	}
	pub fn set_last_tx_block(key: &T::AccountId, last_tx_block: u64) {
		LastTxBlock::<T>::insert(key, last_tx_block)
	}

	// Configure tx rate limiting
	pub fn get_tx_rate_limit() -> u64 {
		TxRateLimit::<T>::get()
	}
	pub fn set_tx_rate_limit(tx_rate_limit: u64) {
		TxRateLimit::<T>::put(tx_rate_limit)
	}

	pub fn get_max_name_length() -> u16 {
		MaxNameLength::<T>::get()
	}

	pub fn set_max_name_length(max_name_length: u16) {
		MaxNameLength::<T>::put(max_name_length)
	}

	pub fn get_immunity_period(netuid: u16) -> u16 {
		ImmunityPeriod::<T>::get(netuid)
	}
	pub fn set_immunity_period(netuid: u16, immunity_period: u16) {
		ImmunityPeriod::<T>::insert(netuid, immunity_period);
	}

	pub fn get_min_allowed_weights(netuid: u16) -> u16 {
		let min_allowed_weights = MinAllowedWeights::<T>::get(netuid);
		let n = Self::get_subnet_n(netuid);
		// if n < min_allowed_weights, then return n
		if (n < min_allowed_weights) {
			return n
		} else {
			return min_allowed_weights
		}
	}
	pub fn set_min_allowed_weights(netuid: u16, min_allowed_weights: u16) {
		MinAllowedWeights::<T>::insert(netuid, min_allowed_weights);
	}

	pub fn get_max_allowed_weights(netuid: u16) -> u16 {
		let max_allowed_weights = MaxAllowedWeights::<T>::get(netuid);
		let n = Self::get_subnet_n(netuid);
		// if n < min_allowed_weights, then return n
		if (n < max_allowed_weights) {
			return n
		} else {
			return max_allowed_weights
		}
	}
	pub fn set_max_allowed_weights(netuid: u16, max_allowed_weights: u16) {
		MaxAllowedWeights::<T>::insert(netuid, max_allowed_weights);
	}

	pub fn get_max_allowed_uids(netuid: u16) -> u16 {
		MaxAllowedUids::<T>::get(netuid)
	}
	pub fn set_max_allowed_uids(netuid: u16, max_allowed: u16) {
		MaxAllowedUids::<T>::insert(netuid, max_allowed);
	}

	pub fn get_max_allowed_modules() -> u16 {
		MaxAllowedModules::<T>::get()
	}

	pub fn total_n() -> u16 {
		let mut total_n: u16 = 0;
		for (netuid, n) in <N<T> as IterableStorageMap<u16, u16>>::iter() {
			total_n += n;
		}
		return total_n
	}

	pub fn enough_space_for_n(n: u16) -> bool {
		let total_n: u16 = Self::total_n();
		let max_allowed_modules: u16 = Self::get_max_allowed_modules();
		return total_n + n <= max_allowed_modules
	}

	pub fn set_max_allowed_modules(max_allowed_modules: u16) {
		MaxAllowedModules::<T>::put(max_allowed_modules)
	}

	pub fn get_uids(netuid: u16) -> Vec<u16> {
		let n = Self::get_subnet_n(netuid);
		return (0..n).collect()
	}
	pub fn get_keys(netuid: u16) -> Vec<T::AccountId> {
		let uids: Vec<u16> = Self::get_uids(netuid);
		let keys: Vec<T::AccountId> =
			uids.iter().map(|uid| Self::get_key_for_uid(netuid, *uid)).collect();
		return keys
	}

	pub fn get_uid_key_tuples(netuid: u16) -> Vec<(u16, T::AccountId)> {
		return <Keys<T> as IterableStorageDoubleMap<u16, u16, T::AccountId>>::iter_prefix(netuid)
			.map(|(uid, key)| (uid, key))
			.collect()
	}

	pub fn get_names(netuid: u16) -> Vec<Vec<u8>> {
		let mut names = Vec::<Vec<u8>>::new();
		for (uid, name) in
			<Names<T> as IterableStorageDoubleMap<u16, u16, Vec<u8>>>::iter_prefix(netuid)
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

	pub fn check_subnet_storage(netuid: u16) -> bool {
		let n = Self::get_subnet_n(netuid);
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
		return true
	}

	pub fn get_emissions(netuid: u16) -> Vec<u64> {
		Emission::<T>::get(netuid)
	}
	pub fn get_incentives(netuid: u16) -> Vec<u16> {
		Incentive::<T>::get(netuid)
	}
	pub fn get_dividends(netuid: u16) -> Vec<u16> {
		Dividends::<T>::get(netuid)
	}
	pub fn get_last_update(netuid: u16) -> Vec<u64> {
		LastUpdate::<T>::get(netuid)
	}
	pub fn get_max_registrations_per_block() -> u16 {
		MaxRegistrationsPerBlock::<T>::get()
	}
	pub fn set_max_registrations_per_block(max_registrations_per_block: u16) {
		MaxRegistrationsPerBlock::<T>::set(max_registrations_per_block);
	}

	pub fn is_registered(netuid: u16, key: &T::AccountId) -> bool {
		return Uids::<T>::contains_key(netuid, &key)
	}

	pub fn is_uid_registered(netuid: u16, uid: u16) -> bool {
		return Keys::<T>::contains_key(netuid, uid)
	}
}
