use crate::*;
use frame_support::pallet_prelude::DispatchResult;
use pallet_governance_api::GovernanceApi;
use pallet_emission_api::{SubnetConsensus, SubnetEmissionApi};
use sp_runtime::DispatchError;
use substrate_fixed::types::I64F64;

impl<T: Config> Pallet<T> {
    pub fn add_subnet(
        changeset: SubnetChangeset<T>,
        netuid: Option<u16>,
    ) -> Result<u16, DispatchError> {
        let netuid = netuid.unwrap_or_else(|| match SubnetGaps::<T>::get().first().copied() {
            Some(removed) => removed,
            None => Self::get_total_subnets(),
        });

        let name = changeset.params().name.clone();
        changeset.apply(netuid)?;
        N::<T>::insert(netuid, 0);
        T::set_emission_storage(netuid, 0);
        SubnetRegistrationsThisInterval::<T>::mutate(|value| *value = value.saturating_add(1));
        SubnetRegistrationBlock::<T>::set(netuid, Some(Self::get_current_block_number()));

        // Insert the minimum burn to the netuid,
        // to prevent free registrations the first target registration interval.
        let min_burn = GeneralBurnConfiguration::<T>::default_for(BurnType::Module).min_burn;
        Burn::<T>::set(netuid, min_burn);

        SubnetGaps::<T>::mutate(|subnets| subnets.remove(&netuid));
        T::create_yuma_subnet(netuid);

        // --- 6. Emit the new network event.
        Self::deposit_event(Event::NetworkAdded(netuid, name.into_inner()));

        Ok(netuid)
    }

    fn clear_subnet_includes(netuid: u16) {
        for storage_type in SubnetIncludes::all() {
            storage_type.remove_storage::<T>(netuid)
        }
    }

    pub fn clear_subnet_only_accounts_data(subnet_id: u16) {
        // Get all accounts in the specified subnet
        let subnet_accounts: BTreeSet<AccountIdOf<T>> =
            Uids::<T>::iter_prefix(subnet_id).map(|(account, _)| account).collect();

        // Get all accounts from other subnets
        let accounts_in_other_subnets: BTreeSet<AccountIdOf<T>> = Uids::<T>::iter()
            .filter(|(net_id, _, _)| net_id != &subnet_id)
            .map(|(_, account, _)| account)
            .collect();

        // Clear data for accounts that exist only in this subnet
        subnet_accounts
            .difference(&accounts_in_other_subnets)
            .for_each(|subnet_only_account| {
                // Clear stakes
                Self::remove_stake_from_storage(subnet_only_account);
                // Clear validator fees
                ValidatorFeeConfig::<T>::remove(subnet_only_account);
            });
    }

    pub fn remove_subnet(netuid: u16) {
        if !Self::if_subnet_exist(netuid) {
            return;
        }

        if !T::can_remove_subnet(netuid) {
            return;
        }

        // --- Delete Global-Subnet Storage ---

        // Potentially Remove Stake & Delegation Fee
        // Automatically remove the stake & delegation fee of modules that are only registered on
        // this subnet. This is because it's not desirable for module to be **globally**
        // unregistered with "active" stake storage or "active" delegation fee storage.
        Self::clear_subnet_only_accounts_data(netuid);

        // --- Delete Subnet Includes Storage For All Pallets ---

        Self::clear_subnet_includes(netuid);
        <T as GovernanceApi<T::AccountId>>::clear_subnet_includes(netuid);
        <T as SubnetEmissionApi<T::AccountId>>::clear_subnet_includes(netuid);

        // --- Mutate Subnet Gaps & Emit The Event ---

        SubnetGaps::<T>::mutate(|subnets| subnets.insert(netuid));

        Self::deposit_event(Event::NetworkRemoved(netuid));
    }

    pub fn do_update_subnet(
        origin: T::RuntimeOrigin,
        netuid: u16,
        changeset: SubnetChangeset<T>,
    ) -> DispatchResult {
        let key = ensure_signed(origin)?;

        // -- 1. Make sure the netuid exists
        ensure!(
            SubnetNames::<T>::contains_key(netuid),
            Error::<T>::NetuidDoesNotExist
        );

        // --2. Ensury Authority - only the founder can update the network on authority mode.
        ensure!(Founder::<T>::get(netuid) == key, Error::<T>::NotFounder);

        // -4. Apply the changeset.
        changeset.apply(netuid)?;

        // --- 5. Ok and done.
        Ok(())
    }

    // --- Setters ---

    pub fn set_max_allowed_uids(netuid: u16, max_allowed_uids: u16) -> DispatchResult {
        let n: u16 = N::<T>::get(netuid);
        ensure!(n <= max_allowed_uids, Error::<T>::InvalidMaxAllowedUids);
        MaxAllowedUids::<T>::insert(netuid, max_allowed_uids);
        Ok(())
    }

    pub fn set_last_update_for_uid(netuid: u16, uid: u16, last_update: u64) {
        LastUpdate::<T>::mutate(netuid, |vec| {
            if let Some(idx) = vec.get_mut(uid as usize) {
                *idx = last_update;
            }
        });
    }

    pub fn copy_last_update_for_uid(netuid: u16, origin: u16, target: u16) {
        LastUpdate::<T>::mutate(netuid, |vec| {
            if let Some(val) = vec.get(target as usize).cloned() {
                if let Some(idx) = vec.get_mut(origin as usize) {
                    *idx = val;
                }
            }
        });
    }

    // --- Getters ---

    pub fn get_min_allowed_weights(netuid: u16) -> u16 {
        let min_allowed_weights = MinAllowedWeights::<T>::get(netuid);
        min_allowed_weights.min(N::<T>::get(netuid))
    }

    pub fn get_netuid_for_name(name: &[u8]) -> Option<u16> {
        SubnetNames::<T>::iter().find(|(_, n)| n == name).map(|(id, _)| id)
    }
    /// Returs the key under the network uid as a Result. Ok if the uid is taken.
    #[inline]
    pub fn get_key_for_uid(netuid: u16, module_uid: u16) -> Option<T::AccountId> {
        Keys::<T>::get(netuid, module_uid)
    }
    ///Returns the uid of the key in the network as a Result. Ok if the key has a slot.
    #[inline]
    pub fn get_uid_for_key(netuid: u16, key: &T::AccountId) -> Option<u16> {
        Uids::<T>::get(netuid, key)
    }

    pub fn get_current_block_number() -> u64 {
        TryInto::try_into(<frame_system::Pallet<T>>::block_number())
            .ok()
            .expect("blockchain will not exceed 2^64 blocks; QED.")
    }

    // --- Util ---
    pub fn calculate_founder_emission(netuid: u16, mut token_emission: u64) -> (u64, u64) {
        let founder_share: u16 = FounderShare::<T>::get(netuid).min(100);
        if founder_share == 0u16 {
            return (token_emission, 0);
        }

        let founder_emission_ratio: I64F64 = I64F64::from_num(founder_share.min(100))
            .checked_div(I64F64::from_num(100))
            .unwrap_or_default();

        let founder_emission = founder_emission_ratio
            .checked_mul(I64F64::from_num(token_emission))
            .map(|result| result.to_num::<u64>())
            .unwrap_or_default();

        token_emission = token_emission.saturating_sub(founder_emission);

        (token_emission, founder_emission)
    }

    pub fn get_ownership_ratios(
        netuid: u16,
        module_key: &T::AccountId,
    ) -> Vec<(T::AccountId, I64F64)> {
        let stake_from_vector = Self::get_stake_from_vector(module_key);
        let _uid = Self::get_uid_for_key(netuid, module_key);
        let mut total_stake_from: I64F64 = I64F64::from_num(0);

        let mut ownership_vector: Vec<(T::AccountId, I64F64)> = Vec::new();

        for (k, v) in stake_from_vector.into_iter() {
            let ownership = I64F64::from_num(v);
            ownership_vector.push((k.clone(), ownership));
            total_stake_from = total_stake_from.saturating_add(ownership);
        }

        // add the module itself, if it has stake of its own
        if total_stake_from == I64F64::from_num(0) {
            ownership_vector.push((module_key.clone(), I64F64::from_num(0)));
        } else {
            ownership_vector = ownership_vector
                .into_iter()
                .map(|(k, v)| (k, v.checked_div(total_stake_from).unwrap_or_default()))
                .collect();
        }

        ownership_vector
    }

    pub fn is_registered(network: Option<u16>, key: &T::AccountId) -> bool {
        match network {
            Some(netuid) => Uids::<T>::contains_key(netuid, key),
            None => N::<T>::iter_keys().any(|netuid| Uids::<T>::contains_key(netuid, key)),
        }
    }

    pub fn if_subnet_exist(netuid: u16) -> bool {
        N::<T>::contains_key(netuid)
    }

    pub fn key_registered(netuid: u16, key: &T::AccountId) -> bool {
        Uids::<T>::contains_key(netuid, key)
            || Keys::<T>::iter_prefix_values(netuid).any(|k| &k == key)
    }

    pub fn get_total_subnets() -> u16 {
        N::<T>::iter_keys().count() as u16
    }

    /// Calculates the number of blocks until the next epoch for a subnet.
    ///
    /// # Arguments
    ///
    /// * `netuid` - The ID of the subnet.
    /// * `tempo` - The tempo (frequency) of the subnet's epochs.
    /// * `block_number` - The current block number.
    ///
    /// # Returns
    ///
    /// The number of blocks until the next epoch
    pub fn blocks_until_next_epoch(netuid: u16, block_number: u64) -> u64 {
        let tempo = Tempo::<T>::get(netuid);

        if tempo == 0 {
            return u64::MAX;
        }

        (block_number.saturating_add(u64::from(netuid)))
            .checked_rem(u64::from(tempo))
            .unwrap_or(1000)
    }

    pub fn is_rootnet(netuid: u16) -> bool {
        matches!(
            T::get_subnet_consensus_type(netuid),
            Some(SubnetConsensus::Root)
        )
    }
}
