use super::*;

use frame_support::storage::IterableStorageDoubleMap;

// import vec
use sp_arithmetic::per_things::Percent;
use sp_std::vec::Vec;

impl<T: Config> Pallet<T> {
    pub fn do_add_stake_multiple(
        origin: T::RuntimeOrigin,
        netuid: u16,
        module_keys: Vec<T::AccountId>,
        amounts: Vec<u64>,
    ) -> dispatch::DispatchResult {
        let key = ensure_signed(origin.clone())?;
        let amounts_sum: u64 = amounts.iter().sum();
        ensure!(
            Self::has_enough_balance(&key, amounts_sum),
            Error::<T>::NotEnoughStaketoWithdraw
        );
        ensure!(
            amounts.len() == module_keys.len(),
            Error::<T>::DifferentLengths
        );

        for (i, m_key) in module_keys.iter().enumerate() {
            Self::do_add_stake(origin.clone(), netuid, m_key.clone(), amounts[i as usize])?;
        }
        Ok(())
    }

    pub fn do_transfer_multiple(
        origin: T::RuntimeOrigin,
        destinations: Vec<T::AccountId>,
        amounts: Vec<u64>,
    ) -> dispatch::DispatchResult {
        let key = ensure_signed(origin.clone())?;
        let amounts_sum: u64 = amounts.iter().sum();
        ensure!(
            Self::has_enough_balance(&key, amounts_sum),
            Error::<T>::NotEnoughBalanceToTransfer
        );
        ensure!(
            amounts.len() == destinations.len(),
            Error::<T>::DifferentLengths
        );

        for (i, m_key) in destinations.iter().enumerate() {
            ensure!(
                Self::has_enough_balance(&key, amounts[i as usize]),
                Error::<T>::NotEnoughBalanceToTransfer
            );
            Self::transfer_balance_to_account(&key, &m_key.clone(), amounts[i as usize]);
        }
        Ok(())
    }

    pub fn do_remove_stake_multiple(
        origin: T::RuntimeOrigin,
        netuid: u16,
        module_keys: Vec<T::AccountId>,
        amounts: Vec<u64>,
    ) -> dispatch::DispatchResult {
        let key = ensure_signed(origin.clone())?;
        ensure!(
            amounts.len() == module_keys.len(),
            Error::<T>::DifferentLengths
        );

        for (i, m_key) in module_keys.iter().enumerate() {
            ensure!(
                Self::has_enough_stake(netuid, &key, &m_key.clone(), amounts[i as usize]),
                Error::<T>::NotEnoughStaketoWithdraw
            );
            Self::do_remove_stake(origin.clone(), netuid, m_key.clone(), amounts[i as usize])?;
        }
        Ok(())
    }

    pub fn do_transfer_stake(
        origin: T::RuntimeOrigin,
        netuid: u16,
        module_key: T::AccountId,
        new_module_key: T::AccountId,
        amount: u64,
    ) -> dispatch::DispatchResult {
        let key = ensure_signed(origin.clone())?;
        ensure!(
            Self::is_registered(netuid, &module_key.clone()),
            Error::<T>::NotRegistered
        );
        ensure!(
            Self::is_registered(netuid, &new_module_key.clone()),
            Error::<T>::NotRegistered
        );
        ensure!(
            Self::has_enough_stake(netuid, &key, &module_key, amount),
            Error::<T>::NotEnoughStaketoWithdraw
        );

        Self::do_remove_stake(origin.clone(), netuid, module_key.clone(), amount)?;
        Self::do_add_stake(origin.clone(), netuid, new_module_key.clone(), amount)?;
        Ok(())
    }

    //
    pub fn do_add_stake(
        origin: T::RuntimeOrigin,
        netuid: u16,
        module_key: T::AccountId,
        amount: u64,
    ) -> dispatch::DispatchResult {
        // --- 1. We check that the transaction is signed by the caller and retrieve the
        // T::AccountId key information.
        let key = ensure_signed(origin)?;

        // --- 1. Ensure we don't exceed tx rate limit
        // ensure!( !Self::exceeds_tx_rate_limit(&key), Error::<T>::TxRateLimitExceeded);

        ensure!(
            Self::is_registered(netuid, &module_key.clone()),
            Error::<T>::NotRegistered
        );

        ensure!(
            Self::has_enough_balance(&key, amount),
            Error::<T>::NotEnoughBalanceToStake
        );

        let stake_before_add: u64 = Self::get_stake_to_module(netuid, &key, &module_key.clone());
        let balance_before_add: u64 = Self::get_balance_u64(&key);
        let module_stake_before_add: u64 = Self::get_stake_for_key(netuid, &module_key);
        let removed_balance: bool =
            Self::remove_balance_from_account(&key, Self::u64_to_balance(amount).unwrap());
        ensure!(removed_balance, Error::<T>::BalanceNotRemoved);
        Self::increase_stake(netuid, &key, &module_key, amount);

        let stake_after_add: u64 = Self::get_stake_to_module(netuid, &key, &module_key.clone());
        let balance_after_add: u64 = Self::get_balance_u64(&key);
        let module_stake_after_add = Self::get_stake_for_key(netuid, &module_key);

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

        // --- 5. Emit the staking event.
        Self::deposit_event(Event::StakeAdded(key, module_key, amount));

        // --- 6. Ok and return.get_total_emissions
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

        ensure!(
            Self::is_registered(netuid, &module_key.clone()),
            Error::<T>::NotRegistered
        );

        // --- 6. Ensure we don't exceed tx rate limit
        // ensure!( !Self::exceeds_tx_rate_limit(&key), Error::<T>::TxRateLimitExceeded );

        // --- 5. Ensure that we can conver this u64 to a balance.
        ensure!(
            Self::has_enough_stake(netuid, &key, &module_key, amount),
            Error::<T>::NotEnoughStaketoWithdraw
        );
        let stake_to_be_added_as_currency = Self::u64_to_balance(amount);
        ensure!(
            stake_to_be_added_as_currency.is_some(),
            Error::<T>::CouldNotConvertToBalance
        );

        let stake_before_remove: u64 = Self::get_stake_to_module(netuid, &key, &module_key.clone());
        let balance_before_remove: u64 = Self::get_balance_u64(&key);
        let module_stake_before_remove: u64 = Self::get_stake_for_key(netuid, &module_key);

        // --- 7. We remove the balance from the key.
        Self::decrease_stake(netuid, &key, &module_key, amount);
        Self::add_balance_to_account(&key, Self::u64_to_balance(amount).unwrap());
        // --- 9. Emit the unstaking event.

        let stake_after_remove: u64 = Self::get_stake_to_module(netuid, &key, &module_key.clone());
        let balance_after_remove: u64 = Self::get_balance_u64(&key);
        let module_stake_after_remove = Self::get_stake_for_key(netuid, &module_key);

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

    // Returns the total amount of stake in the staking table.
    //
    pub fn get_total_subnet_stake(netuid: u16) -> u64 {
        TotalStake::<T>::get(netuid)
    }

    // Returns the total amount of stake in the staking table.
    pub fn total_stake() -> u64 {
        let mut total_stake: u64 = 0;
        for (_netuid, subnet_total_stake) in TotalStake::<T>::iter() {
            total_stake += subnet_total_stake;
        }
        total_stake
    }

    // Returns the stake under the cold - hot pairing in the staking table.
    //
    pub fn get_stake(netuid: u16, key: &T::AccountId) -> u64 {
        Stake::<T>::get(netuid, key)
    }

    pub fn get_stakes(netuid: u16) -> Vec<u64> {
        let _n = Self::get_subnet_n(netuid);
        let mut stakes: Vec<u64> = Vec::new();
        let uid_key_tuples: Vec<(u16, T::AccountId)> = Self::get_uid_key_tuples(netuid);
        for (_uid, key) in uid_key_tuples {
            let stake: u64 = Self::get_stake(netuid, &key);
            stakes.push(stake);
        }
        stakes
    }

    // Returns the stake under the cold - hot pairing in the staking table.
    pub fn key_account_exists(netuid: u16, key: &T::AccountId) -> bool {
        Uids::<T>::contains_key(netuid, &key)
    }

    // Returns the delegation fee of a module
    pub fn get_delegation_fee(netuid: u16, module_key: &T::AccountId) -> Percent {
        DelegationFee::<T>::get(netuid, module_key)
    }

    // Returns true if the cold-hot staking account has enough balance to fufil the amount.
    //
    pub fn has_enough_stake(
        netuid: u16,
        key: &T::AccountId,
        module_key: &T::AccountId,
        amount: u64,
    ) -> bool {
        Self::get_stake_to_module(netuid, key, module_key) >= amount
    }

    pub fn get_self_stake(netuid: u16, key: &T::AccountId) -> u64 {
        Self::get_stake_to_module(netuid, key, key)
    }

    pub fn get_stake_to_total(netuid: u16, key: &T::AccountId) -> u64 {
        let mut total_stake_to: u64 = 0;
        for (_k, v) in Self::get_stake_to_vector(netuid, key) {
            total_stake_to += v;
        }
        total_stake_to
    }

    pub fn get_stake_to_module(netuid: u16, key: &T::AccountId, module_key: &T::AccountId) -> u64 {
        let mut state_to: u64 = 0;
        for (k, v) in Self::get_stake_to_vector(netuid, key) {
            if k == module_key.clone() {
                state_to = v;
            }
        }

        state_to
    }

    pub fn get_stake_to_vector(netuid: u16, key: &T::AccountId) -> Vec<(T::AccountId, u64)> {
        StakeTo::<T>::get(netuid, key)
    }

    pub fn set_stake_to_vector(
        netuid: u16,
        key: &T::AccountId,
        stake_to_vector: Vec<(T::AccountId, u64)>,
    ) {
        // we want to remove any keys that have a stake of 0, as these are from outside the subnet
        // and can bloat the chain
        if stake_to_vector.len() == 0 {
            StakeTo::<T>::remove(netuid, key);
            return;
        }
        StakeTo::<T>::insert(netuid, key, stake_to_vector);
    }

    pub fn set_stake_from_vector(
        netuid: u16,
        module_key: &T::AccountId,
        stake_from_vector: Vec<(T::AccountId, u64)>,
    ) {
        StakeFrom::<T>::insert(netuid, module_key, stake_from_vector);
    }

    pub fn get_stake_from_vector(
        netuid: u16,
        module_key: &T::AccountId,
    ) -> Vec<(T::AccountId, u64)> {
        StakeFrom::<T>::get(netuid, module_key)
            .into_iter()
            .collect::<Vec<(T::AccountId, u64)>>()
    }
    pub fn get_total_stake_from(netuid: u16, module_key: &T::AccountId) -> u64 {
        let stake_from_vector: Vec<(T::AccountId, u64)> =
            Self::get_stake_from_vector(netuid, module_key);
        let mut total_stake_from: u64 = 0;
        for (_k, v) in stake_from_vector {
            total_stake_from += v;
        }
        total_stake_from
    }

    pub fn get_total_stake_to_global(key: &T::AccountId) -> u64 {
        let mut total_stake_to: u64 = 0;
        let total_networks: u16 = TotalSubnets::<T>::get();
        for netuid in 0..total_networks {
            total_stake_to += Self::get_total_stake_to(netuid, key);
        }
        total_stake_to
    }

    pub fn get_total_stake_to(netuid: u16, key: &T::AccountId) -> u64 {
        let stake_to_vector: Vec<(T::AccountId, u64)> = Self::get_stake_to_vector(netuid, key);
        let mut total_stake_to: u64 = 0;
        for (_k, v) in stake_to_vector {
            total_stake_to += v;
        }
        total_stake_to
    }

    // INCREASE

    pub fn increase_stake(
        netuid: u16,
        key: &T::AccountId,
        module_key: &T::AccountId,
        amount: u64,
    ) -> bool {
        let mut stake_from_vector: Vec<(T::AccountId, u64)> =
            Self::get_stake_from_vector(netuid, module_key);
        let mut found_key_in_vector: bool = false;
        for (i, (k, v)) in stake_from_vector.clone().iter().enumerate() {
            if *k == *key {
                stake_from_vector[i] = (k.clone(), (*v).saturating_add(amount));
                found_key_in_vector = true;
            }
        }

        // if we didnt find the key in the vector, we need to add it
        if !found_key_in_vector {
            stake_from_vector.push((key.clone(), amount));
        }

        // reset the stake to vector, as we have updated the stake_to_vector
        let mut found_key_in_vector: bool = false;
        let mut stake_to_vector: Vec<(T::AccountId, u64)> = Self::get_stake_to_vector(netuid, key);

        for (i, (k, v)) in stake_to_vector.clone().iter().enumerate() {
            if *k == *module_key {
                stake_to_vector[i] = (k.clone(), (*v).saturating_add(amount));
                found_key_in_vector = true;
            }
        }

        if !found_key_in_vector {
            stake_to_vector.push((module_key.clone(), amount));
        }

        Self::set_stake_to_vector(netuid, key, stake_to_vector);
        Self::set_stake_from_vector(netuid, module_key, stake_from_vector);

        Stake::<T>::insert(
            netuid,
            module_key,
            Stake::<T>::get(netuid, module_key).saturating_add(amount),
        );
        TotalStake::<T>::insert(netuid, TotalStake::<T>::get(netuid).saturating_add(amount));
        true
    }

    pub fn decrease_stake(
        netuid: u16,
        key: &T::AccountId,
        module_key: &T::AccountId,
        amount: u64,
    ) -> bool {
        // FROM DELEGATE STAKE
        let mut stake_from_vector: Vec<(T::AccountId, u64)> =
            Self::get_stake_from_vector(netuid, module_key).clone();

        let mut idx_to_replace: usize = usize::MAX;

        let mut end_idx: usize = stake_from_vector.len() - 1;
        for (i, (k, stake_amount)) in stake_from_vector.clone().iter().enumerate() {
            if *k == *key {
                let remaining_stake: u64 = (*stake_amount).saturating_sub(amount);
                stake_from_vector[i] = (k.clone(), remaining_stake);
                if remaining_stake == 0 {
                    // we need to remove this entry if its zero
                    idx_to_replace = i;
                }
                break;
            }
        }
        if idx_to_replace != usize::MAX {
            stake_from_vector.swap(idx_to_replace, end_idx);
            stake_from_vector.remove(end_idx);
        }

        Self::set_stake_from_vector(netuid, module_key, stake_from_vector);

        let mut stake_to_vector: Vec<(T::AccountId, u64)> = Self::get_stake_to_vector(netuid, key);
        // TO STAKE
        idx_to_replace = usize::MAX;
        end_idx = stake_to_vector.len() - 1;

        for (i, (k, v)) in stake_to_vector.clone().iter().enumerate() {
            if *k == *module_key {
                let remaining_stake: u64 = (*v).saturating_sub(amount);
                stake_to_vector[i] = (k.clone(), remaining_stake);
                if remaining_stake == 0 {
                    idx_to_replace = i;
                }
                break;
            }
        }

        if idx_to_replace != usize::MAX {
            stake_to_vector.swap(idx_to_replace, end_idx);
            stake_to_vector.remove(end_idx);
        }

        Self::set_stake_to_vector(netuid, key, stake_to_vector);

        // --- 8. We add the balancer to the key.  If the above fails we will not credit this key.
        Stake::<T>::insert(
            netuid,
            module_key,
            Stake::<T>::get(netuid, module_key).saturating_sub(amount),
        );
        TotalStake::<T>::insert(netuid, TotalStake::<T>::get(netuid).saturating_sub(amount));

        true
    }

    // Decreases the stake on the cold - hot pairing by the amount while decreasing other counters.
    //
    pub fn remove_stake_from_storage(netuid: u16, module_key: &T::AccountId) {
        let stake_from_vector: Vec<(T::AccountId, u64)> =
            Self::get_stake_from_vector(netuid, module_key);
        for (_i, (delegate_key, delegate_stake_amount)) in stake_from_vector.iter().enumerate() {
            Self::decrease_stake(netuid, delegate_key, module_key, *delegate_stake_amount);
            Self::add_balance_to_account(
                delegate_key,
                Self::u64_to_balance(*delegate_stake_amount).unwrap(),
            );
        }

        StakeFrom::<T>::remove(netuid, &module_key);
        Stake::<T>::remove(netuid, &module_key);
    }

    pub fn u64_to_balance(
        input: u64,
    ) -> Option<
        <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance,
    > {
        input.try_into().ok()
    }

    pub fn add_balance_to_account_u64(key: &T::AccountId, amount: u64) {
        let _ = T::Currency::deposit_creating(&key, Self::u64_to_balance(amount).unwrap()); // Infallibe
    }
    pub fn add_balance_to_account(
        key: &T::AccountId,
        amount: <<T as Config>::Currency as Currency<<T as system::Config>::AccountId>>::Balance,
    ) {
        let _ = T::Currency::deposit_creating(&key, amount); // Infallibe
    }

    pub fn transfer_balance_to_account(
        from: &T::AccountId,
        to: &T::AccountId,
        amount: u64,
    ) -> bool {
        match T::Currency::transfer(
            &from,
            &to,
            Self::u64_to_balance(amount).unwrap(),
            ExistenceRequirement::KeepAlive,
        ) {
            Ok(_result) => true,
            Err(_error) => false,
        }
    }

    pub fn set_balance_on_account(
        key: &T::AccountId,
        amount: <<T as Config>::Currency as Currency<<T as system::Config>::AccountId>>::Balance,
    ) {
        T::Currency::make_free_balance_be(&key, amount);
    }

    pub fn can_remove_balance_from_account(key: &T::AccountId, amount_64: u64) -> bool {
        let amount_as_balance = Self::u64_to_balance(amount_64);
        if amount_as_balance.is_none() {
            return false;
        }
        let amount = amount_as_balance.unwrap();
        let current_balance = Self::get_balance(key);
        if amount > current_balance {
            return false;
        }
        // This bit is currently untested. @todo
        let new_potential_balance = current_balance - amount;
        let can_withdraw: bool = T::Currency::ensure_can_withdraw(
            &key,
            amount,
            WithdrawReasons::except(WithdrawReasons::TIP),
            new_potential_balance,
        )
        .is_ok();
        can_withdraw
    }

    pub fn get_balance(
        key: &T::AccountId,
    ) -> <<T as Config>::Currency as Currency<<T as system::Config>::AccountId>>::Balance {
        T::Currency::free_balance(&key)
    }

    pub fn balance_to_u64(
        input: <<T as Config>::Currency as Currency<<T as system::Config>::AccountId>>::Balance,
    ) -> u64 {
        input.try_into().ok().unwrap()
    }

    pub fn get_balance_u64(key: &T::AccountId) -> u64 {
        Self::balance_to_u64(Self::get_balance(key))
    }

    pub fn total_balance_for_keys(keys: Vec<T::AccountId>) -> u64 {
        let mut total_balance: u64 = 0;
        for key in keys {
            total_balance += Self::get_balance_u64(&key);
        }
        total_balance
    }

    pub fn has_enough_balance(key: &T::AccountId, amount: u64) -> bool {
        Self::get_balance_u64(key) > amount || amount == 0
    }

    pub fn num_stakedto_keys(netuid: u16, key: &T::AccountId) -> u16 {
        Self::get_stake_to_vector(netuid, key).len() as u16
    }

    pub fn remove_balance_from_account_u64(key: &T::AccountId, amount: u64) -> bool {
        match T::Currency::withdraw(
            &key,
            Self::u64_to_balance(amount).unwrap(),
            WithdrawReasons::except(WithdrawReasons::TIP),
            ExistenceRequirement::KeepAlive,
        ) {
            Ok(_result) => true,
            Err(_error) => false,
        }
    }

    pub fn remove_balance_from_account(
        key: &T::AccountId,
        amount: <<T as Config>::Currency as Currency<<T as system::Config>::AccountId>>::Balance,
    ) -> bool {
        match T::Currency::withdraw(
            &key,
            amount,
            WithdrawReasons::except(WithdrawReasons::TIP),
            ExistenceRequirement::KeepAlive,
        ) {
            Ok(_result) => true,
            Err(_error) => false,
        }
    }

    // get the least staked network
    pub fn least_staked_module_key(netuid: u16) -> T::AccountId {
        let mut min_stake: u64 = u64::MAX;
        let _min_stake_uid: u16 = 0;
        let mut module_key: T::AccountId = Self::get_subnet_founder(netuid);
        for (m_key, m_stake) in
            <Stake<T> as IterableStorageDoubleMap<u16, T::AccountId, u64>>::iter_prefix(netuid)
        {
            if m_stake <= min_stake {
                min_stake = m_stake;
                module_key = m_key;
            }
        }

        module_key
    }

    pub fn least_staked_module_uid(netuid: u16) -> u16 {
        // least_staked_module_uid
        Self::get_uid_for_key(netuid, &Self::least_staked_module_key(netuid))
    }
}
