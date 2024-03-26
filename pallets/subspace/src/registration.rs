use crate::module::ModuleChangeset;

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

    // TODO:
    //- check ip
    // - add ability to set delegaiton fee, straight in registration
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
            // Create subnet if it does not exist.
            Self::add_subnet_from_registration(network_name, stake, &key)?
        };

        // --- 5. Ensure the caller has enough stake to register.
        let min_stake: u64 = MinStake::<T>::get(netuid);
        let current_burn: u64 = Self::get_burn();
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
        Self::check_module_limits(netuid);

        // --- 8. Register the module.
        let changeset = ModuleChangeset::new(name, address);
        let uid: u16 = Self::append_module(netuid, &module_key, changeset)?;

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
        RegistrationsPerBlock::<T>::mutate(|val| *val += 1);
        RegistrationsThisInterval::<T>::mutate(|val| *val += 1);

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

    pub fn get_lowest_uid(netuid: u16) -> u16 {
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
                if module_age > immunity_period {
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
        name: Vec<u8>,
        stake: u64,
        founder_key: &T::AccountId,
    ) -> Result<u16, sp_runtime::DispatchError> {
        let num_subnets: u16 = Self::num_subnets();
        let max_subnets: u16 = Self::get_global_max_allowed_subnets();

        let target_subnet = if num_subnets >= max_subnets {
            let (min_stake_netuid, min_stake) = Self::least_staked_netuid();
            ensure!(stake > min_stake, Error::<T>::NotEnoughStakeToStartNetwork);
            Self::remove_subnet(min_stake_netuid);
            Some(min_stake_netuid)
        } else {
            None
        };

        let params = SubnetParams {
            name,
            founder: founder_key.clone(),
            ..Self::default_subnet_params()
        };

        Ok(Self::add_subnet(params, target_subnet))
    }

    pub fn check_module_limits(netuid: u16) {
        // Check if we have reached the max allowed modules for the network
        if Self::global_n() >= Self::get_max_allowed_modules() {
            // Get the least staked network (subnet)
            let (least_staked_netuid, _) = Self::least_staked_netuid();

            // Deregister the lowest priority node in the least staked network
            Self::remove_module(
                least_staked_netuid,
                Self::get_lowest_uid(least_staked_netuid),
            );
        } else if Self::get_subnet_n(netuid) >= Self::get_max_allowed_uids(netuid) {
            // If we reach the max allowed modules for this network,
            // then we replace the lowest priority node in the current network
            Self::remove_module(netuid, Self::get_lowest_uid(netuid));
        }
    }
}
