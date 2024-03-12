use super::*;
use frame_support::{
	pallet_prelude::{Decode, DispatchResult, Encode},
	storage::IterableStorageMap,
	IterableStorageDoubleMap,
};

extern crate alloc;
use alloc::vec::Vec;
use codec::Compact;
use sp_arithmetic::per_things::Percent;
use sp_std::vec;

#[derive(Decode, Encode, PartialEq, Eq, Clone, Debug)]
pub struct ModuleInfo<T: Config> {
	params: ModuleParams<T>,
	state: ModuleState<T>,
}

impl<T: Config> Pallet<T> {
	pub fn do_update_module(
		origin: T::RuntimeOrigin,
		netuid: u16,
		params: ModuleParams<T>,
	) -> DispatchResult {
		// --- 1. We check the callers (key) signature.
		let key = ensure_signed(origin)?;
		let uid: u16 = Self::get_uid_for_key(netuid, &key);
		Self::check_module_params(netuid, params.clone())?;
		Self::set_module_params(netuid, uid, params);
		// --- 8. Return is successful dispatch.
		Ok(())
	}

	pub fn check_module_params(netuid: u16, params: ModuleParams<T>) -> DispatchResult {
		// if len(name) > 0, then we update the name.
		assert!(params.name.len() > 0);
		ensure!(
			params.name.len() <= Self::get_global_max_name_length() as usize,
			Error::<T>::ModuleNameTooLong
		);
		assert!(params.address.len() > 0);
		ensure!(
			params.address.len() <= Self::get_global_max_name_length() as usize,
			Error::<T>::ModuleAddressTooLong
		);
		// delegation fee is a percent

		Ok(())
	}

	pub fn get_module_key(netuid: u16, uid: u16) -> T::AccountId {
		Self::module_state(netuid, uid).module_key
	}

	// Replace the module under this uid.
	pub fn remove_module(netuid: u16, uid: u16) {
		// 1. Get the old key under this position.
		let n = Self::get_subnet_n_uids(netuid);

		if n == 0 {
			/// No modules in the network.
			return
		}

		let replace_uid = n - 1;

		let module_state = Self::module_state(netuid, uid);
		let replace_module_state = Self::module_state(netuid, replace_uid);

		let module_params = Self::module_params(netuid, uid);
		let replace_module_params = Self::module_params(netuid, replace_uid);

		log::debug!(
			"remote_subnet( netuid: {:?} | uid : {:?} | new_key: {:?} ) ",
			netuid,
			uid,
			replace_module_state.module_key
		);

		ModuleStateStorage::<T>::insert(netuid, uid, replace_module_state);
		ModuleStateStorage::<T>::remove(netuid, replace_uid);

		ModuleParamsStorage::<T>::insert(netuid, uid, replace_module_params);
		ModuleParamsStorage::<T>::remove(netuid, replace_uid);

		// 3. Remove the network if it is empty.
		SubnetStateStorage::<T>::mutate(netuid, |subnet_state| {
			subnet_state.n_uids -= 1;
		});

		// remove the network if it is empty
		if Self::get_subnet_n_uids(netuid) == 0 {
			Self::remove_subnet(netuid);
		}

		// remove stake from old key and add to new key
		Self::remove_stake_from(netuid, uid);
	}

	// Appends the uid to the network.
	pub fn append_module(netuid: u16, key: &T::AccountId, name: Vec<u8>, address: Vec<u8>) -> u16 {
		// 1. Get the next uid. This is always equal to subnetwork_n.
		let uid: u16 = Self::get_subnet_n_uids(netuid);
		let block_number = Self::get_current_block_as_u64();

		log::debug!("append_module( netuid: {:?} | uid: {:?} | new_key: {:?} ) ", netuid, uid, key);

		ModuleStateStorage::<T>::insert(
			netuid,
			uid,
			ModuleState {
				uid,
				module_key: key.clone(),
				incentive: 0,
				trust: 0,
				dividend: 0,
				emission: 0,
				last_update: block_number,
				registration_block: block_number,
				stake: 0,
				stake_from: vec![],
				profit_shares: vec![],
			},
		);

		ModuleParamsStorage::<T>::insert(
			netuid,
			uid,
			ModuleParams {
				name,
				address,
				delegation_fee: Percent::from_percent(20u8),
				controller: T::AccountId::decode(
					&mut sp_runtime::traits::TrailingZeroInput::zeroes(),
				)
				.unwrap(),
				weights: vec![],
			},
		);

		SubnetStateStorage::<T>::mutate(netuid, |subnet_state| {
			subnet_state.n_uids += 1;
		});

		uid
	}

	pub fn get_netuid_and_uid(module_key: &T::AccountId) -> (u16, u16) {
		for netuid in Self::netuids() {
			for (uid, module_state) in <ModuleStateStorage<T> as IterableStorageDoubleMap<
				u16,
				u16,
				ModuleState<T>,
			>>::iter_prefix(netuid)
			{
				if *module_key == module_state.module_key {
					return (netuid, uid);
				}
			}
		}

		(u16::MAX, u16::MAX)
	}

	pub fn get_uid_for_key(netuid: u16, module_key: &T::AccountId) -> u16 {
		for (uid, module_state) in <ModuleStateStorage<T> as IterableStorageDoubleMap<
			u16,
			u16,
			ModuleState<T>,
		>>::iter_prefix(netuid)
		{
			if *module_key == module_state.module_key {
				return uid;
			}
		}

		u16::MAX
	}

	pub fn set_module_params(netuid: u16, uid: u16, module_params: ModuleParams<T>) {
		ModuleParamsStorage::<T>::insert(netuid, uid, module_params);
	}

	pub fn set_module_state(netuid: u16, uid: u16, module_state: ModuleState<T>) {
		ModuleStateStorage::<T>::insert(netuid, uid, module_state);
	}
}
