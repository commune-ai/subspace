use super::*;
use frame_support::pallet_prelude::DispatchResult;
use pallet_subnet_emission_api::SubnetConsensus;
use sp_core::Get;

impl<T: Config> Pallet<T> {
    /// Sets weights for a node in a specific subnet.
    /// # Arguments
    ///
    /// * `origin` - The origin of the call, must be a signed account.
    /// * `netuid` - The ID of the subnet.
    /// * `uids` - A vector of UIDs to set weights for.
    /// * `values` - A vector of weight values corresponding to the UIDs.
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    ///
    /// * The caller's signature is invalid.
    /// * The `uids` and `values` vectors are not of equal length.
    /// * The specified subnet does not exist.
    /// * The caller is not registered in the specified subnet.
    /// * The maximum number of set weight calls per epoch has been reached.
    /// * The daily limit for root network has been exceeded.
    /// * There are duplicate UIDs in the `uids` vector.
    /// * Any of the UIDs are invalid for the specified subnet.
    /// * The number of weights is outside the allowed range.
    /// * The caller attempts to set a weight for themselves (except in subnet 0).
    /// * The caller doesn't have enough stake to set the specified weights.
    /// * The caller has no stake.
    ///
    /// # Effects
    ///
    /// If successful, this function will:
    ///
    /// 1. Normalize the provided weight values.
    /// 2. Update the weights for the specified UIDs in storage.
    /// 3. Update the last activity timestamp for the caller in the subnet.
    /// 4. Emit a `WeightsSet` event.
    ///
    /// # Notes
    ///
    /// - The function includes various checks to ensure the integrity and validity of the
    ///   weight-setting operation.
    /// - Weight normalization is performed to ensure a consistent scale across all weights.
    /// - The function tracks the number of weight-setting calls per epoch to prevent abuse.
    /// - For the root network (netuid 0), additional daily limit checks are performed.
    pub fn do_set_weights(
        origin: T::RuntimeOrigin,
        netuid: u16,
        uids: Vec<u16>,
        values: Vec<u16>,
    ) -> dispatch::DispatchResult {
        let key = ensure_signed(origin)?;
        let Some(uid) = Self::get_uid_for_key(netuid, &key) else {
            return Err(Error::<T>::ModuleDoesNotExist.into());
        };
        if Pallet::<T>::get_delegated_stake(&key) < pallet::MinValidatorStake::<T>::get(netuid) {
            return Err(Error::<T>::NotEnoughStakeToSetWeights.into());
        }
        Self::validate_input(uid, &uids, &values, netuid)?;
        Self::handle_rate_limiting(uid, netuid, &key)?;
        Self::validate_stake(&key, uids.len())?;
        Self::check_whitelisted(netuid, &uids)?;
        Self::finalize_weights(netuid, uid, &uids, &values)?;
        Self::remove_rootnet_delegation(netuid, key);
        Ok(())
    }

    fn validate_input(uid: u16, uids: &[u16], values: &[u16], netuid: u16) -> DispatchResult {
        ensure!(
            uids.len() == values.len(),
            Error::<T>::WeightVecNotEqualSize
        );
        ensure!(
            Self::if_subnet_exist(netuid),
            Error::<T>::NetworkDoesNotExist
        );
        ensure!(!Self::contains_duplicates(uids), Error::<T>::DuplicateUids);
        Self::validate_uids_length(uids.len(), netuid)?;
        Self::perform_uid_validity_check(uids, netuid)?;
        ensure!(
            Self::is_rootnet(netuid) || !uids.contains(&uid),
            Error::<T>::NoSelfWeight
        );
        Ok(())
    }

    fn validate_stake(key: &T::AccountId, uids_len: usize) -> DispatchResult {
        let stake = Self::get_delegated_stake(key);
        let min_stake_per_weight = MinWeightStake::<T>::get();
        let min_stake_for_weights = min_stake_per_weight.checked_mul(uids_len as u64).unwrap_or(0);
        ensure!(
            stake >= min_stake_for_weights,
            Error::<T>::NotEnoughStakePerWeight
        );
        ensure!(stake > 0, Error::<T>::NotEnoughStakeToSetWeights);
        Ok(())
    }

    fn validate_uids_length(len: usize, netuid: u16) -> DispatchResult {
        let min_allowed_length = Self::get_min_allowed_weights(netuid) as usize;
        let max_allowed_length = MaxAllowedWeights::<T>::get(netuid) as usize; //.min(N::<T>::get(netuid)) as usize;
        ensure!(
            len >= min_allowed_length && len <= max_allowed_length,
            Error::<T>::InvalidUidsLength
        );
        Ok(())
    }

    fn check_whitelisted(netuid: u16, uids: &[u16]) -> DispatchResult {
        // Only perform the whitelist check if EnforceWhitelist is true
        if T::EnforceWhitelist::get() {
            let consensus_netuid = T::get_consensus_netuid(SubnetConsensus::Linear);

            // Early return if consensus_netuid is None or doesn't match the given netuid
            if consensus_netuid.map_or(true, |cn| cn != netuid) {
                return Ok(());
            }

            let whitelisted = T::whitelisted_keys();

            uids.iter().try_for_each(|&uid| {
                let key = Self::get_key_for_uid(netuid, uid).ok_or(Error::<T>::InvalidUid)?;

                if !whitelisted.contains(&key) {
                    return Err(Error::<T>::UidNotWhitelisted.into());
                }

                Ok(())
            })
        } else {
            // If EnforceWhitelist is false, always return Ok
            Ok(())
        }
    }

    fn finalize_weights(netuid: u16, uid: u16, uids: &[u16], values: &[u16]) -> DispatchResult {
        let normalized_values = Self::normalize_weights(values);
        let zipped_weights: Vec<(u16, u16)> = uids.iter().copied().zip(normalized_values).collect();
        Weights::<T>::insert(netuid, uid, zipped_weights);
        WeightSetAt::<T>::insert(netuid, uid, Self::get_current_block_number());
        let current_block = Self::get_current_block_number();
        Self::set_last_update_for_uid(netuid, uid, current_block);
        Self::deposit_event(Event::WeightsSet(netuid, uid));
        Ok(())
    }
    // ----------
    // Utils
    // ----------
    fn remove_rootnet_delegation(netuid: u16, key: T::AccountId) {
        if Self::is_rootnet(netuid) {
            RootnetControlDelegation::<T>::remove(key);
        }
    }

    fn contains_duplicates(items: &[u16]) -> bool {
        let mut seen = sp_std::collections::btree_set::BTreeSet::new();
        items.iter().any(|item| !seen.insert(item))
    }

    pub fn perform_uid_validity_check(uids: &[u16], netuid: u16) -> DispatchResult {
        ensure!(
            uids.iter().all(|&uid| Self::uid_exist_on_network(netuid, uid)),
            Error::<T>::InvalidUid
        );
        Ok(())
    }

    pub fn uid_exist_on_network(netuid: u16, uid: u16) -> bool {
        if Self::is_rootnet(netuid) {
            N::<T>::contains_key(uid)
        } else {
            Keys::<T>::contains_key(netuid, uid)
        }
    }

    // ----------------
    // Rate limiting
    // ----------------

    fn handle_rate_limiting(uid: u16, netuid: u16, key: &T::AccountId) -> dispatch::DispatchResult {
        if let Some(max_set_weights) =
            MaximumSetWeightCallsPerEpoch::<T>::get(netuid).filter(|r| *r > 0)
        {
            let set_weight_uses = SetWeightCallsPerEpoch::<T>::mutate(netuid, key, |value| {
                *value = value.saturating_add(1);
                *value
            });
            ensure!(
                set_weight_uses <= max_set_weights,
                Error::<T>::MaxSetWeightsPerEpochReached
            );
        }
        Self::check_rootnet_daily_limit(netuid, uid)
    }

    fn check_rootnet_daily_limit(netuid: u16, module_id: u16) -> DispatchResult {
        if Self::is_rootnet(netuid) {
            ensure!(
                RootNetWeightCalls::<T>::get(module_id).is_none(),
                Error::<T>::MaxSetWeightsPerEpochReached
            );
            RootNetWeightCalls::<T>::set(module_id, Some(()));
        }
        Ok(())
    }

    // ----------------
    // Normalization
    // ----------------
    pub fn normalize_weights(weights: &[u16]) -> Vec<u16> {
        let sum: u64 = weights.iter().map(|&x| u64::from(x)).sum();
        if sum == 0 {
            return weights.to_vec();
        }
        weights
            .iter()
            .map(|&x| {
                u64::from(x)
                    .checked_mul(u64::from(u16::MAX))
                    .and_then(|product| product.checked_div(sum))
                    .and_then(|result| result.try_into().ok())
                    .unwrap_or(0)
            })
            .collect()
    }

    /// Clears the set weight rate limiter for a given subnet.
    ///
    /// # Arguments
    ///
    /// * `netuid` - The ID of the subnet.
    ///
    /// This function removes all entries from the SetWeightCallsPerEpoch storage
    /// for the specified subnet.
    pub fn clear_set_weight_rate_limiter(netuid: u16) {
        let _ = SetWeightCallsPerEpoch::<T>::clear_prefix(netuid, u32::MAX, None);
    }

    pub fn do_delegate_rootnet_control(
        origin: T::RuntimeOrigin,
        target: T::AccountId,
    ) -> DispatchResult {
        let key = ensure_signed(origin)?;

        let rootnet_id = T::get_consensus_netuid(SubnetConsensus::Root)
            .ok_or(Error::<T>::RootnetSubnetNotFound)?;

        let Some(origin_uid) = Self::get_uid_for_key(rootnet_id, &key) else {
            return Err(Error::<T>::ModuleDoesNotExist.into());
        };

        if Self::get_uid_for_key(rootnet_id, &target).is_none() {
            return Err(Error::<T>::ModuleDoesNotExist.into());
        };

        if RootnetControlDelegation::<T>::get(&target).is_some() {
            return Err(Error::<T>::TargetIsDelegatingControl.into());
        }

        Self::check_rootnet_daily_limit(rootnet_id, origin_uid)?;

        RootnetControlDelegation::<T>::set(key, Some(target));

        Ok(())
    }

    pub fn copy_delegated_weights(block: u64) {
        use core::ops::Rem;
        if block.rem(5400) == 0 {
            let Some(rootnet_id) = T::get_consensus_netuid(SubnetConsensus::Root) else {
                return;
            };

            for (origin, target) in RootnetControlDelegation::<T>::iter().collect::<Vec<_>>() {
                let Some(target_uid) = Self::get_uid_for_key(rootnet_id, &target) else {
                    continue;
                };

                let Some(origin_uid) = Self::get_uid_for_key(rootnet_id, &origin) else {
                    continue;
                };

                let weights = Weights::<T>::get(rootnet_id, target_uid);
                Weights::<T>::set(rootnet_id, origin_uid, weights);
            }
        }
    }
}
