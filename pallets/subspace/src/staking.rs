use super::*;
use substrate_fixed::types::{I64F64};

impl<T: Config> Pallet<T> { 


    //
	pub fn do_add_stake(
        origin: T::RuntimeOrigin, 
        netuid: u16,
        stake_to_be_added: u64
    ) -> dispatch::DispatchResult {
        // --- 1. We check that the transaction is signed by the caller and retrieve the T::AccountId key information.
        let key = ensure_signed( origin )?;
        

		// --- 1. Ensure we don't exceed tx rate limit
		ensure!( !Self::exceeds_tx_rate_limit( Self::get_last_tx_block(&key), Self::get_current_block_as_u64() ), Error::<T>::TxRateLimitExceeded );

        
        log::info!("do_add_stake( origin:{:?} stake_to_be_added:{:?} )", key, stake_to_be_added );

        // --- 2. Checks
        let stake_as_balance = Self::u64_to_balance( stake_to_be_added );
        ensure!( stake_as_balance.is_some(), Error::<T>::CouldNotConvertToBalance );
        ensure!( Self::can_remove_balance_from_account( &key, stake_as_balance.unwrap() ), Error::<T>::NotEnoughBalanceToStake );

        // --- 4. If we reach here, add the balance to the key.
        Self::increase_stake_on_account(netuid, &key, stake_to_be_added );
 
        // --- 5. Emit the staking event.
        log::info!("StakeAdded( key:{:?}, stake_to_be_added:{:?} )", key, stake_to_be_added );
        Self::deposit_event( Event::StakeAdded( key, stake_to_be_added ) );

        // --- 6. Ok and return.
        Ok(())
    }


    pub fn do_remove_stake(
        origin: T::RuntimeOrigin, 
        netuid: u16,
        stake_to_be_removed: u64
    ) -> dispatch::DispatchResult {

        // --- 1. We check the transaction is signed by the caller and retrieve the T::AccountId key information.
        let key = ensure_signed( origin )?;
        log::info!("do_remove_stake( origin:{:?} stake_to_be_removed:{:?} )", key, stake_to_be_removed );



		// --- 6. Ensure we don't exceed tx rate limit
		ensure!( !Self::exceeds_tx_rate_limit( Self::get_last_tx_block(&key), Self::get_current_block_as_u64() ), Error::<T>::TxRateLimitExceeded );

        // --- 5. Ensure that we can conver this u64 to a balance.
        ensure!( Self::has_enough_stake(netuid, &key, stake_to_be_removed ), Error::<T>::NotEnoughStaketoWithdraw );
        let stake_to_be_added_as_currency = Self::u64_to_balance( stake_to_be_removed );
        ensure!( stake_to_be_added_as_currency.is_some(), Error::<T>::CouldNotConvertToBalance );

        // --- 7. We remove the balance from the key.
        Self::decrease_stake_on_account(netuid,  &key, stake_to_be_removed );

        // --- 9. Emit the unstaking event.
        log::info!("StakeRemoved( key:{:?}, stake_to_be_removed:{:?} )", key, stake_to_be_removed );
        Self::deposit_event( Event::StakeRemoved( key, stake_to_be_removed ) );

        // --- 10. Done and ok.
        Ok(())
    }


    // Returns the total amount of stake in the staking table.
    //
    pub fn get_total_subnet_stake(netuid:u16) -> u64 { 
        return TotalSubnetStake::<T>::get(netuid);
    }
    pub fn get_total_stake() -> u64 { 
        return TotalStake::<T>::get();
    }


    // Returns the total amount of stake in the staking table.
    //
    pub fn get_token_emmision(netuid:u16) -> u64 { 

        let subnet_stake: I64F64 =I64F64::from_num( Self::get_total_subnet_stake(netuid));
        let total_stake: I64F64 = I64F64::from_num(Self::get_total_stake());
        let mut subnet_ratio: I64F64 = I64F64::from_num(0);
        if total_stake > I64F64::from_num(0) {
            subnet_ratio =  subnet_stake/total_stake;
        }
        let token_emission: u64 = subnet_ratio.to_num::<u64>();

        return token_emission;

    }



    // Returns the stake under the cold - hot pairing in the staking table.
    //
    pub fn get_stake(netuid:u16, key: &T::AccountId ) -> u64 { 
        return Stake::<T>::get(netuid,  key );
    }


    pub fn key_account_exists(netuid:u16, key : &T::AccountId) -> bool {
        return Uids::<T>::contains_key(netuid, &key) ; 
    }

    // Returns true if the cold-hot staking account has enough balance to fufil the decrement.
    //
    pub fn has_enough_stake(netuid: u16, key: &T::AccountId, decrement: u64 ) -> bool {
        return Self::get_stake(netuid ,  key ) >= decrement;
    }



    // Increases the stake on the cold - hot pairing by increment while also incrementing other counters.
    // This function should be called rather than set_stake under account.
    // 
    pub fn increase_stake_on_account(netuid:u16, key: &T::AccountId, increment: u64 ){

        // --- 2. We convert the stake u64 into a balancer.
        let stake_as_balance = Self::u64_to_balance( increment );
        // --- 6. Ensure the remove operation from the key is a success.
        Self::remove_balance_from_account( &key, stake_as_balance.unwrap() );

        Stake::<T>::insert(netuid, key, Stake::<T>::get(netuid, key).saturating_add( increment ) );
        TotalSubnetStake::<T>::insert(netuid , TotalSubnetStake::<T>::get(netuid).saturating_add( increment ) );
        TotalStake::<T>::put(TotalStake::<T>::get().saturating_add( increment ) );

    }


    // Decreases the stake on the cold - hot pairing by the decrement while decreasing other counters.
    //
    pub fn decrease_stake_on_account(netuid:u16, key: &T::AccountId, decrement: u64 ) {

        let stake_to_be_added_as_currency = Self::u64_to_balance( decrement );

        // --- 8. We add the balancer to the key.  If the above fails we will not credit this key.
        Self::add_balance_to_account( &key, stake_to_be_added_as_currency.unwrap() );
        Stake::<T>::insert( netuid, key, Stake::<T>::get(netuid,  key).saturating_sub( decrement ) );
        TotalStake::<T>::put(TotalStake::<T>::get().saturating_sub( decrement ) );
        TotalSubnetStake::<T>::insert(netuid, TotalSubnetStake::<T>::get(netuid).saturating_sub( decrement ) );
    }

    // Decreases the stake on the cold - hot pairing by the decrement while decreasing other counters.
    //
    pub fn decrease_all_stake_on_account(netuid:u16, key: &T::AccountId ) {

        let decrement = Stake::<T>::get(netuid,  &key);
        Self::decrease_stake_on_account(netuid, &key, decrement );
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