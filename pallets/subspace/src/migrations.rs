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

pub mod v8 {
    use self::{
        global::BurnConfiguration,
        old_storage::{
            AdjustmentAlpha, GlobalDaoTreasury, MaxBurn, MinBurn, TargetRegistrationsInterval,
            TargetRegistrationsPerInterval,
        },
    };
    use super::*;

    pub mod old_storage {
        use super::*;
        use frame_support::{pallet_prelude::ValueQuery, storage_alias};

        #[storage_alias]
        pub type MinBurn<T: Config> = StorageValue<Pallet<T>, u64, ValueQuery>;

        #[storage_alias]
        pub type MaxBurn<T: Config> = StorageValue<Pallet<T>, u64, ValueQuery>;

        #[storage_alias]
        pub type AdjustmentAlpha<T: Config> = StorageValue<Pallet<T>, u64, ValueQuery>;

        #[storage_alias]
        pub type TargetRegistrationsInterval<T: Config> = StorageValue<Pallet<T>, u16, ValueQuery>;

        #[storage_alias]
        pub type TargetRegistrationsPerInterval<T: Config> =
            StorageValue<Pallet<T>, u16, ValueQuery>;

        #[storage_alias]
        pub type GlobalDaoTreasury<T: Config> = StorageValue<Pallet<T>, u64, ValueQuery>;
    }

    pub struct MigrateToV8<T>(sp_std::marker::PhantomData<T>);

    impl<T: Config> OnRuntimeUpgrade for MigrateToV8<T> {
        fn on_runtime_upgrade() -> Weight {
            let on_chain_version = StorageVersion::get::<Pallet<T>>();

            if on_chain_version != 7 {
                log::info!("Storage v8 already updated");
                return Weight::zero();
            }

            let mut gaps = BTreeSet::new();
            let netuids: BTreeSet<_> = N::<T>::iter_keys().collect();
            for netuid in 0..netuids.last().copied().unwrap_or_default() {
                if !netuids.contains(&netuid) {
                    gaps.insert(netuid);
                }
            }

            log::info!("Existing subnets: {netuids:?}");
            log::info!("Updated subnets gaps: {gaps:?}");
            SubnetGaps::<T>::set(gaps);

            let burn_config = BurnConfiguration::<T> {
                min_burn: MinBurn::<T>::get(),
                max_burn: MaxBurn::<T>::get(),
                adjustment_alpha: AdjustmentAlpha::<T>::get(),
                adjustment_interval: TargetRegistrationsInterval::<T>::get(),
                expected_registrations: TargetRegistrationsPerInterval::<T>::get(),
                _pd: PhantomData,
            };

            if let Err(err) = burn_config.apply() {
                log::error!("error migrating burn configurations: {err:?}")
            } else {
                log::info!("Migrated burn-related params to BurnConfig in v8");
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

            StorageVersion::new(8).put::<Pallet<T>>();
            T::DbWeight::get().writes(1)
        }
    }
}
