use super::*;

impl<T: Config> Pallet<T> {
    /***********************************************************
     * do_add_stake() - main function called from parent module
     ***********************************************************/

    pub fn do_add_stake(origin: T::Origin, stake_to_be_added: u64) -> dispatch::DispatchResult
    {
        // ---- We check the transaction is signed by the caller
        // and retrieve the T::AccountId pubkey information.
        let key = ensure_signed(origin)?;
        //debug(&("--- Called add_stake with key id {:?}, key {:?} and amount_staked {:?}", key, key, stake_to_be_added));

        // Check if the key is active
        ensure!(Self::is_key_active(&key), Error::<T>::NotRegistered);
        let module = Self::get_module_for_key(&key);

        // Check if uid is active
        ensure!(Self::is_uid_active(module.uid), Error::<T>::NotRegistered);

        // ---- We check that the ModuleMetadata is linked to the calling
        // key, otherwise throw a NonAssociatedkey error.
        ensure!(Self::module_belongs_to_key(&module, &key), Error::<T>::NonAssociatedKey);

        // ---- We check that the calling key contains enough funds to
        // create the staking transaction.
        let stake_as_balance = Self::u64_to_balance(stake_to_be_added);
        ensure!(stake_as_balance.is_some(), Error::<T>::CouldNotConvertToBalance);

        ensure!(Self::can_remove_balance_from_key_account(&key, stake_as_balance.unwrap()), Error::<T>::NotEnoughBalanceToStake);
        ensure!(Self::remove_balance_from_key_account(&key, stake_as_balance.unwrap()) == true, Error::<T>::BalanceWithdrawalError);
        Self::add_stake_to_module(module.uid, stake_to_be_added);

        // ---- Emit the staking event.
        Self::deposit_event(Event::StakeAdded(key, stake_to_be_added));

        // --- ok and return.
        Ok(())
    }

    /// This function removes stake from a key account and puts into a key account.
    /// This function should be called through an extrinsic signed with the keypair's private
    /// key. It takes a key account id and an ammount as parameters.
    ///
    /// Generally, this function works as follows
    /// 1) A Check is performed to see if the key is active (ie, the node using the key is subscribed)
    /// 2) The module metadata associated with the key is retrieved, and is checked if it is subscribed with the supplied cold key
    /// 3) If these checks pass, inflation is emitted to the nodes' peers
    /// 4) If the account has enough stake, the requested amount it transferred to the key account
    /// 5) The total amount of stake is reduced after transfer is complete
    ///
    /// It throws the following errors if there is something wrong
    /// - NotRegistered : The suplied key is not in use. This ususally means a node that uses this key has not subscribed yet, or has unsubscribed
    /// - NonAssociatedkey : The supplied key account id is not subscribed using the supplied cold key
    /// - NotEnoughStaketoWithdraw : The ammount of stake available in the key account is lower than the requested amount
    /// - CouldNotConvertToBalance : A conversion error occured while converting stake from u64 to Balance
    ///
    pub fn do_remove_stake(origin: T::Origin , stake_to_be_removed: u64) -> dispatch::DispatchResult {

        // ---- We check the transaction is signed by the caller
        // and retrieve the T::AccountId pubkey information.
        let key = ensure_signed(origin)?;

        // ---- We query the Module set for the ModuleMetadata stored under
        // the passed key.
        ensure!(Self::is_key_active(&key), Error::<T>::NotRegistered);
        let module = Self::get_module_for_key(&key);

        // Check if uid is active
        ensure!(Self::is_uid_active(module.uid), Error::<T>::NotRegistered);

        // ---- We check that the key has enough stake to withdraw
        // and then withdraw from the account.
        ensure!(Self::has_enough_stake(&module, stake_to_be_removed), Error::<T>::NotEnoughStaketoWithdraw);
        let stake_to_be_added_as_currency = Self::u64_to_balance(stake_to_be_removed);
        ensure!(stake_to_be_added_as_currency.is_some(), Error::<T>::CouldNotConvertToBalance);

        // --- We perform the withdrawl by converting the stake to a u64 balance
        // and deposit the balance into the key account. If the key account
        // does not exist it is created.
        Self::add_balance_to_key_account(&key, stake_to_be_added_as_currency.unwrap());
        Self::remove_stake_from_module(module.uid, stake_to_be_removed);

        // ---- Emit the unstaking event.
        Self::deposit_event(Event::StakeRemoved(key, stake_to_be_removed));

        // --- Done and ok.
        Ok(())
    }


    /********************************
    --==[[  Helper functions   ]]==--
    *********************************/

    pub fn get_stake_of_module_key_account_by_uid(uid: u32) -> u64 {
        return Self::get_module_for_uid(uid).stake
    }

    /// Increases the amount of stake of the entire stake pool by the supplied amount
    ///
    pub fn increase_total_stake(increment: u64) {
        // --- We update the total staking pool with the new funds.
        let total_stake: u64 = TotalStake::<T>::get();

        // Sanity check
        debug_assert!(increment <= u64::MAX.saturating_sub(total_stake));

        TotalStake::<T>::put(total_stake.saturating_add(increment));
    }

    /// Reduces the amount of stake of the entire stake pool by the supplied amount
    ///
    pub fn decrease_total_stake(decrement: u64) {
        // --- We update the total staking pool with the removed funds.
        let total_stake: u64 = TotalStake::<T>::get();

        // Sanity check so that total stake does not underflow past 0
        debug_assert!(decrement <= total_stake);

        TotalStake::<T>::put(total_stake.saturating_sub(decrement));
    }

    /// Increases the amount of stake in a module's key account by the amount provided
    /// The uid parameter identifies the module holding the key account
    ///
    /// Calling function should make sure the uid exists within the system
    /// This function should always increase the total stake, so the operation
    /// of inserting new stake for a module and the increment of the total stake is
    /// atomic. This is important because at some point the fraction of stake/total stake
    /// is calculated and this should always <= 1. Having this function be atomic, fills this
    /// requirement.
    ///
    pub fn add_stake_to_module(uid: u32, amount: u64) {
        debug_assert!(Self::is_uid_active(uid));

        let mut module: ModuleMetadataOf<T> = Self::get_module_for_uid( uid );
        let prev_stake: u64 = module.stake;

        // This should never happen. If a user has this ridiculous amount of stake,
        // we need to come up with a better solution
        debug_assert!(u64::MAX.saturating_sub(amount) > prev_stake);

        let new_stake = prev_stake.saturating_add(amount);
        module.stake = new_stake;
        Modules::<T>::insert(uid, module);

        Self::increase_total_stake(amount);
    }

    /// Decreases the amount of stake in a module's key account by the amount provided
    /// The uid parameter identifies the module holding the key account.
    /// When using this function, it is important to also increase another account by the same value,
    /// as otherwise value gets lost.
    ///
    /// A check if there is enough stake in the key account should have been performed
    /// before this function is called. If not, the node will crap out.
    ///
    /// Furthermore, a check to see if the uid is active before this method is called is also required
    ///
    pub fn remove_stake_from_module(uid: u32, amount: u64) {
        debug_assert!(Self::is_uid_active(uid));

        let mut module: ModuleMetadataOf<T> = Self::get_module_for_uid( uid );
        let stake: u64 = module.stake;

        // By this point, there should be enough stake in the key account for this to work.
        debug_assert!(stake >= amount);
        module.stake = module.stake.saturating_sub(amount);

        Modules::<T>::insert(uid, module);
        Self::decrease_total_stake(amount);
    }

    /// This adds stake (balance) to a cold key account. It takes the account id of the key account and a Balance as parameters.
    /// The Balance parameter is a from u64 converted number. This is needed for T::Currency to work.
    /// Make sure stake is removed from another account before calling this method, otherwise you'll end up with double the value
    ///
    pub fn add_balance_to_key_account(key: &T::AccountId, amount: <<T as Config>::Currency as Currency<<T as system::Config>::AccountId>>::Balance) {
        T::Currency::deposit_creating(&key, amount); // Infallibe
    }

    /// This removes stake from the key. This should be used together with the function to store the stake
    /// in the hot key account.
    /// The internal mechanics can fail. When this happens, this function returns false, otherwise true
    /// The output of this function MUST be checked before writing the amount to the key account
    ///
    ///
    pub fn remove_balance_from_key_account(key: &T::AccountId, amount: <<T as Config>::Currency as Currency<<T as system::Config>::AccountId>>::Balance) -> bool {
        return match T::Currency::withdraw(&key, amount, WithdrawReasons::except(WithdrawReasons::TIP), ExistenceRequirement::KeepAlive) {
            Ok(_result) => {
                true
            }
            Err(_error) => {
                false
            }
        };
    }

    /// Checks if the module as specified in the module parameter has subscribed with the cold key
    /// as specified in the key parameter. See fn subscribe() for more info.
    ///
    pub fn module_belongs_to_key(module: &ModuleMetadataOf<T>, key: &T::AccountId) -> bool {
        return module.key == *key;
    }

    /// Checks if the key account has enough balance to be able to withdraw the specified amount.
    ///
    pub fn can_remove_balance_from_key_account(key: &T::AccountId, amount: <<T as Config>::Currency as Currency<<T as system::Config>::AccountId>>::Balance) -> bool {
        let current_balance = Self::get_key_balance(key);
        if amount > current_balance {
            return false;
        }

        // This bit is currently untested. @todo
        let new_potential_balance = current_balance - amount;
        let can_withdraw = T::Currency::ensure_can_withdraw(&key, amount, WithdrawReasons::except(WithdrawReasons::TIP), new_potential_balance).is_ok();
        can_withdraw
    }

    /// Returns the current balance in the cold key account
    ///
    pub fn get_balance(key: &T::AccountId) -> <<T as Config>::Currency as Currency<<T as system::Config>::AccountId>>::Balance {
        return T::Currency::free_balance(&key);
    }
    /// Returns the current balance in the cold key account
    ///
    pub fn get_key_balance(key: &T::AccountId) -> <<T as Config>::Currency as Currency<<T as system::Config>::AccountId>>::Balance {
        return T::Currency::free_balance(&key);
    }

    /// Checks if the key account of the specified account has enough stake to be able to withdraw
    /// the requested amount.
    ///
    pub fn has_enough_stake(module: &ModuleMetadataOf<T>, amount: u64) -> bool {
        return module.stake >= amount;
    }

    /// Returns true if there is an entry for uid in the Stake map,
    /// false otherwise
    ///
    pub fn has_key_account(uid: &u32) -> bool {
        return Modules::<T>::contains_key(*uid);
    }

    /// This calculates the fraction of the total amount of stake the specfied module owns.
    /// This function is part of the algorithm that calculates the emission of this modules
    /// to its peers. See fn calculate_emission_for_module()
    ///
    /// This function returns 0 if the total amount of stake is 0, or the amount of stake the
    /// module has is 0.
    ///
    /// Otherwise, it returns the result of module_stake / total stake
    ///
    pub fn calculate_stake_fraction_for_module(module: &ModuleMetadataOf<T>) -> U64F64 {
        let total_stake = U64F64::from_num(TotalStake::<T>::get());
        let module_stake = U64F64::from_num(module.stake);

        // Total stake is 0, this should virtually never happen, but is still here because it could
        if total_stake == U64F64::from_num(0) {
            return U64F64::from_num(0);
        }

        // Module stake is zero. This means there will be nothing to emit
        if module_stake == U64F64::from_num(0) {
            return U64F64::from_num(0);
        }

        let stake_fraction = module_stake / total_stake;

        return stake_fraction;
    }

    /// Calculates the proportion of the stake a module has to the total stake.
    /// As such, the result of this function should ALWAYS be a number between
    /// 0 and 1 (inclusive).
    pub fn calulate_stake_fraction(stake: u64, total_stake: u64) -> U64F64 {
        return U64F64::from_num(stake) / U64F64::from_num(total_stake);
    }
}

