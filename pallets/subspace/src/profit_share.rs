use frame_support::pallet_prelude::DispatchResult;

use super::*;

impl<T: Config> Pallet<T> {
    pub fn do_add_profit_shares(
        origin: T::RuntimeOrigin,
        keys: Vec<T::AccountId>,
        shares: Vec<u16>,
    ) -> DispatchResult {
        let key = ensure_signed(origin)?;

        // needs to be registered as a network
        ensure!(
            Self::is_key_registered_on_any_network(&key),
            Error::<T>::NotRegistered
        );

        ensure!(!keys.is_empty(), Error::<T>::EmptyKeys);
        ensure!(keys.len() == shares.len(), Error::<T>::DifferentLengths);

        let total_shares: u32 = shares.iter().map(|&x| x as u32).sum();
        ensure!(total_shares > 0, Error::<T>::InvalidShares);

        let normalized_shares: Vec<u16> = shares
            .iter()
            .map(|&share| (share as u64 * u16::MAX as u64 / total_shares as u64) as u16)
            .collect();

        let total_normalized_shares: u16 = normalized_shares.iter().sum();

        // Ensure the profit shares add up to the unit
        let mut adjusted_shares = normalized_shares;
        if total_normalized_shares < u16::MAX {
            let diff = u16::MAX - total_normalized_shares;
            for i in 0..diff {
                let idx = (i % adjusted_shares.len() as u16) as usize;
                adjusted_shares[idx] += 1;
            }
        }

        let profit_share_tuples: Vec<(T::AccountId, u16)> =
            keys.into_iter().zip(adjusted_shares).collect();

        ProfitShares::<T>::insert(&key, profit_share_tuples.clone());

        ensure!(
            ProfitShares::<T>::get(&key).len() == profit_share_tuples.len(),
            Error::<T>::ProfitSharesNotAdded
        );

        Ok(())
    }

    pub fn get_profit_share_emissions(
        key: &T::AccountId,
        emission: u64,
    ) -> Vec<(T::AccountId, u64)> {
        let profit_shares = ProfitShares::<T>::get(key);

        profit_shares
            .into_iter()
            .map(|(share_key, share_ratio)| {
                let share_emission = emission * share_ratio as u64 / u16::MAX as u64;
                (share_key, share_emission)
            })
            .collect()
    }

    #[cfg(debug_assertions)]
    pub fn get_profit_shares(key: T::AccountId) -> Vec<(T::AccountId, u16)> {
        ProfitShares::<T>::get(&key)
    }
}
