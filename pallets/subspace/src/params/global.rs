use crate::*;
use frame_support::pallet_prelude::{DispatchResult, MaxEncodedLen};
use pallet_governance_api::GovernanceConfiguration;
use scale_info::TypeInfo;
use sp_arithmetic::per_things::Percent;

#[derive(
    Decode, Encode, PartialEq, Eq, Clone, TypeInfo, frame_support::DebugNoBound, MaxEncodedLen,
)]
#[scale_info(skip_type_params(T))]
pub struct GlobalParams<T: Config> {
    // max
    pub max_name_length: u16,             // max length of a network name
    pub min_name_length: u16,             // min length of a network name
    pub max_allowed_subnets: u16,         // max number of subnets allowed
    pub max_allowed_modules: u16,         // max number of modules allowed per subnet
    pub max_registrations_per_block: u16, // max number of registrations per block
    pub max_allowed_weights: u16,         // max number of weights per module

    // mins
    pub floor_stake_delegation_fee: Percent, // min delegation fee
    pub floor_validator_weight_fee: Percent, // min weight-setting delegation fee
    pub floor_founder_share: u8,             // min founder share
    pub min_weight_stake: u64,               // min weight stake required

    // S0 governance
    pub curator: T::AccountId,
    pub general_subnet_application_cost: u64,

    // Other
    pub subnet_immunity_period: u64,
    pub governance_config: GovernanceConfiguration,

    pub kappa: u16,
    pub rho: u16,
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
            floor_stake_delegation_fee: MinFees::<T>::get().stake_delegation_fee,
            floor_validator_weight_fee: MinFees::<T>::get().validator_weight_fee,
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

    #[deny(unused_variables)]
    pub fn set_global_params(params: GlobalParams<T>) -> DispatchResult {
        Self::check_global_params(&params)?;

        let GlobalParams {
            max_name_length,
            min_name_length,
            max_allowed_subnets,
            max_allowed_modules,
            max_registrations_per_block,
            max_allowed_weights,
            floor_stake_delegation_fee,
            floor_validator_weight_fee,
            floor_founder_share,
            min_weight_stake,
            curator,
            general_subnet_application_cost,
            subnet_immunity_period,
            governance_config,
            kappa,
            rho,
        } = params.clone();

        // Network parameters
        MaxNameLength::<T>::put(max_name_length);
        MinNameLength::<T>::put(min_name_length);
        MaxAllowedSubnets::<T>::put(max_allowed_subnets);
        MaxAllowedModules::<T>::put(max_allowed_modules);

        // Update minimum fees
        MinFees::<T>::put(MinimumFees {
            stake_delegation_fee: floor_stake_delegation_fee,
            validator_weight_fee: floor_validator_weight_fee,
        });

        // Registration and weight parameters
        MaxRegistrationsPerBlock::<T>::set(max_registrations_per_block);
        MaxAllowedWeightsGlobal::<T>::put(max_allowed_weights);
        MinWeightStake::<T>::put(min_weight_stake);

        // Governance and administrative parameters
        T::set_curator(&curator);
        FloorFounderShare::<T>::put(floor_founder_share);
        SubnetImmunityPeriod::<T>::put(subnet_immunity_period);
        T::update_global_governance_configuration(governance_config)
            .expect("invalid governance configuration");

        // Cost and operational parameters
        T::set_general_subnet_application_cost(general_subnet_application_cost);
        Kappa::<T>::set(kappa);
        Rho::<T>::set(rho);

        Self::deposit_event(Event::GlobalParamsUpdated(params));
        Ok(())
    }

    #[deny(unused_variables)]
    pub fn check_global_params(params: &GlobalParams<T>) -> DispatchResult {
        let GlobalParams {
            max_name_length,
            min_name_length,
            max_allowed_subnets,
            max_allowed_modules,
            max_registrations_per_block,
            max_allowed_weights,
            floor_stake_delegation_fee,
            floor_validator_weight_fee,
            floor_founder_share,
            min_weight_stake: _,
            curator: _,
            general_subnet_application_cost,
            subnet_immunity_period,
            governance_config,
            kappa,
            rho,
        } = params;

        let old_params = Self::global_params();

        // Name length validations
        ensure!(*max_name_length > 0, Error::<T>::InvalidMaxNameLength);
        ensure!(
            *min_name_length < *max_name_length,
            Error::<T>::InvalidMinNameLenght
        );

        // Fee validations using ValidatorFees validation
        ValidatorFees::new::<T>(*floor_stake_delegation_fee, *floor_validator_weight_fee)
            .map_err(|_| Error::<T>::InvalidMinFees)?;

        // Additional validation to ensure fees don't decrease
        ensure!(
            floor_stake_delegation_fee >= &old_params.floor_stake_delegation_fee,
            Error::<T>::CannotDecreaseFee
        );
        ensure!(
            floor_validator_weight_fee >= &old_params.floor_validator_weight_fee,
            Error::<T>::CannotDecreaseFee
        );
        // Subnet and module validations
        ensure!(
            *max_allowed_subnets > 0,
            Error::<T>::InvalidMaxAllowedSubnets
        );
        ensure!(
            *max_allowed_modules > 0,
            Error::<T>::InvalidMaxAllowedModules
        );

        // Registration and weight validations
        ensure!(
            *max_registrations_per_block > 0,
            Error::<T>::InvalidMaxRegistrationsPerBlock
        );
        ensure!(
            *max_allowed_weights > 0,
            Error::<T>::InvalidMaxAllowedWeights
        );

        // Cost and stake validations
        ensure!(
            *general_subnet_application_cost > 0,
            Error::<T>::InvalidGeneralSubnetApplicationCost
        );

        // Governance validations
        ensure!(
            governance_config.proposal_expiration > 100,
            Error::<T>::InvalidProposalExpiration
        );

        ensure!(
            *floor_founder_share > 0 && *floor_founder_share <= 100,
            Error::<T>::InvalidFloorFounderShare
        );
        ensure!(
            *subnet_immunity_period > 0,
            Error::<T>::InvalidSubnetImmunityPeriod
        );
        ensure!(*kappa > 0, Error::<T>::InvalidKappa);
        ensure!(*rho > 0, Error::<T>::InvalidRho);

        Ok(())
    }
}
