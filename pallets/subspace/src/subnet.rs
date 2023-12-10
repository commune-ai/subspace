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


	pub fn do_remove_network(origin: T::RuntimeOrigin, netuid: u16) -> DispatchResult {
		let key = ensure_signed(origin)?;
		// --- 1. Ensure the network name does not already exist.

		ensure!(Self::if_subnet_netuid_exists(netuid), Error::<T>::SubnetNameAlreadyExists);
		ensure!(Self::is_subnet_founder(netuid, &key), Error::<T>::NotFounder);

		Self::remove_network_for_netuid(netuid);
		// --- 16. Ok and done.
		Ok(())
	}

	pub fn do_update_subnet(
		origin: T::RuntimeOrigin,
		netuid: u16,
		params: SubnetParams,
	) -> DispatchResult {
		let key = ensure_signed(origin)?;
		// only the founder can update the network on authority mode
		assert!(is_vec_str(params.vote_mode.clone(), "authority"));
		ensure!(Self::if_subnet_netuid_exists(netuid), Error::<T>::SubnetNameAlreadyExists);
		ensure!(Self::is_subnet_founder(netuid, &key), Error::<T>::NotFounder);
		Self::check_subnet_params(params.clone());
		Self::set_subnet_params(netuid, params);
		// --- 16. Ok and done.
		Ok(())
	}

	pub fn subnet_params(netuid: u16) -> SubnetParams {
		SubnetParams {
			immunity_period: ImmunityPeriod::<T>::get(netuid),
			min_allowed_weights: MinAllowedWeights::<T>::get(netuid),
			max_allowed_weights: MaxAllowedWeights::<T>::get(netuid),
			max_allowed_uids: MaxAllowedUids::<T>::get(netuid),
			min_stake: MinStake::<T>::get(netuid),
			tempo: Tempo::<T>::get(netuid),
			name: <Vec<u8>>::new(),
			burn_rate: BurnRate::<T>::get(netuid),
			vote_threshold: SubnetVoteThreshold::<T>::get(netuid),
			vote_mode:SubnetVoteMode::<T>::get(netuid),
			min_burn: MinBurn::<T>::get(netuid),
			trust_ratio: TrustRatio::<T>::get(netuid),
			self_vote: SelfVote::<T>::get(netuid)
		}
	}

	pub fn set_max_allowed_uids(netuid: u16, max_allowed_uids: u16) {
		let n: u16 = Self::get_subnet_n(netuid);
		if max_allowed_uids < n {
			let remainder_n: u16 = n - max_allowed_uids;
			for i in 0..remainder_n {
				Self::remove_module(netuid, Self::get_lowest_uid(netuid));
			}
		}

		MaxAllowedUids::<T>::insert(netuid, max_allowed_uids);


	}



	pub fn set_subnet_params(netuid: u16, mut params: SubnetParams) {

		// TEMPO, IMMUNITY_PERIOD, MIN_ALLOWED_WEIGHTS, MAX_ALLOWED_WEIGHTS, MAX_ALLOWED_UIDS,
		// MAX_IMMUNITY_RATIO
		Self::set_tempo(netuid, params.tempo);

		Self::set_immunity_period(netuid, params.immunity_period);

		Self::set_max_allowed_weights(netuid, params.max_allowed_weights);

		Self::set_min_allowed_weights(netuid, params.min_allowed_weights);

		Self::set_min_stake(netuid, params.min_stake);

		Self::set_max_allowed_uids(netuid, params.max_allowed_uids);

		Self::set_subnet_vote_threshold(netuid, params.vote_threshold);

		Self::set_vote_mode_subnet(netuid, params.vote_mode);

		Self::set_burn_rate(netuid, params.burn_rate);

		Self::set_subnet_name(netuid, params.name);

		Self::set_min_burn(netuid, params.min_burn);

		Self::set_trust_ratio(netuid, params.trust_ratio);

		Self::set_self_vote(netuid, params.self_vote)


	}



	pub fn set_subnet_name(netuid: u16, name: Vec<u8>) {
		if name.len() > 0 {
			let old_name: Vec<u8> = Self::get_name_for_netuid(netuid);
			Name2Subnet::<T>::remove(old_name.clone());
			Name2Subnet::<T>::insert(name.clone(), netuid)
		}
	}



	pub fn uid_in_immunity(netuid: u16, uid: u16) -> bool {
		let block_at_registration: u64 = Self::get_module_registration_block(netuid, uid);
		let immunity_period: u64 = Self::get_immunity_period(netuid) as u64;
		let current_block: u64 = Self::get_current_block_as_u64();
		return current_block - block_at_registration < immunity_period
	}

	pub fn default_subnet_params() -> SubnetParams {
		// get an invalid 
		let default_netuid: u16 = Self::get_number_of_subnets() + 1;
		return Self::subnet_params(default_netuid)
	}

	pub fn subnet_info(netuid: u16) -> SubnetInfo<T> {
		let subnet_params: SubnetParams = Self::subnet_params(netuid);
		return SubnetInfo {
			params: subnet_params,
			netuid,
			stake: TotalStake::<T>::get(netuid),
			emission: SubnetEmission::<T>::get(netuid),
			n: N::<T>::get(netuid),
			founder: Founder::<T>::get(netuid),
		}
	}

	pub fn default_subnet() -> SubnetInfo<T> {
		let netuid: u16 = Self::get_number_of_subnets() + 1;
		return Self::subnet_info(netuid)
	}

	pub fn is_subnet_founder(netuid: u16, key: &T::AccountId) -> bool {
		return Founder::<T>::get(netuid) == *key
	}

	pub fn set_subnet_founder(netuid: u16, key: &T::AccountId) {
		Founder::<T>::insert(netuid, key.clone());
	}

	pub fn get_subnet_founder(netuid: u16) -> T::AccountId {
		return Founder::<T>::get(netuid)
	}

	pub fn set_self_vote(netuid: u16, self_vote: bool) {
		SelfVote::<T>::insert(netuid, self_vote);
	}

	pub fn get_self_vote(netuid: u16) -> bool {
		return SelfVote::<T>::get(netuid)
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

	pub fn add_network(params: SubnetParams) -> u16 {

		// --- 1. Enfnsure that the network name does not already exist.
		let total_networks: u16 = TotalSubnets::<T>::get();
		let max_networks = MaxAllowedSubnets::<T>::get();
		let netuid = total_networks;

		Tempo::<T>::insert(netuid, params.tempo);
		MaxAllowedUids::<T>::insert(netuid, params.max_allowed_uids);
		ImmunityPeriod::<T>::insert(netuid, params.immunity_period);
		MinAllowedWeights::<T>::insert(netuid, params.min_allowed_weights);
		MaxAllowedWeights::<T>::insert(netuid, params.max_allowed_weights);
		MinStake::<T>::insert(netuid, params.min_stake);
		Name2Subnet::<T>::insert(params.name.clone(), netuid);
		BurnRate::<T>::insert(netuid, params.burn_rate);
		// set stat once network is created
		TotalSubnets::<T>::mutate(|n| *n += 1);
		N::<T>::insert(netuid, 0);

		// --- 6. Emit the new network event.
		Self::deposit_event(Event::NetworkAdded(netuid, params.name.clone()));

		return netuid
	}
	
	// Initializes a new subnetwork under netuid with parameters.
	//
	pub fn if_subnet_name_exists(name: Vec<u8>) -> bool {
		return Name2Subnet::<T>::contains_key(name.clone()).into()
	}

	pub fn subnet_name_exists(name: Vec<u8>) -> bool {
		return Name2Subnet::<T>::contains_key(name.clone()).into()
	}

	pub fn if_subnet_netuid_exists(netuid: u16) -> bool {
		return Name2Subnet::<T>::contains_key(Self::get_name_for_netuid(netuid)).into()
	}

	pub fn get_netuid_for_name(name: Vec<u8>) -> u16 {
		let netuid: u16 = Name2Subnet::<T>::get(name.clone());
		return netuid
	}

	pub fn get_name_for_netuid(netuid: u16) -> Vec<u8> {
		for (name, _netuid) in <Name2Subnet<T> as IterableStorageMap<Vec<u8>, u16>>::iter() {
			if _netuid == netuid {
				return name
			}
		}
		return Vec::new()
	}

	pub fn is_network_founder(netuid: u16, key: &T::AccountId) -> bool {
		// Returns true if the account is the founder of the network.
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
		Name2Subnet::<T>::remove(name.clone());
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
		PendingDeregisterUids::<T>::remove(netuid);

		// --- 2. Erase network parameters.
		Founder::<T>::remove(netuid);
		MinStake::<T>::remove(netuid);
		Tempo::<T>::remove(netuid);
		MaxAllowedUids::<T>::remove(netuid);
		ImmunityPeriod::<T>::remove(netuid);
		MinAllowedWeights::<T>::remove(netuid);
		MaxAllowedWeights::<T>::remove(netuid);
		BurnRate::<T>::remove(netuid);
		SelfVote::<T>::remove(netuid);

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
			subnets_info.push(Self::subnet_info(netuid));
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

	pub fn get_name_for_uid(netuid: u16, uid: u16) -> Vec<u8> {
		return Names::<T>::get(netuid, uid)
	}


	pub fn get_trust_ratio(netuid: u16) -> u16 {
		return TrustRatio::<T>::get(netuid)
	}

	pub fn set_trust_ratio(netuid: u16, trust_ratio: u16) {
		TrustRatio::<T>::insert(netuid, trust_ratio);
	}


	pub fn get_quadradic_voting(netuid: u16) -> bool {
		return QuadraticVoting::<T>::get(netuid)
	}

	pub fn set_quadradic_voting(netuid: u16, quadradic_voting: bool) {
		QuadraticVoting::<T>::insert(netuid, quadradic_voting);
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

	// we need to prefix the voting power by the network uid

	pub fn set_subnet_vote_threshold(netuid: u16, vote_threshold: u16) {
		SubnetVoteThreshold::<T>::insert(netuid, vote_threshold);
	}

	pub fn get_vote_mode_subnet(netuid: u16) -> Vec<u8> {
		return SubnetVoteMode::<T>::get(netuid)
	}

	pub fn set_vote_mode_subnet(netuid: u16, vote_mode: Vec<u8>) {
		SubnetVoteMode::<T>::insert(netuid, vote_mode);
	}
	
	pub fn get_subnet_vote_threshold(netuid: u16) -> u16 {
		return SubnetVoteThreshold::<T>::get(netuid)
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

	pub fn netuids() -> Vec<u16> {

		let mut netuids : Vec<u16> = Vec::new();
		for (netuid, _net_n) in <N<T> as IterableStorageMap<u16, u16>>::iter() {
			netuids.push(netuid);
		}
		return netuids
	}

	// ========================
	// ==== Global Setters ====
	// ========================
	pub fn set_tempo(netuid: u16, tempo: u16) {
		Tempo::<T>::insert(netuid, tempo);
	}


	pub fn set_min_burn(netuid: u16, min_burn: u64) {
		MinBurn::<T>::insert(netuid, min_burn);
	}

	pub fn get_min_burn(netuid: u16) -> u64 {
		MinBurn::<T>::get(netuid).into()
	}







	pub fn set_burn_rate(netuid: u16, burn_rate: u16) {
		if burn_rate > 100 {
			BurnRate::<T>::insert(netuid, 100);
			return
		}
		if burn_rate < 0 {
			BurnRate::<T>::insert(netuid, 0);
			return
		}
		BurnRate::<T>::insert(netuid, burn_rate);
	}

	pub fn get_burn_emission_per_epoch(netuid: u16) -> u64 {
		let burn_rate: u16 = BurnRate::<T>::get(netuid);
		let epoch_emission: u64 = Self::get_subnet_emission(netuid);
		let n: u16 = Self::get_subnet_n(netuid);
		// get the float and convert to u64
		if n == 0 {
			return 0
		}
		let burn_emission_per_epoch: u64 = (burn_rate as u64 * epoch_emission)  / (n * 100) as u64;
		return burn_emission_per_epoch
	}

	pub fn get_burn_rate(netuid: u16) -> u16 {
		return BurnRate::<T>::get(netuid)
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

	pub fn get_emission_for_key(netuid: u16, key: &T::AccountId) -> u64 {
		let uid = Self::get_uid_for_key(netuid, key);
		return Self::get_emission_for_uid(netuid, uid)
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
	pub fn set_max_registrations_per_block(max_registrations_per_block: u16) {
		MaxRegistrationsPerBlock::<T>::set(max_registrations_per_block);
	}

	pub fn is_registered(netuid: u16, key: &T::AccountId) -> bool {
		return Uids::<T>::contains_key(netuid, &key)
	}

	pub fn is_uid_registered(netuid: u16, uid: u16) -> bool {
		return Keys::<T>::contains_key(netuid, uid)
	}



    pub fn check_subnet_params(params: SubnetParams) -> DispatchResult{
        // checks if params are valid

        // check if the name already exists
        ensure!(!Self::if_subnet_name_exists(params.name.clone()),Error::<T>::SubnetNameAlreadyExists );

        // check valid tempo		
		assert!(params.max_allowed_weights >= params.min_allowed_weights, "Invalid max_allowed_weights");
		
		assert!(params.max_allowed_weights <= params.max_allowed_uids, "Invalid max_allowed_weights");
                
		assert!(params.burn_rate <= 100, "Invalid burn_rate");
                
		assert!(params.burn_rate <= 100, "Invalid vote_threshold");
		
		assert!(params.min_burn <= params.min_stake, "Invalid min_burn");
		
		// ensure the trust_ratio is between 0 and 100
		assert!(params.trust_ratio <= 100, "Invalid trust_ratio");

		// ensure the vode_mode is in "authority", "stake", "quadratic"
		assert!(
			is_vec_str(params.vote_mode.clone(),"authority") ||
			is_vec_str(params.vote_mode.clone(),"stake") ||
			is_vec_str(params.vote_mode.clone(),"quadratic"),
		);
        Ok(())


    }
}
