use crate::*;

use frame_support::pallet_prelude::DispatchResult;
use pallet_governance_api::VoteMode;
use sp_core::Get;
use sp_runtime::{BoundedVec, DispatchError};
use sp_std::ops::{Deref, DerefMut};

use frame_support::pallet_prelude::*;
use pallet_governance_api::GovernanceConfiguration;
use substrate_fixed::types::I64F64;

#[derive(
    Decode, Encode, PartialEq, Eq, Clone, frame_support::DebugNoBound, TypeInfo, MaxEncodedLen,
)]
#[scale_info(skip_type_params(T))]
pub struct SubnetParams<T: Config> {
    pub founder: T::AccountId,
    pub founder_share: u16,
    pub immunity_period: u16,
    pub incentive_ratio: u16,
    pub max_allowed_uids: u16,
    pub max_allowed_weights: u16,
    pub min_allowed_weights: u16,
    pub max_weight_age: u64,
    pub name: BoundedVec<u8, ConstU32<256>>,
    pub metadata: Option<BoundedVec<u8, ConstU32<120>>>,
    pub tempo: u16,
    pub maximum_set_weight_calls_per_epoch: Option<u16>,
    // --- Consensus ---
    pub bonds_ma: u64,
    pub module_burn_config: GeneralBurnConfiguration<T>,
    pub min_validator_stake: u64,
    pub max_allowed_validators: Option<u16>,
    pub governance_config: GovernanceConfiguration,
    // ---  Weight Encryption ---
}

pub struct DefaultSubnetParams<T: Config>(sp_std::marker::PhantomData<((), T)>);

impl<T: Config> DefaultSubnetParams<T> {
    /// Returns default subnet parameters.
    /// Default values are generated from constants defined in the subnet_includes macro.
    pub fn get() -> SubnetParams<T> {
        SubnetParams {
            name: BoundedVec::default(),
            tempo: TempoDefaultValue::get(),
            immunity_period: ImmunityPeriodDefaultValue::get(),
            min_allowed_weights: MinAllowedWeightsDefaultValue::get(),
            max_allowed_weights: MaxAllowedWeightsDefaultValue::get(),
            max_allowed_uids: MaxAllowedUidsDefaultValue::get(),
            max_weight_age: MaxWeightAgeDefaultValue::get(),
            founder_share: FloorFounderShare::<T>::get() as u16,
            incentive_ratio: IncentiveRatioDefaultValue::get(),
            founder: DefaultKey::<T>::get(),
            maximum_set_weight_calls_per_epoch: None,
            bonds_ma: BondsMovingAverageDefaultValue::get(),

            // --- Registrations ---
            module_burn_config: GeneralBurnConfiguration::<T>::default_for(BurnType::Module),
            min_validator_stake: T::DefaultMinValidatorStake::get(),
            max_allowed_validators: None,
            governance_config: GovernanceConfiguration {
                vote_mode: VoteMode::Authority,
                ..Default::default()
            },
            metadata: None,

        }
    }
}

/// Wrapper for enhanced type safety.
/// This exists to make sure we always validate the parameters before using them.
#[derive(Clone, Debug)]
pub struct ValidatedSubnetParams<T: Config> {
    inner: SubnetParams<T>,
    _validated: PhantomData<()>,
}

#[derive(Debug)]
pub struct SubnetChangeset<T: Config> {
    params: ValidatedSubnetParams<T>,
    _validated: PhantomData<()>,
}

impl<T: Config> Deref for ValidatedSubnetParams<T> {
    type Target = SubnetParams<T>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T: Config> DerefMut for ValidatedSubnetParams<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

const MIN_TEMPO: u16 = 25;
const MAX_FOUNDER_SHARE: u16 = 100;
const MAX_INCENTIVE_RATIO: u16 = 100;
const MIN_ALLOWED_WEIGHTS: u16 = 1;
const MAX_VALIDATOR_STAKE: u64 = 250_000_000_000_000;
const MAX_COPIER_MARGIN: f64 = 1.0;
const MIN_ALLOWED_VALIDATORS: u16 = 10;
const MIN_SET_WEIGHT_CALLS: u16 = 1;
const MAX_ENCRYPTION_DURATION: u64 = 10_800 * 2; // 2 days

impl<T: Config> ValidatedSubnetParams<T> {
    pub fn new(params: SubnetParams<T>, netuid: Option<u16>) -> Result<Self, DispatchError> {
        Self::validate(netuid, &params)?;
        Ok(Self {
            inner: params,
            _validated: PhantomData,
        })
    }

    pub fn into_inner(self) -> SubnetParams<T> {
        self.inner
    }

    #[deny(unused_variables)]
    fn validate(netuid: Option<u16>, params: &SubnetParams<T>) -> DispatchResult {
        let global_params = Pallet::<T>::global_params();

        // ! Keep It Written Like This To Enhance Safety Of Unused
        let SubnetParams {
            founder: _, // complete freedom
            founder_share,
            immunity_period: _, // complete freedom
            incentive_ratio,
            max_allowed_uids,
            max_allowed_weights,
            min_allowed_weights,
            max_weight_age,
            name,
            metadata,
            tempo,
            maximum_set_weight_calls_per_epoch,
            bonds_ma: _,           // TODO: validate
            module_burn_config: _, // not validated
            min_validator_stake,
            max_allowed_validators,
            governance_config: _,      // TODO: validate
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
            *min_allowed_weights >= MIN_ALLOWED_WEIGHTS,
            Error::<T>::InvalidMinAllowedWeights
        );

        // Validate metadata if present
        if let Some(meta) = metadata {
            ensure!(!meta.is_empty(), Error::<T>::InvalidSubnetMetadata);
        }

        // Validate tempo and weight age
        ensure!(tempo >= &MIN_TEMPO, Error::<T>::InvalidTempo);
        ensure!(
            *max_weight_age > u64::from(*tempo),
            Error::<T>::InvalidMaxWeightAge
        );

        // Validate UIDs
        ensure!(*max_allowed_uids > 0, Error::<T>::InvalidMaxAllowedUids);

        // Validate shares and ratios
        ensure!(
            *founder_share <= MAX_FOUNDER_SHARE,
            Error::<T>::InvalidFounderShare
        );
        ensure!(
            *founder_share >= FloorFounderShare::<T>::get() as u16,
            Error::<T>::InvalidFounderShare
        );
        ensure!(
            *incentive_ratio <= MAX_INCENTIVE_RATIO,
            Error::<T>::InvalidIncentiveRatio
        );

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
            *min_validator_stake <= MAX_VALIDATOR_STAKE,
            Error::<T>::InvalidMinValidatorStake
        );

        if let Some(max_validators) = max_allowed_validators {
            ensure!(
                *max_validators >= MIN_ALLOWED_VALIDATORS,
                Error::<T>::InvalidMaxAllowedValidators
            );
        }

        if let Some(max_calls) = maximum_set_weight_calls_per_epoch {
            ensure!(
                *max_calls >= MIN_SET_WEIGHT_CALLS,
                Error::<T>::InvalidMaximumSetWeightCallsPerEpoch
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

impl<T: Config> SubnetChangeset<T> {
    pub fn params(&self) -> &ValidatedSubnetParams<T> {
        &self.params
    }

    pub fn new(params: SubnetParams<T>) -> Result<Self, DispatchError> {
        Ok(Self {
            params: ValidatedSubnetParams::new(params, None)?,
            _validated: PhantomData,
        })
    }

    pub fn update(netuid: u16, params: SubnetParams<T>) -> Result<Self, DispatchError> {
        Ok(Self {
            params: ValidatedSubnetParams::new(params, Some(netuid))?,
            _validated: PhantomData,
        })
    }

    #[deny(unused_variables)]
    pub fn apply(self, netuid: u16) -> DispatchResult {
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
        } = self.params.into_inner();

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
        if let Some(max_calls) = maximum_set_weight_calls_per_epoch {
            if max_calls == 0 {
                MaximumSetWeightCallsPerEpoch::<T>::remove(netuid);
            } else {
                MaximumSetWeightCallsPerEpoch::<T>::insert(netuid, max_calls);
            }
        }
        T::update_subnet_governance_configuration(netuid, governance_config)?;
        if let Some(meta) = &metadata {
            SubnetMetadata::<T>::insert(netuid, meta);
        }
        MaxAllowedValidators::<T>::insert(netuid, max_allowed_validators);
        Pallet::<T>::deposit_event(Event::SubnetParamsUpdated(netuid));

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
            maximum_set_weight_calls_per_epoch: MaximumSetWeightCallsPerEpoch::<T>::get(netuid),
            bonds_ma: BondsMovingAverage::<T>::get(netuid),

            // --- Registrations ---
            module_burn_config: ModuleBurnConfig::<T>::get(netuid),
            min_validator_stake: MinValidatorStake::<T>::get(netuid),
            max_allowed_validators: MaxAllowedValidators::<T>::get(netuid),
            governance_config: T::get_subnet_governance_configuration(netuid),
            metadata: SubnetMetadata::<T>::get(netuid),

        }
    }
}
