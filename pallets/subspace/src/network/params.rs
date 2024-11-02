use crate::*;

use frame_support::{
    pallet_prelude::DispatchResult, storage::IterableStorageMap, IterableStorageDoubleMap,
};
use pallet_subnet_emission_api::SubnetConsensus;
use sp_core::Get;

use sp_runtime::{BoundedVec, DispatchError};
use sp_std::vec::Vec;
use substrate_fixed::types::I64F64;

// ---------------------------------
// Subnet Parameters
// ---------------------------------

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

    #[deny(unused_variables)]
    pub fn apply(self, netuid: u16) -> DispatchResult {
        Self::validate_params(Some(netuid), &self.params)?;

        let SubnetParams {
            founder,
            founder_share,
            immunity_period,
            incentive_ratio,
            max_allowed_uids,
            max_allowed_weights,
            min_allowed_weights,
            max_weight_age,
            name,
            metadata,
            tempo,
            maximum_set_weight_calls_per_epoch,
            bonds_ma,
            module_burn_config,
            min_validator_stake,
            max_allowed_validators,
            governance_config,
            use_weights_encryption,
            copier_margin,
            max_encryption_period,
        } = self.params;

        // Use all fields
        Pallet::<T>::set_max_allowed_uids(netuid, max_allowed_uids)?;
        SubnetNames::<T>::insert(netuid, name.into_inner());
        Founder::<T>::insert(netuid, &founder);
        FounderShare::<T>::insert(netuid, founder_share);
        Tempo::<T>::insert(netuid, tempo);
        ImmunityPeriod::<T>::insert(netuid, immunity_period);
        MaxAllowedWeights::<T>::insert(netuid, max_allowed_weights);
        MaxWeightAge::<T>::insert(netuid, max_weight_age);
        MinAllowedWeights::<T>::insert(netuid, min_allowed_weights);
        IncentiveRatio::<T>::insert(netuid, incentive_ratio);
        BondsMovingAverage::<T>::insert(netuid, bonds_ma);
        module_burn_config.apply_module_burn(netuid)?;
        MinValidatorStake::<T>::insert(netuid, min_validator_stake);

        if maximum_set_weight_calls_per_epoch == 0 {
            MaximumSetWeightCallsPerEpoch::<T>::remove(netuid);
        } else {
            MaximumSetWeightCallsPerEpoch::<T>::insert(netuid, maximum_set_weight_calls_per_epoch);
        }

        T::update_subnet_governance_configuration(netuid, governance_config)?;

        if let Some(meta) = &metadata {
            SubnetMetadata::<T>::insert(netuid, meta);
        }
        MaxAllowedValidators::<T>::insert(netuid, max_allowed_validators);
        MaxEncryptionPeriod::<T>::insert(netuid, max_encryption_period);
        UseWeightsEncryption::<T>::insert(netuid, use_weights_encryption);
        CopierMargin::<T>::insert(netuid, copier_margin);

        Pallet::<T>::deposit_event(Event::SubnetParamsUpdated(netuid));

        Ok(())
    }

    // TODO: validate everything
    #[deny(unused_variables)]
    pub fn validate_params(netuid: Option<u16>, params: &SubnetParams<T>) -> DispatchResult {
        let global_params = Pallet::<T>::global_params();

        // Destructure all fields to ensure we validate everything
        let SubnetParams {
            founder: _, // not validated
            founder_share,
            immunity_period: _, // not validated
            incentive_ratio,
            max_allowed_uids,
            max_allowed_weights,
            min_allowed_weights,
            max_weight_age,
            name,
            metadata,
            tempo,
            maximum_set_weight_calls_per_epoch: _, // not validated
            bonds_ma: _,                           // not validated
            module_burn_config: _,                 // not validated
            min_validator_stake,
            max_allowed_validators,
            governance_config: _,      // not validated
            use_weights_encryption: _, // not validated
            copier_margin,
            max_encryption_period,
        } = params;

        // Validate min/max weights relationship
        ensure!(
            *min_allowed_weights <= *max_allowed_weights,
            Error::<T>::InvalidMinAllowedWeights
        );

        ensure!(
            *max_allowed_weights <= global_params.max_allowed_weights,
            Error::<T>::InvalidMaxAllowedWeights
        );

        ensure!(
            *min_allowed_weights >= 1,
            Error::<T>::InvalidMinAllowedWeights
        );

        // Validate metadata if present
        if let Some(meta) = metadata {
            ensure!(!meta.is_empty(), Error::<T>::InvalidSubnetMetadata);
        }

        // Validate tempo and weight age
        ensure!(*tempo >= 25, Error::<T>::InvalidTempo);
        ensure!(
            *max_weight_age > *tempo as u64,
            Error::<T>::InvalidMaxWeightAge
        );

        // Validate UIDs
        ensure!(*max_allowed_uids > 0, Error::<T>::InvalidMaxAllowedUids);

        // Validate shares and ratios
        ensure!(*founder_share <= 100, Error::<T>::InvalidFounderShare);
        ensure!(
            *founder_share >= FloorFounderShare::<T>::get() as u16,
            Error::<T>::InvalidFounderShare
        );
        ensure!(*incentive_ratio <= 100, Error::<T>::InvalidIncentiveRatio);

        // Validate weights
        ensure!(
            *max_allowed_weights <= MaxAllowedWeightsGlobal::<T>::get(),
            Error::<T>::InvalidMaxAllowedWeights
        );

        ensure!(
            netuid.map_or(true, |netuid| *max_allowed_uids >= N::<T>::get(netuid)),
            Error::<T>::InvalidMaxAllowedUids
        );

        // Validate stakes and margins
        ensure!(
            *min_validator_stake <= 250_000_000_000_000,
            Error::<T>::InvalidMinValidatorStake
        );

        ensure!(*copier_margin <= 1, Error::<T>::InvalidCopierMargin);

        // Validate optional validators
        if let Some(max_validators) = max_allowed_validators {
            ensure!(
                *max_validators >= 10,
                Error::<T>::InvalidMaxAllowedValidators
            );
        }

        // Validate encryption period
        if let Some(encryption_period) = max_encryption_period {
            ensure!(
                *encryption_period >= 360 && *encryption_period <= T::MaxEncryptionDuration::get(),
                Error::<T>::InvalidMaxEncryptionPeriod
            );
        }

        // Validate subnet name
        match Pallet::<T>::get_netuid_for_name(name) {
            Some(id) if netuid.is_some_and(|netuid| netuid == id) => { /* subnet kept same name */ }
            Some(_) => return Err(Error::<T>::SubnetNameAlreadyExists.into()),
            None => {
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
    pub fn subnet_params(netuid: u16) -> SubnetParams<T> {
        SubnetParams {
            founder: Founder::<T>::get(netuid),
            founder_share: FounderShare::<T>::get(netuid),
            tempo: Tempo::<T>::get(netuid),
            immunity_period: ImmunityPeriod::<T>::get(netuid),
            max_allowed_weights: MaxAllowedWeights::<T>::get(netuid),
            max_allowed_uids: MaxAllowedUids::<T>::get(netuid),
            max_weight_age: MaxWeightAge::<T>::get(netuid),
            min_allowed_weights: MinAllowedWeights::<T>::get(netuid),
            name: BoundedVec::truncate_from(SubnetNames::<T>::get(netuid)),
            incentive_ratio: IncentiveRatio::<T>::get(netuid),
            maximum_set_weight_calls_per_epoch: MaximumSetWeightCallsPerEpoch::<T>::get(netuid)
                .unwrap_or_default(),
            bonds_ma: BondsMovingAverage::<T>::get(netuid),

            // Registrations
            module_burn_config: ModuleBurnConfig::<T>::get(netuid),
            min_validator_stake: MinValidatorStake::<T>::get(netuid),
            max_allowed_validators: MaxAllowedValidators::<T>::get(netuid),
            governance_config: T::get_subnet_governance_configuration(netuid),
            metadata: SubnetMetadata::<T>::get(netuid),
            use_weights_encryption: UseWeightsEncryption::<T>::get(netuid),
            copier_margin: CopierMargin::<T>::get(netuid),
            max_encryption_period: MaxEncryptionPeriod::<T>::get(netuid),
        }
    }
}
