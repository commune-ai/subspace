use crate::voting::AUTHORITY_MODE;
// TODO: deposit events on sets

use super::*;
use frame_support::pallet_prelude::DispatchResult;
use sp_arithmetic::per_things::Percent;

use system::ensure_root;

impl<T: Config> Pallet<T> {
    pub fn global_params() -> GlobalParams {
        GlobalParams {
            max_name_length: Self::get_global_max_name_length(),
            max_allowed_subnets: Self::get_global_max_allowed_subnets(),
            max_allowed_modules: Self::get_max_allowed_modules(),
            max_registrations_per_block: Self::get_max_registrations_per_block(),
            target_registrations_interval: Self::get_target_registrations_interval(),
            target_registrations_per_interval: Self::get_target_registrations_per_interval(),
            unit_emission: Self::get_unit_emission(),
            tx_rate_limit: Self::get_tx_rate_limit(),
            vote_threshold: Self::get_global_vote_threshold(),
            max_proposals: Self::get_max_proposals(),
            vote_mode: Self::get_vote_mode_global(),
            burn_rate: Self::get_burn_rate(),
            min_burn: Self::get_min_burn(),
            max_burn: Self::get_max_burn(),
            adjustment_alpha: Self::get_adjustment_alpha(),
            min_stake: Self::get_min_stake_global(),
            min_delegation_fee: Self::get_min_deleg_fee_global(),
            min_weight_stake: Self::get_min_weight_stake(),
            max_allowed_weights: Self::get_max_allowed_weights_global(),
        }
    }

    // TODO: make sure there are checks for all values
    pub fn check_global_params(params: GlobalParams) -> DispatchResult {
        // checks if params are valid
        let og_params = Self::global_params();

        // check if the name already exists
        ensure!(params.max_name_length > 0, Error::<T>::InvalidMaxNameLength);

        ensure!(params.min_delegation_fee.deconstruct() <= 100, Error::<T>::InvalidMinDelegationFee);

        ensure!(
            params.max_allowed_subnets > 0,
            Error::<T>::InvalidMaxAllowedSubnets
        );

        ensure!(
            params.max_allowed_modules > 0,
            Error::<T>::InvalidMaxAllowedModules
        );

        ensure!(
            params.max_registrations_per_block > 0,
            Error::<T>::InvalidMaxRegistrationsPerBlock
        );

        ensure!(
            params.target_registrations_interval > 0,
            Error::<T>::InvalidTargetRegistrationsInterval
        );

        ensure!(
            params.vote_threshold < 100,
            Error::<T>::InvalidVoteThreshold
        );

        ensure!(params.max_proposals > 0, Error::<T>::InvalidMaxProposals);

        ensure!(
            params.unit_emission <= og_params.unit_emission,
            Error::<T>::InvalidUnitEmission
        );

        ensure!(params.tx_rate_limit > 0, Error::<T>::InvalidTxRateLimit);

        // Make sure that the burn rate is below 100%
        ensure!(params.burn_rate <= 100, Error::<T>::InvalidBurnRate);

        // Make sure that the burn rate is at least 0.1 $ COMAI, it can't be
        // zero, because the whole dynamic burn system would get broken.
        ensure!(params.min_burn >= 100_000_000, Error::<T>::InvalidMinBurn);

        // Make sure that the maximum burn is larger than minimum burn
        ensure!(
            params.max_burn > params.min_burn,
            Error::<T>::InvalidMaxBurn
        );

        Ok(())
    }

    pub fn set_global_params(params: GlobalParams) {
        Self::set_global_max_name_length(params.max_name_length);
        Self::set_global_max_allowed_subnets(params.max_allowed_subnets);
        Self::set_max_allowed_modules(params.max_allowed_modules);
        Self::set_max_registrations_per_block(params.max_registrations_per_block);
        Self::set_target_registrations_interval(params.target_registrations_interval);
        Self::set_target_registrations_per_interval(params.target_registrations_per_interval);
        Self::set_adjustment_alpha(params.adjustment_alpha);
        Self::set_unit_emission(params.unit_emission);
        Self::set_tx_rate_limit(params.tx_rate_limit);
        Self::set_global_vote_threshold(params.vote_threshold);
        Self::set_max_proposals(params.max_proposals);
        Self::set_vote_mode_global(params.vote_mode);
        Self::set_burn_rate(params.burn_rate);
        Self::set_min_burn(params.min_burn);
        Self::set_max_burn(params.max_burn);
        Self::set_min_weight_stake(params.min_weight_stake);
        Self::set_min_stake_global(params.min_stake);
        Self::set_min_deleg_fee_global(params.min_delegation_fee);
    }

    pub fn get_registrations_this_interval() -> u16 {
        RegistrationsThisInterval::<T>::get()
    }

    pub fn get_target_registrations_per_interval() -> u16 {
        TargetRegistrationsPerInterval::<T>::get()
    }

    pub fn set_target_registrations_per_interval(target_interval: u16) {
        TargetRegistrationsPerInterval::<T>::set(target_interval)
    }

    pub fn get_min_weight_stake() -> u64 {
        MinWeightStake::<T>::get()
    }
    pub fn set_min_weight_stake(min_weight_stake: u64) {
        MinWeightStake::<T>::put(min_weight_stake)
    }

    pub fn get_max_allowed_weights_global() -> u16 {
        MaxAllowedWeightsGlobal::<T>::get()
    }

    pub fn get_min_stake_global() -> u64 {
        MinStakeGlobal::<T>::get()
    }
    pub fn set_min_stake_global(min_stake: u64) {
        MinStakeGlobal::<T>::put(min_stake)
    }

    pub fn get_min_deleg_fee_global() -> Percent {
        MinDelegationFeeGlobal::<T>::get()
    }

    pub fn set_min_deleg_fee_global(delegation_fee: Percent) {
        MinDelegationFeeGlobal::<T>::put(delegation_fee)
    }

    pub fn set_vote_mode_global(vote_mode: Vec<u8>) {
        VoteModeGlobal::<T>::put(vote_mode);
    }

    pub fn get_vote_mode_global() -> Vec<u8> {
        VoteModeGlobal::<T>::get()
    }
    pub fn get_burn_rate() -> u16 {
        BurnRate::<T>::get().min(100)
    }

    pub fn get_burn() -> u64 {
        Burn::<T>::get()
    }

    pub fn set_burn(burn: u64) {
        Burn::<T>::set(burn)
    }

    pub fn set_burn_rate(burn_rate: u16) {
        BurnRate::<T>::put(burn_rate.min(100));
    }

    pub fn set_max_proposals(max_proposals: u64) {
        MaxProposals::<T>::put(max_proposals);
    }

    pub fn get_max_proposals() -> u64 {
        MaxProposals::<T>::get()
    }

    pub fn get_global_vote_threshold() -> u16 {
        GlobalVoteThreshold::<T>::get()
    }

    pub fn set_global_vote_threshold(vote_threshold: u16) {
        GlobalVoteThreshold::<T>::put(vote_threshold);
    }

    pub fn get_max_registrations_per_block() -> u16 {
        MaxRegistrationsPerBlock::<T>::get()
    }

    pub fn set_max_registrations_per_block(max_registrations_per_block: u16) {
        MaxRegistrationsPerBlock::<T>::set(max_registrations_per_block);
    }

    pub fn get_target_registrations_interval() -> u16 {
        TargetRegistrationsInterval::<T>::get()
    }

    pub fn set_target_registrations_interval(target_registrations_interval: u16) {
        TargetRegistrationsInterval::<T>::set(target_registrations_interval);
    }

    pub fn get_global_max_name_length() -> u16 {
        MaxNameLength::<T>::get()
    }

    pub fn set_global_max_name_length(max_name_length: u16) {
        MaxNameLength::<T>::put(max_name_length)
    }

    pub fn do_update_global(origin: T::RuntimeOrigin, params: GlobalParams) -> DispatchResult {
        ensure_root(origin)?;
        ensure!(
            Self::get_vote_mode_global() == AUTHORITY_MODE,
            Error::<T>::InvalidVoteMode
        );
        Self::set_global_params(params);
        Ok(())
    }

    pub fn global_n() -> u16 {
        let mut global_n: u16 = 0;
        for netuid in Self::netuids() {
            global_n += N::<T>::get(netuid);
        }
        global_n
    }

    pub fn get_global_stake_to(key: &T::AccountId) -> u64 {
        // get all of the stake to
        let total_networks: u16 = TotalSubnets::<T>::get();
        let mut total_stake_to = 0;

        for netuid in 0..total_networks {
            total_stake_to += Self::get_total_stake_to(netuid, key);
        }

        total_stake_to
    }

    // Configure tx rate limiting
    pub fn get_tx_rate_limit() -> u64 {
        TxRateLimit::<T>::get()
    }
    pub fn set_tx_rate_limit(tx_rate_limit: u64) {
        TxRateLimit::<T>::put(tx_rate_limit)
    }

    pub fn get_min_burn() -> u64 {
        MinBurn::<T>::get()
    }

    pub fn set_min_burn(min_burn: u64) {
        MinBurn::<T>::put(min_burn);
    }

    pub fn get_max_burn() -> u64 {
        MaxBurn::<T>::get()
    }

    pub fn set_max_burn(max_burn: u64) {
        MaxBurn::<T>::put(max_burn);
    }

    pub fn get_adjustment_alpha() -> u64 {
        AdjustmentAlpha::<T>::get()
    }

    pub fn set_adjustment_alpha(adjustment_alpha: u64) {
        AdjustmentAlpha::<T>::put(adjustment_alpha);
    }
}
