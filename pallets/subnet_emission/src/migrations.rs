use crate::*;
use frame_support::{
    pallet_prelude::Weight,
    traits::{OnRuntimeUpgrade, StorageVersion},
};

pub mod v2 {
    use super::*;

    pub struct MigrateToV2<T>(sp_std::marker::PhantomData<T>);

    impl<T: Config> OnRuntimeUpgrade for MigrateToV2<T> {
        fn on_runtime_upgrade() -> frame_support::weights::Weight {
            let on_chain_version = StorageVersion::get::<Pallet<T>>();
            if on_chain_version != 1 {
                log::info!("Storage v2 already updated");
                return Weight::zero();
            }
            crate::UnitEmission::<T>::put(6427777777);
            StorageVersion::new(2).put::<Pallet<T>>();
            log::info!("Migrated to v2");
            Weight::zero()
        }
    }
}
