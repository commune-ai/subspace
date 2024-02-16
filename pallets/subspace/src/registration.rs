use super::*;
use crate::system::ensure_root;
use frame_support::{pallet_prelude::DispatchResult};
use frame_system::ensure_signed;
use sp_arithmetic::per_things::Percent;
use sp_core::{H256, U256};
use sp_io::hashing::{keccak_256, sha2_256};
use sp_std::{convert::TryInto, vec::Vec};
use substrate_fixed::types::I32F32;
use sp_std::vec;
use system::pallet_prelude::BlockNumberFor;
// IterableStorageMap
use frame_support::{
	storage::IterableStorageMap,
};


const LOG_TARGET: &'static str = "runtime::subspace::registration";

impl<T: Config> Pallet<T> {
	pub fn do_register(
		origin: T::RuntimeOrigin,
		network: Vec<u8>, // network name
		name: Vec<u8>, // module name
		address: Vec<u8>, // module address
		stake_amount: u64, // stake amount
		module_key: T::AccountId, // module key
	) -> DispatchResult {
		// --- 1. Check that the caller has signed the transaction.
		let key = ensure_signed(origin.clone())?;
		ensure!(
			RegistrationsPerBlock::<T>::get() <= MaxRegistrationsPerBlock::<T>::get(),
			Error::<T>::TooManyRegistrationsPerBlock
		);
		// --- 2. Ensure we are not exceeding the max allowed registrations per block.
		ensure!(
			Self::has_enough_balance(&key, stake_amount),
			Error::<T>::NotEnoughBalanceToRegister
		);

		// -- 3. resolve the network in case it doesnt exisst
		if !Self::subnet_name_exists(network.clone()) {
			Self::add_subnet_from_registration(network.clone(), stake_amount, &key)?;
		}

		// get the netuid
		let netuid = Self::get_netuid_for_name(network.clone());

		ensure!(
			Self::enough_stake_to_register(netuid, stake_amount),
			Error::<T>::NotEnoughStakeToRegister
		);

		ensure!(!Self::key_registered(netuid, &key), Error::<T>::KeyAlreadyRegistered);
		
		Self::check_module_limits(netuid);
		
		let uid = Self::append_module(netuid, &module_key, name.clone(), address.clone());

		// adding the stake amount
		Self::do_add_stake(origin.clone(), netuid, module_key.clone(), stake_amount)?;
		
		let min_burn: u64 = Self::get_min_burn();

		// CONSTANT INITIAL BURN
		if min_burn > 0 {
			ensure!(stake_amount >= min_burn, Error::<T>::NotEnoughStakeToRegister);
			Self::decrease_stake(netuid, &key, &module_key, min_burn);
			let current_stake = Self::get_total_stake_to(netuid, &key);
			ensure!(current_stake == stake_amount.saturating_sub(min_burn), Error::<T>::NotEnoughStakeToRegister);
		}
		// ---Deposit successful event.

		RegistrationsPerBlock::<T>::mutate(|val| *val += 1);
		Self::deposit_event(Event::ModuleRegistered(netuid, uid, module_key.clone()));

		// --- 5. Ok and done.
		Ok(())
	}


	pub fn do_deregister(
		origin: T::RuntimeOrigin,
		netuid: u16,
	) -> DispatchResult {
		// --- 1. Check that the caller has signed the transaction.
		let key = ensure_signed(origin.clone())?;

		ensure!(
			Self::key_registered(netuid, &key),
			Error::<T>::NotRegistered
		);

		// --- 2. Ensure we are not exceeding the max allowed registrations per block.
		let uid: u16 = Self::get_uid_for_key(netuid, &key);

		Self::remove_module(netuid, uid);
		ensure!(
			!Self::key_registered(netuid, &key),
			Error::<T>::StillRegistered
		);

		// --- 5. Ok and done.
		Ok(())
	}

	pub fn enough_stake_to_register(netuid: u16, stake_amount: u64) -> bool {
		let min_stake: u64 = MinStake::<T>::get(netuid);
		let min_burn = Self::get_min_burn();
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



	pub fn add_subnet_from_registration(
		name: Vec<u8>,
		stake: u64,
		founder_key: &T::AccountId,
	) ->  DispatchResult{
		// use default parameters
		//

		let num_subnets: u16 = Self::num_subnets();
		let max_subnets: u16 = Self::get_global_max_allowed_subnets();
		// if we have not reached the max number of subnets, then we can start a new one
		if num_subnets >= max_subnets {
			let mut min_stake: u64 = u64::MAX;
			let mut min_stake_netuid : u16 = max_subnets.saturating_sub(1); // the default last ui
			for (netuid, net_stake) in <TotalStake<T> as IterableStorageMap<u16, u64>>::iter() {
				if net_stake <= min_stake {
					min_stake = net_stake;
					min_stake_netuid = netuid;
				}
			}
			ensure!(stake > min_stake , Error::<T>::NotEnoughStakeToStartNetwork);
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




}

