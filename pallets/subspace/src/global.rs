use super::*;
use frame_support::pallet_prelude::{DispatchResult, MaxEncodedLen};
use sp_core::Get;
use sp_runtime::DispatchError;

#[derive(
    Clone, TypeInfo, Decode, Encode, PartialEq, Eq, frame_support::DebugNoBound, MaxEncodedLen,
)]
#[scale_info(skip_type_params(T))]
pub struct BurnConfiguration<T> {
    /// min burn the adjustment algorithm can set
    pub min_burn: u64,
    /// max burn the adjustment algorithm can set
    pub max_burn: u64,
    /// the steepness with which the burn curve will increase
    /// every interval
    pub adjustment_alpha: u64,
    /// interval in blocks for the burn to be adjusted
    pub adjustment_interval: u16,
    /// the number of registrations expected per interval, if
    /// below, burn gets decreased, it is increased otherwise
    pub expected_registrations: u16,
    pub _pd: PhantomData<T>,
}

impl<T: Config> Default for BurnConfiguration<T> {
    fn default() -> Self {
        Self {
            min_burn: 4_000_000_000,
            max_burn: 250_000_000_000,
            adjustment_alpha: u64::MAX / 2,
            adjustment_interval: DefaultTempo::<T>::get() * 2,
            expected_registrations: DefaultTempo::<T>::get(),
            _pd: PhantomData,
        }
    }
}

impl<T: Config> BurnConfiguration<T> {
    pub fn apply(self) -> Result<(), DispatchError> {
        ensure!(self.min_burn >= 100_000_000, Error::<T>::InvalidMinBurn);

        ensure!(self.max_burn > self.min_burn, Error::<T>::InvalidMaxBurn);

        ensure!(
            self.expected_registrations > 0,
            Error::<T>::InvalidTargetRegistrationsPerInterval
        );

        ensure!(
            self.adjustment_interval > 0,
            Error::<T>::InvalidTargetRegistrationsInterval
        );

        BurnConfig::<T>::set(self);

        Ok(())
    }
}

impl<T: Config> Pallet<T> {
    pub fn global_params() -> GlobalParams<T> {
        GlobalParams {
            // network
            max_name_length: MaxNameLength::<T>::get(),
            min_name_length: MinNameLength::<T>::get(),
            max_allowed_subnets: MaxAllowedSubnets::<T>::get(),
            max_allowed_modules: MaxAllowedModules::<T>::get(),
            curator: Curator::<T>::get(),
            floor_founder_share: FloorFounderShare::<T>::get(),
            floor_delegation_fee: FloorDelegationFee::<T>::get(),

            // burn & registrations
            max_registrations_per_block: MaxRegistrationsPerBlock::<T>::get(),
            burn_config: BurnConfig::<T>::get(),

            // weights
            max_allowed_weights: MaxAllowedWeightsGlobal::<T>::get(),
            subnet_stake_threshold: SubnetStakeThreshold::<T>::get(),
            min_weight_stake: MinWeightStake::<T>::get(),
            // proposals
            proposal_cost: ProposalCost::<T>::get(), // denominated in $COMAI
            proposal_expiration: ProposalExpiration::<T>::get(), /* denominated in the number of
                                                      * blocks */
            proposal_participation_threshold: ProposalParticipationThreshold::<T>::get(), /* denominated
                                                                                          in percent of the overall network stake */
            // s0 config
            general_subnet_application_cost: GeneralSubnetApplicationCost::<T>::get(),
        }
    }

    pub fn set_global_params(params: GlobalParams<T>) {
        // Check if the params are valid
        Self::check_global_params(&params).expect("global params are invalid");

        // Network
        MaxNameLength::<T>::put(params.max_name_length);
        MaxAllowedSubnets::<T>::put(params.max_allowed_subnets);
        MaxAllowedModules::<T>::put(params.max_allowed_modules);
        FloorDelegationFee::<T>::put(params.floor_delegation_fee);

        // burn & registrations
        MaxRegistrationsPerBlock::<T>::set(params.max_registrations_per_block);
        MinWeightStake::<T>::put(params.min_weight_stake);
        SubnetStakeThreshold::<T>::put(params.subnet_stake_threshold);
        FloorDelegationFee::<T>::put(params.floor_delegation_fee);
        Curator::<T>::put(params.curator);
        FloorFounderShare::<T>::put(params.floor_founder_share);

        // weights
        MaxAllowedWeightsGlobal::<T>::put(params.max_allowed_weights);
        MinWeightStake::<T>::put(params.min_weight_stake);

        // proposals
        ProposalCost::<T>::put(params.proposal_cost);
        ProposalExpiration::<T>::put(params.proposal_expiration);
        ProposalParticipationThreshold::<T>::put(params.proposal_participation_threshold);

        // burn
        params.burn_config.apply().expect("invalid burn configuration");
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
            params.max_allowed_weights > 0,
            Error::<T>::InvalidMaxAllowedWeights
        );

        // Proposal checks
        ensure!(params.proposal_cost > 0, Error::<T>::InvalidProposalCost);

        ensure!(
            params.general_subnet_application_cost > 0,
            Error::<T>::InvalidGeneralSubnetApplicationCost
        );

        ensure!(
            params.proposal_expiration % 100 == 0,
            Error::<T>::InvalidProposalExpiration
        );

        ensure!(
            params.proposal_participation_threshold.deconstruct() <= 100,
            Error::<T>::InvalidProposalParticipationThreshold
        );

        Ok(())
    }
}
