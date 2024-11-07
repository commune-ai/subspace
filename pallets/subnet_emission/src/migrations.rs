// use crate::*;
// use frame_support::{
//     pallet_prelude::Weight,
//     traits::{Get, OnRuntimeUpgrade, StorageVersion},
// };

// pub mod v6 {
//     use super::*;

//     pub struct MigrateToV6<T>(sp_std::marker::PhantomData<T>);

//     impl<T: Config> OnRuntimeUpgrade for MigrateToV6<T> {
//         fn on_runtime_upgrade() -> frame_support::weights::Weight {
//             let on_chain_version = StorageVersion::get::<Pallet<T>>();
//             if on_chain_version != 5 {
//                 log::info!("Storage v4 already updated");
//                 return Weight::zero();
//             }

//             StorageVersion::new(6).put::<Pallet<T>>();

//             let _ = Authorities::<T>::kill();
//             let _ = DecryptionNodes::<T>::kill();
//             let _ = SubnetDecryptionData::<T>::clear(u32::MAX, None);
//             let _ = BannedDecryptionNodes::<T>::clear(u32::MAX, None);
//             log::info!("Migrated to v2");

//             T::DbWeight::get().reads_writes(2, 2)
//         }
//     }
// }
