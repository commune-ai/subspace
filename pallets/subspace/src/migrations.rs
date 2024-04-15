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

// Delegation update, migrations.
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
                log::info!("Storage v1 already updated");
                Weight::zero()
            }
        }
    }
}

// Proposal update, migrations.
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
                log::info!("Migrating to V2");

                // Migrate StakeFrom storage
                for (netuid, module_key, stake_from) in old_storage::StakeFrom::<T>::iter() {
                    let new_stake_from: BTreeMap<T::AccountId, u64> =
                        stake_from.into_iter().collect();
                    StakeFrom::<T>::insert(netuid, module_key, new_stake_from);
                }
                log::info!("Migrated StakeFrom");

                // Migrate StakeTo storage
                for (netuid, account_id, stake_to) in old_storage::StakeTo::<T>::iter() {
                    let new_stake_to: BTreeMap<T::AccountId, u64> = stake_to.into_iter().collect();
                    StakeTo::<T>::insert(netuid, account_id, new_stake_to);
                }
                log::info!("Migrated StakeTo");

                for (netuid, mode) in old_storage::VoteModeSubnet::<T>::iter() {
                    let mode = match &mode[..] {
                        b"authority" => VoteMode::Authority,
                        b"stake" => VoteMode::Vote,
                        _ => {
                            log::warn!("invalid vote mode {:?}", core::str::from_utf8(&mode));
                            VoteMode::Vote
                        }
                    };
                    VoteModeSubnet::<T>::insert(netuid, mode);
                }
                log::info!("Migrated VoteMode");

                // Update the storage version to 2
                StorageVersion::new(2).put::<Pallet<T>>();

                log::info!("Migrated to v2");
                T::DbWeight::get().reads_writes(1, 1)
            } else {
                log::info!("Storage v2 already updated");
                Weight::zero()
            }
        }
    }
}

// Incentives update, migrations.
pub mod v3 {
    use super::*;

    pub struct MigrateToV3<T>(sp_std::marker::PhantomData<T>);

    impl<T: Config> OnRuntimeUpgrade for MigrateToV3<T> {
        fn on_runtime_upgrade() -> Weight {
            let on_chain_version = StorageVersion::get::<Pallet<T>>();

            // Migrate Burn to v3
            if on_chain_version == 2 {
                // Query for the threshold of stake that subnet needs to have
                let subnet_stake_threshold = SubnetStakeThreshold::<T>::get();
                // Here we will just use the lower bound of the burn
                let old_burn = Pallet::<T>::global_params().min_burn;

                // Find the highest netuid from the subnetnames
                let largest_netuid = SubnetNames::<T>::iter_keys().max().unwrap_or(0);
                // Iterate through all netuids and insert the old burn (minimum) value for each
                // this is important as we don't want free registrations right after the runtime
                // udpate
                for netuid in 0..=largest_netuid {
                    Burn::<T>::insert(netuid, old_burn);
                    // update the emission that are below the threshold
                    let emission_for_netuid =
                        Pallet::<T>::calculate_network_emission(netuid, subnet_stake_threshold);
                    // empty out the module emissin on that subnet, as their epoch won't run, we
                    // don't want to confuse the user querying for the storage
                    if emission_for_netuid == 0 {
                        let name_count = Name::<T>::iter_prefix(netuid).count();
                        let new_emission = vec![0; name_count];
                        Emission::<T>::insert(netuid, new_emission);
                    }
                    SubnetEmission::<T>::insert(netuid, emission_for_netuid);

                    Active::<T>::mutate(netuid, |v| v.push(true));
                    Consensus::<T>::mutate(netuid, |v| v.push(0));
                    PruningScores::<T>::mutate(netuid, |v| v.push(0));
                    Rank::<T>::mutate(netuid, |v| v.push(0));
                    Trust::<T>::mutate(netuid, |v| v.push(0));
                    ValidatorPermits::<T>::mutate(netuid, |v| v.push(false));
                    ValidatorTrust::<T>::mutate(netuid, |v| v.push(0));
                }

                StorageVersion::new(3).put::<Pallet<T>>();
                log::info!("Migrated subnets to v3");
                T::DbWeight::get().writes(largest_netuid as u64)
            } else {
                log::info!("Storage v3 already updated");
                Weight::zero()
            }
        }
    }
}
