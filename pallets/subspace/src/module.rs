use super::*;
use frame_support::pallet_prelude::{Decode, DispatchResult, Encode};

extern crate alloc;
use alloc::vec::Vec;

use sp_arithmetic::per_things::Percent;

#[derive(Decode, Encode, PartialEq, Eq, Clone, Debug)]
pub struct ModuleStats<T: Config> {
    pub last_update: u64,
    pub registration_block: u64,
    pub stake_from: Vec<(T::AccountId, u64)>, /* map of key to stake on this module/key *
                                               * (includes delegations) */
    pub emission: u64,
    pub incentive: u16,
    pub dividends: u16,
    pub weights: Vec<(u16, u16)>, // Vec of (uid, weight)
}

#[derive(Decode, Encode, PartialEq, Eq, Clone, Debug)]
pub struct ModuleInfo<T: Config> {
    params: ModuleParams<T>,
    stats: ModuleStats<T>,
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
        Self::check_module_params(netuid, &params)?;
        Self::set_module_params(netuid, uid, params);
        // --- 8. Return is successful dispatch.
        Ok(())
    }

    /// Checks whether the module params are valid. Name and address must be non-empty and below the
    /// max name length allowed.
    pub fn check_module_params(_netuid: u16, params: &ModuleParams<T>) -> DispatchResult {
        let max_name_length = MaxNameLength::<T>::get() as usize;

        assert!(!params.name.is_empty());
        ensure!(
            params.name.len() <= max_name_length,
            Error::<T>::ModuleNameTooLong
        );
        assert!(!params.address.is_empty());
        ensure!(
            params.address.len() <= max_name_length,
            Error::<T>::ModuleAddressTooLong
        );

        Ok(())
    }

    pub fn module_params(netuid: u16, uid: u16) -> ModuleParams<T> {
        ModuleParams {
            name: Self::get_module_name(netuid, uid),
            address: Self::get_module_address(netuid, uid),
            delegation_fee: Self::get_module_delegation_fee(netuid, uid),
            controller: Self::get_key_for_uid(netuid, uid),
        }
    }

    pub fn set_module_params(netuid: u16, uid: u16, module_params: ModuleParams<T>) {
        Self::set_module_name(netuid, uid, module_params.name);
        Self::set_module_address(netuid, uid, module_params.address);
        Self::set_module_delegation_fee(netuid, uid, module_params.delegation_fee);
    }

    pub fn get_module_address(netuid: u16, uid: u16) -> Vec<u8> {
        Address::<T>::get(netuid, uid)
    }

    pub fn set_module_address(netuid: u16, uid: u16, address: Vec<u8>) {
        Address::<T>::insert(netuid, uid, address);
    }

    pub fn get_module_delegation_fee(netuid: u16, uid: u16) -> Percent {
        let key = Self::get_key_for_uid(netuid, uid);
        let delegation_fee: Percent = DelegationFee::<T>::get(netuid, key);
        delegation_fee
    }

    pub fn set_module_delegation_fee(netuid: u16, uid: u16, delegation_fee: Percent) {
        let key = Self::get_key_for_uid(netuid, uid);
        DelegationFee::<T>::insert(netuid, key, delegation_fee);
    }

    pub fn get_module_name(netuid: u16, uid: u16) -> Vec<u8> {
        Name::<T>::get(netuid, uid)
    }

    pub fn set_module_name(netuid: u16, uid: u16, name: Vec<u8>) {
        Name::<T>::insert(netuid, uid, name.clone());
    }

    // Replace the module under this uid.
    pub fn remove_module(netuid: u16, uid: u16) {
        // 1. Get the old key under this position.
        let n = Self::get_subnet_n(netuid);
        if n == 0 {
            // No modules in the network.
            return;
        }
        let uid_key: T::AccountId = Keys::<T>::get(netuid, uid);
        let replace_uid = n - 1;
        let replace_key: T::AccountId = Keys::<T>::get(netuid, replace_uid);

        log::debug!(
            "remote_subnet( netuid: {:?} | uid : {:?} | new_key: {:?} ) ",
            netuid,
            uid,
            uid_key
        );

        // HANDLE THE KEY AND UID ASSOCIATIONS
        Uids::<T>::insert(netuid, &replace_key, uid); // Remove old key - uid association.
        Keys::<T>::insert(netuid, uid, &replace_key); // Make key - uid association.
        Uids::<T>::remove(netuid, &uid_key); // Remove old key - uid association.
        Keys::<T>::remove(netuid, replace_uid); // Remove key - uid association.

        // pop frm incentive vector and push to new key
        let mut incentive: Vec<u16> = Incentive::<T>::get(netuid);
        let mut dividends: Vec<u16> = Dividends::<T>::get(netuid);
        let mut last_update: Vec<u64> = LastUpdate::<T>::get(netuid);
        let mut emission: Vec<u64> = Emission::<T>::get(netuid);

        // swap consensus vectors
        incentive[uid as usize] = incentive[replace_uid as usize];
        dividends[uid as usize] = dividends[replace_uid as usize];
        emission[uid as usize] = emission[replace_uid as usize];
        last_update[uid as usize] = last_update[replace_uid as usize];

        // pop the last element (which is now a duplicate)
        incentive.pop();
        dividends.pop();
        emission.pop();
        last_update.pop();

        // update the vectors
        Incentive::<T>::insert(netuid, incentive); // Make uid - key association.
        Dividends::<T>::insert(netuid, dividends); // Make uid - key association.
        Emission::<T>::insert(netuid, emission); // Make uid - key association.
        LastUpdate::<T>::insert(netuid, last_update); // Make uid - key association.

        // SWAP WEIGHTS
        Weights::<T>::insert(netuid, uid, Weights::<T>::get(netuid, replace_uid)); // Make uid - key association.
        Weights::<T>::remove(netuid, replace_uid); // Make uid - key association.

        // HANDLE THE REGISTRATION BLOCK
        RegistrationBlock::<T>::insert(
            netuid,
            uid,
            RegistrationBlock::<T>::get(netuid, replace_uid),
        ); // Fill block at registration.
        RegistrationBlock::<T>::remove(netuid, replace_uid); // Fill block at registration.

        // HANDLE THE ADDRESS
        Address::<T>::insert(netuid, uid, Address::<T>::get(netuid, replace_uid)); // Fill module info.
        Address::<T>::remove(netuid, replace_uid); // Fill module info.

        // HANDLE THE NAMES
        Name::<T>::insert(netuid, uid, Name::<T>::get(netuid, replace_uid)); // Fill module namespace.
        Name::<T>::remove(netuid, replace_uid); // Fill module namespace.

        // HANDLE THE DELEGATION FEE
        DelegationFee::<T>::insert(
            netuid,
            &replace_key,
            DelegationFee::<T>::get(netuid, &uid_key),
        ); // Make uid - key association.
        DelegationFee::<T>::remove(netuid, &uid_key); // Make uid - key association.

        // 3. Remove the network if it is empty.
        N::<T>::mutate(netuid, |v| *v -= 1); // Decrease the number of modules in the network.

        // remove the network if it is empty
        if N::<T>::get(netuid) == 0 {
            Self::remove_subnet(netuid);
        }

        // remove stake from old key and add to new key
        Self::remove_stake_from_storage(netuid, &uid_key);
    }

    // Appends the uid to the network (without increasing stake).
    pub fn append_module(netuid: u16, key: &T::AccountId, name: Vec<u8>, address: Vec<u8>) -> u16 {
        // 1. Get the next uid. This is always equal to subnetwork_n.
        let uid: u16 = Self::get_subnet_n(netuid);
        let block_number = Self::get_current_block_number();
        log::debug!("append_module( netuid: {netuid:?} | uid: {key:?} | new_key: {uid:?})");
        // 3. Expand with new position.
        Emission::<T>::append(netuid, 0);
        Incentive::<T>::append(netuid, 0);
        Dividends::<T>::append(netuid, 0);
        LastUpdate::<T>::append(netuid, block_number);

        // 4. Insert new account information.
        Keys::<T>::insert(netuid, uid, key); // Make key - uid association.
        Uids::<T>::insert(netuid, key, uid); // Make uid - key association.
        RegistrationBlock::<T>::insert(netuid, uid, block_number); // Fill block at registration.
        Name::<T>::insert(netuid, uid, name); // Fill module namespace.
        Address::<T>::insert(netuid, uid, address); // Fill module info.
        DelegationFee::<T>::insert(netuid, key, DelegationFee::<T>::get(netuid, key)); // Make uid - key association.

        N::<T>::insert(netuid, N::<T>::get(netuid) + 1); // Increase the number of modules in the network.

        // increase the stake of the new key
        Self::increase_stake(netuid, key, key, 0);

        uid
    }

    pub fn get_module_stats(netuid: u16, uid: u16) -> ModuleStats<T> {
        let key = Self::get_key_for_uid(netuid, uid);
        let emission = Self::get_emission_for_uid(netuid, uid);
        let incentive = Self::get_incentive_for_uid(netuid, uid);
        let dividends = Self::get_dividends_for_uid(netuid, uid);
        let last_update = Self::get_last_update_for_uid(netuid, uid);

        let weights: Vec<(u16, u16)> = Weights::<T>::get(netuid, uid)
            .iter()
            .filter_map(|(i, w)| if *w > 0 { Some((*i, *w)) } else { None })
            .collect();
        let stake_from: Vec<(T::AccountId, u64)> = StakeFrom::<T>::get(netuid, key);
        let registration_block = Self::get_registration_block_for_uid(netuid, uid);

        ModuleStats {
            stake_from,
            emission,
            incentive,
            dividends,
            last_update,
            registration_block,
            weights,
        }
    }
}
