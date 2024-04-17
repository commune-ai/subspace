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
// TODO: fix migration, now it assumes no subnet gaps
pub mod v3 {
    use crate::voting::VoteMode;

    use super::*;
    use sp_core::bytes::from_hex;

    pub struct MigrateToV3<T>(sp_std::marker::PhantomData<T>);

    impl<T: Config> OnRuntimeUpgrade for MigrateToV3<T> {
        fn on_runtime_upgrade() -> Weight {
            let on_chain_version = StorageVersion::get::<Pallet<T>>();

            // Migrate Burn to v3
            if on_chain_version == 2 {
                // Query for the threshold of stake that subnet needs to have
                let subnet_stake_threshold = SubnetStakeThreshold::<T>::get();

                // Migrating registration adjustment
                // Here we will just use the lower bound of the `old` min burn
                let old_burn_min_burn = 2500000000; // 2.5 $COMAI tokens

                MaxBurn::<T>::put(150000000000); // Migrate the max_burn to 150 $COMAI tokens
                                                 // Find the highest netuid from the subnetnames
                MaxRegistrationsPerBlock::<T>::put(5); // Old is 10
                TargetRegistrationsPerInterval::<T>::put(20); // Old is 25

                // Iterate through all netuids and insert the old burn (minimum) value for each
                // this is important as we don't want free registrations right after the runtime
                // udpate
                for netuid in N::<T>::iter_keys() {
                    Burn::<T>::insert(netuid, old_burn_min_burn);
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

                    // If the subnet has more modules than allowed, remove the lowest ones.
                    let max_allowed = MaxAllowedUids::<T>::get(netuid);
                    let currently_registered = Pallet::<T>::get_subnet_n(netuid);
                    let overflown = currently_registered.saturating_sub(max_allowed);
                    for _ in 0..overflown {
                        Pallet::<T>::remove_module(
                            netuid,
                            Pallet::<T>::get_lowest_uid(netuid, true),
                        );
                    }
                }
                log::info!("Emission and consensus updated");

                // Due to the incoming incentives refactoring, `max_stake` value
                // is no longer needed to be limited on the subnet 0
                let general_netuid = 0;
                MaxStake::<T>::insert(general_netuid, u64::MAX);
                log::info!("Min stake migrated");

                log::info!("Setting subnet 0 to vote mode");
                VoteModeSubnet::<T>::set(0, VoteMode::Vote);

                // Migrate the nominator, to the DAO bot mutli-sig account
                // -> `5EZJYuTFdkzkLZew7Tnm7phuZrejHBks4XPz3UDZdMh11ALA`
                // Decode the multi-sig account from its base58 representation

                // (Anyone can pass a proposal that automatically changes this account)
                // The bot approach has been approved in the proposal ID 2
                let multi_sig_account_hex = "35455a4a59755446646b7a6b4c5a657737546e6d377068755a72656a48426b733458507a3355445a644d683131414c41";
                let multi_sig_account_bytes = match from_hex(multi_sig_account_hex) {
                    Ok(bytes) => bytes,
                    Err(_) => {
                        log::warn!(
                            "Failed to decode multi-sig account hex. Using default nominator."
                        );
                        DefaultNominator::<T>::get().encode()
                    }
                };

                let multi_sig_account = match T::AccountId::decode(
                    &mut &multi_sig_account_bytes[..],
                ) {
                    Ok(account) => account,
                    Err(_) => {
                        log::warn!("Failed to convert multi-sig account bytes to AccountId. Using default nominator.");
                        DefaultNominator::<T>::get()
                    }
                };
                Nominator::<T>::put(multi_sig_account);
                log::info!("Nominator migration completed.");

                // update the storage version
                StorageVersion::new(3).put::<Pallet<T>>();
                log::info!("Migrated subnets to v3");

                T::DbWeight::get().reads_writes(1, 1)
            } else {
                log::info!("Storage v3 already updated");
                Weight::zero()
            }
        }
    }
}
