use crate::*;
use frame_support::{
    pallet_prelude::ValueQuery,
    traits::{ConstU32, Get, StorageVersion},
};

pub mod v2 {
    use dao::CuratorApplication;
    use frame_support::{traits::OnRuntimeUpgrade, weights::Weight};

    use super::*;

    pub mod old_storage {
        use super::*;
        use dao::ApplicationStatus;
        use frame_support::{pallet_prelude::TypeInfo, storage_alias, Identity};
        use pallet_subspace::AccountIdOf;
        use parity_scale_codec::{Decode, Encode};
        use sp_runtime::BoundedVec;

        #[derive(Encode, Decode, TypeInfo)]
        pub struct CuratorApplication<T: Config> {
            pub id: u64,
            pub user_id: T::AccountId,
            pub paying_for: T::AccountId,
            pub data: BoundedVec<u8, ConstU32<256>>,
            pub status: ApplicationStatus,
            pub application_cost: u64,
        }

        #[storage_alias]
        pub type CuratorApplications<T: Config> =
            StorageMap<Pallet<T>, Identity, u64, CuratorApplication<T>>;

        #[storage_alias]
        pub type LegitWhitelist<T: Config> =
            StorageMap<Pallet<T>, Identity, AccountIdOf<T>, u8, ValueQuery>;
    }

    pub struct MigrateToV2<T>(sp_std::marker::PhantomData<T>);

    impl<T: Config> OnRuntimeUpgrade for MigrateToV2<T> {
        fn on_runtime_upgrade() -> frame_support::weights::Weight {
            let on_chain_version = StorageVersion::get::<Pallet<T>>();
            if on_chain_version != 1 {
                log::info!("Storage v2 already updated");
                return Weight::zero();
            }

            StorageVersion::new(2).put::<Pallet<T>>();

            CuratorApplications::<T>::translate(
                |_key, old_value: v2::old_storage::CuratorApplication<T>| {
                    Some(CuratorApplication {
                        id: old_value.id,
                        user_id: old_value.user_id,
                        paying_for: old_value.paying_for,
                        data: old_value.data,
                        status: old_value.status,
                        application_cost: old_value.application_cost,
                        block_number: 0,
                    })
                },
            );

            let old_whitelist: Vec<_> = old_storage::LegitWhitelist::<T>::iter().collect();
            _ = old_storage::LegitWhitelist::<T>::clear(u32::MAX, None);

            for (account, _) in old_whitelist {
                LegitWhitelist::<T>::insert(account, ());
            }

            log::info!("Migrated to v2");

            T::DbWeight::get().reads_writes(2, 2)
        }
    }
}
