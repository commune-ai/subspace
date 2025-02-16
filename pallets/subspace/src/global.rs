use crate::*;
use frame_support::pallet_prelude::{DispatchResult, MaxEncodedLen};
use pallet_governance_api::GovernanceConfiguration;
use scale_info::TypeInfo;
use sp_arithmetic::per_things::Percent;
use frame_system::Config;

#[derive(
    Decode, Encode, PartialEq, Eq, Clone, TypeInfo, frame_support::DebugNoBound, MaxEncodedLen,
)]
#[scale_info(skip_type_params(T))]
pub struct GlobalParams<T: Config> {
    pub max_name_length: u16,
    pub min_name_length: u16, 
    pub max_allowed_modules: u16,
    pub max_registrations_per_block: u16,
    pub max_allowed_weights: u16,
    pub floor_founder_share: u8,
    pub min_weight_stake: u64,
    pub curator: T::AccountId,
    pub governance_config: GovernanceConfiguration,
}
impl<T: Config + GovernanceApi> Pallet<T> {
    pub fn global_params() -> GlobalParams<T> {
        GlobalParams {
            max_name_length: MaxNameLength::<T>::get(),
            min_name_length: MinNameLength::<T>::get(),
            max_allowed_modules: MaxAllowedModules::<T>::get(),
            max_registrations_per_block: MaxRegistrationsPerBlock::<T>::get(),
            max_allowed_weights: MaxAllowedWeights::<T>::get(),
            floor_founder_share: FloorFounderShare::<T>::get(),
            min_weight_stake: MinWeightStake::<T>::get(),
            curator: T::get_curator(),
            governance_config: T::get_global_governance_configuration(),
        }
    }

    #[deny(unused_variables)]
    pub fn set_global_params(params: GlobalParams<T>) -> DispatchResult {
        Self::check_global_params(&params)?;

        let GlobalParams {
            max_name_length,
            min_name_length,
            max_allowed_modules,
            max_registrations_per_block,
            max_allowed_weights,
            floor_founder_share,
            min_weight_stake,
            curator,
            governance_config,
        } = params.clone();

        // Network parameters
        MaxNameLength::<T>::put(max_name_length);
        MinNameLength::<T>::put(min_name_length);
        MaxAllowedModules::<T>::put(max_allowed_modules);

        // Registration and weight parameters
        MaxRegistrationsPerBlock::<T>::set(max_registrations_per_block);
        // Governance and administrative parameters
        T::set_curator(&curator);
        T::update_global_governance_configuration(governance_config)
            .expect("invalid governance configuration");

        // Cost and operational parameters

        Self::deposit_event(Event::GlobalParamsUpdated(params));
        Ok(())
    }

    #[deny(unused_variables)]
    pub fn check_global_params(params: &GlobalParams<T>) -> DispatchResult {
        let GlobalParams {
            max_name_length,
            min_name_length,
            max_allowed_modules,
            max_registrations_per_block,
            max_allowed_weights,
            floor_founder_share,
            min_weight_stake: _,
            curator: _,
            governance_config,
        } = params;

        let old_params = Self::global_params();

        // Name length validations
        ensure!(*max_name_length > 0, Error::<T>::InvalidMaxNameLength);
        ensure!(
            *min_name_length < *max_name_length,
            Error::<T>::InvalidMinNameLenght
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

        // Governance validations
        ensure!(
            governance_config.proposal_expiration > 100,
            Error::<T>::InvalidProposalExpiration
        );


        Ok(())
    }
}
