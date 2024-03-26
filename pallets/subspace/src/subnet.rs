use crate::voting::{AUTHORITY_MODE, STAKE_MODE};

use super::*;

use frame_support::{
    pallet_prelude::DispatchResult, storage::IterableStorageMap, IterableStorageDoubleMap,
};

use sp_std::vec::Vec;
use substrate_fixed::types::I64F64;
extern crate alloc;

impl<T: Config> Pallet<T> {
    #[cfg(debug_assertions)]
    pub fn do_remove_subnet(origin: T::RuntimeOrigin, netuid: u16) -> DispatchResult {
        let key = ensure_signed(origin)?;
        // --- 1. Ensure the network name does not already exist.

        ensure!(
            Self::if_subnet_netuid_exists(netuid),
            Error::<T>::SubnetNameAlreadyExists
        );
        ensure!(
            Self::is_subnet_founder(netuid, &key),
            Error::<T>::NotFounder
        );

        Self::remove_subnet(netuid);
        // --- 16. Ok and done.
        Ok(())
    }

    pub fn do_update_subnet(
        origin: T::RuntimeOrigin,
        netuid: u16,
        params: SubnetParams<T>,
    ) -> DispatchResult {
        let key = ensure_signed(origin)?;
        // only the founder can update the network on authority mode

        ensure!(
            Self::get_vote_mode_subnet(netuid) == AUTHORITY_MODE,
            Error::<T>::NotAuthorityMode
        );
        ensure!(
            Self::if_subnet_netuid_exists(netuid),
            Error::<T>::SubnetNameAlreadyExists
        );
        ensure!(
            Self::is_subnet_founder(netuid, &key),
            Error::<T>::NotFounder
        );
        ensure!(
            Self::if_subnet_netuid_exists(netuid),
            Error::<T>::SubnetNameAlreadyExists
        );
        ensure!(
            Self::is_subnet_founder(netuid, &key),
            Error::<T>::NotFounder
        );

        Self::set_subnet_params(netuid, params);

        Self::deposit_event(Event::SubnetParamsUpdated(netuid));

        // --- 16. Ok and done.
        Ok(())
    }

    pub fn subnet_params(netuid: u16) -> SubnetParams<T> {
        SubnetParams {
            immunity_period: ImmunityPeriod::<T>::get(netuid),
            min_allowed_weights: MinAllowedWeights::<T>::get(netuid),
            max_allowed_weights: MaxAllowedWeights::<T>::get(netuid),
            max_allowed_uids: MaxAllowedUids::<T>::get(netuid),
            max_stake: MaxStake::<T>::get(netuid),
            max_weight_age: MaxWeightAge::<T>::get(netuid),
            min_stake: MinStake::<T>::get(netuid),
            tempo: Tempo::<T>::get(netuid),
            name: SubnetNames::<T>::get(netuid),
            vote_threshold: VoteThresholdSubnet::<T>::get(netuid),
            vote_mode: VoteModeSubnet::<T>::get(netuid),
            trust_ratio: TrustRatio::<T>::get(netuid),
            founder_share: FounderShare::<T>::get(netuid),
            incentive_ratio: IncentiveRatio::<T>::get(netuid),
            founder: Founder::<T>::get(netuid),
        }
    }

    pub fn check_subnet_params(params: &SubnetParams<T>) -> DispatchResult {
        // checks if params are valid

        let global_params = Self::global_params();

        // check valid tempo
        ensure!(
            params.min_allowed_weights <= params.max_allowed_weights,
            Error::<T>::InvalidMinAllowedWeights
        );

        ensure!(
            params.min_allowed_weights >= 1,
            Error::<T>::InvalidMinAllowedWeights
        );

        ensure!(
            params.max_allowed_weights <= global_params.max_allowed_weights,
            Error::<T>::InvalidMaxAllowedWeights
        );

        // the  global params must be larger than the global min_stake
        ensure!(
            params.min_stake >= global_params.min_stake,
            Error::<T>::InvalidMinStake
        );

        ensure!(
            params.max_stake > params.min_stake,
            Error::<T>::InvalidMaxStake
        );

        ensure!(params.tempo > 0, Error::<T>::InvalidTempo);

        ensure!(
            params.max_weight_age > params.tempo as u64,
            Error::<T>::InvalidMaxWeightAge
        );

        // ensure the trust_ratio is between 0 and 100
        ensure!(params.trust_ratio <= 100, Error::<T>::InvalidTrustRatio);

        // ensure the vode_mode is in "authority", "stake"
        ensure!(
            params.vote_mode.clone() == AUTHORITY_MODE || params.vote_mode.clone() == STAKE_MODE,
            Error::<T>::InvalidVoteMode
        );

        ensure!(
            params.immunity_period > 0,
            Error::<T>::InvalidImmunityPeriod
        );

        ensure!(
            params.max_allowed_uids > 0,
            Error::<T>::InvalidMaxAllowedUids
        );

        ensure!(
            params.vote_threshold <= 100,
            Error::<T>::InvalidVoteThreshold
        );

        ensure!(params.founder_share <= 100, Error::<T>::InvalidFounderShare);

        ensure!(
            params.incentive_ratio <= 100,
            Error::<T>::InvalidIncentiveRatio
        );

        Ok(())
    }

    pub fn set_subnet_params(netuid: u16, params: SubnetParams<T>) {
        // Check if the params are valid
        Self::check_subnet_params(&params).expect("subnet params are invalid");

        Self::set_founder(netuid, params.founder);
        Self::set_founder_share(netuid, params.founder_share);
        Self::set_tempo(netuid, params.tempo);
        Self::set_immunity_period(netuid, params.immunity_period);
        Self::set_max_allowed_weights(netuid, params.max_allowed_weights);
        Self::set_max_allowed_uids(netuid, params.max_allowed_uids);
        Self::set_max_stake(netuid, params.max_stake);
        Self::set_max_weight_age(netuid, params.max_weight_age);
        Self::set_min_allowed_weights(netuid, params.min_allowed_weights);
        Self::set_min_stake(netuid, params.min_stake);
        Self::set_name_subnet(netuid, params.name);
        Self::set_trust_ratio(netuid, params.trust_ratio);
        Self::set_vote_threshold_subnet(netuid, params.vote_threshold);
        Self::set_vote_mode_subnet(netuid, params.vote_mode);
        Self::set_incentive_ratio(netuid, params.incentive_ratio);
    }

    pub fn if_subnet_exist(netuid: u16) -> bool {
        N::<T>::contains_key(netuid)
    }

    #[cfg(debug_assertions)]
    pub fn get_min_stake(netuid: u16) -> u64 {
        MinStake::<T>::get(netuid)
    }

    pub fn set_min_stake(netuid: u16, stake: u64) {
        MinStake::<T>::insert(netuid, stake)
    }

    pub fn set_max_stake(netuid: u16, stake: u64) {
        MaxStake::<T>::insert(netuid, stake)
    }

    // get the least staked network
    pub fn least_staked_netuid() -> (u16, u64) {
        let mut min_stake: u64 = u64::MAX;
        let mut min_stake_netuid: u16 = Self::get_global_max_allowed_subnets() - 1;
        for (netuid, net_stake) in <TotalStake<T> as IterableStorageMap<u16, u64>>::iter() {
            if net_stake <= min_stake {
                min_stake = net_stake;
                min_stake_netuid = netuid;
            }
        }
        (min_stake_netuid, min_stake)
    }

    pub fn address_vector(netuid: u16) -> Vec<Vec<u8>> {
        let mut addresses: Vec<Vec<u8>> = Vec::new();
        for (_uid, address) in
            <Address<T> as IterableStorageDoubleMap<u16, u16, Vec<u8>>>::iter_prefix(netuid)
        {
            addresses.push(address);
        }
        addresses
    }

    pub fn name_vector(netuid: u16) -> Vec<Vec<u8>> {
        let mut names: Vec<Vec<u8>> = Vec::new();
        for (_uid, name) in
            <Name<T> as IterableStorageDoubleMap<u16, u16, Vec<u8>>>::iter_prefix(netuid)
        {
            names.push(name);
        }
        names
    }

    pub fn set_max_allowed_uids(netuid: u16, mut max_allowed_uids: u16) {
        let n: u16 = Self::get_subnet_n(netuid);
        if max_allowed_uids < n {
            // limit it at 256 at a time

            let mut remainder_n: u16 = n - max_allowed_uids;
            let max_remainder = 256;
            if remainder_n > max_remainder {
                // remove the modules in small amounts, as this can be a heavy load on the chain
                remainder_n = max_remainder;
                max_allowed_uids = n - remainder_n;
            }
            // remove the modules by adding the to the deregister queue
            for i in 0..remainder_n {
                let next_uid: u16 = n - 1 - i;
                Self::remove_module(netuid, next_uid);
            }
        }

        MaxAllowedUids::<T>::insert(netuid, max_allowed_uids);
    }

    pub fn set_name_subnet(netuid: u16, name: Vec<u8>) {
        // set the name if it doesnt exist
        if !Self::subnet_name_exists(name.clone()) {
            SubnetNames::<T>::insert(netuid, name.clone());
        }
    }

    pub fn default_subnet_params() -> SubnetParams<T> {
        // get an invalid
        let default_netuid: u16 = Self::num_subnets() + 1;
        Self::subnet_params(default_netuid)
    }

    #[cfg(debug_assertions)]
    pub fn subnet_info(netuid: u16) -> SubnetInfo<T> {
        let subnet_params: SubnetParams<T> = Self::subnet_params(netuid);
        SubnetInfo {
            params: subnet_params,
            netuid,
            stake: TotalStake::<T>::get(netuid),
            emission: SubnetEmission::<T>::get(netuid),
            n: N::<T>::get(netuid),
            founder: Founder::<T>::get(netuid),
        }
    }

    // TODO: see if we can optimize this further
    pub fn does_module_name_exist(netuid: u16, name: &[u8]) -> bool {
        <Name<T> as IterableStorageDoubleMap<u16, u16, Vec<u8>>>::iter_prefix(netuid)
            .any(|(_, existing)| existing == name)
    }

    pub fn is_subnet_founder(netuid: u16, key: &T::AccountId) -> bool {
        Founder::<T>::get(netuid) == *key
    }

    pub fn get_unit_emission() -> u64 {
        UnitEmission::<T>::get()
    }

    pub fn set_unit_emission(unit_emission: u64) {
        UnitEmission::<T>::put(unit_emission)
    }

    // Returns the total amount of stake in the staking table.
    pub fn get_total_emission_per_block() -> u64 {
        let total_stake: u64 = Self::total_stake();
        let unit_emission: u64 = Self::get_unit_emission();
        let mut emission_per_block: u64 = unit_emission; // assuming 8 second block times
        let halving_total_stake_checkpoints: Vec<u64> =
            [10_000_000, 20_000_000, 30_000_000, 40_000_000]
                .iter()
                .map(|x| x * unit_emission)
                .collect();
        for (i, having_stake) in halving_total_stake_checkpoints.iter().enumerate() {
            let halving_factor = 2u64.pow((i) as u32);
            if total_stake < *having_stake {
                emission_per_block /= halving_factor;
                break;
            }
        }

        emission_per_block
    }

    #[cfg(debug_assertions)]
    pub fn get_total_subnet_balance(netuid: u16) -> u64 {
        let keys = Self::get_keys(netuid);
        return keys.iter().map(|x| Self::get_balance_u64(x)).sum();
    }

    pub fn calculate_network_emission(netuid: u16) -> u64 {
        let subnet_stake: I64F64 = I64F64::from_num(Self::get_total_subnet_stake(netuid));
        let total_stake: I64F64 = I64F64::from_num(Self::total_stake());

        let subnet_ratio = if total_stake > I64F64::from_num(0) {
            subnet_stake / total_stake
        } else {
            let n = TotalSubnets::<T>::get();
            if n > 1 {
                I64F64::from_num(1) / I64F64::from_num(n)
            } else {
                // n == 1
                I64F64::from_num(1)
            }
        };

        let total_emission_per_block: u64 = Self::get_total_emission_per_block();
        let token_emission: u64 =
            (subnet_ratio * I64F64::from_num(total_emission_per_block)).to_num::<u64>();

        SubnetEmission::<T>::insert(netuid, token_emission);

        token_emission
    }

    pub fn get_subnet_emission(netuid: u16) -> u64 {
        Self::calculate_network_emission(netuid)
    }

    pub fn add_subnet(params: SubnetParams<T>, netuid: Option<u16>) -> u16 {
        // --- 1. Enfnsure that the network name does not already exist.
        let netuid = netuid.unwrap_or_else(TotalSubnets::<T>::get);

        Self::set_subnet_params(netuid, params.clone());
        TotalSubnets::<T>::mutate(|n| *n += 1);
        N::<T>::insert(netuid, 0);

        // --- 6. Emit the new network event.
        Self::deposit_event(Event::NetworkAdded(netuid, params.name));

        netuid
    }

    // Initializes a new subnetwork under netuid with parameters.
    pub fn subnet_name_exists(name: Vec<u8>) -> bool {
        for (_netuid, _name) in <SubnetNames<T> as IterableStorageMap<u16, Vec<u8>>>::iter() {
            if _name == name {
                return true;
            }
        }
        false
    }

    pub fn if_subnet_netuid_exists(netuid: u16) -> bool {
        SubnetNames::<T>::contains_key(netuid)
    }

    pub fn get_netuid_for_name(name: &[u8]) -> Option<u16> {
        SubnetNames::<T>::iter().find(|(_, n)| n == name).map(|(id, _)| id)
    }

    pub fn get_subnet_name(netuid: u16) -> Vec<u8> {
        SubnetNames::<T>::get(netuid)
    }

    pub fn remove_netuid_stake_strorage(netuid: u16) {
        // --- 1. Erase network stake, and remove network from list of networks.
        for (key, _stated_amount) in
            <Stake<T> as IterableStorageDoubleMap<u16, T::AccountId, u64>>::iter_prefix(netuid)
        {
            Self::remove_stake_from_storage(netuid, &key);
        }
        // --- 4. Remove all stake.
        Stake::<T>::remove_prefix(netuid, None);
        TotalStake::<T>::remove(netuid);
    }

    pub fn remove_subnet(netuid: u16) -> u16 {
        // TODO: handle errors
        #![allow(unused_must_use)]

        // --- 2. Ensure the network to be removed exists.
        if !Self::if_subnet_exist(netuid) {
            return 0;
        }

        Self::remove_netuid_stake_strorage(netuid);

        SubnetNames::<T>::remove(netuid);
        Name::<T>::clear_prefix(netuid, u32::max_value(), None);
        Address::<T>::clear_prefix(netuid, u32::max_value(), None);
        Uids::<T>::clear_prefix(netuid, u32::max_value(), None);
        Keys::<T>::clear_prefix(netuid, u32::max_value(), None);

        // Remove consnesus vectors
        Weights::<T>::clear_prefix(netuid, u32::max_value(), None);
        Emission::<T>::remove(netuid);
        Incentive::<T>::remove(netuid);
        Dividends::<T>::remove(netuid);
        Trust::<T>::remove(netuid);
        LastUpdate::<T>::remove(netuid);
        DelegationFee::<T>::clear_prefix(netuid, u32::max_value(), None);
        RegistrationBlock::<T>::clear_prefix(netuid, u32::max_value(), None);

        // --- 2. Erase subnet parameters.
        Founder::<T>::remove(netuid);
        FounderShare::<T>::remove(netuid);
        ImmunityPeriod::<T>::remove(netuid);
        IncentiveRatio::<T>::remove(netuid);
        MaxAllowedUids::<T>::remove(netuid);
        MaxAllowedWeights::<T>::remove(netuid);
        MaxStake::<T>::remove(netuid);
        MinAllowedWeights::<T>::remove(netuid);
        MinStake::<T>::remove(netuid);
        SelfVote::<T>::remove(netuid);
        SubnetEmission::<T>::remove(netuid);
        Tempo::<T>::remove(netuid);
        TrustRatio::<T>::remove(netuid);
        VoteModeSubnet::<T>::remove(netuid);
        VoteThresholdSubnet::<T>::remove(netuid);

        // Adjust the total number of subnets. and remove the subnet from the list of subnets.
        N::<T>::remove(netuid);
        TotalSubnets::<T>::mutate(|val| *val -= 1);
        // --- 4. Emit the event.
        Self::deposit_event(Event::NetworkRemoved(netuid));

        netuid
    }

    // Returns the number of filled slots on a network.
    ///
    pub fn get_subnet_n(netuid: u16) -> u16 {
        N::<T>::get(netuid)
    }

    // Returns true if the uid is set on the network.
    //
    pub fn is_uid_exist_on_network(netuid: u16, uid: u16) -> bool {
        Keys::<T>::contains_key(netuid, uid)
    }

    // Returns true if the key holds a slot on the network.
    //
    pub fn is_key_registered_on_network(netuid: u16, key: &T::AccountId) -> bool {
        Uids::<T>::contains_key(netuid, key)
    }

    pub fn key_registered(netuid: u16, key: &T::AccountId) -> bool {
        Uids::<T>::contains_key(netuid, key)
    }

    pub fn is_key_registered_on_any_network(key: &T::AccountId) -> bool {
        for netuid in Self::netuids() {
            if Uids::<T>::contains_key(netuid, key) {
                return true;
            }
        }
        false
    }

    // Returs the key under the network uid as a Result. Ok if the uid is taken.
    //
    pub fn get_key_for_uid(netuid: u16, module_uid: u16) -> T::AccountId {
        Keys::<T>::try_get(netuid, module_uid).unwrap()
    }

    // Returns the uid of the key in the network as a Result. Ok if the key has a slot.
    //
    pub fn get_uid_for_key(netuid: u16, key: &T::AccountId) -> u16 {
        Uids::<T>::get(netuid, key).unwrap_or(0)
    }

    pub fn get_trust_ratio(netuid: u16) -> u16 {
        TrustRatio::<T>::get(netuid)
    }

    pub fn set_trust_ratio(netuid: u16, trust_ratio: u16) {
        TrustRatio::<T>::insert(netuid, trust_ratio);
    }

    /// Returns the stake of the uid on network or 0 if it doesnt exist.
    #[cfg(debug_assertions)]
    pub fn get_stake_for_uid(netuid: u16, module_uid: u16) -> u64 {
        Self::get_stake_for_key(netuid, &Self::get_key_for_uid(netuid, module_uid))
    }

    // we need to prefix the voting power by the network uid

    pub fn set_vote_threshold_subnet(netuid: u16, vote_threshold: u16) {
        VoteThresholdSubnet::<T>::insert(netuid, vote_threshold);
    }

    pub fn get_vote_mode_subnet(netuid: u16) -> Vec<u8> {
        VoteModeSubnet::<T>::get(netuid)
    }

    pub fn set_vote_mode_subnet(netuid: u16, vote_mode: Vec<u8>) {
        VoteModeSubnet::<T>::insert(netuid, vote_mode);
    }

    pub fn get_stake_for_key(netuid: u16, key: &T::AccountId) -> u64 {
        Stake::<T>::get(netuid, key)
    }

    // Return the total number of subnetworks available on the chain.
    //
    pub fn num_subnets() -> u16 {
        TotalSubnets::<T>::get()
    }

    pub fn netuids() -> Vec<u16> {
        <N<T> as IterableStorageMap<u16, u16>>::iter()
            .map(|(netuid, _)| netuid)
            .collect()
    }

    // ========================
    // ==== Global Setters ====
    // ========================
    // TEMPO (MIN IS 100)
    pub fn set_tempo(netuid: u16, tempo: u16) {
        Tempo::<T>::insert(netuid, tempo.max(100));
    }

    #[cfg(debug_assertions)]
    pub fn get_tempo(netuid: u16) -> u16 {
        Tempo::<T>::get(netuid).max(100)
    }

    // FOUNDER SHARE (MAX IS 100)
    pub fn set_founder_share(netuid: u16, founder_share: u16) {
        FounderShare::<T>::insert(netuid, founder_share.min(100));
    }
    pub fn get_founder_share(netuid: u16) -> u16 {
        FounderShare::<T>::get(netuid).min(100)
    }

    pub fn get_registration_block_for_uid(netuid: u16, uid: u16) -> u64 {
        RegistrationBlock::<T>::get(netuid, uid)
    }

    pub fn get_incentive_ratio(netuid: u16) -> u16 {
        IncentiveRatio::<T>::get(netuid).min(100)
    }
    pub fn set_incentive_ratio(netuid: u16, incentive_ratio: u16) {
        IncentiveRatio::<T>::insert(netuid, incentive_ratio.min(100));
    }

    pub fn get_founder(netuid: u16) -> T::AccountId {
        Founder::<T>::get(netuid)
    }

    pub fn set_founder(netuid: u16, founder: T::AccountId) {
        Founder::<T>::insert(netuid, founder);
    }

    #[cfg(debug_assertions)]
    pub fn get_burn_emission_per_epoch(netuid: u16) -> u64 {
        let burn_rate: u16 = BurnRate::<T>::get();
        let epoch_emission: u64 = Self::get_subnet_emission(netuid);
        let n: u16 = Self::get_subnet_n(netuid);
        // get the float and convert to u64
        if n == 0 {
            return 0;
        }
        let burn_rate_float: I64F64 = I64F64::from_num(burn_rate) / I64F64::from_num(n * 100);
        let burn_emission_per_epoch: u64 =
            (I64F64::from_num(epoch_emission) * burn_rate_float).to_num::<u64>();
        burn_emission_per_epoch
    }

    // ========================
    // ==== Global Getters ====
    // ========================
    pub fn get_current_block_number() -> u64 {
        TryInto::try_into(<frame_system::Pallet<T>>::block_number())
            .ok()
            .expect("blockchain will not exceed 2^64 blocks; QED.")
    }

    // Emission is the same as the Yomama params

    pub fn set_last_update_for_uid(netuid: u16, uid: u16, last_update: u64) {
        let mut updated_last_update_vec = Self::get_last_update(netuid);
        if (uid as usize) < updated_last_update_vec.len() {
            updated_last_update_vec[uid as usize] = last_update;
            LastUpdate::<T>::insert(netuid, updated_last_update_vec);
        }
    }

    #[cfg(debug_assertions)]
    pub fn get_emission_for_key(netuid: u16, key: &T::AccountId) -> u64 {
        let uid = Self::get_uid_for_key(netuid, key);
        Self::get_emission_for_uid(netuid, uid)
    }

    pub fn get_emission_for_uid(netuid: u16, uid: u16) -> u64 {
        Emission::<T>::get(netuid).get(uid as usize).copied().unwrap_or_default()
    }
    pub fn get_incentive_for_uid(netuid: u16, uid: u16) -> u16 {
        Incentive::<T>::get(netuid).get(uid as usize).copied().unwrap_or_default()
    }
    pub fn get_dividends_for_uid(netuid: u16, uid: u16) -> u16 {
        Dividends::<T>::get(netuid).get(uid as usize).copied().unwrap_or_default()
    }
    pub fn get_last_update_for_uid(netuid: u16, uid: u16) -> u64 {
        LastUpdate::<T>::get(netuid).get(uid as usize).copied().unwrap_or_default()
    }

    pub fn get_global_max_allowed_subnets() -> u16 {
        MaxAllowedSubnets::<T>::get()
    }
    pub fn set_global_max_allowed_subnets(max_allowed_subnets: u16) {
        MaxAllowedSubnets::<T>::put(max_allowed_subnets)
    }
    // ============================
    // ==== Subnetwork Getters ====
    // ============================
    pub fn get_pending_emission(netuid: u16) -> u64 {
        PendingEmission::<T>::get(netuid)
    }

    pub fn get_registrations_this_block() -> u16 {
        RegistrationsPerBlock::<T>::get()
    }

    pub fn get_module_registration_block(netuid: u16, uid: u16) -> u64 {
        RegistrationBlock::<T>::get(netuid, uid)
    }

    pub fn get_immunity_period(netuid: u16) -> u16 {
        ImmunityPeriod::<T>::get(netuid)
    }
    pub fn set_immunity_period(netuid: u16, immunity_period: u16) {
        ImmunityPeriod::<T>::insert(netuid, immunity_period);
    }

    pub fn get_min_allowed_weights(netuid: u16) -> u16 {
        let min_allowed_weights = MinAllowedWeights::<T>::get(netuid);
        let n = Self::get_subnet_n(netuid);
        // if n < min_allowed_weights, then return n
        if n < min_allowed_weights {
            n
        } else {
            min_allowed_weights
        }
    }
    pub fn set_min_allowed_weights(netuid: u16, min_allowed_weights: u16) {
        MinAllowedWeights::<T>::insert(netuid, min_allowed_weights);
    }

    pub fn get_max_allowed_weights(netuid: u16) -> u16 {
        let max_allowed_weights = MaxAllowedWeights::<T>::get(netuid);
        let n = Self::get_subnet_n(netuid);
        // if n < min_allowed_weights, then return n
        max_allowed_weights.min(n)
    }
    pub fn set_max_allowed_weights(netuid: u16, max_allowed_weights: u16) {
        let global_params = Self::global_params();
        MaxAllowedWeights::<T>::insert(
            netuid,
            max_allowed_weights.min(global_params.max_allowed_weights),
        );
    }

    pub fn get_max_allowed_uids(netuid: u16) -> u16 {
        MaxAllowedUids::<T>::get(netuid)
    }

    pub fn get_max_allowed_modules() -> u16 {
        MaxAllowedModules::<T>::get()
    }

    pub fn set_max_allowed_modules(max_allowed_modules: u16) {
        MaxAllowedModules::<T>::put(max_allowed_modules)
    }

    pub fn get_uids(netuid: u16) -> Vec<u16> {
        let n = Self::get_subnet_n(netuid);
        (0..n).collect()
    }
    pub fn get_keys(netuid: u16) -> Vec<T::AccountId> {
        let uids: Vec<u16> = Self::get_uids(netuid);
        let keys: Vec<T::AccountId> =
            uids.iter().map(|uid| Self::get_key_for_uid(netuid, *uid)).collect();
        keys
    }

    pub fn get_uid_key_tuples(netuid: u16) -> Vec<(u16, T::AccountId)> {
        let n = Self::get_subnet_n(netuid);
        let mut uid_key_tuples = Vec::<(u16, T::AccountId)>::new();
        for uid in 0..n {
            let key = Self::get_key_for_uid(netuid, uid);
            uid_key_tuples.push((uid, key));
        }
        uid_key_tuples
    }

    pub fn get_names(netuid: u16) -> Vec<Vec<u8>> {
        let mut names = Vec::<Vec<u8>>::new();
        for (_uid, name) in
            <Name<T> as IterableStorageDoubleMap<u16, u16, Vec<u8>>>::iter_prefix(netuid)
        {
            names.push(name);
        }
        names
    }

    pub fn get_addresses(netuid: u16) -> Vec<T::AccountId> {
        let mut addresses = Vec::<T::AccountId>::new();
        for (key, _uid) in
            <Uids<T> as IterableStorageDoubleMap<u16, T::AccountId, u16>>::iter_prefix(netuid)
        {
            addresses.push(key);
        }
        addresses
    }

    #[cfg(debug_assertions)]
    pub fn check_subnet_storage(netuid: u16) -> bool {
        let n = Self::get_subnet_n(netuid);
        let uids = Self::get_uids(netuid);
        let keys = Self::get_keys(netuid);
        let names = Self::get_names(netuid);
        let addresses = Self::get_addresses(netuid);
        let emissions = Self::get_emissions(netuid);
        let incentives = Self::get_incentives(netuid);
        let dividends = Self::get_dividends(netuid);
        let last_update = Self::get_last_update(netuid);

        if (n as usize) != uids.len() {
            return false;
        }
        if (n as usize) != keys.len() {
            return false;
        }
        if (n as usize) != names.len() {
            return false;
        }
        if (n as usize) != addresses.len() {
            return false;
        }
        if (n as usize) != emissions.len() {
            return false;
        }
        if (n as usize) != incentives.len() {
            return false;
        }
        if (n as usize) != dividends.len() {
            return false;
        }
        if (n as usize) != last_update.len() {
            return false;
        }

        // length of addresss
        let name_vector = Self::name_vector(netuid);
        if (n as usize) != name_vector.len() {
            return false;
        }

        // length of addresss
        let address_vector = Self::address_vector(netuid);
        if (n as usize) != address_vector.len() {
            return false;
        }

        true
    }

    #[cfg(debug_assertions)]
    pub fn get_trust(netuid: u16) -> Vec<u16> {
        Trust::<T>::get(netuid)
    }

    pub fn get_emissions(netuid: u16) -> Vec<u64> {
        Emission::<T>::get(netuid)
    }
    pub fn get_incentives(netuid: u16) -> Vec<u16> {
        Incentive::<T>::get(netuid)
    }

    pub fn get_dividends(netuid: u16) -> Vec<u16> {
        Dividends::<T>::get(netuid)
    }
    pub fn get_last_update(netuid: u16) -> Vec<u64> {
        LastUpdate::<T>::get(netuid)
    }

    pub fn is_registered(netuid: u16, key: &T::AccountId) -> bool {
        Uids::<T>::contains_key(netuid, key)
    }

    pub fn set_max_weight_age(netuid: u16, max_weight_age: u64) {
        MaxWeightAge::<T>::insert(netuid, max_weight_age);
    }
}
