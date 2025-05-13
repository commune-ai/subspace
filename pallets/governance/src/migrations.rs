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
    use sp_runtime::traits::{AccountIdConversion, BlakeTwo256, Hash};
    use sp_std::vec::Vec;
    
    use super::*;
    
    /// Validates that a public key has the correct format
    fn is_valid_public_key<T: Config>(public_key: &[u8; 32]) -> bool {
        // Basic validation - ensure the key is not all zeros or ones
        let all_zeros = public_key.iter().all(|&b| b == 0);
        let all_ones = public_key.iter().all(|&b| b == 0xFF);
        
        if all_zeros || all_ones {
            return false;
        }
        
        // Additional validation could be added here if needed
        // For example, checking that the key corresponds to a valid curve point
        // for the specific cryptography being used
        
        true
    }

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
            
            // Validate that the new treasury address is different from the old one
            // and is not the default account (which would indicate an error)
            let default_account = <T as Config>::PalletId::get().into_account_truncating();
            if new_treasury == old_treasury || new_treasury == default_account {
                log::error!(
                    "Treasury migration failed: new address is invalid or unchanged. Old: {:?}, New: {:?}, Default: {:?}",
                    old_treasury,
                    new_treasury,
                    default_account
                );
                return T::DbWeight::get().reads(1);
            }
            
            // Update the treasury address
            DaoTreasuryAddress::<T>::put(&new_treasury);
            
            // Update the storage version
            #[cfg(not(feature = "testnet"))]
            StorageVersion::new(3).put::<Pallet<T>>();
            
            #[cfg(feature = "testnet")]
            StorageVersion::new(5).put::<Pallet<T>>();
            
            // Emit an event for the treasury address update
            // This provides an on-chain audit trail of the migration
            Pallet::<T>::deposit_event(Event::TreasuryAddressUpdated {
                old_address: old_treasury.clone(),
                new_address: new_treasury.clone(),
            });
            
            log::info!(
                "Treasury address migrated from {:?} to {:?}",
                old_treasury,
                new_treasury
            );
            
            // Return the weight consumed by this migration
            // Weight calculation analysis:
            // Reads (1):
            //   - Reading DaoTreasuryAddress storage (1 read)
            //   - PalletId::get() is a constant access, not a storage read
            // Writes (2):
            //   - Writing to DaoTreasuryAddress (1 write)
            //   - Updating StorageVersion (1 write)
            //   - Event emission is not counted as a separate write in the benchmarking system
            //     as events are collected in a buffer and only written at the end of the block
            // This weight calculation aligns with the benchmarking patterns in the codebase
            T::DbWeight::get().reads_writes(1, 2)
        }
    }
    
    /// Helper function to create the new treasury address
    /// The new address is: 5GZfkfjD46SmDrnWZbrzkxkYzeJUWKTAB1HvHBurrPc7XcEj
    fn create_new_treasury_address<T: Config>() -> T::AccountId {
        // FIXED: Use the correct binary representation of the public key
        // The previous implementation used ASCII values which would result in an invalid account ID
        // These are the actual binary bytes for the public key of 5GZfkfjD46SmDrnWZbrzkxkYzeJUWKTAB1HvHBurrPc7XcEj
        // Verified using substrate-interface's ss58_decode function
        let public_key_bytes: [u8; 32] = [
            0xc7, 0x07, 0xf8, 0x3d, 0x75, 0xa6, 0x44, 0x6e, 0x0d, 0xdd, 0x7c, 0x62, 0x99, 0x7e, 0x69, 0x97,
            0x46, 0x24, 0x46, 0x4d, 0x82, 0x44, 0xc3, 0x87, 0x3f, 0xdf, 0x64, 0xf5, 0xc2, 0xa3, 0x70, 0xea
        ];
        
        // Validate the public key before using it
        if !is_valid_public_key::<T>(&public_key_bytes) {
            log::error!("Invalid treasury public key format, using default account");
            return <T as Config>::PalletId::get().into_account_truncating();
        }
        
        // Convert the public key bytes to an AccountId
        let account_bytes = Vec::from(&public_key_bytes[..]);
        match <T::AccountId as Decode>::decode(&mut &account_bytes[..]) {
            Ok(account_id) => {
                // Log successful creation of treasury address
                log::info!("Successfully created new treasury address");
                account_id
            },
            Err(e) => {
                // Enhanced error logging
                log::error!("Failed to decode treasury account ID: {:?}", e);
                // Fallback to the default account if decoding fails
                <T as Config>::PalletId::get().into_account_truncating()
            }
        }
    }
}
