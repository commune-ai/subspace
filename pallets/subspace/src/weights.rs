use super::*;

impl<T: Config> Pallet<T> {
    // Returns true if the items contain duplicates.
    fn contains_duplicates(items: &[u16]) -> bool {
        let mut seen = sp_std::collections::btree_set::BTreeSet::new();
        items.iter().any(|item| !seen.insert(item))
    }

    pub fn do_set_weights(
        origin: T::RuntimeOrigin,
        netuid: u16,
        uids: Vec<u16>,
        values: Vec<u16>,
    ) -> dispatch::DispatchResult {
        // --- 1. Check the caller's signature. This is the key of a registered account.
        let key = ensure_signed(origin)?;

        // --- 2. Check that the length of uid list and value list are equal for this network.
        ensure!(
            uids.len() == values.len(),
            Error::<T>::WeightVecNotEqualSize
        );

        // --- 3. Check to see if this is a valid network.
        ensure!(
            Self::if_subnet_exist(netuid),
            Error::<T>::NetworkDoesNotExist
        );

        // --- 4. Check to see if the key is registered to the passed network.
        ensure!(
            Self::is_key_registered_on_network(netuid, &key),
            Error::<T>::NotRegistered
        );

        // --- 5. Get the module uid of associated key on network netuid.
        let uid: u16 = Self::get_uid_for_key(netuid, &key);

        // --- 6. Ensure the passed uids contain no duplicates.
        ensure!(!Self::contains_duplicates(&uids), Error::<T>::DuplicateUids);

        // --- 7. Ensure that the passed uids are valid for the network.
        ensure!(
            uids.iter().all(|&uid| Self::is_uid_exist_on_network(netuid, uid)),
            Error::<T>::InvalidUid
        );

        // --- 8. Check the allowed length of uids.
        let min_allowed_length: usize = Self::get_min_allowed_weights(netuid) as usize;
        let max_allowed_length: usize = Self::get_max_allowed_weights(netuid) as usize;
        ensure!(
            uids.len() >= min_allowed_length && uids.len() <= max_allowed_length,
            Error::<T>::InvalidUidsLength
        );

        // --- 9. Ensure the uid is not setting weights for itself.
        ensure!(!uids.contains(&uid), Error::<T>::NoSelfWeight);

        // --- 10. Get the stake for the key.
        let stake: u64 = Self::get_stake_for_key(netuid, &key);

        // --- 11. Check if the stake per weight is greater than the required minimum stake.
        let min_stake_per_weight: u64 = Self::get_min_weight_stake();
        let min_stake_for_weights: u64 = min_stake_per_weight * uids.len() as u64;
        ensure!(
            stake >= min_stake_for_weights,
            Error::<T>::NotEnoughStakePerWeight
        );

        // --- 12. Ensure the key has enough stake to set weights.
        ensure!(stake > 0, Error::<T>::NotEnoughStakeToSetWeights);

        // --- 13. Normalize the weights.
        let normalized_values = Self::normalize_weights(values);

        // --- 14. Zip weights for sinking to storage map.
        let zipped_weights: Vec<(u16, u16)> = uids
            .iter()
            .zip(normalized_values.iter())
            .map(|(&uid, &val)| (uid, val))
            .collect();

        // --- 15. Set weights under netuid, uid double map entry.
        Weights::<T>::insert(netuid, uid, zipped_weights);

        // --- 16. Set the activity for the weights on this network.
        let current_block: u64 = Self::get_current_block_number();
        Self::set_last_update_for_uid(netuid, uid, current_block);

        // --- 17. Emit the tracking event.
        Self::deposit_event(Event::WeightsSet(netuid, uid));

        Ok(())
    }

    // Implace normalizes the passed positive integer weights so that they sum to u16 max value.
    pub fn normalize_weights(weights: Vec<u16>) -> Vec<u16> {
        let sum: u64 = weights.iter().map(|&x| x as u64).sum();
        if sum == 0 {
            return weights;
        }
        weights
            .into_iter()
            .map(|x| ((x as u64 * u16::max_value() as u64) / sum) as u16)
            .collect()
    }
}
