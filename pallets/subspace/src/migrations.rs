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
// TODO:

/*

# Migrate
StakeTo and StakeFrom

pub type StakeTo<T: Config> = StorageDoubleMap<
    _,
    Identity,
    u16,
    Identity,
    T::AccountId,
    BTreeMap<T::AccountId, u64>,
    ValueQuery,
>;

where we used `BTreeMap<T::AccountId, u64>,` instead of `Vec<(T::AccountId, u64)>`

*/

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
