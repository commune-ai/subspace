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
    use super::*;

    pub struct MigrateToV2<T>(sp_std::marker::PhantomData<T>);

    impl<T: Config> OnRuntimeUpgrade for MigrateToV2<T> {
        fn on_runtime_upgrade() -> Weight {
            let on_chain_version = StorageVersion::get::<Pallet<T>>();

            // Migrate Burn to v2
            if on_chain_version == 1 {
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

                StorageVersion::new(2).put::<Pallet<T>>();
                log::info!("Migrated subnets to v2");
                T::DbWeight::get().writes(largest_netuid as u64)
            } else {
                log::info!("Subnets already updated");
                Weight::zero()
            }
        }
    }
}
