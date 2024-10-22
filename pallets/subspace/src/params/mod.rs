pub mod global;

use crate::Config;

use crate::{BurnType, DefaultKey, FloorFounderShare, GeneralBurnConfiguration};
use pallet_governance_api::VoteMode;
use sp_std::vec::Vec;

use frame_support::{pallet_prelude::*, BoundedVec};
use pallet_governance_api::GovernanceConfiguration;
use sp_arithmetic::per_things::Percent;
use substrate_fixed::types::I64F64;

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
    pub floor_delegation_fee: Percent, // min delegation fee
    pub floor_founder_share: u8,       // min founder share
    pub min_weight_stake: u64,         // min weight stake required

    // S0 governance
    pub curator: T::AccountId,
    pub general_subnet_application_cost: u64,

    // Other
    pub subnet_immunity_period: u64,
    pub governance_config: GovernanceConfiguration,

    pub kappa: u16,
    pub rho: u16,
}

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
}

#[derive(Decode, Encode, PartialEq, Eq, Clone, Debug, TypeInfo)]
#[scale_info(skip_type_params(T))]
pub struct ModuleParams<T: Config> {
    pub name: Vec<u8>,
    pub address: Vec<u8>,
    pub delegation_fee: Percent,
    pub metadata: Option<Vec<u8>>,
    pub controller: T::AccountId,
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
        }
    }
}
