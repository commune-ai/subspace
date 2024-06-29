use super::*;

use frame_support::traits::{Get, StorageInstance, StorageVersion};

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

pub mod v12 {
    use super::*;
    use frame_support::{traits::OnRuntimeUpgrade, weights::Weight};
    use sp_std::collections::btree_map::BTreeMap;

    pub mod old_storage {
        use super::*;
        use frame_support::{pallet_prelude::ValueQuery, storage_alias, Identity};
        use sp_std::collections::btree_map::BTreeMap;

        #[storage_alias]
        pub type Stake<T: Config> =
            StorageDoubleMap<Pallet<T>, Identity, u16, Identity, AccountIdOf<T>, u64, ValueQuery>;

        #[storage_alias]
        pub type StakeFrom<T: Config> = StorageDoubleMap<
            Pallet<T>,
            Identity,
            u16,
            Identity,
            AccountIdOf<T>,
            BTreeMap<AccountIdOf<T>, u64>,
            ValueQuery,
        >;

        #[storage_alias]
        pub type StakeTo<T: Config> = StorageDoubleMap<
            Pallet<T>,
            Identity,
            u16,
            Identity,
            AccountIdOf<T>,
            BTreeMap<AccountIdOf<T>, u64>,
            ValueQuery,
        >;

        #[storage_alias]
        pub type TotalStake<T: Config> = StorageMap<Pallet<T>, Identity, u16, u64, ValueQuery>;
    }

    pub struct MigrateToV12<T>(sp_std::marker::PhantomData<T>);

    impl<T: Config> OnRuntimeUpgrade for MigrateToV12<T> {
        fn on_runtime_upgrade() -> Weight {
            let on_chain_version = StorageVersion::get::<Pallet<T>>();

            if on_chain_version != 11 {
                log::info!("Storage v12 already updated");
                return Weight::zero();
            }
            log::info!("Migrating storage to v12");

            // Download existing data into separate types
            let old_stake: BTreeMap<(u16, AccountIdOf<T>), u64> = old_storage::Stake::<T>::iter()
                .map(|(netuid, account, stake)| ((netuid, account), stake))
                .collect();
            let old_stake_from: BTreeMap<(u16, AccountIdOf<T>), BTreeMap<AccountIdOf<T>, u64>> =
                old_storage::StakeFrom::<T>::iter()
                    .map(|(netuid, account, stakes)| ((netuid, account), stakes))
                    .collect();
            let old_stake_to: BTreeMap<(u16, AccountIdOf<T>), BTreeMap<AccountIdOf<T>, u64>> =
                old_storage::StakeTo::<T>::iter()
                    .map(|(netuid, account, stakes)| ((netuid, account), stakes))
                    .collect();

            // Clear the problematic stake storages
            // We tried to do this with the old storage instead, after migration, but experienced
            // decoding issues.
            let _ = Stake::<T>::clear(u32::MAX, None);
            let _ = StakeTo::<T>::clear(u32::MAX, None);
            let _ = StakeFrom::<T>::clear(u32::MAX, None);

            // Migrate Stake, getting rid of netuid
            for ((_, account), stake) in old_stake {
                let current_stake = Stake::<T>::get(&account);
                Stake::<T>::insert(&account, current_stake.saturating_add(stake));
            }
            log::info!("Migrated Stake");

            // Migrate StakeFrom
            for ((_, from), stakes) in old_stake_from {
                for (to, amount) in stakes {
                    let current_amount = StakeFrom::<T>::get(&from, &to);
                    StakeFrom::<T>::insert(&from, &to, current_amount.saturating_add(amount));
                }
            }
            log::info!("Migrated StakeFrom");

            // Migrate StakeTo
            for ((_, to), stakes) in old_stake_to {
                for (from, amount) in stakes {
                    let current_amount = StakeFrom::<T>::get(&from, &to);
                    StakeTo::<T>::insert(&from, &to, current_amount.saturating_add(amount));
                }
            }
            log::info!("Migrated StakeTo");

            // Migrate TotalStake (unchanged)
            let total_stake: u64 =
                old_storage::TotalStake::<T>::iter().map(|(_, stake)| stake).sum();
            TotalStake::<T>::put(total_stake);
            old_storage::TotalStake::<T>::remove_all(None);
            log::info!("Migrated TotalStake");

            StorageVersion::new(12).put::<Pallet<T>>();

            T::DbWeight::get().reads_writes(1, 1)
        }
    }
}
