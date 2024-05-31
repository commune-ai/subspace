use super::*;

use frame_support::{
    traits::{Get, OnRuntimeUpgrade, StorageInstance, StorageVersion},
    weights::Weight,
};

impl<T: Config> StorageInstance for Pallet<T> {
    fn pallet_prefix() -> &'static str {
        "Subspace"
    }

    const STORAGE_PREFIX: &'static str = "Subspace";
}

use sp_core::crypto::Ss58Codec;
use sp_runtime::AccountId32;

pub fn ss58_to_account_id<T: Config>(
    ss58_address: &str,
) -> Result<T::AccountId, sp_core::crypto::PublicError> {
    let account_id = AccountId32::from_ss58check(ss58_address)?;
    let account_id_vec = account_id.encode();
    Ok(T::AccountId::decode(&mut &account_id_vec[..]).unwrap())
}

pub mod v9 {
    use super::*;

    pub struct MigrateToV9<T>(sp_std::marker::PhantomData<T>);

    impl<T: Config> OnRuntimeUpgrade for MigrateToV9<T> {
        fn on_runtime_upgrade() -> Weight {
            let on_chain_version = StorageVersion::get::<Pallet<T>>();

            if on_chain_version != 8 {
                log::info!("Storage v9 already updated");
                return Weight::zero();
            }

            // Clear the old registration storages
            // ====================================

            let _ = TargetRegistrationsInterval::<T>::clear(u32::MAX, None);
            let _ = TargetRegistrationsPerInterval::<T>::clear(u32::MAX, None);
            let _ = MaxRegistrationsPerInterval::<T>::clear(u32::MAX, None);

            let new_registration_interval = 142;
            let new_target_registrations_per_interval = 4;
            let max_registration_per_interval = 42;

            for netuid in N::<T>::iter_keys() {
                TargetRegistrationsInterval::<T>::insert(netuid, new_registration_interval);
                TargetRegistrationsPerInterval::<T>::insert(
                    netuid,
                    new_target_registrations_per_interval,
                );
                MaxRegistrationsPerInterval::<T>::insert(netuid, max_registration_per_interval);
            }
            log::info!("Migrated Registration Intervals to V8");

            log::info!(
                "Target registration per interval are {:?}",
                TargetRegistrationsPerInterval::<T>::iter().collect::<Vec<_>>()
            );

            log::info!(
                "Target registration interval are {:?}",
                TargetRegistrationsInterval::<T>::iter().collect::<Vec<_>>()
            );

            log::info!(
                "Max registration per interval are {:?}",
                MaxRegistrationsPerInterval::<T>::iter().collect::<Vec<_>>()
            );

            StorageVersion::new(9).put::<Pallet<T>>();
            log::info!("Migrated Registration Intervals to V9");

            T::DbWeight::get().writes(1)
        }
    }
}
pub mod v10 {
    use super::*;

    pub struct MigrateToV10<T>(sp_std::marker::PhantomData<T>);

    impl<T: Config> OnRuntimeUpgrade for MigrateToV10<T> {
        fn on_runtime_upgrade() -> Weight {
            let on_chain_version = StorageVersion::get::<Pallet<T>>();

            if on_chain_version != 9 {
                log::info!("Storage v10 already updated");
                return Weight::zero();
            }

            // Allow more scaling in max_allowed_modules
            MaxAllowedModules::<T>::put(20_000);
            log::info!("Migrated MaxAllowedModules to V10");

            let subnet_0_netuid = 0;
            // Due to the size of the migration, we don't scale below 7k modules
            // Additial modules will be removed by the next migration
            let max_allowed_uids = 7_000; // Current 8k +
                                          // Register modules on s0 that

            let mut total_modules = N::<T>::get(subnet_0_netuid);
            while total_modules > max_allowed_uids {
                let lowest_uid = Pallet::<T>::get_lowest_uid(subnet_0_netuid, false);
                if let Some(uid) = lowest_uid {
                    Pallet::<T>::remove_module(subnet_0_netuid, uid);
                    total_modules -= 1;
                } else {
                    break;
                }
            }
            MaxAllowedUids::<T>::insert(subnet_0_netuid, max_allowed_uids);
            log::info!("Migrated Modules on subnet 0 to V10");

            log::info!(
                "Module amount on subnet 0 is {:?}",
                N::<T>::get(subnet_0_netuid)
            );

            StorageVersion::new(10).put::<Pallet<T>>();
            log::info!("Migrated Registration Intervals to V9");

            let mut gaps = BTreeSet::new();
            let netuids: BTreeSet<_> = N::<T>::iter_keys().collect();
            for netuid in 0..netuids.last().copied().unwrap_or_default() {
                if !netuids.contains(&netuid) {
                    gaps.insert(netuid);
                }
            }

            log::info!("Existing subnets: {netuids:?}");
            log::info!("Updated subnets gaps: {gaps:?}");
            SubnetGaps::<T>::set(gaps);

            T::DbWeight::get().writes(1)
        }
    }
}
