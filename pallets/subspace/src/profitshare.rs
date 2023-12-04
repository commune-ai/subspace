use core::ops::Add;

use frame_support::{pallet_prelude::DispatchResult};
use substrate_fixed::types::{I110F18, I32F32, I64F64, I96F32};

use super::*;



impl<T: Config> Pallet<T> {
    pub fn do_add_profit_shares(
        origin: T::RuntimeOrigin,
        keys: Vec<T::AccountId>,
        shares: Vec<u16>
    ) -> DispatchResult {

        let key = ensure_signed(origin)?;

        // needs to be registered as a network
        ensure!(
            Self::is_key_registered_on_any_network(&key),
            Error::<T>::NotRegistered
        );
        assert!(keys.len() > 0);
        assert!(keys.len() == shares.len()); // make sure the keys and shares are the same length

        let mut total_shares: u16 = 0;
        for share in shares.iter() {
            total_shares = total_shares.add(*share);
        }
        let unit: u16 = ProfitShareUnit::<T>::get();
        normalized_shares_float: I32F32 = Vec::new();
        // normalize shares
        for share in shares.iter_mut() {
            normalized_shares_float.push(I32F32::from(*share*unit) / I32F32::from(total_shares));
        }
        // convert the normalized shares to u16
        let normalize_shares: Vec<u16> = normalized_shares_float.iter().map(|x| x.to_num::<u16>()).collect::<Vec<u16>>();
        
        // check that the normalized shares add up to the unit
        assert!(normalize_shares.iter().sum::<u16>() == unit);

        // now send the normalized shares to the profit share pallet
        profit_share_tuples : Vec<(T::AccountId, u16)> = keys.iter().zip(normalize_shares.iter()).collect::<Vec<(T::AccountId, u16)>>();
        ProfitShares::<T>::insert(&key, profit_share_tuples);

        Ok(())

        
    }


    pub fn split_emission_on_profit_shares(
        key: T::AccountId,
        emission: u64,
    ) -> Vec<(T::AccountId, u64)> {


        let key = ensure_signed(origin)?;

        // needs to be registered as a network
        ensure!(
            Self::is_key_registered_on_any_network(&key),
            Error::<T>::NotRegistered
        );
        let profit_shares = ProfitShares::<T>::get(&key);
        let mut emission_shares: Vec<(T::AccountId, u64)> = Vec::new();
        for (key, share) in profit_shares.iter() {
            emission_shares.push((key, emission*share));
        }
        return emission_shares;
        
        
    }


    pub fn get_profit_shares(
        key: T::AccountId,
    ) -> Vec<(T::AccountId, u16)> {
        let key = ensure_signed(origin)?;

        // needs to be registered as a network
        ensure!(
            Self::is_key_registered_on_any_network(&key),
            Error::<T>::NotRegistered
        );
        return ProfitShares::<T>::get(&key);;
        
    }

}


