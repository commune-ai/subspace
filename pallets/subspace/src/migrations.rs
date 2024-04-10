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

pub mod v1 {
    use super::*;

    pub struct MigrateToV1<T>(sp_std::marker::PhantomData<T>);

    impl<T: Config> OnRuntimeUpgrade for MigrateToV1<T> {
        fn on_runtime_upgrade() -> Weight {
            let on_chain_version = StorageVersion::get::<Pallet<T>>();

            // Migrate DelegationFee to v1
            if on_chain_version == 0 {
                let min_deleg_fee_global = Pallet::<T>::global_params().floor_delegation_fee;

                // Iterate through all entries in the DelegationFee storage map
                for (netuid, account_id, delegation_fee) in DelegationFee::<T>::iter() {
                    if delegation_fee < min_deleg_fee_global {
                        // Update the delegation fee to the minimum value
                        DelegationFee::<T>::insert(netuid, account_id, min_deleg_fee_global);
                    }
                }

                StorageVersion::new(1).put::<Pallet<T>>();
                log::info!("Migrated DelegationFee to v1");
                T::DbWeight::get().writes(1)
            } else {
                log::info!("DelegationFee already updated");
                Weight::zero()
            }
        }
    }
}

pub mod v2 {
    use crate::voting::VoteMode;

    use super::*;

    pub mod old_storage {
        use super::*;
        use frame_support::{pallet_prelude::ValueQuery, storage_alias, Identity};

        type AccountId<T> = <T as frame_system::Config>::AccountId;

        #[storage_alias]
        pub(super) type StakeFrom<T: Config> = StorageDoubleMap<
            Pallet<T>,
            Identity,
            u16,
            Identity,
            AccountId<T>,
            Vec<(AccountId<T>, u64)>,
            ValueQuery,
        >;

        #[storage_alias]
        pub(super) type StakeTo<T: Config> = StorageDoubleMap<
            Pallet<T>,
            Identity,
            u16,
            Identity,
            AccountId<T>,
            Vec<(AccountId<T>, u64)>,
            ValueQuery,
        >;

        #[storage_alias]
        pub(super) type VoteModeSubnet<T: Config> =
            StorageMap<Pallet<T>, Identity, u16, Vec<u8>, ValueQuery>;
    }

    pub struct MigrateToV2<T>(sp_std::marker::PhantomData<T>);

    impl<T: Config> OnRuntimeUpgrade for MigrateToV2<T> {
        fn on_runtime_upgrade() -> Weight {
            let on_chain_version = StorageVersion::get::<Pallet<T>>();

            if on_chain_version == 1 {
                // Migrate StakeFrom storage
                for (netuid, module_key, stake_from) in old_storage::StakeFrom::<T>::iter() {
                    let new_stake_from: BTreeMap<T::AccountId, u64> =
                        stake_from.into_iter().collect();
                    StakeFrom::<T>::insert(netuid, module_key, new_stake_from);
                }

                // Migrate StakeTo storage
                for (netuid, account_id, stake_to) in old_storage::StakeTo::<T>::iter() {
                    let new_stake_to: BTreeMap<T::AccountId, u64> = stake_to.into_iter().collect();
                    StakeTo::<T>::insert(netuid, account_id, new_stake_to);
                }

                for (netuid, mode) in old_storage::VoteModeSubnet::<T>::iter() {
                    let mode = match &mode[..] {
                        b"authority" => VoteMode::Authority,
                        b"stake" => VoteMode::Stake,
                        _ => panic!("invalid vote mode {:?}", core::str::from_utf8(&mode)),
                    };
                    VoteModeSubnet::<T>::insert(netuid, mode);
                }

                // Update the storage version to 2
                StorageVersion::new(2).put::<Pallet<T>>();
                log::info!("Migrated StakeFrom, StakeTo and VoteMode to v2");

                // Return the appropriate weight for the migration
                T::DbWeight::get().reads_writes(1, 1)
            } else {
                log::info!("StakeFrom, StakeTo, VoteMode are already updated to v2");
                Weight::zero()
            }
        }
    }
}
