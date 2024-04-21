use super::*;

use sp_arithmetic::per_things::Percent;
use sp_std::{collections::btree_map::BTreeMap, vec::Vec};

impl<T: Config> Pallet<T> {
    /// Adds stake to multiple modules in a single transaction
    pub fn do_add_stake_multiple(
        origin: T::RuntimeOrigin,
        netuid: u16,
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
        ensure!(amounts.len() > 0, Error::<T>::EmptyKeys);

        // -- 2.2 Make sure they are not above 100
        // the reason for this check at staking is that it has no fee,
        // in transfer multiple, this is not needed, as user pays gass
        ensure!(amounts.len() <= 100, Error::<T>::TooManyKeys);

        // --- 3. Check if the caller has enough balance to stake
        let total_amount: u64 = amounts.iter().sum();
        ensure!(
            Self::has_enough_balance(&key, total_amount),
            Error::<T>::NotEnoughStaketoWithdraw
        );

        // --- 4. Add stake to each module
        for (m_key, amount) in module_keys.iter().zip(amounts.iter()) {
            // do not allow zero amounts in add_stake
            Self::do_add_stake(origin.clone(), netuid, m_key.clone(), *amount)?;
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
            Self::transfer_balance_to_account(&key, m_key, *amount);
        }

        // --- 5. Done and ok
        Ok(())
    }

    pub fn do_remove_stake_multiple(
        origin: T::RuntimeOrigin,
        netuid: u16,
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
        ensure!(amounts.len() > 0, Error::<T>::EmptyKeys);

        // -- 2.2 Make sure they are not above 100
        ensure!(amounts.len() <= 100, Error::<T>::TooManyKeys);

        // --- 3. Remove stake from each module
        for (m_key, amount) in module_keys.iter().zip(amounts.iter()) {
            ensure!(
                Self::has_enough_stake(netuid, &key, m_key, *amount),
                Error::<T>::NotEnoughStaketoWithdraw
            );
            Self::do_remove_stake(origin.clone(), netuid, m_key.clone(), *amount)?;
        }

        // --- 4. Done and ok
        Ok(())
    }

    /// Transfers stake from one module to another
    pub fn do_transfer_stake(
        origin: T::RuntimeOrigin,
        netuid: u16,
        module_key: T::AccountId,
        new_module_key: T::AccountId,
        amount: u64,
    ) -> dispatch::DispatchResult {
        // --- 1. We check that the transaction is signed by the caller and retrieve the
        let key = ensure_signed(origin.clone())?;

        // --- 2. Check if both modules are registered
        // --- 2.1 old module check
        ensure!(
            Self::is_registered(netuid, &module_key),
            Error::<T>::NotRegistered
        );
        // --- 2.2 new module check
        ensure!(
            Self::is_registered(netuid, &new_module_key),
            Error::<T>::NotRegistered
        );

        // --- 3. Check if the caller has enough stake in the old module
        ensure!(
            Self::has_enough_stake(netuid, &key, &module_key, amount),
            Error::<T>::NotEnoughStaketoWithdraw
        );

        // --- 4. Remove stake from the source module and add it to the destination module
        Self::do_remove_stake(origin.clone(), netuid, module_key.clone(), amount)?;
        // don't allow zero stakes
        Self::do_add_stake(origin.clone(), netuid, new_module_key, amount)?;

        // --- 5. Done and ok
        Ok(())
    }

    pub fn do_add_stake(
        origin: T::RuntimeOrigin,
        netuid: u16,
        module_key: T::AccountId,
        amount: u64,
    ) -> dispatch::DispatchResult {
        // --- 1. We check that the transaction is signed by the caller and retrieve the
        // T::AccountId key information.
        let key = ensure_signed(origin)?;

        // --- 2. We check that the module is registered.
        ensure!(
            Self::is_registered(netuid, &module_key.clone()),
            Error::<T>::NotRegistered
        );

        // --- 3. We check that the caller has enough balance to stake.
        ensure!(
            Self::has_enough_balance(&key, amount),
            Error::<T>::NotEnoughBalanceToStake
        );

        // --- 4. Make sure we can convert to balance
        let removed_balance_as_currency = Self::u64_to_balance(amount);
        ensure!(
            removed_balance_as_currency.is_some(),
            Error::<T>::CouldNotConvertToBalance
        );

        // -- 5. Check before values
        let stake_before_add: u64 = Self::get_stake_to_module(netuid, &key, &module_key.clone());
        let balance_before_add: u64 = Self::get_balance_u64(&key);
        let module_stake_before_add: u64 = Self::get_stake_for_key(netuid, &module_key);

        // --- 6. We remove the balance from the key.
        let removed_balance: bool =
            Self::remove_balance_from_account(&key, removed_balance_as_currency.unwrap());

        ensure!(removed_balance, Error::<T>::BalanceCouldNotBeRemoved);

        // --- 7. We add the stake to the module.
        Self::increase_stake(netuid, &key, &module_key, amount);

        // -- 8. Check after values
        let stake_after_add: u64 = Self::get_stake_to_module(netuid, &key, &module_key.clone());
        let balance_after_add: u64 = Self::get_balance_u64(&key);
        let module_stake_after_add = Self::get_stake_for_key(netuid, &module_key);

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
        netuid: u16,
        module_key: T::AccountId,
        amount: u64,
    ) -> dispatch::DispatchResult {
        // --- 1. We check the transaction is signed by the caller and retrieve the T::AccountId key
        // information.
        let key = ensure_signed(origin)?;

        // --- 2. We check that the module is registered.
        ensure!(
            Self::is_registered(netuid, &module_key.clone()),
            Error::<T>::NotRegistered
        );

        // --- 3. We check that the caller has enough stake in the module.
        ensure!(
            Self::has_enough_stake(netuid, &key, &module_key, amount),
            Error::<T>::NotEnoughStaketoWithdraw
        );

        // --- 4. Make sure we can convert to balance
        let stake_to_be_added_as_currency = Self::u64_to_balance(amount);
        ensure!(
            stake_to_be_added_as_currency.is_some(),
            Error::<T>::CouldNotConvertToBalance
        );

        // -- 5. Check before values
        let stake_before_remove: u64 = Self::get_stake_to_module(netuid, &key, &module_key.clone());
        let balance_before_remove: u64 = Self::get_balance_u64(&key);
        let module_stake_before_remove: u64 = Self::get_stake_for_key(netuid, &module_key);

        // --- 6. We remove the balance from the key.
        Self::decrease_stake(netuid, &key, &module_key, amount);

        // --- 7. We add the balancer to the key. If the above fails we will not credit this key.
        Self::add_balance_to_account(&key, Self::u64_to_balance(amount).unwrap());

        // --- 8. Check after values
        let stake_after_remove: u64 = Self::get_stake_to_module(netuid, &key, &module_key.clone());
        let balance_after_remove: u64 = Self::get_balance_u64(&key);
        let module_stake_after_remove = Self::get_stake_for_key(netuid, &module_key);

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

    /// Returns the total amount of stake in the staking table.
    pub fn get_total_subnet_stake(netuid: u16) -> u64 {
        TotalStake::<T>::get(netuid)
    }

    /// Returns the total amount of stake in the staking table.
    pub fn total_stake() -> u64 {
        TotalStake::<T>::iter().map(|(_, stake)| stake).sum()
    }

    /// Returns the stake under the cold - hot pairing in the staking table.
    pub fn get_stake(netuid: u16, key: &T::AccountId) -> u64 {
        Stake::<T>::get(netuid, key)
    }

    #[cfg(debug_assertions)]
    pub fn get_stakes(netuid: u16) -> Vec<u64> {
        Self::get_uid_key_tuples(netuid)
            .into_iter()
            .map(|(_, key)| Self::get_stake(netuid, &key))
            .collect()
    }

    // Returns the delegation fee of a module
    pub fn get_delegation_fee(netuid: u16, module_key: &T::AccountId) -> Percent {
        let min_deleg_fee_global = Self::get_floor_delegation_fee();
        let delegation_fee = DelegationFee::<T>::get(netuid, module_key);

        delegation_fee.max(min_deleg_fee_global)
    }

    pub fn has_enough_stake(
        netuid: u16,
        key: &T::AccountId,
        module_key: &T::AccountId,
        amount: u64,
    ) -> bool {
        amount > 0 && Self::get_stake_to_module(netuid, key, module_key) >= amount
    }

    #[cfg(debug_assertions)]
    pub fn get_self_stake(netuid: u16, key: &T::AccountId) -> u64 {
        Self::get_stake_to_module(netuid, key, key)
    }

    #[cfg(debug_assertions)]
    pub fn get_stake_to_total(netuid: u16, key: &T::AccountId) -> u64 {
        Self::get_stake_to_vector(netuid, key).into_values().sum()
    }

    pub fn get_stake_to_module(netuid: u16, key: &T::AccountId, module_key: &T::AccountId) -> u64 {
        Self::get_stake_to_vector(netuid, key)
            .into_iter()
            .find(|(k, _)| k == module_key)
            .map(|(_, v)| v)
            .unwrap_or(0)
    }

    pub fn get_stake_to_vector(netuid: u16, key: &T::AccountId) -> BTreeMap<T::AccountId, u64> {
        StakeTo::<T>::get(netuid, key)
    }

    pub fn set_stake_to_vector(
        netuid: u16,
        key: &T::AccountId,
        stake_to_vector: BTreeMap<T::AccountId, u64>,
    ) {
        if stake_to_vector.is_empty() {
            StakeTo::<T>::remove(netuid, key);
        } else {
            StakeTo::<T>::insert(netuid, key, stake_to_vector);
        }
    }

    pub fn set_stake_from_vector(
        netuid: u16,
        module_key: &T::AccountId,
        stake_from_vector: BTreeMap<T::AccountId, u64>,
    ) {
        StakeFrom::<T>::insert(netuid, module_key, stake_from_vector);
    }

    pub fn get_stake_from_vector(
        netuid: u16,
        module_key: &T::AccountId,
    ) -> BTreeMap<T::AccountId, u64> {
        StakeFrom::<T>::get(netuid, module_key).into_iter().collect::<BTreeMap<_, _>>()
    }

    pub fn get_total_stake_to(netuid: u16, key: &T::AccountId) -> u64 {
        Self::get_stake_to_vector(netuid, key).into_values().sum()
    }

    pub fn increase_stake(
        netuid: u16,
        key: &T::AccountId,
        module_key: &T::AccountId,
        amount: u64,
    ) -> bool {
        let mut stake_from_vector = Self::get_stake_from_vector(netuid, module_key);
        let found_key_in_vector = stake_from_vector.iter_mut().find(|(k, _)| *k == key);
        if let Some((_, v)) = found_key_in_vector {
            *v = v.saturating_add(amount);
        } else {
            stake_from_vector.insert(key.clone(), amount);
        }

        // reset the stake to vector, as we have updated the stake_to_vector
        let mut stake_to_vector = Self::get_stake_to_vector(netuid, key);
        let found_key_in_vector = stake_to_vector.iter_mut().find(|(k, _)| *k == module_key);
        if let Some((_, v)) = found_key_in_vector {
            *v = v.saturating_add(amount);
        } else {
            stake_to_vector.insert(module_key.clone(), amount);
        }

        Self::set_stake_to_vector(netuid, key, stake_to_vector);
        Self::set_stake_from_vector(netuid, module_key, stake_from_vector);

        Stake::<T>::mutate(netuid, module_key, |stake| {
            *stake = stake.saturating_add(amount)
        });
        TotalStake::<T>::mutate(netuid, |total_stake| {
            *total_stake = total_stake.saturating_add(amount)
        });

        true
    }

    pub fn decrease_stake(
        netuid: u16,
        key: &T::AccountId,
        module_key: &T::AccountId,
        amount: u64,
    ) -> bool {
        // FROM DELEGATE STAKE
        let mut stake_from_vector = Self::get_stake_from_vector(netuid, module_key);
        for (k, v) in stake_from_vector.iter_mut() {
            if k == key {
                let remaining_stake = v.saturating_sub(amount);
                *v = remaining_stake;
                break;
            }
        }
        stake_from_vector.retain(|_, v| *v != 0);
        Self::set_stake_from_vector(netuid, module_key, stake_from_vector);

        let mut stake_to_vector = Self::get_stake_to_vector(netuid, key);
        // TO STAKE
        for (k, v) in stake_to_vector.iter_mut() {
            if k == module_key {
                let remaining_stake = v.saturating_sub(amount);
                *v = remaining_stake;
                break;
            }
        }
        stake_to_vector.retain(|_, v| *v != 0);
        Self::set_stake_to_vector(netuid, key, stake_to_vector);

        // --- 8. We add the balancer to the key. If the above fails we will not credit this key.
        Stake::<T>::mutate(netuid, module_key, |stake| {
            *stake = stake.saturating_sub(amount)
        });
        TotalStake::<T>::mutate(netuid, |total_stake| {
            *total_stake = total_stake.saturating_sub(amount)
        });
        true
    }

    // Decreases the stake by the amount while decreasing other counters.
    pub fn remove_stake_from_storage(netuid: u16, module_key: &T::AccountId) {
        let stake_from_vector = Self::get_stake_from_vector(netuid, module_key);
        for (delegate_key, delegate_stake_amount) in stake_from_vector.iter() {
            Self::decrease_stake(netuid, delegate_key, module_key, *delegate_stake_amount);
            Self::add_balance_to_account(
                delegate_key,
                Self::u64_to_balance(*delegate_stake_amount).unwrap(),
            );
        }

        StakeFrom::<T>::remove(netuid, module_key);
        Stake::<T>::remove(netuid, module_key);
    }

    pub fn add_balance_to_account(
        key: &T::AccountId,
        amount: <<T as Config>::Currency as Currency<<T as system::Config>::AccountId>>::Balance,
    ) {
        let _ = T::Currency::deposit_creating(key, amount); // Infallibe
    }

    pub fn transfer_balance_to_account(
        from: &T::AccountId,
        to: &T::AccountId,
        amount: u64,
    ) -> bool {
        match T::Currency::transfer(
            from,
            to,
            Self::u64_to_balance(amount).unwrap(),
            ExistenceRequirement::KeepAlive,
        ) {
            Ok(_result) => true,
            Err(_error) => false,
        }
    }

    pub fn get_balance(
        key: &T::AccountId,
    ) -> <<T as Config>::Currency as Currency<<T as system::Config>::AccountId>>::Balance {
        T::Currency::free_balance(key)
    }

    pub fn balance_to_u64(
        input: <<T as Config>::Currency as Currency<<T as system::Config>::AccountId>>::Balance,
    ) -> u64 {
        input.try_into().ok().unwrap()
    }

    pub fn u64_to_balance(
        input: u64,
    ) -> Option<
        <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance,
    > {
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
        amount: <<T as Config>::Currency as Currency<<T as system::Config>::AccountId>>::Balance,
    ) -> bool {
        T::Currency::withdraw(
            key,
            amount,
            WithdrawReasons::except(WithdrawReasons::TIP),
            ExistenceRequirement::KeepAlive,
        )
        .is_ok()
    }
}
