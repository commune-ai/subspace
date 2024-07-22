use super::*;

use frame_support::pallet_prelude::DispatchResult;
use sp_arithmetic::per_things::Percent;
pub struct SubnetDistributionParameters;

#[derive(Debug)]
pub struct ModuleChangeset {
    pub name: Option<Vec<u8>>,
    pub address: Option<Vec<u8>>,
    pub delegation_fee: Option<Percent>,
    pub metadata: Option<Vec<u8>>,
}

impl ModuleChangeset {
    #[must_use]
    pub fn new(
        name: Vec<u8>,
        address: Vec<u8>,
        delegation_fee: Percent,
        metadata: Option<Vec<u8>>,
    ) -> Self {
        Self {
            name: Some(name),
            address: Some(address),
            delegation_fee: Some(delegation_fee),
            metadata,
        }
    }

    #[must_use]
    pub fn update<T: Config>(
        params: &ModuleParams<T>,
        name: Vec<u8>,
        address: Vec<u8>,
        delegation_fee: Option<Percent>,
        metadata: Option<Vec<u8>>,
    ) -> Self {
        Self {
            name: (name != params.name).then_some(name),
            address: (address != params.address).then_some(address),
            delegation_fee,
            metadata,
        }
    }

    /// Checks whether the module params are valid. Name and address must be non-empty and below the
    /// max name length allowed.
    pub fn apply<T: Config>(
        self,
        netuid: u16,
        key: T::AccountId,
        uid: u16,
    ) -> Result<(), sp_runtime::DispatchError> {
        let max = MaxNameLength::<T>::get() as usize;
        let min = MinNameLength::<T>::get() as usize;

        if let Some(name) = self.name {
            ensure!(!name.is_empty(), Error::<T>::InvalidModuleName);
            ensure!(name.len() <= max, Error::<T>::ModuleNameTooLong);
            ensure!(name.len() >= min, Error::<T>::ModuleNameTooShort);
            core::str::from_utf8(&name).map_err(|_| Error::<T>::InvalidModuleName)?;
            ensure!(
                !Pallet::<T>::does_module_name_exist(netuid, &name),
                Error::<T>::ModuleNameAlreadyExists
            );

            Name::<T>::insert(netuid, uid, name);
        }

        if let Some(addr) = self.address {
            ensure!(!addr.is_empty(), Error::<T>::InvalidModuleAddress);
            ensure!(addr.len() <= max, Error::<T>::ModuleAddressTooLong);
            core::str::from_utf8(&addr).map_err(|_| Error::<T>::InvalidModuleAddress)?;

            Address::<T>::insert(netuid, uid, addr);
        }

        if let Some(fee) = self.delegation_fee {
            let floor = FloorDelegationFee::<T>::get();
            ensure!(fee >= floor, Error::<T>::InvalidMinDelegationFee);

            DelegationFee::<T>::insert(netuid, &key, fee);
        }

        if let Some(metadata) = self.metadata {
            ensure!(!metadata.is_empty(), Error::<T>::InvalidModuleMetadata);
            ensure!(metadata.len() <= 59, Error::<T>::ModuleMetadataTooLong);
            core::str::from_utf8(&metadata).map_err(|_| Error::<T>::InvalidModuleMetadata)?;

            Metadata::<T>::insert(netuid, &key, metadata);
        }

        Pallet::<T>::deposit_event(Event::ModuleUpdated(netuid, key));
        Ok(())
    }
}

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
            delegation_fee: DelegationFee::<T>::get(netuid, key),
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

        // SWAP WEIGHTS
        Weights::<T>::insert(netuid, uid, Weights::<T>::get(netuid, replace_uid)); // Make uid - key association.
        Weights::<T>::remove(netuid, replace_uid); // Make uid - key association.

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
        Metadata::<T>::insert(
            netuid,
            &module_key,
            Metadata::<T>::get(netuid, &replace_key).unwrap_or_default(),
        );
        Metadata::<T>::remove(netuid, &replace_key);

        // HANDLE THE NAMES
        Name::<T>::insert(netuid, uid, Name::<T>::get(netuid, replace_uid));
        Name::<T>::remove(netuid, replace_uid);

        // HANDLE THE DELEGATION FEE
        DelegationFee::<T>::insert(
            netuid,
            &module_key,
            DelegationFee::<T>::get(netuid, &replace_key),
        ); // Make uid - key association.
        DelegationFee::<T>::remove(netuid, &replace_key); // Make uid - key association.

        // remove stake from old key and add to new key
        Self::remove_stake_from_storage(&module_key);

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
