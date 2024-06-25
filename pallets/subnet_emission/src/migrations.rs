use super::*;
use core::marker::PhantomData;

use frame_support::{
    pallet_prelude::{ValueQuery, Weight},
    traits::{OnRuntimeUpgrade, StorageVersion},
    Identity,
};
use pallet_subspace::Vec;

#[derive(Default)]
pub struct InitialMigration<T>(PhantomData<T>);

pub mod old_storage {
    use super::*;
    use frame_support::storage_alias;

    #[storage_alias]
    pub type UnitEmission<T: Config> = StorageValue<Pallet<T>, u64, ValueQuery>;

    #[storage_alias]
    pub type PendingEmission<T: Config> = StorageMap<Pallet<T>, Identity, u16, u64, ValueQuery>;
}

// TODO:
impl<T: Config + pallet_subspace::Config> OnRuntimeUpgrade for InitialMigration<T> {
    fn on_runtime_upgrade() -> frame_support::weights::Weight {
        if StorageVersion::get::<Pallet<T>>() != 0 {
            return frame_support::weights::Weight::zero();
        }
        log::info!("Initializing subnet pricing pallet, importing proposals...");

        let old_unit_emission = old_storage::UnitEmission::<T>::get();
        crate::UnitEmission::<T>::put(old_unit_emission);
        log::info!(
            "Migrated UnitEmission: {:?}",
            crate::UnitEmission::<T>::get()
        );

        let old_pending_emission = old_storage::PendingEmission::<T>::iter().collect::<Vec<_>>();
        for (subnet_id, emission) in old_pending_emission {
            crate::PendingEmission::<T>::insert(subnet_id, emission);
        }

        log::info!(
            "Migrated PendingEmission: {:?}",
            crate::PendingEmission::<T>::iter().collect::<Vec<_>>()
        );

        Weight::zero()
    }
}
