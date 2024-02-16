use super::*;
use crate::system::ensure_root;
use frame_support::pallet_prelude::DispatchResult;
use frame_system::ensure_signed;
use sp_arithmetic::per_things::Percent;
use sp_core::{H256, U256};
use sp_io::hashing::{keccak_256, sha2_256};
use sp_std::{convert::TryInto, vec, vec::Vec};
use substrate_fixed::types::I32F32;
use system::pallet_prelude::BlockNumberFor;
// IterableStorageMap
use frame_support::storage::IterableStorageMap;

const LOG_TARGET: &'static str = "runtime::subspace::registration";

impl<T: Config> Pallet<T> {
	pub fn do_register(
		origin: T::RuntimeOrigin,
		network: Vec<u8>,         // network name
		name: Vec<u8>,            // module name
		address: Vec<u8>,         // module address
		stake_amount: u64,        // stake amount
		module_key: T::AccountId, // module key
	) -> DispatchResult {
		// --- 1. Check that the caller has signed the transaction.
		let key = ensure_signed(origin.clone())?;

		// --- 2. Ensure, that we are not exceeding the max allowed
		// registrations per block.
		ensure!(
			RegistrationsPerBlock::<T>::get() < MaxRegistrationsPerBlock::<T>::get(),
			Error::<T>::TooManyRegistrationsPerBlock
		);

		// --- 3. Ensure the caller has enough balance to register. We need to
		// ensure that the stake that the user wants to register with,
		// is already present as a balance.
		ensure!(
			Self::has_enough_balance(&key, stake_amount),
			Error::<T>::NotEnoughBalanceToRegister
		);

		// --- 4. Resolve the network in case it doesn't exist
		if !Self::subnet_name_exists(network.clone()) {
			// If the subnet doesn't exist, registration will create it.
			Self::add_subnet_from_registration(network.clone(), stake_amount, &key)?;
		}

		// --- 5. Ensure the caller has enough stake to register.
		let netuid: u16 = Self::get_netuid_for_name(network.clone());
		let min_stake: u64 = MinStake::<T>::get(netuid);
		let min_burn: u64 = Self::get_min_burn();

		// also ensures that in the case min_burn is present, the stake is enough
		// as burn, will be decreased from the stake on the module
		ensure!(
			Self::enough_stake_to_register(netuid, min_stake, min_burn, stake_amount),
			Error::<T>::NotEnoughStakeToRegister
		);

		// --- 6. Ensure the module key is not already registered.
		ensure!(!Self::key_registered(netuid, &key), Error::<T>::KeyAlreadyRegistered);

		// --- 7. Check if we are exceeding the max allowed modules per network.
		// If we do deregister slot.
		Self::check_module_limits(netuid);

		// --- 8. Register the module.
		let uid: u16 = Self::append_module(netuid, &module_key, name.clone(), address.clone());

		// --- 9. Add the stake to the module, now that it is registered on the network.
		Self::do_add_stake(origin.clone(), netuid, module_key.clone(), stake_amount)?;

		// constant -> min_burn logic
		if min_burn > 0 {
			// if min burn is present, decrease the stake by the min burn
			Self::decrease_stake(netuid, &key, &module_key, min_burn);
		}

		// --- 10. Increment the number of registrations per block.
		RegistrationsPerBlock::<T>::mutate(|val| *val += 1);

		// --- Deposit successful event.
		Self::deposit_event(Event::ModuleRegistered(netuid, uid, module_key.clone()));

		// --- 11. Ok and done.
		Ok(())
	}

	pub fn do_deregister(origin: T::RuntimeOrigin, netuid: u16) -> DispatchResult {
		// --- 1. Check that the caller has signed the transaction.
		let key = ensure_signed(origin.clone())?;

		ensure!(Self::key_registered(netuid, &key), Error::<T>::NotRegistered);

		// --- 2. Ensure we are not exceeding the max allowed registrations per block.
		let uid: u16 = Self::get_uid_for_key(netuid, &key);

		Self::remove_module(netuid, uid);
		ensure!(!Self::key_registered(netuid, &key), Error::<T>::StillRegistered);

		// --- 5. Ok and done.
		Ok(())
	}

	pub fn enough_stake_to_register(
		netuid: u16,
		min_stake: u64,
		min_burn: u64,
		stake_amount: u64,
	) -> bool {
		// the amount has to cover, the minimal stake as well as burn if it's present
		return stake_amount >= (min_stake + min_burn)
	}

	pub fn vec_to_hash(vec_hash: Vec<u8>) -> H256 {
		let de_ref_hash = &vec_hash; // b: &Vec<u8>
		let de_de_ref_hash: &[u8] = &de_ref_hash; // c: &[u8]
		let real_hash: H256 = H256::from_slice(de_de_ref_hash);
		return real_hash
	}

	// Determine which peer to prune from the network by finding the element with the lowest pruning
	// score out of immunity period. If all modules are in immunity period, return node with lowest
	// prunning score. This function will always return an element to prune.

	pub fn get_pruning_score_for_uid(netuid: u16, uid: u16) -> u64 {
		let vec: Vec<u64> = Emission::<T>::get(netuid);
		if (uid as usize) < vec.len() {
			return vec[uid as usize]
		} else {
			return 0 as u64
		}
	}
	pub fn get_lowest_uid(netuid: u16) -> u16 {
		let n: u16 = Self::get_subnet_n(netuid);

		let mut min_score: u64 = u64::MAX;
		let mut lowest_priority_uid: u16 = 0;
		let mut prune_uids: Vec<u16> = Vec::new();
		let current_block = Self::get_current_block_as_u64();
		let immunity_period: u64 = Self::get_immunity_period(netuid) as u64;

		for module_uid_i in 0..n {
			let pruning_score: u64 = Self::get_pruning_score_for_uid(netuid, module_uid_i);

			// Find min pruning score.

			if min_score > pruning_score {
				let block_at_registration: u64 =
					Self::get_module_registration_block(netuid, module_uid_i);
				let module_age: u64 = current_block.saturating_sub(block_at_registration);
				// only allow modules that have greater than immunity period
				if module_age > immunity_period {
					lowest_priority_uid = module_uid_i;
					min_score = pruning_score;
					if min_score == 0 {
						break
					}
				}
			}
		}

		return lowest_priority_uid
	}

	pub fn add_subnet_from_registration(
		name: Vec<u8>,
		stake: u64,
		founder_key: &T::AccountId,
	) -> DispatchResult {
		// use default parameters
		//

		let num_subnets: u16 = Self::num_subnets();
		let max_subnets: u16 = Self::get_global_max_allowed_subnets();
		// if we have not reached the max number of subnets, then we can start a new one
		if num_subnets >= max_subnets {
			let mut min_stake: u64 = u64::MAX;
			let mut min_stake_netuid: u16 = max_subnets.saturating_sub(1); // the default last ui
			for (netuid, net_stake) in <TotalStake<T> as IterableStorageMap<u16, u64>>::iter() {
				if net_stake <= min_stake {
					min_stake = net_stake;
					min_stake_netuid = netuid;
				}
			}
			ensure!(stake > min_stake, Error::<T>::NotEnoughStakeToStartNetwork);
			Self::remove_subnet(min_stake_netuid);
		}
		// if we have reached the max number of subnets, then we can start a new one if the stake is
		// greater than the least staked network
		let mut params: SubnetParams<T> = Self::default_subnet_params();
		params.name = name.clone();
		params.founder = founder_key.clone();
		let netuid = Self::add_subnet(params);

		Ok(())
	}

	pub fn check_module_limits(netuid: u16) {
		// check if we have reached the max allowed modules,
		// if so deregister the lowest priority node

		// replace a node if we reach the max allowed modules for the network
		if Self::global_n() >= Self::get_max_allowed_modules() {
			// get the least staked network (subnet)
			let least_staked_netuid: u16 = Self::least_staked_netuid();

			// deregister the lowest priority node
			Self::remove_module(least_staked_netuid, Self::get_lowest_uid(least_staked_netuid));

		// if we reach the max allowed modules for this network,
		// then we replace the lowest priority node
		} else if Self::get_subnet_n(netuid) >= Self::get_max_allowed_uids(netuid) {
			// deregister the lowest priority node
			Self::remove_module(netuid, Self::get_lowest_uid(netuid));
		}
	}
}
