use crate::*;

use frame_support::pallet_prelude::{DispatchResult, MaxEncodedLen};
use sp_core::Get;
use sp_runtime::DispatchError;

/// This struct is used for both global (Subnet Burn) and MAP parameters (Module Burn)
#[derive(
    Clone, TypeInfo, Decode, Encode, PartialEq, Eq, frame_support::DebugNoBound, MaxEncodedLen,
)]
#[scale_info(skip_type_params(T))]
pub struct GeneralBurnConfiguration<T> {
    /// min burn the adjustment algorithm can set
    pub min_burn: u64,
    /// max burn the adjustment algorithm can set
    pub max_burn: u64,
    /// the steepness with which the burn curve will increase
    /// every interval
    pub adjustment_alpha: u64,
    /// interval in blocks for the burn to be adjusted
    pub target_registrations_interval: u16,
    /// the number of registrations expected per interval, if
    /// below, burn gets decreased, it is increased otherwise
    pub target_registrations_per_interval: u16,
    /// the maximum number of registrations accepted per interval
    pub max_registrations_per_interval: u16,
    pub _pd: PhantomData<T>,
}

pub enum BurnType {
    Subnet,
    Module,
}

impl<T: Config> Default for GeneralBurnConfiguration<T> {
    fn default() -> Self {
        Self::module_burn_default()
    }
}

impl<T: Config> GeneralBurnConfiguration<T> {
    fn subnet_burn_default() -> Self {
        Self {
            min_burn: T::DefaultSubnetMinBurn::get(),
            max_burn: 100_000_000_000_000,
            adjustment_alpha: u64::MAX / 2,
            target_registrations_interval: 5_400,
            target_registrations_per_interval: 1,
            max_registrations_per_interval: T::DefaultMaxSubnetRegistrationsPerInterval::get(),
            _pd: PhantomData,
        }
    }

    pub fn module_burn_default() -> Self {
        Self {
            min_burn: T::DefaultModuleMinBurn::get(),
            max_burn: 150_000_000_000,
            adjustment_alpha: u64::MAX / 2,
            target_registrations_interval: 142,
            target_registrations_per_interval: 3,
            max_registrations_per_interval: T::DefaultMaxRegistrationsPerInterval::get(),
            _pd: PhantomData,
        }
    }

    pub fn default_for(burn_type: BurnType) -> Self {
        match burn_type {
            BurnType::Subnet => Self::subnet_burn_default(),
            BurnType::Module => Self::module_burn_default(),
        }
    }

    pub fn apply_module_burn(self, netuid: u16) -> Result<(), DispatchError> {
        ensure!(
            self.min_burn >= T::DefaultModuleMinBurn::get(),
            Error::<T>::InvalidMinBurn
        );
        ensure!(self.max_burn > self.min_burn, Error::<T>::InvalidMaxBurn);
        ensure!(
            self.adjustment_alpha > 0,
            Error::<T>::InvalidAdjustmentAlpha
        );
        ensure!(
            self.target_registrations_interval >= 10,
            Error::<T>::InvalidTargetRegistrationsInterval
        );
        ensure!(
            self.target_registrations_per_interval >= 1,
            Error::<T>::InvalidTargetRegistrationsPerInterval
        );
        ensure!(
            self.max_registrations_per_interval >= 1,
            Error::<T>::InvalidMaxRegistrationsPerInterval
        );
        ensure!(
            self.max_registrations_per_interval >= self.target_registrations_per_interval,
            Error::<T>::InvalidMaxRegistrationsPerInterval
        );

        ModuleBurnConfig::<T>::set(netuid, self);

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
            // registrations
            max_registrations_per_block: MaxRegistrationsPerBlock::<T>::get(),
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
