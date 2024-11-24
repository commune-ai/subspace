use crate::*;
use frame_support::{
    pallet_prelude::Weight,
    traits::{OnRuntimeUpgrade, StorageVersion},
};

// pub mod v1 {
//     use super::*;

//     pub struct MigrateToV1<T>(sp_std::marker::PhantomData<T>);

//     impl<T: Config> OnRuntimeUpgrade for MigrateToV1<T> {
//         fn on_runtime_upgrade() -> frame_support::weights::Weight {
//             let on_chain_version = StorageVersion::get::<Pallet<T>>();
//             if on_chain_version != 0 {
//                 log::info!("Storage v1 already updated");
//                 return Weight::zero();
//             }

//             StorageVersion::new(1).put::<Pallet<T>>();

//             pallet_subspace::migrations::v15::old_storage::Weights::<T>::iter().for_each(
//                 |(netuid, uid, values)| {
//                     log::info!("migrating weights for netuid: {}, uid: {}", netuid, uid);
//                     Weights::<T>::insert(netuid, uid, values);
//                 },
//             );

//             // Just for clarity, (although not explicitly needed)
//             let _ =
//                 pallet_subspace::migrations::v15::old_storage::Weights::<T>::clear(u32::MAX,
// None);

//             log::info!("Migrated to v1");

//             Weight::zero()
//         }
//     }
// }

pub mod v1 {
    use core::u32;

    use super::*;

    pub struct MigrateToV1<T>(sp_std::marker::PhantomData<T>);

    impl<T: Config> OnRuntimeUpgrade for MigrateToV1<T> {
        fn on_runtime_upgrade() -> frame_support::weights::Weight {
            let on_chain_version = StorageVersion::get::<Pallet<T>>();
            if on_chain_version != 15 {
                log::info!("Storage v1 already updated");
                return Weight::zero();
            }

            StorageVersion::new(16).put::<Pallet<T>>();

            let _ = BannedDecryptionNodes::<T>::clear(u32::MAX, None);
            let _ = DecryptionNodeBanQueue::<T>::clear(u32::MAX, None);

            // Just for clarity, (although not explicitly needed)
            log::info!("Migrated to v1");

            Weight::zero()
        }
    }
}

// // Kill all weight DEW related data
// let _ = ConsensusParameters::<T>::clear(u32::MAX, None);
// let _ = SubnetDecryptionData::<T>::clear(u32::MAX, None);
// let _ = ConsensusParameters::<T>::clear(u32::MAX, None);
// let _ = WeightEncryptionData::<T>::clear(u32::MAX, None);
// let _ = DecryptedWeights::<T>::clear(u32::MAX, None);
// let _ = BannedDecryptionNodes::<T>::clear(u32::MAX, None);
// let _ = DecryptionNodes::<T>::kill();

// let _ = WeightSettingDelegation::<T>::clear(u32::MAX, None);
// let _ = Weights::<T>::clear_prefix(5, u32::MAX, None);
