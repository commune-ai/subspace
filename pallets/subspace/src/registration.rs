use super::*;
use crate::{module::ModuleChangeset, subnet::SubnetChangeset};
use frame_support::storage::with_storage_layer;

use frame_support::pallet_prelude::DispatchResult;
use frame_system::ensure_signed;
use sp_core::Get;
use substrate_fixed::types::I110F18;

impl<T: Config> Pallet<T> {
    // TODO:
    // make registration cost for rootnet (s0) to 0
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
                let subnet_burn_config = SubnetBurnConfig::<T>::get();
                ensure!(
                    SubnetRegistrationsThisInterval::<T>::get()
                        < subnet_burn_config.max_registrations,
                    Error::<T>::TooManySubnetRegistrationsPerInterval
                );

                let params = SubnetParams {
                    name: network_name.try_into().map_err(|_| Error::<T>::SubnetNameTooLong)?,
                    founder: key.clone(),
                    ..DefaultSubnetParams::<T>::get()
                };
                let changeset = SubnetChangeset::new(params)?;
                let burn = SubnetBurn::<T>::get();

                Self::remove_balance_from_account(
                    &key,
                    Self::u64_to_balance(burn).ok_or(Error::<T>::CouldNotConvertToBalance)?,
                )
                .map_err(|_| Error::<T>::NotEnoughBalanceToRegisterSubnet)?;

                Self::add_subnet_from_registration(changeset)?
            }
        };

        // 4.1 Ensure, that we are not exceeding the max allowed
        // registrations per interval.
        ensure!(
            RegistrationsThisInterval::<T>::get(netuid)
                < MaxRegistrationsPerInterval::<T>::get(netuid),
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

        // Account for root validator register requirements
        Self::check_rootnet_registration_requirements(netuid, stake)?;

        let fee = DefaultDelegationFee::<T>::get();
        // --- 8. Register the module and changeset.
        let module_changeset = ModuleChangeset::new(name, address, fee, metadata);

        let uid: u16 = Self::append_module(netuid, &module_key, module_changeset)?;

        // --- 9. Add the stake to the module, now that it is registered on the network.
        // allow to register with zero stake
        Self::do_add_stake(origin, module_key.clone(), stake)?;

        // constant -> current_burn logic
        if current_burn > 0 {
            // if min burn is present, decrease the stake by the min burn
            Self::decrease_stake(&key, &module_key, current_burn);
        }

        // Make sure that the registration went through.
        ensure!(
            Self::key_registered(netuid, &module_key),
            Error::<T>::NotRegistered
        );

        // Add validator permit if rootnet
        Self::add_rootnet_validator(netuid, uid)?;

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

        Self::remove_module(netuid, uid, true)?;
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
        changeset: SubnetChangeset<T>,
    ) -> Result<u16, sp_runtime::DispatchError> {
        let num_subnets: u16 = TotalSubnets::<T>::get();
        let max_subnets: u16 = MaxAllowedSubnets::<T>::get();

        // if we have not reached the max number of subnets, then we can start a new one
        let target_subnet = if num_subnets >= max_subnets {
            let lowest_emission_netuid = T::get_lowest_emission_netuid();
            let netuid = lowest_emission_netuid.ok_or(sp_runtime::DispatchError::Other(
                "No valid netuid to deregister",
            ))?;

            // if the stake is greater than the least staked network, then we can start a new one
            Self::remove_subnet(netuid);
            Some(netuid)
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
            // Subnet is full, replace lowest priority node
            Self::replace_lowest_priority_node(netuid, false)
        } else if Self::global_n_modules() >= MaxAllowedModules::<T>::get() {
            // Global limit reached, remove from lowest emission subnet
            Self::remove_from_lowest_emission_subnet()
        } else {
            Ok(())
        }
    }

    fn replace_lowest_priority_node(netuid: u16, ignore_immunity: bool) -> DispatchResult {
        if let Some(uid) = Self::get_lowest_uid(netuid, ignore_immunity) {
            Self::remove_module(netuid, uid, false)
        } else {
            Err(Error::<T>::NetworkIsImmuned.into())
        }
    }

    fn remove_from_lowest_emission_subnet() -> DispatchResult {
        if let Some(subnet_id) = T::get_lowest_emission_netuid() {
            if let Some(module_uid) = Self::get_lowest_uid(subnet_id, true) {
                Self::remove_module(subnet_id, module_uid, true)
            } else {
                Err(Error::<T>::NetworkIsImmuned.into())
            }
        } else {
            Err(Error::<T>::NetworkIsImmuned.into())
        }
    }

    // --------------------------
    // Rootnet utils
    // --------------------------

    fn check_rootnet_registration_requirements(netuid: u16, stake: u64) -> DispatchResult {
        if netuid == ROOTNET_ID
            && Self::get_validator_count(ROOTNET_ID)
                >= MaxAllowedValidators::<T>::get(ROOTNET_ID).unwrap_or(u16::MAX) as usize
        {
            let permits = ValidatorPermits::<T>::get(ROOTNET_ID);
            let (lower_stake_validator, lower_stake) = Keys::<T>::iter_prefix(ROOTNET_ID)
                .filter(|(uid, _)| permits.get(*uid as usize).is_some_and(|b| *b))
                .map(|(_, key)| (key.clone(), Stake::<T>::get(key)))
                .min_by_key(|(_, stake)| *stake)
                .ok_or(Error::<T>::ArithmeticError)?;

            ensure!(stake >= lower_stake, Error::<T>::NotEnoughStakeToRegister);

            let lower_stake_validator_uid =
                Self::get_uid_for_key(ROOTNET_ID, &lower_stake_validator);

            Self::remove_module(ROOTNET_ID, lower_stake_validator_uid, true)?
        }
        Ok(())
    }

    fn add_rootnet_validator(netuid: u16, module_uid: u16) -> DispatchResult {
        if netuid == ROOTNET_ID {
            let mut validator_permits = ValidatorPermits::<T>::get(ROOTNET_ID);
            if validator_permits.len() <= module_uid as usize {
                return Err(Error::<T>::NotRegistered.into());
            }

            if let Some(permit) = validator_permits.get_mut(module_uid as usize) {
                *permit = true;
                ValidatorPermits::<T>::set(ROOTNET_ID, validator_permits);
            }
        }

        Ok(())
    }

    fn get_validator_count(netuid: u16) -> usize {
        ValidatorPermits::<T>::get(netuid).into_iter().filter(|b| *b).count()
    }

    pub fn clear_rootnet_daily_weight_calls(block: u64) {
        if block.checked_rem(10_800).is_some_and(|r| r == 0) {
            let _ = RootNetWeightCalls::<T>::clear(u32::MAX, None);
        }
    }

    // --------------------------
    // Registration Burn
    // --------------------------

    // This code is running under the `on_initialize` hook
    pub fn adjust_registration_parameters(block_number: u64) {
        // For subnet prices
        let subnet_config = SubnetBurnConfig::<T>::get();
        let subnet_burn = SubnetBurn::<T>::get();
        Self::adjust_burn_parameters(
            block_number,
            subnet_config.adjustment_interval,
            SubnetRegistrationsThisInterval::<T>::get(),
            subnet_config.expected_registrations,
            subnet_config.adjustment_alpha,
            subnet_config.min_burn,
            subnet_config.max_burn,
            subnet_burn,
            |adjusted_burn| {
                SubnetBurn::<T>::set(adjusted_burn);
                SubnetRegistrationsThisInterval::<T>::set(0);
            },
        );

        // For subnet modules
        RegistrationsPerBlock::<T>::mutate(|val| *val = 0);

        for (netuid, _) in Tempo::<T>::iter() {
            let module_config = BurnConfig::<T>::get();
            let module_burn = Burn::<T>::get(netuid);
            Self::adjust_burn_parameters(
                block_number,
                TargetRegistrationsInterval::<T>::get(netuid),
                RegistrationsThisInterval::<T>::get(netuid),
                TargetRegistrationsPerInterval::<T>::get(netuid),
                AdjustmentAlpha::<T>::get(netuid),
                module_config.min_burn,
                module_config.max_burn,
                module_burn,
                |adjusted_burn| {
                    Burn::<T>::insert(netuid, adjusted_burn);
                    RegistrationsThisInterval::<T>::insert(netuid, 0);
                },
            );
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn adjust_burn_parameters<F>(
        block_number: u64,
        adjustment_interval: u16,
        registrations_this_interval: u16,
        target_registrations: u16,
        alpha: u64,
        min_burn: u64,
        max_burn: u64,
        current_burn: u64,
        update_fn: F,
    ) where
        F: FnOnce(u64),
    {
        let reached_interval =
            block_number.checked_rem(u64::from(adjustment_interval)).is_some_and(|r| r == 0);

        if !reached_interval {
            return;
        }

        let adjusted_burn = Self::adjust_burn(
            current_burn,
            registrations_this_interval,
            target_registrations,
            alpha,
            min_burn,
            max_burn,
        );

        update_fn(adjusted_burn);
    }

    fn adjust_burn(
        current_burn: u64,
        registrations_this_interval: u16,
        target_registrations_per_interval: u16,
        adjustment_alpha: u64,
        min_burn: u64,
        max_burn: u64,
    ) -> u64 {
        let updated_burn: I110F18 = I110F18::from_num(current_burn)
            .checked_mul(I110F18::from_num(
                registrations_this_interval.saturating_add(target_registrations_per_interval),
            ))
            .unwrap_or_default()
            .checked_div(I110F18::from_num(
                target_registrations_per_interval.saturating_add(target_registrations_per_interval),
            ))
            .unwrap_or_default();
        let alpha: I110F18 = I110F18::from_num(adjustment_alpha)
            .checked_div(I110F18::from_num(u64::MAX))
            .unwrap_or_else(|| I110F18::from_num(0));
        let next_value: I110F18 = alpha
            .checked_mul(I110F18::from_num(current_burn))
            .unwrap_or_else(|| I110F18::from_num(0))
            .saturating_add(
                I110F18::from_num(1.0)
                    .saturating_sub(alpha)
                    .checked_mul(updated_burn)
                    .unwrap_or_else(|| I110F18::from_num(0)),
            );
        if next_value >= I110F18::from_num(max_burn) {
            max_burn
        } else if next_value <= I110F18::from_num(min_burn) {
            min_burn
        } else {
            next_value.to_num::<u64>()
        }
    }

    pub fn get_block_at_registration(netuid: u16) -> Vec<u64> {
        let n = N::<T>::get(netuid) as usize;
        let mut block_at_registration: Vec<u64> = vec![0; n];

        for (module_uid, block) in block_at_registration.iter_mut().enumerate() {
            let module_uid = module_uid as u16;

            if Keys::<T>::contains_key(netuid, module_uid) {
                *block = RegistrationBlock::<T>::get(netuid, module_uid);
            }
        }

        block_at_registration
    }

    // --------------------
    // Subnet 0 Utils
    // --------------------

    // TODO:
    // add a flag to enable this, this falg will be on false when the global stake launches,
    // afterwards we do it
    pub(crate) fn deregister_not_whitelisted_modules(mut remaining: Weight) -> Weight {
        use crate::weights::WeightInfo;

        const MAX_MODULES: usize = 5;

        let db_weight = T::DbWeight::get();

        let mut weight = db_weight.reads(2);

        let find_id_weight = db_weight.reads(1);
        let deregister_weight = crate::weights::SubstrateWeight::<T>::deregister();

        if !remaining
            .all_gte(weight.saturating_add(find_id_weight).saturating_add(deregister_weight))
        {
            log::info!("not enough weight remaining: {remaining:?}");
            return Weight::zero();
        }

        let s0_keys: BTreeSet<_> = Keys::<T>::iter_prefix_values(0).collect();
        let whitelisted = T::whitelisted_keys();

        let not_whitelisted = s0_keys.difference(&whitelisted);

        remaining = remaining.saturating_sub(weight);

        for not_whitelisted in not_whitelisted.take(MAX_MODULES) {
            log::info!("deregistering module {not_whitelisted:?}");

            // we'll need at least to read outbound lane state, kill a message and update lane state
            if !remaining.all_gte(find_id_weight.saturating_add(deregister_weight)) {
                log::info!("not enough weight remaining: {remaining:?}");
                break;
            }

            let uid = Uids::<T>::get(0, not_whitelisted);
            weight = weight.saturating_add(find_id_weight);
            remaining = remaining.saturating_sub(find_id_weight);

            if let Some(uid) = uid {
                let Err(err) = with_storage_layer(|| Self::remove_module(0, uid, true)) else {
                    weight = weight.saturating_add(deregister_weight);
                    remaining = remaining.saturating_sub(deregister_weight);
                    continue;
                };

                log::error!("failed to deregister module {uid} due to: {err:?}");
            }
        }

        weight
    }
}
