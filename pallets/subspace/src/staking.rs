use super::*;

use sp_arithmetic::per_things::Percent;
use sp_runtime::DispatchError;
use sp_std::{collections::btree_map::BTreeMap, vec::Vec};

impl<T: Config> Pallet<T> {
    /// Adds stake to multiple modules in a single transaction
    pub fn do_add_stake_multiple(
        origin: T::RuntimeOrigin,
        module_keys: Vec<T::AccountId>,
        amounts: Vec<u64>,
    ) -> dispatch::DispatchResult {
        // --- 1. We check that the transaction is signed by the caller and retrieve the
        let key = ensure_signed(origin.clone())?;

        // --- 2. Ensure that the lengths of the module_keys and amounts are the same
        ensure!(
            amounts.len() == module_keys.len(),
            Error::<T>::DifferentLengths
        );

        // --- 2.1 make sure that the lengths are not zero
        ensure!(!amounts.is_empty(), Error::<T>::EmptyKeys);

        // -- 2.2 Make sure they are not above 100
        // the reason for this check at staking is that it has no fee,
        // in transfer multiple, this is not needed, as user pays gass
        ensure!(amounts.len() <= 100, Error::<T>::TooManyKeys);

        let minimum_allowed_stake = MinimumAllowedStake::<T>::get();
        ensure!(
            amounts.iter().all(|amount| amount >= &minimum_allowed_stake),
            Error::<T>::StakeTooSmall
        );

        // --- 3. Check if the caller has enough balance to stake
        let total_amount: u64 = amounts.iter().sum();
        ensure!(
            Self::has_enough_balance(&key, total_amount),
            Error::<T>::NotEnoughStakeToWithdraw
        );

        // --- 4. Add stake to each module
        for (m_key, amount) in module_keys.iter().zip(amounts.iter()) {
            // do not allow zero amounts in add_stake
            Self::do_add_stake(origin.clone(), m_key.clone(), *amount)?;
        }

        // --- 5. Done and ok
        Ok(())
    }

    pub fn do_transfer_multiple(
        origin: T::RuntimeOrigin,
        destinations: Vec<T::AccountId>,
        amounts: Vec<u64>,
    ) -> dispatch::DispatchResult {
        // --- 1. We check that the transaction is signed by the caller and retrieve the
        let key = ensure_signed(origin.clone())?;

        // --- 2. Ensure that the lengths of the module_keys and amounts are the same
        ensure!(
            amounts.len() == destinations.len(),
            Error::<T>::DifferentLengths
        );

        // --- 3. Check if the caller has enough balance to transfer
        let total_amount: u64 = amounts.iter().sum();
        ensure!(
            Self::has_enough_balance(&key, total_amount), // do not allow zero stakes.
            Error::<T>::NotEnoughBalanceToTransfer
        );

        // --- 4. Transfer balance to each destination
        for (m_key, amount) in destinations.iter().zip(amounts.iter()) {
            ensure!(
                Self::has_enough_balance(&key, *amount), // do not allow zero stakes.
                Error::<T>::NotEnoughBalanceToTransfer
            );
            Self::transfer_balance_to_account(&key, m_key, *amount)?;
        }

        // --- 5. Done and ok
        Ok(())
    }

    pub fn do_remove_stake_multiple(
        origin: T::RuntimeOrigin,
        module_keys: Vec<T::AccountId>,
        amounts: Vec<u64>,
    ) -> dispatch::DispatchResult {
        // --- 1. We check that the transaction is signed by the caller and retrieve the
        let key = ensure_signed(origin.clone())?;

        // --- 2. Ensure that the lengths of the module_keys and amounts are the same
        ensure!(
            amounts.len() == module_keys.len(),
            Error::<T>::DifferentLengths
        );

        // --- 2.1 make sure that the lengths are not zero
        ensure!(!amounts.is_empty(), Error::<T>::EmptyKeys);

        // -- 2.2 Make sure they are not above 100
        ensure!(amounts.len() <= 100, Error::<T>::TooManyKeys);

        let minimum_allowed_stake = MinimumAllowedStake::<T>::get();
        ensure!(
            amounts.iter().all(|amount| amount >= &minimum_allowed_stake),
            Error::<T>::StakeTooSmall
        );

        // --- 3. Remove stake from each module
        for (m_key, amount) in module_keys.iter().zip(amounts.iter()) {
            ensure!(
                Self::has_enough_stake(&key, m_key, *amount),
                Error::<T>::NotEnoughStakeToWithdraw
            );
            Self::do_remove_stake(origin.clone(), m_key.clone(), *amount)?;
        }

        // --- 4. Done and ok
        Ok(())
    }

    /// Transfers stake from one module to another
    pub fn do_transfer_stake(
        origin: T::RuntimeOrigin,
        module_key: T::AccountId,
        new_module_key: T::AccountId,
        amount: u64,
    ) -> dispatch::DispatchResult {
        // --- 1. We check that the transaction is signed by the caller and retrieve the
        let key = ensure_signed(origin.clone())?;

        // --- 2. Check if both modules are registered
        // --- 2.1 old module check
        ensure!(
            Self::is_registered(None, &module_key),
            Error::<T>::ModuleDoesNotExist
        );
        // --- 2.2 new module check
        ensure!(
            Self::is_registered(None, &new_module_key),
            Error::<T>::ModuleDoesNotExist
        );

        ensure!(
            amount >= MinimumAllowedStake::<T>::get(),
            Error::<T>::StakeTooSmall
        );

        // --- 3. Check if the caller has enough stake in the old module
        ensure!(
            Self::has_enough_stake(&key, &module_key, amount),
            Error::<T>::NotEnoughStakeToWithdraw
        );

        // --- 4. Remove stake from the source module and add it to the destination module
        Self::do_remove_stake(origin.clone(), module_key.clone(), amount)?;
        // don't allow zero stakes
        Self::do_add_stake(origin.clone(), new_module_key, amount)?;

        // --- 5. Done and ok
        Ok(())
    }

    pub fn do_add_stake(
        origin: T::RuntimeOrigin,
        module_key: T::AccountId,
        amount: u64,
    ) -> dispatch::DispatchResult {
        // --- 1. We check that the transaction is signed by the caller and retrieve the
        // T::AccountId key information.
        let key = ensure_signed(origin)?;

        // --- 2. We check that the caller has enough balance to stake.
        ensure!(
            Self::has_enough_balance(&key, amount),
            Error::<T>::NotEnoughBalanceToStake
        );

        // --- 2.1 check the size is enough
        ensure!(
            amount >= MinimumAllowedStake::<T>::get(),
            Error::<T>::StakeTooSmall
        );

        // --- 3. We check that the module is registered.
        ensure!(
            Self::is_registered(None, &module_key.clone()),
            Error::<T>::ModuleDoesNotExist
        );

        // --- 4. Make sure we can convert to balance
        let removed_balance_as_currency = Self::u64_to_balance(amount);
        ensure!(
            removed_balance_as_currency.is_some(),
            Error::<T>::CouldNotConvertToBalance
        );

        // -- 5. Check before values
        let stake_before_add: u64 = Self::get_stake_to_module(&key, &module_key.clone());
        let balance_before_add: u64 = Self::get_balance_u64(&key);
        let module_stake_before_add: u64 = Self::get_delegated_stake(&module_key);

        // --- 6. We remove the balance from the key.
        Self::remove_balance_from_account(&key, removed_balance_as_currency.unwrap())?;

        // --- 7. We add the stake to the module.
        Self::increase_stake(&key, &module_key, amount);

        // -- 8. Check after values
        let stake_after_add: u64 = Self::get_stake_to_module(&key, &module_key.clone());
        let balance_after_add: u64 = Self::get_balance_u64(&key);
        let module_stake_after_add = Self::get_delegated_stake(&module_key);

        // -- 9. Make sure everything went as expected.
        // Otherwise these ensurers will revert the storage changes.
        ensure!(
            stake_after_add == stake_before_add.saturating_add(amount),
            Error::<T>::StakeNotAdded
        );
        ensure!(
            balance_after_add == balance_before_add.saturating_sub(amount),
            Error::<T>::BalanceNotRemoved
        );
        ensure!(
            module_stake_after_add == module_stake_before_add.saturating_add(amount),
            Error::<T>::StakeNotAdded
        );

        Self::deposit_event(Event::StakeAdded(key, module_key, amount));

        // --- 10. Done and ok.
        Ok(())
    }

    pub fn do_remove_stake(
        origin: T::RuntimeOrigin,
        module_key: T::AccountId,
        amount: u64,
    ) -> dispatch::DispatchResult {
        // --- 1. We check the transaction is signed by the caller and retrieve the T::AccountId key
        // information.
        let key = ensure_signed(origin)?;

        // --- 2. We check that the caller has enough stake in the module.
        ensure!(
            Self::has_enough_stake(&key, &module_key, amount),
            Error::<T>::NotEnoughStakeToWithdraw
        );

        // --- 2.1 check the size is enough
        ensure!(
            amount >= MinimumAllowedStake::<T>::get(),
            Error::<T>::StakeTooSmall
        );

        // --- 3. We check that the module is registered.
        ensure!(
            Self::is_registered(None, &module_key.clone()),
            Error::<T>::ModuleDoesNotExist
        );

        // --- 4. Make sure we can convert to balance
        let stake_to_be_added_as_currency = Self::u64_to_balance(amount);
        ensure!(
            stake_to_be_added_as_currency.is_some(),
            Error::<T>::CouldNotConvertToBalance
        );

        // -- 5. Check before values
        let stake_before_remove: u64 = Self::get_stake_to_module(&key, &module_key.clone());
        let balance_before_remove: u64 = Self::get_balance_u64(&key);
        let module_stake_before_remove: u64 = Self::get_delegated_stake(&module_key);

        // --- 6. We remove the balance from the key.
        Self::decrease_stake(&key, &module_key, amount);

        // --- 7. We add the balancer to the key. If the above fails we will not credit this key.
        Self::add_balance_to_account(&key, Self::u64_to_balance(amount).unwrap());

        // --- 8. Check after values
        let stake_after_remove: u64 = Self::get_stake_to_module(&key, &module_key.clone());
        let balance_after_remove: u64 = Self::get_balance_u64(&key);
        let module_stake_after_remove = Self::get_delegated_stake(&module_key);

        // -- 9. Make sure everything went as expected.
        // Otherwise these ensurers will revert the storage changes.
        ensure!(
            stake_after_remove == stake_before_remove.saturating_sub(amount),
            Error::<T>::StakeNotRemoved
        );
        ensure!(
            balance_after_remove == balance_before_remove.saturating_add(amount),
            Error::<T>::BalanceNotAdded
        );
        ensure!(
            module_stake_after_remove == module_stake_before_remove.saturating_sub(amount),
            Error::<T>::StakeNotRemoved
        );

        Self::deposit_event(Event::StakeRemoved(key, module_key, amount));

        // --- 10. Done and ok.
        Ok(())
    }

    pub fn get_total_subnet_stake(netuid: u16) -> u64 {
        Keys::<T>::iter_prefix(netuid)
            .map(|(_, account_id)| Self::get_delegated_stake(&account_id))
            .sum()
    }

    // Returns the delegation fee of a module
    pub fn get_delegation_fee(netuid: u16, module_key: &T::AccountId) -> Percent {
        let min_deleg_fee_global = FloorDelegationFee::<T>::get();
        let delegation_fee = DelegationFee::<T>::get(netuid, module_key);

        delegation_fee.max(min_deleg_fee_global)
    }

    pub fn has_enough_stake(key: &T::AccountId, module_key: &T::AccountId, amount: u64) -> bool {
        amount > 0 && Self::get_stake_to_module(key, module_key) >= amount
    }

    pub fn get_stake_to_module(key: &T::AccountId, module_key: &T::AccountId) -> u64 {
        Self::get_stake_to_vector(key)
            .into_iter()
            .find(|(k, _)| k == module_key)
            .map(|(_, v)| v)
            .unwrap_or_default()
    }

    pub fn get_stake_to_vector(staker: &T::AccountId) -> BTreeMap<T::AccountId, u64> {
        StakeTo::<T>::iter_prefix(staker).collect()
    }

    pub fn set_stake_to_vector(
        staker: &T::AccountId,
        stake_to_vector: BTreeMap<T::AccountId, u64>,
    ) {
        StakeTo::<T>::remove_prefix(staker, None);
        for (k, v) in stake_to_vector.iter() {
            StakeTo::<T>::insert(staker, k, v);
        }
    }

    pub fn set_stake_from_vector(
        staked: &T::AccountId,
        stake_from_vector: BTreeMap<T::AccountId, u64>,
    ) {
        StakeFrom::<T>::remove_prefix(staked, None);
        for (k, v) in stake_from_vector.iter() {
            StakeFrom::<T>::insert(staked, k, v);
        }
    }

    pub fn get_stake_from_vector(staked: &T::AccountId) -> BTreeMap<T::AccountId, u64> {
        StakeFrom::<T>::iter_prefix(staked).collect::<BTreeMap<_, _>>()
    }

    pub fn get_total_stake_from(staked: &T::AccountId) -> u64 {
        Self::get_stake_from_vector(staked).into_values().sum()
    }

    pub fn increase_stake(staker: &T::AccountId, staked: &T::AccountId, amount: u64) -> bool {
        StakeFrom::<T>::mutate(staked, staker, |stake| {
            *stake = stake.saturating_add(amount);
        });

        StakeTo::<T>::mutate(staker, staked, |stake| {
            *stake = stake.saturating_add(amount);
        });

        TotalStake::<T>::mutate(|total_stake| *total_stake = total_stake.saturating_add(amount));

        true
    }

    pub fn decrease_stake(staker: &T::AccountId, staked: &T::AccountId, amount: u64) {
        let stake_from = StakeFrom::<T>::mutate(staked, staker, |stake| {
            *stake = stake.saturating_sub(amount);
            *stake
        });
        if stake_from == 0 {
            StakeFrom::<T>::remove(staked, staker);
        }

        let stake_to = StakeTo::<T>::mutate(staker, staked, |stake| {
            *stake = stake.saturating_sub(amount);
            *stake
        });
        if stake_to == 0 {
            StakeTo::<T>::remove(staker, staked);
        }

        // Stake::<T>::mutate(staked, |stake| *stake = stake.saturating_sub(amount));
        TotalStake::<T>::mutate(|total_stake| *total_stake = total_stake.saturating_sub(amount));
    }

    // Decreases the stake by the amount while decreasing other counters.
    pub fn remove_stake_from_storage(staked: &T::AccountId) {
        // Get all stakers for this staked account
        let stakers: Vec<T::AccountId> =
            StakeFrom::<T>::iter_prefix(staked).map(|(staker, _)| staker).collect();

        for staker in stakers {
            let delegate_stake_amount = StakeFrom::<T>::take(staked, &staker);
            Self::decrease_stake(&staker, staked, delegate_stake_amount);
            Self::add_balance_to_account(
                &staker,
                Self::u64_to_balance(delegate_stake_amount).unwrap(),
            );
        }

        // Stake::<T>::remove(staked);
    }

    pub fn add_balance_to_account(key: &T::AccountId, amount: BalanceOf<T>) {
        let _ = T::Currency::deposit_creating(key, amount); // Infallibe
    }

    pub fn transfer_balance_to_account(
        from: &T::AccountId,
        to: &T::AccountId,
        amount: u64,
    ) -> Result<(), DispatchError> {
        T::Currency::transfer(
            from,
            to,
            Self::u64_to_balance(amount).unwrap(),
            ExistenceRequirement::KeepAlive,
        )
        .map_err(|_| Error::<T>::NotEnoughBalanceToTransfer)?;

        Ok(())
    }

    pub fn get_balance(key: &T::AccountId) -> BalanceOf<T> {
        T::Currency::free_balance(key)
    }

    pub fn balance_to_u64(input: BalanceOf<T>) -> u64 {
        input.try_into().ok().unwrap()
    }

    pub fn u64_to_balance(input: u64) -> Option<BalanceOf<T>> {
        input.try_into().ok()
    }

    pub fn get_balance_u64(key: &T::AccountId) -> u64 {
        Self::balance_to_u64(Self::get_balance(key))
    }

    pub fn has_enough_balance(key: &T::AccountId, amount: u64) -> bool {
        if amount == 0 {
            false
        } else {
            Self::get_balance_u64(key) >= amount
        }
    }

    pub fn remove_balance_from_account(
        key: &T::AccountId,
        amount: BalanceOf<T>,
    ) -> Result<(), DispatchError> {
        let _ = T::Currency::withdraw(
            key,
            amount,
            WithdrawReasons::except(WithdrawReasons::TIP),
            ExistenceRequirement::KeepAlive,
        )
        .map_err(|_| Error::<T>::BalanceCouldNotBeRemoved)?;

        Ok(())
    }
}
