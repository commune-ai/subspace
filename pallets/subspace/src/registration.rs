use crate::{module::ModuleChangeset, subnet::SubnetChangeset};

use super::*;

use frame_support::pallet_prelude::DispatchResult;
use frame_system::ensure_signed;

use sp_std::vec::Vec;

impl<T: Config> Pallet<T> {
    pub fn do_add_to_whitelist(
        origin: T::RuntimeOrigin,
        module_key: T::AccountId,
    ) -> DispatchResult {
        // --- 1. Check that the caller has signed the transaction.
        let key = ensure_signed(origin)?;

        // --- 2. Ensure that the key is the nominator multisig.
        ensure!(Self::get_nominator() == key, Error::<T>::NotNominator);

        // --- 3. Ensure that the module_key is not already in the whitelist.
        ensure!(
            !Self::is_in_legit_whitelist(&module_key),
            Error::<T>::AlreadyWhitelisted
        );

        // --- 4. Insert the module_key into the whitelist.
        Self::insert_to_whitelist(module_key.clone());

        // -- deposit event
        Self::deposit_event(Event::WhitelistModuleAdded(module_key));

        // --- 5. Ok and done.
        Ok(())
    }

    pub fn do_remove_from_whitelist(
        origin: T::RuntimeOrigin,
        module_key: T::AccountId,
    ) -> DispatchResult {
        // --- 1. Check that the caller has signed the transaction.
        let key = ensure_signed(origin)?;

        // --- 2. Ensure that the key is the nominator multisig.
        ensure!(Self::get_nominator() == key, Error::<T>::NotNominator);

        // --- 3. Ensure that the module_key is in the whitelist.
        ensure!(
            Self::is_in_legit_whitelist(&module_key),
            Error::<T>::NotWhitelisted
        );

        // --- 4. Remove the module_key from the whitelist.
        Self::rm_from_whitelist(&module_key);

        // -- deposit event
        Self::deposit_event(Event::WhitelistModuleRemoved(module_key));

        // --- 5. Ok and done.
        Ok(())
    }

    pub fn do_register(
        origin: T::RuntimeOrigin,
        network_name: Vec<u8>,
        name: Vec<u8>,
        address: Vec<u8>,
        stake: u64,
        module_key: T::AccountId,
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
            Self::has_enough_balance(&key, stake),
            Error::<T>::NotEnoughBalanceToRegister
        );

        // --- 4. Resolve the network in case it doesn't exist
        let netuid = if let Some(netuid) = Self::get_netuid_for_name(&network_name) {
            netuid
        } else {
            let subnet_changeset = SubnetChangeset::new(&network_name, &key, None);
            // Create subnet if it does not exist.
            Self::add_subnet_from_registration(stake, subnet_changeset)?
        };

        // TODO: later, once legit whitelist has been filled up, turn on the code below.
        // We also have to declear a migration, of modules on netuid 0 that are not whitelisted.

        // --- 4.1 Ensure that the module_key is in the whitelist, if netuid is 0.

        // ensure!(
        //     netuid != 0 || Self::is_in_legit_whitelist(&module_key),
        //     Error::<T>::NotWhitelisted
        // );

        //  4.2 If a subnet was removed, we need to swap the netuid with the removed one
        let netuid = match RemovedSubnets::<T>::try_get(netuid) {
            Ok(0) => {
                let new_netuid = RemovedSubnets::<T>::iter().map(|(k, _)| k).min().unwrap();
                RemovedSubnets::<T>::insert(new_netuid, netuid);
                new_netuid
            }
            Ok(target) => target,
            Err(_) => netuid,
        };

        // --- 5. Ensure the caller has enough stake to register.
        let min_stake: u64 = MinStake::<T>::get(netuid);
        let current_burn: u64 = Self::get_burn(netuid);
        // also ensures that in the case current_burn is present, the stake is enough
        // as burn, will be decreased from the stake on the module
        ensure!(
            Self::enough_stake_to_register(min_stake, current_burn, stake),
            Error::<T>::NotEnoughStakeToRegister
        );

        // --- 6. Ensure the module key is not already registered.
        ensure!(
            !Self::key_registered(netuid, &module_key),
            Error::<T>::KeyAlreadyRegistered
        );

        // --- 7. Check if we are exceeding the max allowed modules per network.
        // If we do deregister slot.
        Self::reserve_module_slot(netuid);

        // --- 8. Register the module and changeset.
        let module_changeset = ModuleChangeset::new(name, address);

        let uid: u16 = Self::append_module(netuid, &module_key, module_changeset)?;

        // --- 9. Add the stake to the module, now that it is registered on the network.
        Self::do_add_stake(origin, netuid, module_key.clone(), stake)?;

        // constant -> current_burn logic
        if current_burn > 0 {
            // if min burn is present, decrease the stake by the min burn
            Self::decrease_stake(netuid, &key, &module_key, current_burn);
        }

        // Make sure that the registration went through.
        ensure!(
            Self::key_registered(netuid, &module_key),
            Error::<T>::NotRegistered
        );

        // --- 10. Increment the number of registrations.
        RegistrationsPerBlock::<T>::mutate(|val: &mut u16| *val += 1);
        RegistrationsThisInterval::<T>::mutate(netuid, |registrations| {
            *registrations = registrations.saturating_add(1);
        });

        // --- Deposit successful event.
        Self::deposit_event(Event::ModuleRegistered(netuid, uid, module_key));

        // --- 11. Ok and done.
        Ok(())
    }

    pub fn do_deregister(origin: T::RuntimeOrigin, netuid: u16) -> DispatchResult {
        // --- 1. Check that the caller has signed the transaction.
        let key = ensure_signed(origin)?;

        ensure!(
            Self::key_registered(netuid, &key),
            Error::<T>::NotRegistered
        );

        let uid: u16 = Self::get_uid_for_key(netuid, &key);

        Self::remove_module(netuid, uid);
        ensure!(
            !Self::key_registered(netuid, &key),
            Error::<T>::StillRegistered
        );

        // --- Deposit successful event.
        Self::deposit_event(Event::ModuleDeregistered(netuid, uid, key));
        // --- 3. Ok and done.
        Ok(())
    }

    /// Whether the netuid has enough stake to cover the minimal stake and min burn
    pub fn enough_stake_to_register(min_stake: u64, min_burn: u64, stake_amount: u64) -> bool {
        stake_amount >= (min_stake + min_burn)
    }

    // Determine which peer to prune from the network by finding the element with the lowest pruning
    // score out of immunity period. If all modules are in immunity period, return node with lowest
    // prunning score. This function will always return an element to prune.

    pub fn get_pruning_score_for_uid(netuid: u16, uid: u16) -> u64 {
        let vec: Vec<u64> = Emission::<T>::get(netuid);
        *vec.get(uid as usize).unwrap_or(&0)
    }

    pub fn get_lowest_uid(netuid: u16, ignore_immunity: bool) -> u16 {
        // immunity ignoring is used if the deregistration is forced by global (network) module
        // limit immunity period is considered if the deregistration is forced by subnet
        // module limit
        let n: u16 = Self::get_subnet_n(netuid);

        let mut min_score: u64 = u64::MAX;
        let mut lowest_priority_uid: u16 = 0;
        let _prune_uids: Vec<u16> = Vec::new();
        let current_block = Self::get_current_block_number();
        let immunity_period: u64 = Self::get_immunity_period(netuid) as u64;

        for module_uid_i in 0..n {
            let pruning_score: u64 = Self::get_pruning_score_for_uid(netuid, module_uid_i);

            // Find min pruning score.

            if min_score > pruning_score {
                let block_at_registration: u64 =
                    Self::get_module_registration_block(netuid, module_uid_i);
                let module_age: u64 = current_block.saturating_sub(block_at_registration);

                // only allow modules that have greater than immunity period
                // or if we are ignoring immunity period
                if module_age > immunity_period || ignore_immunity {
                    lowest_priority_uid = module_uid_i;
                    min_score = pruning_score;
                    if min_score == 0 {
                        break;
                    }
                }
            }
        }

        lowest_priority_uid
    }

    pub fn add_subnet_from_registration(
        stake: u64,
        changeset: SubnetChangeset<T>,
    ) -> Result<u16, sp_runtime::DispatchError> {
        let num_subnets: u16 = Self::num_subnets();
        let max_subnets: u16 = Self::get_global_max_allowed_subnets();

        let mut target_subnet = RemovedSubnets::<T>::iter().map(|(k, _)| k).min();

        // if we have not reached the max number of subnets, then we can start a new one
        if num_subnets >= max_subnets {
            let (min_stake_netuid, min_stake) = Self::least_staked_netuid();
            // if the stake is greater than the least staked network, then we can start a new one
            ensure!(stake > min_stake, Error::<T>::NotEnoughStakeToStartNetwork);
            Self::remove_subnet(min_stake_netuid);
            target_subnet = Some(min_stake_netuid);
        }

        Self::add_subnet(changeset, target_subnet)
    }

    /// This function checks whether there are still available module slots on the network. If the
    /// subnet is filled, deregister the least staked module on it, or if the max allowed modules on
    /// the network is reached, deregisters the least staked module on the least staked netuid.
    pub fn reserve_module_slot(netuid: u16) {
        if Self::get_subnet_n(netuid) >= Self::get_max_allowed_uids(netuid) {
            // If we reach the max allowed modules for this subnet,
            // then we replace the lowest priority node in the current subnet
            Self::remove_module(netuid, Self::get_lowest_uid(netuid, false));
        } else if Self::global_n_modules() >= Self::get_max_allowed_modules() {
            // Get the least staked network (subnet) and its least staked module.
            let (subnet_uid, _) = Self::least_staked_netuid();
            let module_uid = Self::get_lowest_uid(subnet_uid, true);

            // Deregister the lowest priority node in the least staked network
            // in this case we should ignore the immunity period,
            // Because if the lowest subnet has unreasonably high immunity period,
            // it could lead to exploitation of the network.
            Self::remove_module(subnet_uid, module_uid);
        }
    }
}
