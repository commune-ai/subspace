use crate::*;

use frame_support::pallet_prelude::DispatchResult;
use pallet_governance_api::VoteMode;
use sp_core::Get;
use sp_runtime::{BoundedVec, DispatchError};

use frame_support::pallet_prelude::*;
use pallet_governance_api::GovernanceConfiguration;
use substrate_fixed::types::I64F64;

#[derive(
    Decode, Encode, PartialEq, Eq, Clone, frame_support::DebugNoBound, TypeInfo, MaxEncodedLen,
)]
#[scale_info(skip_type_params(T))]
pub struct SubnetParams<T: Config> {
    // --- parameters
    pub founder: T::AccountId,
    pub founder_share: u16,    // out of 100
    pub immunity_period: u16,  // immunity period
    pub incentive_ratio: u16,  // out of 100
    pub max_allowed_uids: u16, // Max allowed modules on a subnet
    pub max_allowed_weights: u16, /* max number of weights allowed to be registered in this
                                * pub max_allowed_uids: u16, // max number of uids
                                * allowed to be registered in this subnet */
    pub min_allowed_weights: u16, // min number of weights allowed to be registered in this
    pub max_weight_age: u64,      // max age of a weight
    pub name: BoundedVec<u8, ConstU32<256>>,
    pub metadata: Option<BoundedVec<u8, ConstU32<120>>>,
    pub tempo: u16, // how many blocks to wait before rewarding models
    pub maximum_set_weight_calls_per_epoch: u16,
    // consensus
    pub bonds_ma: u64,
    pub module_burn_config: GeneralBurnConfiguration<T>,
    pub min_validator_stake: u64,
    pub max_allowed_validators: Option<u16>,
    pub governance_config: GovernanceConfiguration,
    // weight copying
    pub use_weights_encryption: bool,
    pub copier_margin: I64F64,
    pub max_encryption_period: Option<u64>,
}

pub struct DefaultSubnetParams<T: Config>(sp_std::marker::PhantomData<((), T)>);

impl<T: Config> DefaultSubnetParams<T> {
    // TODO: not hardcode values here, get them from the storages instead,
    // if they implement default already.
    pub fn get() -> SubnetParams<T> {
        SubnetParams {
            name: BoundedVec::default(),
            tempo: 100,
            immunity_period: 0,
            min_allowed_weights: 1,
            max_allowed_weights: 420,
            max_allowed_uids: 420,
            max_weight_age: 3_600,
            founder_share: FloorFounderShare::<T>::get() as u16,
            incentive_ratio: 50,
            founder: DefaultKey::<T>::get(),
            maximum_set_weight_calls_per_epoch: 0,
            bonds_ma: 900_000,

            // registrations
            module_burn_config: GeneralBurnConfiguration::<T>::default_for(BurnType::Module),
            min_validator_stake: T::DefaultMinValidatorStake::get(),
            max_allowed_validators: None,
            governance_config: GovernanceConfiguration {
                vote_mode: VoteMode::Authority,
                ..Default::default()
            },
            metadata: None,
            use_weights_encryption: T::DefaultUseWeightsEncryption::get(),
            copier_margin: I64F64::from_num(0),
            max_encryption_period: None,
        }
    }
}

#[derive(Debug)]
pub struct SubnetChangeset<T: Config> {
    pub params: SubnetParams<T>,
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
