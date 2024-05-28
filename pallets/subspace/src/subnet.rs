use super::*;

use frame_support::{
    pallet_prelude::DispatchResult, storage::IterableStorageMap, IterableStorageDoubleMap,
};

use self::voting::VoteMode;
use sp_arithmetic::per_things::Percent;
use sp_runtime::DispatchError;
use sp_std::vec::Vec;
use substrate_fixed::types::I64F64;

#[derive(Debug)]
pub struct SubnetChangeset<T: Config> {
    params: SubnetParams<T>,
}

impl<T: Config> SubnetChangeset<T> {
    pub fn new(params: SubnetParams<T>) -> Result<Self, DispatchError> {
        Self::validate_params(None, &params)?;
        Ok(Self { params })
    }

    pub fn update(netuid: u16, params: SubnetParams<T>) -> Result<Self, DispatchError> {
        Self::validate_params(Some(netuid), &params)?;
        Ok(Self { params })
    }

    pub fn apply(self, netuid: u16) -> Result<(), sp_runtime::DispatchError> {
        Self::validate_params(Some(netuid), &self.params)?;

        SubnetNames::<T>::insert(netuid, &self.params.name);
        Founder::<T>::insert(netuid, &self.params.founder);
        FounderShare::<T>::insert(netuid, self.params.founder_share);
        Tempo::<T>::insert(netuid, self.params.tempo);
        ImmunityPeriod::<T>::insert(netuid, self.params.immunity_period);
        MaxAllowedWeights::<T>::insert(netuid, self.params.max_allowed_weights);
        Pallet::<T>::set_max_allowed_uids(netuid, self.params.max_allowed_uids);
        MaxStake::<T>::insert(netuid, self.params.max_stake);
        MaxWeightAge::<T>::insert(netuid, self.params.max_weight_age);
        MinAllowedWeights::<T>::insert(netuid, self.params.min_allowed_weights);
        MinStake::<T>::insert(netuid, self.params.min_stake);
        TrustRatio::<T>::insert(netuid, self.params.trust_ratio);
        IncentiveRatio::<T>::insert(netuid, self.params.incentive_ratio);
        VoteModeSubnet::<T>::insert(netuid, self.params.vote_mode);

        if self.params.maximum_set_weight_calls_per_epoch == 0 {
            MaximumSetWeightCallsPerEpoch::<T>::remove(netuid);
        } else {
            MaximumSetWeightCallsPerEpoch::<T>::insert(
                netuid,
                self.params.maximum_set_weight_calls_per_epoch,
            );
        }

        Pallet::<T>::deposit_event(Event::SubnetParamsUpdated(netuid));

        Ok(())
    }

    pub fn validate_params(netuid: Option<u16>, params: &SubnetParams<T>) -> DispatchResult {
        // checks if params are valid

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
            params.max_stake > params.min_stake,
            Error::<T>::InvalidMaxStake
        );

        // lower tempos might significantly slow down the chain
        ensure!(params.tempo >= 25, Error::<T>::InvalidTempo);

        ensure!(
            params.max_weight_age > params.tempo as u64,
            Error::<T>::InvalidMaxWeightAge
        );

        // ensure the trust_ratio is between 0 and 100
        ensure!(params.trust_ratio <= 100, Error::<T>::InvalidTrustRatio);

        ensure!(
            params.max_allowed_uids > 0,
            Error::<T>::InvalidMaxAllowedUids
        );

        ensure!(params.founder_share <= 100, Error::<T>::InvalidFounderShare);

        ensure!(
            params.founder_share >= FloorFounderShare::<T>::get() as u16,
            Error::<T>::InvalidFounderShare
        );

        ensure!(
            params.incentive_ratio <= 100,
            Error::<T>::InvalidIncentiveRatio
        );

        ensure!(
            params.max_allowed_weights <= Pallet::<T>::get_max_allowed_weights_global(),
            Error::<T>::InvalidMaxAllowedWeights
        );

        ensure!(
            params.min_stake >= Pallet::<T>::get_min_stake_global(),
            Error::<T>::InvalidMinStake
        );

        match Pallet::<T>::get_netuid_for_name(&params.name) {
            Some(id) if netuid.is_some_and(|netuid| netuid == id) => { /* subnet kept same name */ }
            Some(_) => return Err(Error::<T>::SubnetNameAlreadyExists.into()),
            None => {
                let name = &params.name;
                let min = MinNameLength::<T>::get() as usize;
                let max = MaxNameLength::<T>::get() as usize;
                ensure!(!name.is_empty(), Error::<T>::InvalidSubnetName);
                ensure!(name.len() >= min, Error::<T>::SubnetNameTooShort);
                ensure!(name.len() <= max, Error::<T>::SubnetNameTooLong);
                core::str::from_utf8(name).map_err(|_| Error::<T>::InvalidSubnetName)?;
            }
        }

        Ok(())
    }
}

impl<T: Config> Pallet<T> {
    #[cfg(debug_assertions)]
    pub fn do_remove_subnet(origin: T::RuntimeOrigin, netuid: u16) -> DispatchResult {
        let key = ensure_signed(origin)?;

        ensure!(
            Self::if_subnet_netuid_exists(netuid),
            Error::<T>::NetuidDoesNotExist
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
        changeset: SubnetChangeset<T>,
    ) -> DispatchResult {
        let key = ensure_signed(origin)?;

        // only the founder can update the network on authority mode
        ensure!(
            Self::is_subnet_founder(netuid, &key),
            Error::<T>::NotFounder
        );

        // Ensure that the subnet is not in a `Vote` mode.
        // Update by founder can be executed only in `Authority` mode.
        ensure!(
            VoteModeSubnet::<T>::get(netuid) == VoteMode::Authority,
            Error::<T>::InvalidVoteMode
        );

        ensure!(
            Self::if_subnet_netuid_exists(netuid),
            Error::<T>::NetuidDoesNotExist
        );

        // apply the changeset
        changeset.apply(netuid)?;

        // --- 16. Ok and done.
        Ok(())
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

        // lower tempos might significantly slow down the chain
        ensure!(params.tempo >= 25, Error::<T>::InvalidTempo);

        ensure!(
            params.max_weight_age > params.tempo as u64,
            Error::<T>::InvalidMaxWeightAge
        );

        // ensure the trust_ratio is between 0 and 100
        ensure!(params.trust_ratio <= 100, Error::<T>::InvalidTrustRatio);

        ensure!(
            params.max_allowed_uids > 0,
            Error::<T>::InvalidMaxAllowedUids
        );

        ensure!(params.founder_share <= 100, Error::<T>::InvalidFounderShare);

        ensure!(
            params.founder_share >= global_params.floor_founder_share as u16,
            Error::<T>::InvalidFounderShare
        );

        ensure!(
            params.incentive_ratio <= 100,
            Error::<T>::InvalidIncentiveRatio
        );

        Ok(())
    }

    pub fn subnet_params(netuid: u16) -> SubnetParams<T> {
        SubnetParams {
            founder: Founder::<T>::get(netuid),
            founder_share: FounderShare::<T>::get(netuid),
            tempo: Tempo::<T>::get(netuid),
            immunity_period: ImmunityPeriod::<T>::get(netuid),
            max_allowed_weights: MaxAllowedWeights::<T>::get(netuid),
            max_allowed_uids: MaxAllowedUids::<T>::get(netuid),
            max_stake: MaxStake::<T>::get(netuid),
            max_weight_age: MaxWeightAge::<T>::get(netuid),
            min_allowed_weights: MinAllowedWeights::<T>::get(netuid),
            min_stake: MinStake::<T>::get(netuid),
            name: SubnetNames::<T>::get(netuid),
            trust_ratio: TrustRatio::<T>::get(netuid),
            incentive_ratio: IncentiveRatio::<T>::get(netuid),
            maximum_set_weight_calls_per_epoch: MaximumSetWeightCallsPerEpoch::<T>::get(netuid),
            vote_mode: VoteModeSubnet::<T>::get(netuid),
            bonds_ma: BondsMovingAverage::<T>::get(netuid),
            target_registrations_interval: TargetRegistrationsInterval::<T>::get(netuid),
            target_registrations_per_interval: TargetRegistrationsPerInterval::<T>::get(netuid),
            max_registrations_per_interval: MaxRegistrationsPerInterval::<T>::get(netuid),
        }
    }

    pub fn if_subnet_exist(netuid: u16) -> bool {
        N::<T>::contains_key(netuid)
    }

    // stake
    #[cfg(debug_assertions)]
    pub fn get_min_stake(netuid: u16) -> u64 {
        MinStake::<T>::get(netuid)
    }

    // registrations
    pub fn get_registrations_this_interval(netuid: u16) -> u16 {
        RegistrationsThisInterval::<T>::get(netuid)
    }

    pub fn set_registrations_this_interval(netuid: u16, registrations: u16) {
        RegistrationsThisInterval::<T>::insert(netuid, registrations);
    }

    pub fn get_burn(netuid: u16) -> u64 {
        Burn::<T>::get(netuid)
    }

    pub fn set_burn(netuid: u16, burn: u64) {
        Burn::<T>::insert(netuid, burn);
    }

    // get the least staked network
    pub fn least_staked_netuid() -> (u16, u64) {
        TotalStake::<T>::iter().min_by_key(|(_, stake)| *stake).unwrap_or_else(|| {
            let stake = u64::MAX;
            let netuid = Self::get_global_max_allowed_subnets() - 1;
            (netuid, stake)
        })
    }

    pub fn address_vector(netuid: u16) -> Vec<Vec<u8>> {
        Address::<T>::iter_prefix_values(netuid).collect()
    }

    pub fn name_vector(netuid: u16) -> Vec<Vec<u8>> {
        Name::<T>::iter_prefix_values(netuid).collect()
    }

    fn set_max_allowed_uids(netuid: u16, mut max_allowed_uids: u16) {
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
    // TODO: refactor the halving logic, it's not correct.
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
            let halving_factor: u64 = 2u64.pow((i) as u32);
            if total_stake < *having_stake {
                emission_per_block /= halving_factor;
                break;
            }
        }

        emission_per_block
    }

    #[cfg(debug_assertions)]
    // TODO: ger rid of this fn
    pub fn get_total_subnet_balance(netuid: u16) -> u64 {
        let keys = Self::get_keys(netuid);
        return keys.iter().map(|x| Self::get_balance_u64(x)).sum();
    }

    /// Empties out all:
    /// emission, dividends, incentives, trust on the specific netuid.
    fn deactivate_subnet(netuid: u16) {
        let module_count = Self::get_subnet_n(netuid) as usize;
        let zeroed = vec![0; module_count];

        SubnetEmission::<T>::insert(netuid, 0);

        Active::<T>::insert(netuid, vec![true; module_count]);
        Consensus::<T>::insert(netuid, &zeroed);
        Dividends::<T>::insert(netuid, &zeroed);
        Emission::<T>::insert(netuid, vec![0; module_count]);
        Incentive::<T>::insert(netuid, &zeroed);
        PruningScores::<T>::insert(netuid, &zeroed);
        Rank::<T>::insert(netuid, &zeroed);
        Trust::<T>::insert(netuid, &zeroed);
        ValidatorPermits::<T>::insert(netuid, vec![false; module_count]);
        ValidatorTrust::<T>::insert(netuid, &zeroed);
    }

    pub fn calculate_network_emission(netuid: u16, subnet_stake_threshold: Percent) -> u64 {
        let subnet_stake: I64F64 = I64F64::from_num(Self::get_total_subnet_stake(netuid));
        let total_stake: I64F64 = Self::adjust_total_stake(subnet_stake_threshold);

        log::trace!(
            "calculating rewards for netuid {netuid} with stake {subnet_stake:?},
        total stake {total_stake:?},
        threshold {subnet_stake_threshold:?}"
        );

        let subnet_ratio = if total_stake > I64F64::from_num(0) {
            subnet_stake / total_stake
        } else {
            I64F64::from_num(0)
        };

        log::trace!("subnet ratio: {subnet_ratio:?}");

        // Convert subnet_stake_threshold from % to I64F64
        let subnet_stake_threshold_i64f64 =
            I64F64::from_num(subnet_stake_threshold.deconstruct()) / I64F64::from_num(100);

        log::trace!("subnet_stake_threshold_i64f64: {subnet_stake_threshold_i64f64:?}");
        // Check if subnet_ratio meets the subnet_stake_threshold,
        // or netuid is not the general subnet
        if subnet_ratio < subnet_stake_threshold_i64f64 && netuid != 0 {
            // Return early if the threshold is not met,
            // this prevents emission gapping, of subnets that don't meet emission threshold
            Self::deactivate_subnet(netuid);
            return 0;
        }

        let total_emission_per_block: u64 = Self::get_total_emission_per_block();

        log::trace!("total_emission_per_block: {total_emission_per_block}");
        let token_emission: u64 =
            (subnet_ratio * I64F64::from_num(total_emission_per_block)).to_num::<u64>();

        log::trace!("token_emission: {token_emission}");
        SubnetEmission::<T>::insert(netuid, token_emission);

        token_emission
    }

    // This is the total stake of the network without subnets that can not get emission
    // TODO: could be optimized
    pub fn adjust_total_stake(subnet_stake_threshold: Percent) -> I64F64 {
        let total_global_stake = I64F64::from_num(Self::total_stake());
        log::trace!("total_global_stake: {total_global_stake}");
        if total_global_stake == 0 {
            return I64F64::from_num(0);
        }

        let mut total_stake = I64F64::from_num(0);
        let subnet_stake_threshold_i64f64 =
            I64F64::from_num(subnet_stake_threshold.deconstruct()) / I64F64::from_num(100);
        // Iterate over all subnets
        for netuid in N::<T>::iter_keys() {
            let subnet_stake: I64F64 = I64F64::from_num(Self::get_total_subnet_stake(netuid));
            if subnet_stake == 0 {
                continue;
            }
            log::trace!("subnet_stake: {subnet_stake}");
            let subnet_ratio = subnet_stake / total_global_stake;
            log::trace!("subnet_ratio: {subnet_ratio}");
            // Check if subnet_ratio meets the subnet_stake_threshold,
            // or the netuid is the general subnet
            if subnet_ratio >= subnet_stake_threshold_i64f64 || netuid == 0 {
                // Add subnet_stake to total_stake if it meets the threshold
                total_stake += subnet_stake;
            }
        }

        total_stake
    }

    pub fn add_subnet(
        changeset: SubnetChangeset<T>,
        netuid: Option<u16>,
    ) -> Result<u16, DispatchError> {
        let netuid = netuid.unwrap_or_else(|| match RemovedSubnets::<T>::get().first().copied() {
            Some(removed) => removed,
            None => TotalSubnets::<T>::get(),
        });

        let name = changeset.params.name.clone();
        changeset.apply(netuid)?;
        TotalSubnets::<T>::mutate(|n| *n += 1);
        N::<T>::insert(netuid, 0);
        SubnetEmission::<T>::insert(netuid, 0);

        // Insert the minimum burn to the netuid,
        // to prevent free registrations the first target registration interval.
        let min_burn = Self::get_min_burn();
        Burn::<T>::insert(netuid, min_burn);

        RemovedSubnets::<T>::mutate(|subnets| subnets.remove(&netuid));

        // --- 6. Emit the new network event.
        Self::deposit_event(Event::NetworkAdded(netuid, name));

        Ok(netuid)
    }
    // Initializes a new subnetwork under netuid with parameters.
    pub fn subnet_name_exists(name: Vec<u8>) -> bool {
        for (_, _name) in <SubnetNames<T> as IterableStorageMap<u16, Vec<u8>>>::iter() {
            if _name == name {
                return true;
            }
        }
        false
    }

    pub fn if_subnet_netuid_exists(netuid: u16) -> bool {
        SubnetNames::<T>::contains_key(netuid)
    }

    pub fn does_subnet_name_exist(name: &[u8]) -> bool {
        SubnetNames::<T>::iter().any(|(_, n)| n == name)
    }

    pub fn get_netuid_for_name(name: &[u8]) -> Option<u16> {
        SubnetNames::<T>::iter().find(|(_, n)| n == name).map(|(id, _)| id)
    }

    pub fn remove_netuid_stake_strorage(netuid: u16) {
        // --- 1. Erase network stake, and remove network from list of networks.
        for key in Stake::<T>::iter_key_prefix(netuid) {
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
        MaxWeightAge::<T>::remove(netuid);
        Name::<T>::clear_prefix(netuid, u32::max_value(), None);
        Address::<T>::clear_prefix(netuid, u32::max_value(), None);
        Metadata::<T>::clear_prefix(netuid, u32::max_value(), None);
        Uids::<T>::clear_prefix(netuid, u32::max_value(), None);
        Keys::<T>::clear_prefix(netuid, u32::max_value(), None);
        DelegationFee::<T>::clear_prefix(netuid, u32::max_value(), None);

        // Remove consnesus vectors
        Weights::<T>::clear_prefix(netuid, u32::max_value(), None);

        Active::<T>::remove(netuid);
        Consensus::<T>::remove(netuid);
        Dividends::<T>::remove(netuid);
        Emission::<T>::remove(netuid);
        Incentive::<T>::remove(netuid);
        LastUpdate::<T>::remove(netuid);
        PruningScores::<T>::remove(netuid);
        Rank::<T>::remove(netuid);
        Trust::<T>::remove(netuid);
        ValidatorPermits::<T>::remove(netuid);
        ValidatorTrust::<T>::remove(netuid);

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

        // Adjust the total number of subnets. and remove the subnet from the list of subnets.
        N::<T>::remove(netuid);
        TotalSubnets::<T>::mutate(|val| *val -= 1);
        RemovedSubnets::<T>::mutate(|subnets| subnets.insert(netuid));

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

    pub fn key_registered(netuid: u16, key: &T::AccountId) -> bool {
        Uids::<T>::contains_key(netuid, key)
            || Keys::<T>::iter_prefix_values(netuid).any(|k| &k == key)
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
    pub fn get_key_for_uid(netuid: u16, module_uid: u16) -> Option<T::AccountId> {
        Keys::<T>::try_get(netuid, module_uid).ok()
    }

    // Returns the uid of the key in the network as a Result. Ok if the key has a slot.
    //
    pub fn get_uid_for_key(netuid: u16, key: &T::AccountId) -> u16 {
        Uids::<T>::get(netuid, key).unwrap_or(0)
    }

    pub fn get_trust_ratio(netuid: u16) -> u16 {
        TrustRatio::<T>::get(netuid)
    }

    /// Returns the stake of the uid on network or 0 if it doesnt exist.
    #[cfg(debug_assertions)]
    pub fn get_stake_for_uid(netuid: u16, module_uid: u16) -> u64 {
        let Some(key) = Self::get_key_for_uid(netuid, module_uid) else {
            return 0;
        };
        Self::get_stake_for_key(netuid, &key)
    }

    pub fn get_stake_for_key(netuid: u16, key: &T::AccountId) -> u64 {
        Stake::<T>::get(netuid, key)
    }

    // Return the total number of subnetworks available on the chain.
    pub fn num_subnets() -> u16 {
        TotalSubnets::<T>::get()
    }

    pub fn netuids() -> Vec<u16> {
        <N<T> as IterableStorageMap<u16, u16>>::iter()
            .map(|(netuid, _)| netuid)
            .collect()
    }

    pub fn get_tempo(netuid: u16) -> u16 {
        Tempo::<T>::get(netuid)
    }

    // FOUNDER SHARE (MAX IS 100)
    pub fn get_founder_share(netuid: u16) -> u16 {
        FounderShare::<T>::get(netuid).min(100)
    }

    pub fn get_registration_block_for_uid(netuid: u16, uid: u16) -> u64 {
        RegistrationBlock::<T>::get(netuid, uid)
    }

    pub fn get_incentive_ratio(netuid: u16) -> u16 {
        IncentiveRatio::<T>::get(netuid).min(100)
    }

    pub fn get_founder(netuid: u16) -> T::AccountId {
        Founder::<T>::get(netuid)
    }

    #[cfg(debug_assertions)]
    pub fn get_burn_emission_per_epoch(netuid: u16) -> u64 {
        let burn_rate: u16 = BurnRate::<T>::get();
        let threshold: Percent = SubnetStakeThreshold::<T>::get();
        let epoch_emission: u64 = Self::calculate_network_emission(netuid, threshold);
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

    #[cfg(debug_assertions)]
    pub fn get_pending_emission(netuid: u16) -> u64 {
        PendingEmission::<T>::get(netuid)
    }

    #[cfg(debug_assertions)]
    pub fn get_registrations_this_block() -> u16 {
        RegistrationsPerBlock::<T>::get()
    }

    pub fn get_module_registration_block(netuid: u16, uid: u16) -> u64 {
        RegistrationBlock::<T>::get(netuid, uid)
    }

    pub fn get_immunity_period(netuid: u16) -> u16 {
        ImmunityPeriod::<T>::get(netuid)
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

    pub fn get_max_allowed_weights(netuid: u16) -> u16 {
        let max_allowed_weights = MaxAllowedWeights::<T>::get(netuid);
        let n = Self::get_subnet_n(netuid);
        // if n < min_allowed_weights, then return n
        max_allowed_weights.min(n)
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
            uids.iter().map(|uid| Self::get_key_for_uid(netuid, *uid).unwrap()).collect();
        keys
    }

    pub fn get_uid_key_tuples(netuid: u16) -> Vec<(u16, T::AccountId)> {
        let n = Self::get_subnet_n(netuid);
        let mut uid_key_tuples = Vec::<(u16, T::AccountId)>::new();
        for uid in 0..n {
            let key = Self::get_key_for_uid(netuid, uid).unwrap();
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
}
