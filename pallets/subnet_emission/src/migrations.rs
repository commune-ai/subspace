use crate::*;
use frame_support::{
    pallet_prelude::Weight,
    traits::{Get, OnRuntimeUpgrade, StorageVersion},
};

pub mod v3 {
    use super::*;

    pub struct MigrateToV3<T>(sp_std::marker::PhantomData<T>);

    impl<T: Config> OnRuntimeUpgrade for MigrateToV3<T> {
        fn on_runtime_upgrade() -> frame_support::weights::Weight {
            let on_chain_version = StorageVersion::get::<Pallet<T>>();
            if on_chain_version != 2 {
                log::info!("Storage v3 already updated");
                return Weight::zero();
            }

            StorageVersion::new(3).put::<Pallet<T>>();

            let _ = BannedDecryptionNodes::<T>::clear(u32::MAX, None);
            log::info!("Migrated to v2");

            T::DbWeight::get().reads_writes(2, 2)
        }
    }
}
