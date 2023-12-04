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

        let mut total_shares: u32 = shares.iter().map(|x| *x as u32).sum();
        let mut normalized_shares_float: Vec<I64F64> = Vec::new();
        // normalize shares
        for share in shares.iter() {

            let normalized_share = I64F64::from(*share) / I64F64::from(total_shares as u16);
            normalized_shares_float.push(normalized_share * I64F64::from(u16::MAX));
        }
        // convert the normalized shares to u16
        let normalize_shares: Vec<u16> = normalized_shares_float.iter().map(|x| x.to_num::<u16>()).collect::<Vec<u16>>();
        
        // check tssat the normalized shares add up to the unit
        let total_normalized_shares: u16 = normalize_shares.iter().sum::<u16>();

        // now send the normalized shares to the profit share pallet
        let profit_share_tuples : Vec<(T::AccountId, u16)> = keys.iter().zip(normalize_shares.iter()).map(|(x, y)| (x.clone(), *y)).collect();
        ProfitShares::<T>::insert(&key, profit_share_tuples);

        Ok(())

        
    }

    pub fn get_profit_share_emissions(
        key: T::AccountId,
        emission: u64,
    ) -> Vec<(T::AccountId, u64)> {

        let profit_shares = ProfitShares::<T>::get(&key);
        let mut emission_shares: Vec<(T::AccountId, u64)> = Vec::new();
        for (share_key, share_ratio) in profit_shares.iter() {
            let share_emission_float: I96F32 = I96F32::from(emission) * (I96F32::from(*share_ratio) / I96F32::from(u16::MAX));

            let share_emission: u64 = share_emission_float.to_num::<u64>();
            emission_shares.push((share_key.clone(), share_emission));
        }

        return emission_shares;
        
        
    }




    pub fn get_profit_shares(
        key: T::AccountId,
    ) -> Vec<(T::AccountId, u16)> {
        return ProfitShares::<T>::get(&key);;
        
    }

}

