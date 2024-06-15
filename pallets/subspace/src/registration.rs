use crate::{module::ModuleChangeset, subnet::SubnetChangeset};

use super::*;

use frame_support::pallet_prelude::DispatchResult;
use frame_system::ensure_signed;
use sp_core::Get;

impl<T: Config> Pallet<T> {
    // Used on extrinsics, can panic
    #[allow(clippy::arithmetic_side_effects)]
    pub fn do_register(
        origin: T::RuntimeOrigin,
        network_name: Vec<u8>,
        name: Vec<u8>,
        address: Vec<u8>,
        stake: u64,
        module_key: T::AccountId,
        metadata: Option<Vec<u8>>,
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
        let netuid = match Self::get_netuid_for_name(&network_name) {
            Some(netuid) => netuid,
            // Create subnet if it does not exist.
            None => {
                let params = SubnetParams {
                    name: network_name.try_into().map_err(|_| Error::<T>::SubnetNameTooLong)?,
                    founder: key.clone(),
                    ..DefaultSubnetParams::<T>::get()
                };
                let changeset = SubnetChangeset::new(params)?;
                Self::add_subnet_from_registration(stake, changeset)?
            }
        };

        // 4.1 Ensure, that we are not exceeding the max allowed
        // registrations per interval.
        ensure!(
            RegistrationsThisInterval::<T>::get(netuid)
                <= MaxRegistrationsPerInterval::<T>::get(netuid),
            Error::<T>::TooManyRegistrationsPerInterval
        );

        // TODO: later, once legit whitelist has been filled up, turn on the code below.
        // We also have to declear a migration, of modules on netuid 0 that are not whitelisted.

        // --- 4.1 Ensure that the module_key is in the whitelist, if netuid is 0.

        // ensure!(
        //     netuid != 0 || Self::is_in_legit_whitelist(&module_key),
        //     Error::<T>::NotWhitelisted
        // );

        // --- 5. Ensure the caller has enough stake to register.
        let min_stake: u64 = MinStake::<T>::get(netuid);
        let current_burn: u64 = Burn::<T>::get(netuid);
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
        Self::reserve_module_slot(netuid)?;

        let fee = DefaultDelegationFee::<T>::get();
        // --- 8. Register the module and changeset.
        let module_changeset = ModuleChangeset::new(name, address, fee, metadata);

        let uid: u16 = Self::append_module(netuid, &module_key, module_changeset)?;

        // --- 9. Add the stake to the module, now that it is registered on the network.
        // allow to register with zero stake
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

        Self::remove_module(netuid, uid)?;
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
        stake_amount >= min_stake.saturating_add(min_burn)
    }

    // Determine which peer to prune from the network by finding the element with the lowest pruning
    // score out of immunity period. If all modules are in immunity period return None.
    //
    // In case the this lowest uid check is called by the global deregistration call
    // we ignore the immunity period and deregister the lowest uid (No matter the immunity period)
    pub fn get_lowest_uid(netuid: u16, ignore_immunity: bool) -> Option<u16> {
        // Immunity ignoring -> used if the deregistration is forced by global module overflow.
        // Immunity consideration -> used if the deregistration is forced by subnet module overflow.
        let mut min_score: u64 = u64::MAX;

        // This will stay `None` if every module is in immunity period.
        let mut lowest_priority_uid: Option<u16> = None;

        let current_block = Self::get_current_block_number();
        let immunity_period: u64 = ImmunityPeriod::<T>::get(netuid) as u64;

        // Get all the UIDs and their registration blocks from the storage
        let mut uids: Vec<_> = RegistrationBlock::<T>::iter_prefix(netuid).collect();

        // Sort the UIDs based on their registration block in ascending order
        // This will make sure we evaluate old miners first.
        uids.sort_by_key(|a| a.1);

        let emission_vec: Vec<u64> = Emission::<T>::get(netuid);
        for (module_uid_i, block_at_registration) in uids {
            let pruning_score: u64 = *emission_vec.get(module_uid_i as usize).unwrap_or(&0);

            // Find min pruning score.
            if min_score > pruning_score {
                let module_age: u64 = current_block.saturating_sub(block_at_registration);

                // Only allow modules that have greater than immunity period
                // or if we are ignoring immunity period
                if module_age >= immunity_period || ignore_immunity {
                    lowest_priority_uid = Some(module_uid_i);
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
        let num_subnets: u16 = TotalSubnets::<T>::get();
        let max_subnets: u16 = MaxAllowedSubnets::<T>::get();

        // if we have not reached the max number of subnets, then we can start a new one
        let target_subnet = if num_subnets >= max_subnets {
            let (min_stake_netuid, min_stake) = Self::get_least_staked_netuid();
            // if the stake is greater than the least staked network, then we can start a new one
            ensure!(stake > min_stake, Error::<T>::NotEnoughStakeToStartNetwork);
            Self::remove_subnet(min_stake_netuid);
            Some(min_stake_netuid)
        } else {
            None
        };

        Self::add_subnet(changeset, target_subnet)
    }

    // returns the amount of total modules on the network
    pub fn global_n_modules() -> u16 {
        Self::netuids().into_iter().map(N::<T>::get).sum()
    }

    /// This function checks whether there are still available module slots on the network. If the
    /// subnet is filled, deregister the least staked module on it, or if the max allowed modules on
    /// the network is reached, deregisters the least staked module on the least staked netuid.
    pub fn reserve_module_slot(netuid: u16) -> DispatchResult {
        if N::<T>::get(netuid) >= MaxAllowedUids::<T>::get(netuid) {
            // If we reach the max allowed modules for this subnet,
            // then we replace the lowest priority node in the current subnet
            let lowest_uid = Self::get_lowest_uid(netuid, false);
            if let Some(uid) = lowest_uid {
                Self::remove_module(netuid, uid)
            } else {
                Err(Error::<T>::NetworkIsImmuned.into())
            }
        } else if Self::global_n_modules() >= MaxAllowedModules::<T>::get() {
            let (subnet_uid, _) = Self::get_least_staked_netuid();
            let module_uid = Self::get_lowest_uid(subnet_uid, true).unwrap_or(0);

            // Deregister the lowest priority node in the least staked network
            // in this case we should ignore the immunity period,
            // Because if the lowest subnet has unreasonably high immunity period,
            // it could lead to exploitation of the network.
            Self::remove_module(subnet_uid, module_uid)
        } else {
            Ok(())
        }
    }
}
