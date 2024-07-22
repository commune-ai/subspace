use super::*;
use frame_support::pallet_prelude::{DispatchResult, MaxEncodedLen};
use sp_core::Get;
use sp_runtime::DispatchError;

// TODO:
// This will eventually become a subnet parameter (once we have global stake)
// So it will hold truly all burn adjustments.
#[derive(
    Clone, TypeInfo, Decode, Encode, PartialEq, Eq, frame_support::DebugNoBound, MaxEncodedLen,
)]
#[scale_info(skip_type_params(T))]
pub struct BurnConfiguration<T> {
    /// min burn the adjustment algorithm can set
    pub min_burn: u64,
    /// max burn the adjustment algorithm can set
    pub max_burn: u64,
    pub _pd: PhantomData<T>,
}

#[derive(
    Clone, TypeInfo, Decode, Encode, PartialEq, Eq, frame_support::DebugNoBound, MaxEncodedLen,
)]
#[scale_info(skip_type_params(T))]
pub struct SubnetBurnConfiguration<T> {
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
    /// the maximum number of registrations accepted per interval
    pub max_registrations: u16,
    pub _pd: PhantomData<T>,
}

impl<T: Config> Default for BurnConfiguration<T> {
    fn default() -> Self {
        Self {
            min_burn: 4_000_000_000,
            max_burn: 250_000_000_000,
            _pd: PhantomData,
        }
    }
}

// TODO:
// check if these parameters are truly desired
impl<T: Config> Default for SubnetBurnConfiguration<T> {
    fn default() -> Self {
        Self {
            min_burn: 2_000_000_000_000,
            max_burn: 100_000_000_000_000,
            adjustment_alpha: u64::MAX / 2,
            adjustment_interval: 5_400,
            expected_registrations: 1,
            max_registrations: T::DefaultMaxSubnetRegistrationsPerInterval::get(),
            _pd: PhantomData,
        }
    }
}

impl<T: Config> BurnConfiguration<T> {
    pub fn apply(self) -> Result<(), DispatchError> {
        ensure!(self.min_burn >= 100_000_000, Error::<T>::InvalidMinBurn);
        ensure!(self.max_burn > self.min_burn, Error::<T>::InvalidMaxBurn);

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
            curator: T::get_curator(),
            floor_founder_share: FloorFounderShare::<T>::get(),
            floor_delegation_fee: FloorDelegationFee::<T>::get(),
            // burn & registrations
            max_registrations_per_block: MaxRegistrationsPerBlock::<T>::get(),
            burn_config: BurnConfig::<T>::get(),
            // weights
            max_allowed_weights: MaxAllowedWeightsGlobal::<T>::get(),
            min_weight_stake: MinWeightStake::<T>::get(),

            // s0 config
            subnet_immunity_period: SubnetImmunityPeriod::<T>::get(),
            general_subnet_application_cost: T::get_general_subnet_application_cost(),
            kappa: Kappa::<T>::get(),
            rho: Rho::<T>::get(),

            governance_config: T::get_global_governance_configuration(),
        }
    }

    pub fn set_global_params(params: GlobalParams<T>) -> DispatchResult {
        // Check if the params are valid
        Self::check_global_params(&params)?;

        // Network
        MaxNameLength::<T>::put(params.max_name_length);
        MaxAllowedSubnets::<T>::put(params.max_allowed_subnets);
        MaxAllowedModules::<T>::put(params.max_allowed_modules);
        FloorDelegationFee::<T>::put(params.floor_delegation_fee);

        // burn & registrations
        MaxRegistrationsPerBlock::<T>::set(params.max_registrations_per_block);
        MinWeightStake::<T>::put(params.min_weight_stake);
        FloorDelegationFee::<T>::put(params.floor_delegation_fee);

        // TODO: update curator
        T::set_curator(&params.curator);

        FloorFounderShare::<T>::put(params.floor_founder_share);

        // weights
        MaxAllowedWeightsGlobal::<T>::put(params.max_allowed_weights);
        MinWeightStake::<T>::put(params.min_weight_stake);

        T::update_global_governance_configuration(params.governance_config)
            .expect("invalid governance configuration");

        // burn
        params.burn_config.apply()?;

        // Update the general subnet application cost
        T::set_general_subnet_application_cost(params.general_subnet_application_cost);
        Kappa::<T>::set(params.kappa);
        Rho::<T>::set(params.rho);

        Ok(())
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

        ensure!(
            params.general_subnet_application_cost > 0,
            Error::<T>::InvalidGeneralSubnetApplicationCost
        );

        ensure!(
            params.governance_config.proposal_expiration > 100,
            Error::<T>::InvalidProposalExpiration
        );

        Ok(())
    }
}
