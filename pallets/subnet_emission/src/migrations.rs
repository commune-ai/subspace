use crate::*;
use frame_support::{
    pallet_prelude::Weight,
    traits::{Get, OnRuntimeUpgrade, StorageVersion},
};

pub mod v4 {
    use super::*;

    pub struct MigrateToV4<T>(sp_std::marker::PhantomData<T>);

    impl<T: Config> OnRuntimeUpgrade for MigrateToV4<T> {
        fn on_runtime_upgrade() -> frame_support::weights::Weight {
            let on_chain_version = StorageVersion::get::<Pallet<T>>();
            if on_chain_version != 3 {
                log::info!("Storage v4 already updated");
                return Weight::zero();
            }

            StorageVersion::new(4).put::<Pallet<T>>();

            let _ = Authorities::<T>::kill();
            log::info!("Migrated to v2");

            T::DbWeight::get().reads_writes(2, 2)
        }
    }
}
