use super::*;

use frame_support::pallet_prelude::DispatchResult;
use frame_system::ensure_signed;

use sp_std::vec::Vec;

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
        let netuid = if let Some(netuid) = Self::get_netuid_for_name(&network) {
            netuid
        } else {
            // Create subnet if it does not exist.
            Self::add_subnet_from_registration(network, stake_amount, &key)?
        };

        //  4.1 If a subnet was removed, we need to swap the netuid with the removed one
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
        let current_burn: u64 = Self::get_burn();

        // also ensures that in the case current_burn is present, the stake is enough
        // as burn, will be decreased from the stake on the module
        ensure!(
            Self::enough_stake_to_register(netuid, min_stake, current_burn, stake_amount),
            Error::<T>::NotEnoughStakeToRegister
        );

        // --- 6. Ensure the module key is not already registered,
        // and namespace is not already taken.
        ensure!(
            !Self::key_registered(netuid, &module_key),
            Error::<T>::KeyAlreadyRegistered
        );

        ensure!(
            !Self::does_module_name_exist(netuid, name.clone()),
            Error::<T>::NameAlreadyRegistered
        );

        // --- 7. Check if we are exceeding the max allowed modules per network.
        // If we do deregister slot.
        Self::check_module_limits(netuid);

        // --- 8. Register the module.
        let uid: u16 = Self::append_module(netuid, &module_key, name.clone(), address.clone());

        // --- 9. Add the stake to the module, now that it is registered on the network.
        Self::do_add_stake(origin, netuid, module_key.clone(), stake_amount)?;

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

        // --- Deposit successful event.
        Self::deposit_event(Event::ModuleDeregistered(netuid, uid, key));
        // --- 3. Ok and done.
        Ok(())
    }

    /// Whether the netuid has enough stake to cover the minimal stake and min burn
    pub fn enough_stake_to_register(
        _netuid: u16,
        min_stake: u64,
        min_burn: u64,
        stake_amount: u64,
    ) -> bool {
        stake_amount >= (min_stake + min_burn)
    }

    // Determine which peer to prune from the network by finding the element with the lowest pruning
    // score out of immunity period. If all modules are in immunity period, return node with lowest
    // prunning score. This function will always return an element to prune.

    pub fn get_pruning_score_for_uid(netuid: u16, uid: u16) -> u64 {
        let vec: Vec<u64> = Emission::<T>::get(netuid);
        if (uid as usize) < vec.len() {
            vec[uid as usize]
        } else {
            0u64
        }
    }
    pub fn get_lowest_uid(netuid: u16) -> u16 {
        let n: u16 = Self::get_subnet_n(netuid);

        let mut min_score: u64 = u64::MAX;
        let mut lowest_priority_uid: u16 = 0;
        let _prune_uids: Vec<u16> = Vec::new();
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
        let mut target_subnet = RemovedSubnets::<T>::iter().map(|(k, _)| k).min();

        // if we have not reached the max number of subnets, then we can start a new one
        if num_subnets >= max_subnets {
            let (min_stake_netuid, min_stake) = Self::least_staked_netuid();
            target_subnet = Some(min_stake_netuid);
            ensure!(stake > min_stake, Error::<T>::NotEnoughStakeToStartNetwork);
            Self::remove_subnet(min_stake_netuid);
        }

        // if we have reached the max number of subnets, then we can start a new one if the stake is
        // greater than the least staked network
        let mut params: SubnetParams<T> = DefaultSubnetParams::<T>::get();
        params.name = name;
        params.founder = founder_key.clone();

        Ok(Self::add_subnet(params, target_subnet))
    }
    pub fn check_module_limits(netuid: u16) {
        // check if we have reached the max allowed modules,
        // if so deregister the lowest priority node

        // replace a node if we reach the max allowed modules for the network
        if Self::global_n() >= Self::get_max_allowed_modules() {
            // get the least staked network (subnet)
            let (least_staked_netuid, _) = Self::least_staked_netuid();

            // deregister the lowest priority node
            Self::remove_module(
                least_staked_netuid,
                Self::get_lowest_uid(least_staked_netuid),
            );

        // if we reach the max allowed modules for this network,
        // then we replace the lowest priority node
        } else if Self::get_subnet_n(netuid) >= Self::get_max_allowed_uids(netuid) {
            // deregister the lowest priority node
            Self::remove_module(netuid, Self::get_lowest_uid(netuid));
        }
    }
}
