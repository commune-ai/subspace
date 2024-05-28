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

pub mod v8 {
    use super::*;

    pub struct MigrateToV8<T>(sp_std::marker::PhantomData<T>);

    impl<T: Config> OnRuntimeUpgrade for MigrateToV8<T> {
        fn on_runtime_upgrade() -> Weight {
            let on_chain_version = StorageVersion::get::<Pallet<T>>();

            if on_chain_version != 7 {
                log::info!("Storage v8 already updated");
                return Weight::zero();
            }

            let new_registration_interval = 142;
            let new_target_registrations_per_interval = 4;

            for netuid in N::<T>::iter_keys() {
                TargetRegistrationsInterval::<T>::insert(netuid, new_registration_interval);
                TargetRegistrationsPerInterval::<T>::insert(
                    netuid,
                    new_target_registrations_per_interval,
                );
            }
            log::info!("Migrated Registration Intervals to V8");

            MaxRegistrationsPerBlock::<T>::put(3);
            log::info!("Migrated Registration Intervals to V8");

            StorageVersion::new(8).put::<Pallet<T>>();
            log::info!("Migrated Registration Intervals to V8");

            T::DbWeight::get().writes(1)
        }
    }
}
