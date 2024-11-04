use crate::*;

use frame_support::pallet_prelude::DispatchResult;
pub struct SubnetDistributionParameters;

// TODO: refactor whole file

impl<T: Config> Pallet<T> {
    pub fn do_update_module(
        origin: T::RuntimeOrigin,
        netuid: u16,
        changeset: ModuleChangeset,
    ) -> DispatchResult {
        // 1. We check the callers (key) signature.
        let key = ensure_signed(origin)?;
        let uid: u16 = Self::get_uid_for_key(netuid, &key).ok_or(Error::<T>::ModuleDoesNotExist)?;

        // 2. Apply the changeset
        changeset.apply::<T>(netuid, key, uid)?;

        Ok(())
    }

    pub fn does_module_name_exist(netuid: u16, name: &[u8]) -> bool {
        Name::<T>::iter_prefix_values(netuid).any(|existing| existing == name)
    }

    pub fn module_params(netuid: u16, key: &T::AccountId) -> ModuleParams<T> {
        let uid = Uids::<T>::get(netuid, key).unwrap_or(u16::MAX);

        ModuleParams {
            name: Name::<T>::get(netuid, uid),
            address: Address::<T>::get(netuid, uid),
            metadata: Metadata::<T>::get(netuid, key),
            delegation_fee: DelegationFee::<T>::get(key),
            controller: key.clone(),
        }
    }

    /// Appends the uid to the network (without increasing stake).
    pub fn append_module(
        netuid: u16,
        key: &T::AccountId,
        changeset: ModuleChangeset,
    ) -> Result<u16, sp_runtime::DispatchError> {
        // 1. Get the next uid. This is always equal to subnetwork_n.
        let uid: u16 = N::<T>::get(netuid);
        let block_number = Self::get_current_block_number();

        log::debug!("append_module( netuid: {netuid:?} | uid: {key:?} | new_key: {uid:?})");

        // 2. Apply the changeset
        changeset.apply::<T>(netuid, key.clone(), uid)?;

        // 3. Insert new account information.
        Keys::<T>::insert(netuid, uid, key); // Make key - uid association.
        Uids::<T>::insert(netuid, key, uid); // Make uid - key association.
        RegistrationBlock::<T>::insert(netuid, uid, block_number); // Fill block at registration.

        // 4. Expand consensus parameters with new position.
        Active::<T>::append(netuid, true);
        Consensus::<T>::append(netuid, 0);
        Emission::<T>::append(netuid, 0);
        Incentive::<T>::append(netuid, 0);
        Dividends::<T>::append(netuid, 0);
        LastUpdate::<T>::append(netuid, block_number);
        PruningScores::<T>::append(netuid, 0);
        Rank::<T>::append(netuid, 0);
        Trust::<T>::append(netuid, 0);
        ValidatorPermits::<T>::append(netuid, false);
        ValidatorTrust::<T>::append(netuid, 0);

        // 5. Increase the number of modules in the network.
        N::<T>::mutate(netuid, |n| *n = n.saturating_add(1));

        // increase the stake of the new key
        Self::increase_stake(key, key, 0);

        Ok(uid)
    }

    /// Replace the module under this uid.
    pub fn remove_module(
        netuid: u16,
        uid: u16,
        deregister_subnet_if_empty: bool,
    ) -> DispatchResult {
        // 1. Get the old key under this position.
        let n = N::<T>::get(netuid);
        if n == 0 {
            // No modules in the network.
            return Ok(());
        }

        let module_key: T::AccountId =
            Keys::<T>::get(netuid, uid).ok_or(Error::<T>::ModuleDoesNotExist)?;
        let replace_uid = n.saturating_sub(1);
        let replace_key: T::AccountId =
            Keys::<T>::get(netuid, replace_uid).expect("this is infallible");

        log::debug!(
            "remove_module( netuid: {:?} | uid : {:?} | key: {:?} ) ",
            netuid,
            uid,
            module_key
        );

        // HANDLE THE KEY AND UID ASSOCIATIONS
        Uids::<T>::insert(netuid, &replace_key, uid); // Replace UID related to the replaced key.
        Uids::<T>::remove(netuid, &module_key); // Remove old key - uid association.

        Keys::<T>::insert(netuid, uid, &replace_key); // Make key - uid association.
        Keys::<T>::remove(netuid, replace_uid); // Remove key - uid association.

        // pop frm incentive vector and push to new key
        let mut active = Active::<T>::get(netuid);
        let mut consensus = Consensus::<T>::get(netuid);
        let mut dividends = Dividends::<T>::get(netuid);
        let mut emission = Emission::<T>::get(netuid);
        let mut incentive = Incentive::<T>::get(netuid);
        let mut last_update = LastUpdate::<T>::get(netuid);
        let mut pruning_scores = PruningScores::<T>::get(netuid);
        let mut rank = Rank::<T>::get(netuid);
        let mut trust = Trust::<T>::get(netuid);
        let mut validator_permit = ValidatorPermits::<T>::get(netuid);
        let mut validator_trust = ValidatorTrust::<T>::get(netuid);

        macro_rules! update_vectors {
            ($a:expr) => {
                *($a.get_mut(uid as usize)
                    .ok_or(concat!("failed to access uid for array ", stringify!($a)))?) =
                    $a.get(replace_uid as usize).copied().ok_or(concat!(
                        "failed to access replace_uid for array ",
                        stringify!($a)
                    ))?;
            };
        }

        update_vectors![active];
        update_vectors![consensus];
        update_vectors![dividends];
        update_vectors![emission];
        update_vectors![incentive];
        update_vectors![last_update];
        update_vectors![pruning_scores];
        update_vectors![rank];
        update_vectors![trust];
        update_vectors![validator_permit];
        update_vectors![validator_trust];

        // pop the last element (which is now a duplicate)
        active.pop();
        consensus.pop();
        dividends.pop();
        emission.pop();
        incentive.pop();
        last_update.pop();
        pruning_scores.pop();
        rank.pop();
        trust.pop();
        validator_permit.pop();
        validator_trust.pop();

        // update the vectors
        Active::<T>::insert(netuid, active);
        Consensus::<T>::insert(netuid, consensus);
        Dividends::<T>::insert(netuid, dividends);
        Emission::<T>::insert(netuid, emission);
        Incentive::<T>::insert(netuid, incentive);
        LastUpdate::<T>::insert(netuid, last_update);
        PruningScores::<T>::insert(netuid, pruning_scores);
        Rank::<T>::insert(netuid, rank);
        Trust::<T>::insert(netuid, trust);
        ValidatorPermits::<T>::insert(netuid, validator_permit);
        ValidatorTrust::<T>::insert(netuid, validator_trust);

        WeightSetAt::<T>::set(netuid, uid, WeightSetAt::<T>::get(netuid, replace_uid));
        WeightSetAt::<T>::remove(netuid, replace_uid);

        // SWAP WEIGHTS
        let weights = T::remove_weights(netuid, replace_uid);
        T::set_weights(netuid, uid, weights);

        // HANDLE THE REGISTRATION BLOCK
        RegistrationBlock::<T>::insert(
            netuid,
            uid,
            RegistrationBlock::<T>::get(netuid, replace_uid),
        ); // Fill block at registration.
        RegistrationBlock::<T>::remove(netuid, replace_uid);

        // HANDLE THE ADDRESS
        Address::<T>::insert(netuid, uid, Address::<T>::get(netuid, replace_uid));
        Address::<T>::remove(netuid, replace_uid);

        // HANDLE THE METADATA
        Metadata::<T>::remove(netuid, &module_key);

        // HANDLE THE NAMES
        Name::<T>::insert(netuid, uid, Name::<T>::get(netuid, replace_uid));
        Name::<T>::remove(netuid, replace_uid);

        // HANDLE THE DELEGATION FEE
        if Uids::<T>::iter().all(|(_, key, _)| key != module_key) {
            DelegationFee::<T>::remove(&module_key);
            // Remove stake from old key and add to new key
            Self::remove_stake_from_storage(&module_key);
        }

        // 3. Remove the network if it is empty.
        let module_count = N::<T>::mutate(netuid, |v| {
            *v = v.saturating_sub(1);
            *v
        }); // Decrease the number of modules in the network.

        if let Some(key) = Self::get_key_for_uid(uid, netuid) {
            Self::handle_rootnet_module_deregistration(key, netuid);
        }

        // remove the network if it is empty
        if deregister_subnet_if_empty && module_count == 0 {
            Self::remove_subnet(netuid);
        }

        Ok(())
    }

    fn handle_rootnet_module_deregistration(key: T::AccountId, netuid: u16) {
        if Self::is_rootnet(netuid) {
            // Remove the direct delegation for the key
            RootnetControlDelegation::<T>::remove(&key);

            // Remove all delegations to the key
            RootnetControlDelegation::<T>::translate(
                |_, v: T::AccountId| {
                    if v == key {
                        None
                    } else {
                        Some(v)
                    }
                },
            );
        }
    }
}
