use super::*;
use core::marker::PhantomData;

use frame_support::{
    pallet_prelude::Weight,
    traits::{OnRuntimeUpgrade, StorageVersion},
};
use pallet_subnet_emission_api::SubnetConsensus;
use pallet_subspace::Vec;

#[derive(Default)]
pub struct InitialMigration<T>(PhantomData<T>);

// TODO:
impl<T: Config + pallet_subspace::Config> OnRuntimeUpgrade for InitialMigration<T> {
    fn on_runtime_upgrade() -> frame_support::weights::Weight {
        if StorageVersion::get::<Pallet<T>>() != 0 {
            return frame_support::weights::Weight::zero();
        }
        log::info!("Initializing subnet pricing pallet, importing proposals...");

        let old_unit_emission =
            pallet_subspace::migrations::v12::old_storage::UnitEmission::<T>::get();
        if old_unit_emission > 0 {
            crate::UnitEmission::<T>::put(old_unit_emission);
        }
        log::info!(
            "Migrated UnitEmission: {:?}",
            crate::UnitEmission::<T>::get()
        );

        let old_pending_emission =
            pallet_subspace::migrations::v12::old_storage::PendingEmission::<T>::iter()
                .collect::<Vec<_>>();
        for (subnet_id, emission) in old_pending_emission {
            crate::PendingEmission::<T>::insert(subnet_id, emission);
        }

        log::info!(
            "Migrated PendingEmission: {:?}",
            crate::PendingEmission::<T>::iter().collect::<Vec<_>>()
        );

        let old_subnet_emission =
            pallet_subspace::migrations::v12::old_storage::SubnetEmission::<T>::iter()
                .collect::<Vec<_>>();
        for (subnet_id, emission) in old_subnet_emission {
            crate::SubnetEmission::<T>::insert(subnet_id, emission);
        }

        log::info!(
            "Migrated SubnetEmission: {:?}",
            crate::SubnetEmission::<T>::iter().collect::<Vec<_>>()
        );

        for subnet_id in pallet_subspace::N::<T>::iter_keys() {
            if SubnetConsensusType::<T>::get(subnet_id).is_none() {
                log::info!("Setting Yuma consensus for subnet {}", subnet_id);
                SubnetConsensusType::<T>::set(subnet_id, Some(SubnetConsensus::Yuma));
            }
        }

        Weight::zero()
    }
}
