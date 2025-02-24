use crate::*;
// use frame_support::storage::with_storage_layer;

use frame_support::{pallet_prelude::DispatchResult, sp_runtime::DispatchError};
use frame_system::ensure_signed;
use pallet_emission_api::SubnetConsensus;
use sp_core::Get;
use sp_runtime::BoundedVec;
use substrate_fixed::types::I110F18;

impl<T: Config> Pallet<T> {
    /// Default Rootnetwork subnet id
    const ROOTNET_ID: u16 = 0;
    /// Registers a module in a subnet.
    ///
    /// # Arguments
    ///
    /// * `origin` - The origin of the call, must be a signed account.
    /// * `network_name` - The name of the subnet to register in.
    /// * `name` - The name of the module.
    /// * `address` - The address of the module.
    /// * `stake` - The amount of stake to register with.
    /// * `module_key` - The account ID of the module.
    /// * `metadata` - Optional metadata for the module.
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    ///
    /// * The caller's signature is invalid.
    /// * The maximum number of registrations per block has been reached.
    /// * The caller doesn't have enough balance to register.
    /// * The subnet name is too long when creating a new subnet.
    /// * The maximum number of registrations per interval has been reached.
    /// * The stake is insufficient for registration.
    /// * The module key is already registered.
    /// * The maximum number of modules per network has been reached.
    /// * The root network registration requirements are not met.
    ///
    /// # Effects
    ///
    /// If successful, this function will:
    ///
    /// 1. Create a new subnet if the specified network doesn't exist.
    /// 2. Register the module in the specified subnet.
    /// 3. Add the stake to the module.
    /// 4. Update registration counters.
    /// 5. Emit a `ModuleRegistered` event.
    pub fn do_register(
        origin: T::RuntimeOrigin,
        network_name: Vec<u8>,
        name: Vec<u8>,
        address: Vec<u8>,
        module_key: T::AccountId,
        metadata: Option<Vec<u8>>,
    ) -> DispatchResult {
        let key = ensure_signed(origin.clone())?;

        ensure!(
            RegistrationsPerBlock::<T>::get() < MaxRegistrationsPerBlock::<T>::get(),
            Error::<T>::TooManyRegistrationsPerBlock
        );

        let netuid =
            Self::get_netuid_for_name(&network_name).ok_or(Error::<T>::NetworkDoesNotExist)?;

        Self::validate_registration_request(netuid, &key, &module_key)?;

        Self::reserve_module_slot(netuid, &module_key)?;

        let uid = Self::register_module(netuid, &module_key, name, address, metadata)?;
        Self::finalize_registration(netuid, uid, &module_key)?;

        Ok(())
    }

    pub fn do_register_subnet(
        origin: T::RuntimeOrigin,
        network_name: Vec<u8>,
        network_metadata: Option<Vec<u8>>,
    ) -> DispatchResult {
        let key = ensure_signed(origin.clone())?;

        if Self::get_netuid_for_name(&network_name).is_some() {
            return Err(Error::<T>::SubnetNameAlreadyExists.into());
        }

        let bounded_name: BoundedVec<u8, ConstU32<256>> =
            network_name.to_vec().try_into().map_err(|_| Error::<T>::SubnetNameTooLong)?;

        let network_metadata: Option<BoundedVec<u8, ConstU32<120>>> = match network_metadata {
            Some(slice) => {
                Some(slice.to_vec().try_into().map_err(|_| Error::<T>::SubnetNameTooLong)?)
            }
            None => None,
        };

        let params = SubnetParams {
            name: bounded_name,
            metadata: network_metadata,
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

        Self::add_subnet_from_registration(changeset)
    }

    /// Deregisters a module from the specified subnet.
    ///
    /// # Arguments
    ///
    /// * `origin` - The origin of the call, must be a signed extrinsic.
    /// * `netuid` - The unique identifier of the subnet.
    ///
    /// # Returns
    ///
    /// * `DispatchResult` - Ok if successful, or an error if the operation fails.
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// * The origin is not signed.
    /// * The module does not exist in the specified subnet.
    /// * The module cannot be removed from the subnet.
    /// * The key is still registered after removal attempt.
    ///
    /// # Events
    ///
    /// Emits a `ModuleDeregistered` event when successful.
    pub fn do_deregister(origin: T::RuntimeOrigin, netuid: u16) -> DispatchResult {
        // --- 1. Check that the caller has signed the transaction.
        let key = ensure_signed(origin)?;
        // --- 2. Check that the module exists in the subnet.
        let Some(uid) = Self::get_uid_for_key(netuid, &key) else {
            return Err(Error::<T>::ModuleDoesNotExist.into());
        };
        // --- 3. Remove the module from the subnet.
        Self::remove_module(netuid, uid, true)?;
        ensure!(
            !Self::key_registered(netuid, &key),
            Error::<T>::StillRegistered
        );

        // --- 4. Deposit the event
        Self::deposit_event(Event::ModuleDeregistered(netuid, uid, key));
        // --- 5. Ok and done.
        Ok(())
    }

    // --------------------------
    // Registration Utils
    // --------------------------

    fn validate_registration_request(
        netuid: u16,
        key: &T::AccountId,
        module_key: &T::AccountId,
    ) -> DispatchResult {
        let burn_config = ModuleBurnConfig::<T>::get(netuid);
        ensure!(
            RegistrationsThisInterval::<T>::get(netuid)
                < burn_config.max_registrations_per_interval,
            Error::<T>::TooManyRegistrationsPerInterval
        );

        if MaxAllowedUids::<T>::get(netuid) < 1 {
            return Err(Error::<T>::NetworkIsImmuned.into());
        }

        ensure!(
            !Self::key_registered(netuid, module_key),
            Error::<T>::KeyAlreadyRegistered
        );

        let rootnet_id = T::get_consensus_netuid(SubnetConsensus::Root).unwrap_or(Self::ROOTNET_ID);
        if netuid != rootnet_id {
            let burn =
                Self::u64_to_balance(Burn::<T>::get(netuid)).ok_or(Error::<T>::ArithmeticError)?;
            Self::remove_balance_from_account(key, burn)
                .map_err(|_| Error::<T>::NotEnoughBalanceToRegister)?;
        }

        Ok(())
    }

    fn register_module(
        netuid: u16,
        module_key: &T::AccountId,
        name: Vec<u8>,
        address: Vec<u8>,
        metadata: Option<Vec<u8>>,
    ) -> Result<u16, DispatchError> {
        let fees = DefaultValidatorFees::<T>::get();
        let module_changeset = ModuleChangeset::new(name, address, fees, metadata);
        Self::append_module(netuid, module_key, module_changeset)
    }

    fn finalize_registration(netuid: u16, uid: u16, module_key: &T::AccountId) -> DispatchResult {
        ensure!(
            Self::key_registered(netuid, module_key),
            Error::<T>::ModuleDoesNotExist
        );

        RegistrationsPerBlock::<T>::mutate(|val| *val = val.saturating_add(1));
        RegistrationsThisInterval::<T>::mutate(netuid, |registrations| {
            *registrations = registrations.saturating_add(1);
        });

        Self::deposit_event(Event::ModuleRegistered(netuid, uid, module_key.clone()));

        Ok(())
    }

    /// Determines which peer to prune from the network based on the lowest pruning score.
    ///
    /// # Arguments
    ///
    /// * `netuid` - The unique identifier of the subnet.
    /// * `ignore_immunity` - If true, ignores the immunity period when selecting a peer to prune.
    ///
    /// # Returns
    ///
    /// * `Option<u16>` - The UID of the peer to prune, or None if all peers are in immunity period.
    ///
    /// # Behavior
    ///
    /// 1. Filters peers based on immunity period and minimum stake requirements.
    /// 2. Calculates pruning scores for eligible peers.
    /// 3. Selects the peer with the lowest pruning score, prioritizing: a. Peers with zero
    ///    emission, choosing the oldest. b. If no zero emission peers, selects based on lowest
    ///    score and oldest registration.
    ///
    /// # Note
    ///
    /// When `ignore_immunity` is true (e.g., during global deregistration), the function
    /// disregards the immunity period and considers all peers for pruning.
    pub fn get_lowest_uid(netuid: u16, ignore_immunity: bool) -> Option<u16> {
        let current_block = Self::get_current_block_number();
        let immunity_period = ImmunityPeriod::<T>::get(netuid) as u64;
        let emission_vec = Emission::<T>::get(netuid);
        let dividend_vec = Dividends::<T>::get(netuid);
        let incentive_vec = Incentive::<T>::get(netuid);

        let uids: Vec<_> = RegistrationBlock::<T>::iter_prefix(netuid)
            .filter(move |&(uid, block_at_registration)| {
                if ignore_immunity
                    || current_block.saturating_sub(block_at_registration) >= immunity_period
                {
                    !*ValidatorPermits::<T>::get(netuid).get(uid as usize).unwrap_or(&false)
                } else {
                    false
                }
            })
            .map(|(uid, block_at_registration)| {
                let emission =
                    I110F18::from_num(emission_vec.get(uid as usize).copied().unwrap_or_default());

                let dividend_perc =
                    I110F18::from_num(dividend_vec.get(uid as usize).copied().unwrap_or_default());
                let incentive_perc =
                    I110F18::from_num(incentive_vec.get(uid as usize).copied().unwrap_or_default());

                if dividend_perc == 0 && incentive_perc == 0 {
                    return (uid, I110F18::from_num(0), block_at_registration);
                }

                let dividend = dividend_perc
                    .saturating_div(dividend_perc.saturating_add(incentive_perc))
                    .saturating_mul(emission);
                let incentive = incentive_perc
                    .saturating_div(dividend_perc.saturating_add(incentive_perc))
                    .saturating_mul(emission);

                let pruning_score =
                    (I110F18::from_num(0.3).saturating_mul(dividend)).saturating_add(incentive);
                (uid, pruning_score, block_at_registration)
            })
            .collect();

        // Age is secondary to the emission.
        uids.iter()
            // This is usual scenario, that is why we check for oldest 0 emission to return early
            .filter(|&(_, pruning_score, _)| *pruning_score == 0)
            .min_by_key(|&(_, _, block_at_registration)| block_at_registration)
            .or_else(|| {
                uids.iter().min_by(|&(_, score_a, block_a), &(_, score_b, block_b)| {
                    score_a.cmp(score_b).then_with(|| block_a.cmp(block_b))
                })
            })
            .map(|(uid, _, _)| *uid)
    }

    pub fn add_subnet_from_registration(changeset: SubnetChangeset<T>) -> DispatchResult {
        let num_subnets: u16 = Self::get_total_subnets();
        let max_subnets: u16 = MaxAllowedSubnets::<T>::get();

        // RESERVE SUBNET SLOT
        // if we have not reached the max number of subnets, then we can start a new one
        let target_subnet = if num_subnets >= max_subnets {
            let lowest_emission_netuid = T::get_lowest_emission_netuid(false);
            let netuid = lowest_emission_netuid.ok_or(sp_runtime::DispatchError::Other(
                "No valid netuid to deregister",
            ))?;

            // if the stake is greater than the least staked network, then we can start a new one
            Self::remove_subnet(netuid);
            Some(netuid)
        } else {
            None
        };

        Self::add_subnet(changeset, target_subnet)?;
        Ok(())
    }

    /// Reserves a module slot on the specified subnet.
    ///
    /// This function checks whether there are still available module slots on the network.
    /// If the subnet is filled, it deregisters the least staked module on it.
    /// If the maximum allowed modules on the network is reached, it deregisters the least staked
    /// module on the least staked subnet.
    ///
    /// # Arguments
    ///
    /// * `netuid` - The unique identifier of the subnet.
    ///
    /// # Returns
    ///
    /// * `DispatchResult` - Ok(()) if successful, or an error if the operation fails.
    ///
    /// # Behavior
    ///
    /// 1. If the subnet is minable and at capacity, it replaces the lowest priority node.
    /// 2. If the global module limit is reached, it removes a node from the lowest emission subnet.
    /// 3. Otherwise, it allows the new module to be added.
    pub fn reserve_module_slot(netuid: u16, key: &T::AccountId) -> DispatchResult {
        let mineable = T::is_mineable_subnet(netuid);
        let module_count = || N::<T>::get(netuid);

        if mineable && module_count() >= MaxAllowedUids::<T>::get(netuid) {
            return Self::replace_lowest_priority_node(netuid, false);
        }

        let rootnet_id = T::get_consensus_netuid(SubnetConsensus::Root).unwrap_or(Self::ROOTNET_ID);
        if netuid == rootnet_id {
            Self::reserve_rootnet_slot(rootnet_id, key)?;
        }

        if Self::global_n_modules() >= MaxAllowedModules::<T>::get() {
            Self::remove_from_lowest_emission_subnet()?;
        }

        Ok(())
    }

    fn replace_lowest_priority_node(netuid: u16, ignore_immunity: bool) -> DispatchResult {
        if let Some(uid) = Self::get_lowest_uid(netuid, ignore_immunity) {
            Self::remove_module(netuid, uid, false)
        } else {
            Err(Error::<T>::NetworkIsImmuned.into())
        }
    }

    fn remove_from_lowest_emission_subnet() -> DispatchResult {
        if let Some(subnet_id) = T::get_lowest_emission_netuid(true) {
            if let Some(module_uid) = Self::get_lowest_uid(subnet_id, true) {
                Self::remove_module(subnet_id, module_uid, true)
            } else {
                Err(Error::<T>::NetworkIsImmuned.into())
            }
        } else {
            Err(Error::<T>::NetworkIsImmuned.into())
        }
    }

    // --- Rootnet utils ---

    fn reserve_rootnet_slot(rootnet_id: u16, key: &T::AccountId) -> DispatchResult {
        if Uids::<T>::iter_prefix(rootnet_id).count()
            < MaxAllowedUids::<T>::get(rootnet_id) as usize
        {
            return Ok(());
        }

        let (lower_stake_validator, lower_stake) = Keys::<T>::iter_prefix(rootnet_id)
            .map(|(_, key)| (key.clone(), Self::get_delegated_stake(&key)))
            .min_by_key(|(_, stake)| *stake)
            .ok_or(Error::<T>::ArithmeticError)?;

        let stake = Self::get_delegated_stake(key);
        ensure!(stake >= lower_stake, Error::<T>::NotEnoughStakeToRegister);

        let lower_stake_validator_uid = Self::get_uid_for_key(rootnet_id, &lower_stake_validator)
            .ok_or(
            "selected lowest stake validator does not exist, this is really concerning",
        )?;

        Self::remove_module(rootnet_id, lower_stake_validator_uid, true)
    }

    // --- Registration Burn ---

    // This code is running under the `on_initialize` hook
    pub fn adjust_registration_parameters(block_number: u64) {
        // For subnet prices
        let subnet_config = SubnetBurnConfig::<T>::get();
        let subnet_burn = SubnetBurn::<T>::get();
        Self::adjust_burn_parameters(
            block_number,
            subnet_config.target_registrations_interval,
            SubnetRegistrationsThisInterval::<T>::get(),
            subnet_config.target_registrations_per_interval,
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
            let module_config = ModuleBurnConfig::<T>::get(netuid);
            let module_burn = Burn::<T>::get(netuid);
            Self::adjust_burn_parameters(
                block_number,
                module_config.target_registrations_interval,
                RegistrationsThisInterval::<T>::get(netuid),
                module_config.target_registrations_per_interval,
                module_config.adjustment_alpha,
                module_config.min_burn,
                module_config.max_burn,
                module_burn,
                |adjusted_burn| {
                    Burn::<T>::set(netuid, adjusted_burn);
                    RegistrationsThisInterval::<T>::set(netuid, 0);
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

    // --- Util ---

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

    /// returns the amount of total modules on the network
    pub fn global_n_modules() -> u16 {
        N::<T>::iter().map(|(_, value)| value).sum()
    }

    pub fn clear_rootnet_daily_weight_calls(block: u64) {
        // 10_800 == blocks in a day
        if block.checked_rem(10_800).is_some_and(|r| r == 0) {
            let _ = RootNetWeightCalls::<T>::clear(u32::MAX, None);
        }
    }
}
