use super::*;
use core::marker::PhantomData;

use frame_support::{
    pallet_prelude::Weight,
    traits::{OnRuntimeUpgrade, StorageVersion},
};
use pallet_subnet_emission_api::SubnetConsensus;
use pallet_subspace::{Pallet as PalletSubspace, Vec};

#[derive(Default)]
pub struct InitialMigration<T>(PhantomData<T>);

impl<T: Config + pallet_subspace::Config> OnRuntimeUpgrade for InitialMigration<T> {
    fn on_runtime_upgrade() -> frame_support::weights::Weight {
        if StorageVersion::get::<Pallet<T>>() != 0 {
            return frame_support::weights::Weight::zero();
        }
        log::info!("Initializing subnet pricing pallet, importing proposals...");

        // Storage moving Subacpace -> SubnetEmission
        // ---------------------------------------------------------------------

        // --- 1. Unit emission moving
        let old_unit_emission =
            pallet_subspace::migrations::v12::old_storage::UnitEmission::<T>::get();

        if old_unit_emission > 0 {
            crate::UnitEmission::<T>::put(old_unit_emission);
        }
        log::info!(
            "Migrated UnitEmission: {:?}",
            crate::UnitEmission::<T>::get()
        );

        // --- 2.  Pending emission moving
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

        // --- 3. Subnet emission moving
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

        // Initaliazation + Overwrites
        // ---------------------------------------------------------------------

        // --- 4. Subnet consensus initialization

        for subnet_id in pallet_subspace::N::<T>::iter_keys() {
            if SubnetConsensusType::<T>::get(subnet_id).is_none() {
                log::info!("Setting Yuma consensus for subnet {}", subnet_id);
                SubnetConsensusType::<T>::set(subnet_id, Some(SubnetConsensus::Yuma));
            }
        }

        // --- 5. Temporary post update block emission lowering

        let block = PalletSubspace::<T>::get_current_block_number();
        let unit_emission: u64 = 23148148148;

        // --- 5.1 Initalization of new temporary storage
        OriginalUnitEmission::<T>::set(unit_emission);
        log::info!(
            "Original unit emission set to {}",
            OriginalUnitEmission::<T>::get()
        );

        // --- 5.2 Do the actual block emission lowering
        UnitEmission::<T>::set(unit_emission / 3);
        log::info!("Unit emission updated to {}", UnitEmission::<T>::get());

        // --- 5.3 Snapshot the block number for the emission lowering
        EmissionLoweringBlock::<T>::set(block);
        log::info!(
            "EmissionLoweringBlock set to {}",
            EmissionLoweringBlock::<T>::get()
        );

        // --- 6 Done
        log::info!("==Subnet pricing pallet initialized==");

        Weight::zero()
    }
}
