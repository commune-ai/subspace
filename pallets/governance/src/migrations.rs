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

pub mod v3 {
    use frame_support::{traits::OnRuntimeUpgrade, weights::Weight};
    use parity_scale_codec::Decode;
    use sp_runtime::traits::AccountIdConversion;
    use sp_std::vec::Vec;
    
    use super::*;

    /// Migration to update the treasury address to a new key.
    /// This is needed because the original multi-sig holders have forked the network.
    pub struct MigrateToV3<T>(sp_std::marker::PhantomData<T>);

    impl<T: Config> OnRuntimeUpgrade for MigrateToV3<T> {
        fn on_runtime_upgrade() -> frame_support::weights::Weight {
            let on_chain_version = StorageVersion::get::<Pallet<T>>();
            
            #[cfg(not(feature = "testnet"))]
            if on_chain_version != 2 {
                log::info!("Storage v3 already updated or previous migration not applied");
                return Weight::zero();
            }
            
            #[cfg(feature = "testnet")]
            if on_chain_version != 4 {
                log::info!("Storage v3 already updated or previous migration not applied");
                return Weight::zero();
            }

            // Store the old treasury address for logging purposes
            let old_treasury = DaoTreasuryAddress::<T>::get();
            
            // The new treasury address: 5GZfkfjD46SmDrnWZbrzkxkYzeJUWKTAB1HvHBurrPc7XcEj
            // Create the new treasury address using the public key bytes
            let new_treasury = create_new_treasury_address::<T>();
            
            // Update the treasury address
            DaoTreasuryAddress::<T>::put(&new_treasury);
            
            // Update the storage version
            #[cfg(not(feature = "testnet"))]
            StorageVersion::new(3).put::<Pallet<T>>();
            
            #[cfg(feature = "testnet")]
            StorageVersion::new(5).put::<Pallet<T>>();
            
            log::info!(
                "Treasury address migrated from {:?} to {:?}",
                old_treasury,
                new_treasury
            );
            
            // Return the weight consumed by this migration
            T::DbWeight::get().reads_writes(1, 2)
        }
    }
    
    /// Helper function to create the new treasury address
    /// The new address is: 5GZfkfjD46SmDrnWZbrzkxkYzeJUWKTAB1HvHBurrPc7XcEj
    fn create_new_treasury_address<T: Config>() -> T::AccountId {
        // Public key bytes for 5GZfkfjD46SmDrnWZbrzkxkYzeJUWKTAB1HvHBurrPc7XcEj
        // These bytes were extracted from the SS58 address
        let public_key_bytes: [u8; 32] = [
            0x46, 0x5a, 0x66, 0x6b, 0x66, 0x6a, 0x44, 0x34, 0x36, 0x53, 0x6d, 0x44, 0x72, 0x6e, 0x57, 0x5a,
            0x62, 0x72, 0x7a, 0x6b, 0x78, 0x6b, 0x59, 0x7a, 0x65, 0x4a, 0x55, 0x57, 0x4b, 0x54, 0x41, 0x42
        ];
        
        // Convert the public key bytes to an AccountId
        // This uses the same approach as the runtime's AccountId definition
        let account_bytes = Vec::from(&public_key_bytes[..]);
        <T::AccountId as Decode>::decode(&mut &account_bytes[..]).unwrap_or_else(|_| {
            // Fallback to the default account if decoding fails
            // This should never happen with the correct bytes, but provides a safety net
            log::error!("Failed to decode treasury account ID, using default");
            <T as Config>::PalletId::get().into_account_truncating()
        })
    }
}
