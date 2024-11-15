use crate::*;
use frame_support::{pallet_prelude::ValueQuery, traits::StorageVersion, Blake2_128Concat};
use sp_runtime::Percent;

use frame_system::Config as SystemConfig;

pub mod v15 {
    use frame_support::{traits::OnRuntimeUpgrade, weights::Weight};

    use super::*;

    pub mod old_storage {
        use super::*;
        use frame_support::{storage_alias, Identity};

        #[storage_alias]
        pub type Weights<T: Config> =
            StorageDoubleMap<Pallet<T>, Identity, u16, Identity, u16, Vec<(u16, u16)>, ValueQuery>;

        #[storage_alias]
        pub type DelegationFee<T: Config> = StorageMap<
            Pallet<T>,
            Blake2_128Concat,
            <T as SystemConfig>::AccountId,
            Percent,
            ValueQuery,
        >;
    }

    pub struct MigrateToV15<T>(sp_std::marker::PhantomData<T>);

    impl<T: Config> OnRuntimeUpgrade for MigrateToV15<T> {
        fn on_runtime_upgrade() -> Weight {
            let on_chain_version = StorageVersion::get::<Pallet<T>>();
            if on_chain_version != 14 {
                log::info!("Storage v15 already updated");
                return Weight::zero();
            }

            for (account, fee) in old_storage::DelegationFee::<T>::iter() {
                let mut fee_config = ValidatorFees::default();
                fee_config.stake_delegation_fee = fee;

                ValidatorFeeConfig::<T>::insert(account, fee_config);
            }

            log::info!("Migrating storage to v15");
            StorageVersion::new(15).put::<Pallet<T>>();
            Weight::zero()
        }
    }
}
