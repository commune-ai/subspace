use super::*;

impl<T: Config> Pallet<T> {
    /***********************************************************
     * do_add_stake() - main function called from parent module
     ***********************************************************/

    pub fn do_add_stake(origin: T::Origin, hotkey: T::AccountId, stake_to_be_added: u64) -> dispatch::DispatchResult
    {
        // ---- We check the transaction is signed by the caller
        // and retrieve the T::AccountId pubkey information.
        let coldkey = ensure_signed(origin)?;
        //debug(&("--- Called add_stake with coldkey id {:?}, hotkey {:?} and amount_staked {:?}", coldkey, hotkey, stake_to_be_added));

        // Check if the hotkey is active
        ensure!(Self::is_hotkey_active(&hotkey), Error::<T>::NotRegistered);
        let neuron = Self::get_neuron_for_hotkey(&hotkey);

        // Check if uid is active
        ensure!(Self::is_uid_active(neuron.uid), Error::<T>::NotRegistered);

        // ---- We check that the NeuronMetadata is linked to the calling
        // cold key, otherwise throw a NonAssociatedColdKey error.
        ensure!(Self::neuron_belongs_to_coldkey(&neuron, &coldkey), Error::<T>::NonAssociatedColdKey);

        // ---- We check that the calling coldkey contains enough funds to
        // create the staking transaction.
        let stake_as_balance = Self::u64_to_balance(stake_to_be_added);
        ensure!(stake_as_balance.is_some(), Error::<T>::CouldNotConvertToBalance);

        ensure!(Self::can_remove_balance_from_coldkey_account(&coldkey, stake_as_balance.unwrap()), Error::<T>::NotEnoughBalanceToStake);
        ensure!(Self::remove_balance_from_coldkey_account(&coldkey, stake_as_balance.unwrap()) == true, Error::<T>::BalanceWithdrawalError);
        Self::add_stake_to_neuron_hotkey_account(neuron.uid, stake_to_be_added);

        // ---- Emit the staking event.
        Self::deposit_event(Event::StakeAdded(hotkey, stake_to_be_added));

        // --- ok and return.
        Ok(())
    }

    /// This function removes stake from a hotkey account and puts into a coldkey account.
    /// This function should be called through an extrinsic signed with the coldkeypair's private
    /// key. It takes a hotkey account id and an ammount as parameters.
    ///
    /// Generally, this function works as follows
    /// 1) A Check is performed to see if the hotkey is active (ie, the node using the key is subscribed)
    /// 2) The neuron metadata associated with the hotkey is retrieved, and is checked if it is subscribed with the supplied cold key
    /// 3) If these checks pass, inflation is emitted to the nodes' peers
    /// 4) If the account has enough stake, the requested amount it transferred to the coldkey account
    /// 5) The total amount of stake is reduced after transfer is complete
    ///
    /// It throws the following errors if there is something wrong
    /// - NotRegistered : The suplied hotkey is not in use. This ususally means a node that uses this key has not subscribed yet, or has unsubscribed
    /// - NonAssociatedColdKey : The supplied hotkey account id is not subscribed using the supplied cold key
    /// - NotEnoughStaketoWithdraw : The ammount of stake available in the hotkey account is lower than the requested amount
    /// - CouldNotConvertToBalance : A conversion error occured while converting stake from u64 to Balance
    ///
    pub fn do_remove_stake(origin: T::Origin, hotkey: T::AccountId, stake_to_be_removed: u64) -> dispatch::DispatchResult {

        // ---- We check the transaction is signed by the caller
        // and retrieve the T::AccountId pubkey information.
        let coldkey = ensure_signed(origin)?;

        // ---- We query the Neuron set for the NeuronMetadata stored under
        // the passed hotkey.
        ensure!(Self::is_hotkey_active(&hotkey), Error::<T>::NotRegistered);
        let neuron = Self::get_neuron_for_hotkey(&hotkey);

        // Check if uid is active
        ensure!(Self::is_uid_active(neuron.uid), Error::<T>::NotRegistered);

        // ---- We check that the NeuronMetadata is linked to the calling
        // cold key, otherwise throw a NonAssociatedColdKey error.
        ensure!(Self::neuron_belongs_to_coldkey(&neuron, &coldkey), Error::<T>::NonAssociatedColdKey);

        // ---- We check that the hotkey has enough stake to withdraw
        // and then withdraw from the account.
        ensure!(Self::has_enough_stake(&neuron, stake_to_be_removed), Error::<T>::NotEnoughStaketoWithdraw);
        let stake_to_be_added_as_currency = Self::u64_to_balance(stake_to_be_removed);
        ensure!(stake_to_be_added_as_currency.is_some(), Error::<T>::CouldNotConvertToBalance);

        // --- We perform the withdrawl by converting the stake to a u64 balance
        // and deposit the balance into the coldkey account. If the coldkey account
        // does not exist it is created.
        Self::add_balance_to_coldkey_account(&coldkey, stake_to_be_added_as_currency.unwrap());
        Self::remove_stake_from_neuron_hotkey_account(neuron.uid, stake_to_be_removed);

        // ---- Emit the unstaking event.
        Self::deposit_event(Event::StakeRemoved(hotkey, stake_to_be_removed));

        // --- Done and ok.
        Ok(())
    }


    /********************************
    --==[[  Helper functions   ]]==--
    *********************************/

    pub fn get_stake_of_neuron_hotkey_account_by_uid(uid: u32) -> u64 {
        return Self::get_neuron_for_uid(uid).stake
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

    /// Increases the amount of stake in a neuron's hotkey account by the amount provided
    /// The uid parameter identifies the neuron holding the hotkey account
    ///
    /// Calling function should make sure the uid exists within the system
    /// This function should always increase the total stake, so the operation
    /// of inserting new stake for a neuron and the increment of the total stake is
    /// atomic. This is important because at some point the fraction of stake/total stake
    /// is calculated and this should always <= 1. Having this function be atomic, fills this
    /// requirement.
    ///
    pub fn add_stake_to_neuron_hotkey_account(uid: u32, amount: u64) {
        debug_assert!(Self::is_uid_active(uid));

        let mut neuron: NeuronMetadataOf<T> = Self::get_neuron_for_uid( uid );
        let prev_stake: u64 = neuron.stake;

        // This should never happen. If a user has this ridiculous amount of stake,
        // we need to come up with a better solution
        debug_assert!(u64::MAX.saturating_sub(amount) > prev_stake);

        let new_stake = prev_stake.saturating_add(amount);
        neuron.stake = new_stake;
        Neurons::<T>::insert(uid, neuron);

        Self::increase_total_stake(amount);
    }

    /// Decreases the amount of stake in a neuron's hotkey account by the amount provided
    /// The uid parameter identifies the neuron holding the hotkey account.
    /// When using this function, it is important to also increase another account by the same value,
    /// as otherwise value gets lost.
    ///
    /// A check if there is enough stake in the hotkey account should have been performed
    /// before this function is called. If not, the node will crap out.
    ///
    /// Furthermore, a check to see if the uid is active before this method is called is also required
    ///
    pub fn remove_stake_from_neuron_hotkey_account(uid: u32, amount: u64) {
        debug_assert!(Self::is_uid_active(uid));

        let mut neuron: NeuronMetadataOf<T> = Self::get_neuron_for_uid( uid );
        let hotkey_stake: u64 = neuron.stake;

        // By this point, there should be enough stake in the hotkey account for this to work.
        debug_assert!(hotkey_stake >= amount);
        neuron.stake = neuron.stake.saturating_sub(amount);

        Neurons::<T>::insert(uid, neuron);
        Self::decrease_total_stake(amount);
    }

    /// This adds stake (balance) to a cold key account. It takes the account id of the coldkey account and a Balance as parameters.
    /// The Balance parameter is a from u64 converted number. This is needed for T::Currency to work.
    /// Make sure stake is removed from another account before calling this method, otherwise you'll end up with double the value
    ///
    pub fn add_balance_to_coldkey_account(coldkey: &T::AccountId, amount: <<T as Config>::Currency as Currency<<T as system::Config>::AccountId>>::Balance) {
        T::Currency::deposit_creating(&coldkey, amount); // Infallibe
    }

    /// This removes stake from the hotkey. This should be used together with the function to store the stake
    /// in the hot key account.
    /// The internal mechanics can fail. When this happens, this function returns false, otherwise true
    /// The output of this function MUST be checked before writing the amount to the hotkey account
    ///
    ///
    pub fn remove_balance_from_coldkey_account(coldkey: &T::AccountId, amount: <<T as Config>::Currency as Currency<<T as system::Config>::AccountId>>::Balance) -> bool {
        return match T::Currency::withdraw(&coldkey, amount, WithdrawReasons::except(WithdrawReasons::TIP), ExistenceRequirement::KeepAlive) {
            Ok(_result) => {
                true
            }
            Err(_error) => {
                false
            }
        };
    }

    /// Checks if the neuron as specified in the neuron parameter has subscribed with the cold key
    /// as specified in the coldkey parameter. See fn subscribe() for more info.
    ///
    pub fn neuron_belongs_to_coldkey(neuron: &NeuronMetadataOf<T>, coldkey: &T::AccountId) -> bool {
        return neuron.coldkey == *coldkey;
    }

    /// Checks if the coldkey account has enough balance to be able to withdraw the specified amount.
    ///
    pub fn can_remove_balance_from_coldkey_account(coldkey: &T::AccountId, amount: <<T as Config>::Currency as Currency<<T as system::Config>::AccountId>>::Balance) -> bool {
        let current_balance = Self::get_coldkey_balance(coldkey);
        if amount > current_balance {
            return false;
        }

        // This bit is currently untested. @todo
        let new_potential_balance = current_balance - amount;
        let can_withdraw = T::Currency::ensure_can_withdraw(&coldkey, amount, WithdrawReasons::except(WithdrawReasons::TIP), new_potential_balance).is_ok();
        can_withdraw
    }

    /// Returns the current balance in the cold key account
    ///
    pub fn get_coldkey_balance(coldkey: &T::AccountId) -> <<T as Config>::Currency as Currency<<T as system::Config>::AccountId>>::Balance {
        return T::Currency::free_balance(&coldkey);
    }

    /// Checks if the hotkey account of the specified account has enough stake to be able to withdraw
    /// the requested amount.
    ///
    pub fn has_enough_stake(neuron: &NeuronMetadataOf<T>, amount: u64) -> bool {
        return neuron.stake >= amount;
    }

    /// Returns true if there is an entry for uid in the Stake map,
    /// false otherwise
    ///
    pub fn has_hotkey_account(uid: &u32) -> bool {
        return Neurons::<T>::contains_key(*uid);
    }

    /// This calculates the fraction of the total amount of stake the specfied neuron owns.
    /// This function is part of the algorithm that calculates the emission of this neurons
    /// to its peers. See fn calculate_emission_for_neuron()
    ///
    /// This function returns 0 if the total amount of stake is 0, or the amount of stake the
    /// neuron has is 0.
    ///
    /// Otherwise, it returns the result of neuron_stake / total stake
    ///
    pub fn calculate_stake_fraction_for_neuron(neuron: &NeuronMetadataOf<T>) -> U64F64 {
        let total_stake = U64F64::from_num(TotalStake::<T>::get());
        let neuron_stake = U64F64::from_num(neuron.stake);

        // Total stake is 0, this should virtually never happen, but is still here because it could
        if total_stake == U64F64::from_num(0) {
            return U64F64::from_num(0);
        }

        // Neuron stake is zero. This means there will be nothing to emit
        if neuron_stake == U64F64::from_num(0) {
            return U64F64::from_num(0);
        }

        let stake_fraction = neuron_stake / total_stake;

        return stake_fraction;
    }

    /// Calculates the proportion of the stake a neuron has to the total stake.
    /// As such, the result of this function should ALWAYS be a number between
    /// 0 and 1 (inclusive).
    pub fn calulate_stake_fraction(stake: u64, total_stake: u64) -> U64F64 {
        return U64F64::from_num(stake) / U64F64::from_num(total_stake);
    }
}

