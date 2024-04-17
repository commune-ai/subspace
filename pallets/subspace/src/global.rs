use super::*;
use frame_support::pallet_prelude::DispatchResult;
use sp_arithmetic::per_things::Percent;

impl<T: Config> Pallet<T> {
    pub fn global_params() -> GlobalParams<T> {
        GlobalParams {
            // network
            max_name_length: Self::get_global_max_name_length(),
            min_name_length: Self::get_global_min_name_length(),
            max_allowed_subnets: Self::get_global_max_allowed_subnets(),
            max_allowed_modules: Self::get_max_allowed_modules(),
            unit_emission: Self::get_unit_emission(),
            nominator: Self::get_nominator(),
            floor_delegation_fee: Self::get_floor_delegation_fee(),
            // burn & registrations
            max_registrations_per_block: Self::get_max_registrations_per_block(),
            target_registrations_per_interval: Self::get_target_registrations_per_interval(),
            target_registrations_interval: Self::get_target_registrations_interval(),
            burn_rate: Self::get_burn_rate(),
            min_burn: Self::get_min_burn(),
            max_burn: Self::get_max_burn(),
            adjustment_alpha: Self::get_adjustment_alpha(),
            min_stake: Self::get_min_stake_global(),
            // weights
            max_allowed_weights: Self::get_max_allowed_weights_global(),
            subnet_stake_threshold: Self::get_subnet_stake_threshold(),
            min_weight_stake: Self::get_min_weight_stake(),
            // proposals
            proposal_cost: Self::get_proposal_cost(), // denominated in $COMAI
            proposal_expiration: Self::get_proposal_expiration(), /* denominated in the number of
                                                       * blocks */
            proposal_participation_threshold: Self::get_proposal_participation_threshold(), /* denominated
                                                                                            in percent of the overall network stake */
        }
    }

    pub fn check_global_params(params: &GlobalParams<T>) -> DispatchResult {
        // checks if params are valid
        let old_params = Self::global_params();

        // check if the name already exists
        ensure!(params.max_name_length > 0, Error::<T>::InvalidMaxNameLength);

        ensure!(
            params.min_name_length < params.max_name_length,
            Error::<T>::InvalidMinNameLenght
        );

        // we need to ensure that the delegation fee floor is only moven up, moving it down would
        // require a storage migration
        ensure!(
            params.floor_delegation_fee.deconstruct() <= 100
                && params.floor_delegation_fee.deconstruct()
                    >= old_params.floor_delegation_fee.deconstruct(),
            Error::<T>::InvalidMinDelegationFee
        );

        // we can not increase the stake threshold without a migration
        // that would mean that subnets that are getting emission would have to get them erased to 0
        ensure!(
            params.subnet_stake_threshold.deconstruct() <= 100
                && params.subnet_stake_threshold.deconstruct()
                    <= old_params.subnet_stake_threshold.deconstruct(),
            Error::<T>::InvalidSubnetStakeThreshold
        );

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
            params.unit_emission <= old_params.unit_emission,
            Error::<T>::InvalidUnitEmission
        );

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

        ensure!(
            params.target_registrations_per_interval > 0,
            Error::<T>::InvalidTargetRegistrationsPerInterval
        );

        ensure!(
            params.max_allowed_weights > 0,
            Error::<T>::InvalidMaxAllowedWeights
        );

        // Proposal checks
        ensure!(params.proposal_cost > 0, Error::<T>::InvalidProposalCost);

        ensure!(
            params.proposal_expiration % 100 == 0, // for computational reasons
            Error::<T>::InvalidProposalExpiration
        );
        ensure!(
            params.proposal_participation_threshold.deconstruct() <= 100,
            Error::<T>::InvalidProposalParticipationThreshold
        );

        Ok(())
    }

    pub fn set_global_params(params: GlobalParams<T>) {
        // Check if the params are valid
        Self::check_global_params(&params).expect("global params are invalid");

        // Network
        Self::set_global_max_name_length(params.max_name_length);
        Self::set_global_max_allowed_subnets(params.max_allowed_subnets);
        Self::set_max_allowed_modules(params.max_allowed_modules);
        Self::set_unit_emission(params.unit_emission);
        Self::set_floor_delegation_fee(params.floor_delegation_fee);
        // burn & registrations
        Self::set_max_registrations_per_block(params.max_registrations_per_block);
        Self::set_target_registrations_per_interval(params.target_registrations_per_interval);
        Self::set_target_registrations_interval(params.target_registrations_interval);
        Self::set_burn_rate(params.burn_rate);
        Self::set_min_burn(params.min_burn);
        Self::set_max_burn(params.max_burn);
        Self::set_min_weight_stake(params.min_weight_stake);
        Self::set_subnet_stake_threshold(params.subnet_stake_threshold);
        Self::set_adjustment_alpha(params.adjustment_alpha);
        Self::set_min_stake_global(params.min_stake);
        Self::set_floor_delegation_fee(params.floor_delegation_fee);
        Self::set_nominator(params.nominator);

        // weights
        Self::set_max_allowed_weights_global(params.max_allowed_weights);
        Self::set_min_weight_stake(params.min_weight_stake);

        // proposals
        Self::set_proposal_cost(params.proposal_cost);
        Self::set_proposal_expiration(params.proposal_expiration);
        Self::set_proposal_participation_threshold(params.proposal_participation_threshold);
    }

    pub fn get_nominator() -> T::AccountId {
        Nominator::<T>::get()
    }

    pub fn set_nominator(nominator: T::AccountId) {
        Nominator::<T>::put(nominator)
    }

    pub fn get_target_registrations_per_interval() -> u16 {
        TargetRegistrationsPerInterval::<T>::get()
    }

    pub fn set_target_registrations_per_interval(target_interval: u16) {
        TargetRegistrationsPerInterval::<T>::put(target_interval)
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

    pub fn get_subnet_stake_threshold() -> Percent {
        SubnetStakeThreshold::<T>::get()
    }

    pub fn set_subnet_stake_threshold(stake_threshold: Percent) {
        SubnetStakeThreshold::<T>::put(stake_threshold)
    }

    pub fn set_max_allowed_weights_global(max_allowed_weights: u16) {
        MaxAllowedWeightsGlobal::<T>::put(max_allowed_weights)
    }

    pub fn get_min_stake_global() -> u64 {
        MinStakeGlobal::<T>::get()
    }
    pub fn set_min_stake_global(min_stake: u64) {
        MinStakeGlobal::<T>::put(min_stake)
    }

    pub fn get_floor_delegation_fee() -> Percent {
        FloorDelegationFee::<T>::get()
    }

    pub fn set_floor_delegation_fee(delegation_fee: Percent) {
        FloorDelegationFee::<T>::put(delegation_fee)
    }

    pub fn get_burn_rate() -> u16 {
        BurnRate::<T>::get()
    }
    pub fn set_burn_rate(burn_rate: u16) {
        BurnRate::<T>::put(burn_rate.min(100));
    }

    // Proposals
    pub fn get_proposal_cost() -> u64 {
        ProposalCost::<T>::get()
    }

    pub fn set_proposal_cost(proposal_cost: u64) {
        ProposalCost::<T>::put(proposal_cost);
    }

    pub fn set_proposal_expiration(proposal_expiration: u32) {
        ProposalExpiration::<T>::put(proposal_expiration);
    }

    pub fn get_proposal_expiration() -> u32 {
        ProposalExpiration::<T>::get()
    }

    pub fn set_proposal_participation_threshold(proposal_participation_threshold: Percent) {
        ProposalParticipationThreshold::<T>::put(proposal_participation_threshold);
    }

    pub fn get_proposal_participation_threshold() -> Percent {
        ProposalParticipationThreshold::<T>::get()
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

    pub fn get_global_min_name_length() -> u16 {
        MinNameLength::<T>::get()
    }

    pub fn set_global_min_name_length(min_name_length: u16) {
        MinNameLength::<T>::put(min_name_length)
    }

    // returns the amount of total modules on the network
    pub fn global_n_modules() -> u16 {
        let mut global_n: u16 = 0;
        for netuid in Self::netuids() {
            global_n += N::<T>::get(netuid);
        }
        global_n
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

    // Whitelist management
    pub fn is_in_legit_whitelist(account_id: &T::AccountId) -> bool {
        LegitWhitelist::<T>::contains_key(account_id)
    }

    pub fn insert_to_whitelist(module_key: T::AccountId, recommended_weight: u8) {
        LegitWhitelist::<T>::insert(module_key, recommended_weight);
    }

    pub fn rm_from_whitelist(module_key: &T::AccountId) {
        LegitWhitelist::<T>::remove(module_key);
    }
}
