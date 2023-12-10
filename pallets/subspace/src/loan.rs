 
use frame_support::{pallet_prelude::DispatchResult};
use substrate_fixed::types::{I110F18, I32F32, I64F64, I96F32};

use super::*;

impl<T: Config> Pallet<T> {
    pub fn do_add_loan(
        origin: T::RuntimeOrigin,
        to: Vec<T::AccountId>,
        amount: Vec<u16>
        lock_period: u64
    ) -> DispatchResult {

        let key = ensure_signed(origin)?;
        assert!(keys.len() > 0);
        assert!(keys.len() == shares.len()); // make sure the keys and shares are the same length

        assert!(total_normalized_shares == u16::MAX, "normalized shares {} vs {} do not add up to the unit", total_normalized_shares, u16::MAX);
        
        // check tssat the normalized shares add up to the unit
        let total_normalized_shares: u16 = normalize_shares.iter().sum::<u16>();

        // now send the normalized shares to the profit share pallet
        let profit_share_tuples : Vec<(T::AccountId, u16)> = keys.iter().zip(normalize_shares.iter()).map(|(x, y)| (x.clone(), *y)).collect();
        
        
        
        ProfitShares::<T>::insert(&key, profit_share_tuples.clone());

        assert!(ProfitShares::<T>::get(&key).len() == profit_share_tuples.len(), "profit shares not added");

        Ok(())

        
    }


}


