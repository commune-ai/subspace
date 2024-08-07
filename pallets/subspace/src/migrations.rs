use super::*;

use frame_support::traits::{Get, StorageInstance, StorageVersion};

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
pub mod v13 {
    use super::*;
    use frame_support::traits::OnRuntimeUpgrade;
    use sp_runtime::Percent;

    pub mod old_storage {
        use super::*;
        use frame_support::{storage_alias, Blake2_128Concat, Identity};
        use sp_runtime::Percent;

        #[storage_alias]
        pub type DelegationFee<T: Config> =
            StorageDoubleMap<Pallet<T>, Identity, u16, Blake2_128Concat, AccountIdOf<T>, Percent>;
    }

    pub struct MigrateToV13<T>(sp_std::marker::PhantomData<T>);

    impl<T: Config> OnRuntimeUpgrade for MigrateToV13<T> {
        fn on_runtime_upgrade() -> Weight {
            let on_chain_version = StorageVersion::get::<Pallet<T>>();
            if on_chain_version != 12 {
                log::info!("Storage v13 already updated");
                return Weight::zero();
            }
            log::info!("Migrating storage to v13");

            let old_delegation_fee_keys = old_storage::DelegationFee::<T>::iter()
                .map(|(_, key, _)| key)
                .collect::<BTreeSet<_>>();
            let _ = old_storage::DelegationFee::<T>::clear(u32::MAX, None);

            for key in old_delegation_fee_keys {
                DelegationFee::<T>::set(key, Percent::from_percent(5));
            }

            log::info!("Migrated storage to v13");

            StorageVersion::new(13).put::<Pallet<T>>();
            T::DbWeight::get().reads_writes(1, 1)
        }
    }
}
