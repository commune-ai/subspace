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

pub mod v11 {
    use self::{
        global::BurnConfiguration,
        old_storage::{GlobalDaoTreasury, MaxBurn, MinBurn},
    };
    use super::*;

    mod old_storage {
        use super::*;
        use frame_support::{pallet_prelude::ValueQuery, storage_alias};

        #[storage_alias]
        pub type MinBurn<T: Config> = StorageValue<Pallet<T>, u64, ValueQuery>;

        #[storage_alias]
        pub type MaxBurn<T: Config> = StorageValue<Pallet<T>, u64, ValueQuery>;

        #[storage_alias]
        pub type AdjustmentAlpha<T: Config> = StorageValue<Pallet<T>, u64, ValueQuery>;

        #[storage_alias]
        pub type GlobalDaoTreasury<T: Config> = StorageValue<Pallet<T>, u64, ValueQuery>;
    }

    pub struct MigrateToV11<T>(sp_std::marker::PhantomData<T>);

    impl<T: Config> OnRuntimeUpgrade for MigrateToV11<T> {
        fn on_runtime_upgrade() -> Weight {
            let on_chain_version = StorageVersion::get::<Pallet<T>>();

            if on_chain_version != 10 {
                log::info!("Storage v11 already updated");
                return Weight::zero();
            }

            let current_adjustment_alpha = old_storage::AdjustmentAlpha::<T>::get();
            // Nuke the old adjustement alpha storage
            for netuid in N::<T>::iter_keys() {
                AdjustmentAlpha::<T>::insert(netuid, current_adjustment_alpha);
            }
            old_storage::AdjustmentAlpha::<T>::kill();
            log::info!("Migrating adjustment alpha to v11");

            let burn_config = BurnConfiguration::<T> {
                min_burn: MinBurn::<T>::get(),
                max_burn: MaxBurn::<T>::get(),
                _pd: PhantomData,
            };

            if let Err(err) = burn_config.apply() {
                log::error!("error migrating burn configurations: {err:?}")
            } else {
                log::info!("Migrated burn-related params to BurnConfig in v11");
            }

            let old_treasury_balance = GlobalDaoTreasury::<T>::get();
            let treasury_account = DaoTreasuryAddress::<T>::get();
            log::info!("Treasury balance: {old_treasury_balance}");
            Pallet::<T>::add_balance_to_account(
                &treasury_account,
                Pallet::<T>::u64_to_balance(old_treasury_balance).unwrap_or_default(),
            );
            GlobalDaoTreasury::<T>::set(0);

            let account_balance = Pallet::<T>::get_balance_u64(&treasury_account);
            log::info!("Treasury transferred, treasury account now has {account_balance}");
            log::info!("Treasury account: {treasury_account:?}");

            StorageVersion::new(11).put::<Pallet<T>>();
            T::DbWeight::get().writes(1)
        }
    }
}
