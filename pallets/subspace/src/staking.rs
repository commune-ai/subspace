use super::*;

impl<T: Config> Pallet<T> { 


    // ---- The implementation for the extrinsic add_stake: Adds stake to a key account.
    //
    // # Args:
    // 	* 'origin': (<T as frame_system::Config>RuntimeOrigin):
    // 		- The signature of the caller's key.
    //
    // 	* 'key' (T::AccountId):
    // 		- The associated key account.
    //
    // 	* 'stake_to_be_added' (u64):
    // 		- The amount of stake to be added to the key staking account.
    //
    // # Event:
    // 	* StakeAdded;
    // 		- On the successfully adding stake to a global account.
    //
    // # Raises:
    // 	* 'CouldNotConvertToBalance':
    // 		- Unable to convert the passed stake value to a balance.
    //
    // 	* 'NotEnoughBalanceToStake':
    // 		- Not enough balance on the key to add onto the global account.
    //
    // 	* 'NonAssociatedColdKey':
    // 		- The calling key is not associated with this key.
    //
    // 	* 'BalanceWithdrawalError':
    // 		- Errors stemming from transaction pallet.
    //
	// 	* 'TxRateLimitExceeded':
    // 		- Thrown if key has hit transaction rate limit
    //
	pub fn do_add_stake(
        origin: T::RuntimeOrigin, 
        stake_to_be_added: u64
    ) -> dispatch::DispatchResult {
        // --- 1. We check that the transaction is signed by the caller and retrieve the T::AccountId key information.
        let key = ensure_signed( origin )?;
        log::info!("do_add_stake( origin:{:?} stake_to_be_added:{:?} )", key, stake_to_be_added );

        // --- 2. We convert the stake u64 into a balancer.
        let stake_as_balance = Self::u64_to_balance( stake_to_be_added );
        ensure!( stake_as_balance.is_some(), Error::<T>::CouldNotConvertToBalance );
 
        // --- 3. Ensure the callers key has enough stake to perform the transaction.
        ensure!( Self::can_remove_balance_from_account( &key, stake_as_balance.unwrap() ), Error::<T>::NotEnoughBalanceToStake );


        // --- 6. Ensure the remove operation from the key is a success.
        ensure!( Self::remove_balance_from_account( &key, stake_as_balance.unwrap() ) == true, Error::<T>::BalanceWithdrawalError );

		// --- 7. Ensure we don't exceed tx rate limit
		ensure!( !Self::exceeds_tx_rate_limit( Self::get_last_tx_block(&key), Self::get_current_block_as_u64() ), Error::<T>::TxRateLimitExceeded );

        // --- 8. If we reach here, add the balance to the key.
        Self::increase_stake_on_account( &key, stake_to_be_added );
 
        // --- 9. Emit the staking event.
        log::info!("StakeAdded( key:{:?}, stake_to_be_added:{:?} )", key, stake_to_be_added );
        Self::deposit_event( Event::StakeAdded( key, stake_to_be_added ) );

        // --- 10. Ok and return.
        Ok(())
    }

    // ---- The implementation for the extrinsic remove_stake: Removes stake from a key account and adds it onto a key.
    //
    // # Args:
    // 	* 'origin': (<T as frame_system::Config>RuntimeOrigin):
    // 		- The signature of the caller's key.
    //
    // 	* 'key' (T::AccountId):
    // 		- The associated key account.
    //
    // 	* 'stake_to_be_added' (u64):
    // 		- The amount of stake to be added to the key staking account.
    //
    // # Event:
    // 	* StakeRemoved;
    // 		- On the successfully removing stake from the key account.
    //
    // # Raises:
    // 	* 'NotRegistered':
    // 		- Thrown if the account we are attempting to unstake from is non existent.
    //
    // 	* 'NonAssociatedColdKey':
    // 		- Thrown if the key does not own the key we are unstaking from.
    //
    // 	* 'NotEnoughStaketoWithdraw':
    // 		- Thrown if there is not enough stake on the key to withdwraw this amount. 
    //
    // 	* 'CouldNotConvertToBalance':
    // 		- Thrown if we could not convert this amount to a balance.
	//
    // 	* 'TxRateLimitExceeded':
    // 		- Thrown if key has hit transaction rate limit
    //
    //
    pub fn do_remove_stake(
        origin: T::RuntimeOrigin, 
        stake_to_be_removed: u64
    ) -> dispatch::DispatchResult {

        // --- 1. We check the transaction is signed by the caller and retrieve the T::AccountId key information.
        let key = ensure_signed( origin )?;
        log::info!("do_remove_stake( origin:{:?} stake_to_be_removed:{:?} )", key, stake_to_be_removed );

        // --- 4. Ensure that the key has enough stake to withdraw.
        ensure!( Self::has_enough_stake( &key, stake_to_be_removed ), Error::<T>::NotEnoughStaketoWithdraw );

        // --- 5. Ensure that we can conver this u64 to a balance.
        let stake_to_be_added_as_currency = Self::u64_to_balance( stake_to_be_removed );
        ensure!( stake_to_be_added_as_currency.is_some(), Error::<T>::CouldNotConvertToBalance );

		// --- 6. Ensure we don't exceed tx rate limit
		ensure!( !Self::exceeds_tx_rate_limit( Self::get_last_tx_block(&key), Self::get_current_block_as_u64() ), Error::<T>::TxRateLimitExceeded );

        // --- 7. We remove the balance from the key.
        Self::decrease_stake_on_account(  &key, stake_to_be_removed );

        // --- 8. We add the balancer to the key.  If the above fails we will not credit this key.
        Self::add_balance_to_account( &key, stake_to_be_added_as_currency.unwrap() );

        // --- 9. Emit the unstaking event.
        log::info!("StakeRemoved( key:{:?}, stake_to_be_removed:{:?} )", key, stake_to_be_removed );
        Self::deposit_event( Event::StakeRemoved( key, stake_to_be_removed ) );

        // --- 10. Done and ok.
        Ok(())
    }


    // Returns the total amount of stake in the staking table.
    //
    pub fn get_total_stake() -> u64 { 
        return TotalStake::<T>::get();
    }

    // Increases the total amount of stake by the passed amount.
    //
    pub fn increase_total_stake( increment: u64 ) { 
        TotalStake::<T>::put( Self::get_total_stake().saturating_add( increment ) );
    }

    // Decreases the total amount of stake by the passed amount.
    //
    pub fn decrease_total_stake( decrement: u64 ) { 
        TotalStake::<T>::put( Self::get_total_stake().saturating_sub( decrement ) );
    }

    // Returns the total amount of stake under a key (delegative or otherwise)
    //
    pub fn get_total_stake_for_key( key: &T::AccountId ) -> u64 { 
        return TotalKeyStake::<T>::get( key ); 
    }


    // Returns the stake under the cold - hot pairing in the staking table.
    //
    pub fn get_stake( key: &T::AccountId ) -> u64 { 
        return Stake::<T>::get( key );
    }

    pub fn key_account_exists( key : &T::AccountId) -> bool {
        return true ; 
    }

    // Creates a cold - hot pairing account if the key is not already an active account.
    //
    pub fn create_account_if_non_existent(key: &T::AccountId ) {
        if !Self::key_account_exists( key ) {
            Stake::<T>::insert( key, 0 ); 
        }
    }


    // Returns true if the cold-hot staking account has enough balance to fufil the decrement.
    //
    pub fn has_enough_stake( key: &T::AccountId, decrement: u64 ) -> bool {
        return Self::get_stake(  key ) >= decrement;
    }



    // Increases the stake on the cold - hot pairing by increment while also incrementing other counters.
    // This function should be called rather than set_stake under account.
    // 
    pub fn increase_stake_on_account( key: &T::AccountId, increment: u64 ){
        TotalKeyStake::<T>::insert( key, TotalKeyStake::<T>::get(key).saturating_add( increment ) );
        Stake::<T>::insert( key, Stake::<T>::get( key).saturating_add( increment ) );
        TotalStake::<T>::put( TotalStake::<T>::get().saturating_add( increment ) );
        TotalIssuance::<T>::put( TotalIssuance::<T>::get().saturating_add( increment ) );

    }

    // Decreases the stake on the cold - hot pairing by the decrement while decreasing other counters.
    //
    pub fn decrease_stake_on_account( key: &T::AccountId, decrement: u64 ){
        TotalKeyStake::<T>::insert( key, TotalKeyStake::<T>::get(key).saturating_sub( decrement ) );
        Stake::<T>::insert( key, Stake::<T>::get( key).saturating_sub( decrement ) );
        TotalStake::<T>::put( TotalStake::<T>::get().saturating_sub( decrement ) );
        TotalIssuance::<T>::put( TotalIssuance::<T>::get().saturating_sub( decrement ) );
    }

	pub fn u64_to_balance( input: u64 ) -> Option<<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance> { input.try_into().ok() }

    pub fn add_balance_to_account(key: &T::AccountId, amount: <<T as Config>::Currency as Currency<<T as system::Config>::AccountId>>::Balance) {
        T::Currency::deposit_creating(&key, amount); // Infallibe
    }

    pub fn set_balance_on_account(key: &T::AccountId, amount: <<T as Config>::Currency as Currency<<T as system::Config>::AccountId>>::Balance) {
        T::Currency::make_free_balance_be(&key, amount); 
    }

    pub fn can_remove_balance_from_account(key: &T::AccountId, amount: <<T as Config>::Currency as Currency<<T as system::Config>::AccountId>>::Balance) -> bool {
        let current_balance = Self::get_balance(key);
        if amount > current_balance {
            return false;
        }

        // This bit is currently untested. @todo
        let new_potential_balance = current_balance - amount;
        let can_withdraw = T::Currency::ensure_can_withdraw(&key, amount, WithdrawReasons::except(WithdrawReasons::TIP), new_potential_balance).is_ok();
        can_withdraw
    }

    pub fn get_balance(key: &T::AccountId) -> <<T as Config>::Currency as Currency<<T as system::Config>::AccountId>>::Balance {
        return T::Currency::free_balance(&key);
    }


    pub fn remove_balance_from_account(key: &T::AccountId, amount: <<T as Config>::Currency as Currency<<T as system::Config>::AccountId>>::Balance) -> bool {
        return match T::Currency::withdraw(&key, amount, WithdrawReasons::except(WithdrawReasons::TIP), ExistenceRequirement::KeepAlive) {
            Ok(_result) => {
                true
            }
            Err(_error) => {
                false
            }
        };
    }

}