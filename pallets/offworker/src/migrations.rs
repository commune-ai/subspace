use crate::*;
use frame_support::{
    pallet_prelude::Weight,
    traits::{OnRuntimeUpgrade, StorageVersion},
};

pub mod v1 {
    use super::*;

    pub struct MigrateToV1<T>(sp_std::marker::PhantomData<T>);

    impl<T: Config> OnRuntimeUpgrade for MigrateToV1<T> {
        fn on_runtime_upgrade() -> frame_support::weights::Weight {
            let on_chain_version = StorageVersion::get::<Pallet<T>>();
            if on_chain_version != 0 {
                log::info!("Storage v1 already updated");
                return Weight::zero();
            }

            StorageVersion::new(1).put::<Pallet<T>>();

            Authorities::<T>::kill();

            log::info!("Migrated to v1");

            Weight::zero()
        }
    }
}
