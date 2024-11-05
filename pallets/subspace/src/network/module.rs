use crate::*;

use frame_support::pallet_prelude::DispatchResult;
pub struct SubnetDistributionParameters;

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
            delegation_fee: DelegationFee::<T>::get(key),
            metadata: Metadata::<T>::get(netuid, key),
            _pd: PhantomData,
        }
    }

    /// Appends the uid to the network (without increasing stake).
    pub fn append_module(
        netuid: u16,
        key: &T::AccountId,
        changeset: ModuleChangeset,
    ) -> Result<u16, sp_runtime::DispatchError> {
        // 1. Get the next uid and current block number
        let uid: u16 = N::<T>::get(netuid);
        let block_number = Self::get_current_block_number();

        log::debug!("append_module( netuid: {netuid:?} | uid: {key:?} | new_key: {uid:?})");

        // 3. Initialize key storages and required swap storages
        KeyStorageHandler::initialize::<T>(netuid, uid, key)?;
        RegistrationBlock::<T>::insert(netuid, uid, block_number);
        ModuleSwapStorages::Address.initialize::<T>(netuid, uid)?;
        ModuleSwapStorages::Name.initialize::<T>(netuid, uid)?;

        // 4. Expand consensus parameters with new position using ModuleVectors
        for vector in ModuleVectors::all() {
            match vector {
                ModuleVectors::LastUpdate => {
                    let mut vec = LastUpdate::<T>::get(netuid);
                    vec.push(block_number);
                    LastUpdate::<T>::insert(netuid, vec);
                }
                _ => {
                    vector.append::<T>(netuid)?;
                }
            }
        }
        changeset.apply::<T>(netuid, key.clone(), uid)?;

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
        // 1. Check if network has any modules
        let n = N::<T>::get(netuid);
        if n == 0 {
            return Ok(());
        }

        // 2. Get the keys for the current and replacement positions
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

        // 3. Handle key-related storage swaps
        KeyStorageHandler::swap_and_remove::<T>(
            netuid,
            uid,
            replace_uid,
            &module_key,
            &replace_key,
        )?;

        // 4. Handle vector storage items
        for vector in ModuleVectors::all() {
            vector.swap_and_remove::<T>(netuid, uid, replace_uid)?;
        }

        // 5. Handle swap storage items
        for storage in ModuleSwapStorages::all() {
            storage.swap_and_remove::<T>(netuid, uid, replace_uid)?;
        }

        // TODO: move this to the macro as well

        // 6. Handle weights (this might need its own macro category if there are more similar
        //    cases)
        let weights = T::remove_weights(netuid, replace_uid);
        T::set_weights(netuid, uid, weights);

        // 7. Handle Metadata (special case as it only needs removal)
        Metadata::<T>::remove(netuid, &module_key);

        // 8. Handle delegation and stake
        if Uids::<T>::iter().all(|(_, key, _)| key != module_key) {
            DelegationFee::<T>::remove(&module_key);
            Self::remove_stake_from_storage(&module_key);
        }

        // 9. Update network size
        let module_count = N::<T>::mutate(netuid, |v| {
            *v = v.saturating_sub(1);
            *v
        });

        // 10. Handle rootnet deregistration
        if let Some(key) = Self::get_key_for_uid(uid, netuid) {
            Self::handle_rootnet_module_deregistration(key, netuid);
        }

        // 11. Remove subnet if empty
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
