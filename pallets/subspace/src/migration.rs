use super::*;

use frame_support::codec::{Decode, Encode};
use scale_info::TypeInfo;
use frame_support::traits::StorageInstance;
use frame_support::{
    pallet_prelude::StorageValue,
    traits::{Get, OnRuntimeUpgrade, StorageVersion},
    weights::Weight,
};


#[derive(Decode, Encode, TypeInfo, Default)]
pub struct OldGlobalParams {
    pub burn_rate: u16,
    pub max_name_length: u16,
    pub max_allowed_subnets: u16,
    pub max_allowed_modules: u16,
    pub max_registrations_per_block: u16,
    pub max_allowed_weights: u16,
    pub max_proposals: u64,
    pub min_burn: u64,
    pub min_stake: u64,
    pub min_weight_stake: u64,
    pub unit_emission: u64,
    pub tx_rate_limit: u64,
    pub vote_threshold: u16,
    pub vote_mode: Vec<u8>,
}

impl<T: Config> StorageInstance for Pallet<T> {
    fn pallet_prefix() -> &'static str {
        "Subspace"
    }

    const STORAGE_PREFIX: &'static str = "Subspace";
}

pub mod v1 {
    use super::*;

    pub struct MigrateToV1<T>(sp_std::marker::PhantomData<T>);

    impl<T: Config> OnRuntimeUpgrade for MigrateToV1<T> {
        fn on_runtime_upgrade() -> Weight {
            let on_chain_version = StorageVersion::get::<Pallet<T>>();
            if on_chain_version < 2 {
                let encoded = StorageValue::<Pallet<T>, OldGlobalParams>::get().unwrap_or_default().encode();
                let old_global_params = OldGlobalParams::decode(&mut encoded.as_slice())
                    .expect("Decoding old global params failed");

                let new_global_params = GlobalParams {
                    burn_rate: old_global_params.burn_rate,
                    max_name_length: old_global_params.max_name_length,
                    max_allowed_subnets: old_global_params.max_allowed_subnets,
                    max_allowed_modules: old_global_params.max_allowed_modules,
                    max_registrations_per_block: old_global_params.max_registrations_per_block,
                    max_allowed_weights: old_global_params.max_allowed_weights,
                    max_proposals: old_global_params.max_proposals,
                    min_burn: old_global_params.min_burn,
                    min_stake: old_global_params.min_stake,
                    min_weight_stake: old_global_params.min_weight_stake,
                    unit_emission: old_global_params.unit_emission,
                    tx_rate_limit: old_global_params.tx_rate_limit,
                    vote_threshold: old_global_params.vote_threshold,
                    vote_mode: old_global_params.vote_mode,
                    max_burn: DefaultMaxBurn::<T>::get(),
                    min_delegation_fee: DefaultMinDelegationFeeGlobal::<T>::get(),
                    target_registrations_per_interval: DefaultTargetRegistrationsPerInterval::<T>::get(),
                    target_registrations_interval: DefaultTargetRegistrationsInterval::<T>::get(),
                    adjustment_alpha: DefaultAdjustmentAlpha::<T>::get(),
                };

                StorageValue::<Pallet<T>, GlobalParams>::put(&new_global_params);
                StorageVersion::new(2).put::<Pallet<T>>();

                log::info!("Migrated GlobalParams to v2");
                
                // TODO: I am not sure if this is correct at all
                T::DbWeight::get().writes(2)
            } else {
                log::info!("GlobalParams already updated");
                Weight::zero()
            }
        }
    }
}