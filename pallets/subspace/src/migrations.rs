use super::*;

use frame_support::traits::{Get, StorageInstance, StorageVersion};
use global::GeneralBurnConfiguration;

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
pub mod v13 {
    use super::*;
    use frame_support::traits::OnRuntimeUpgrade;
    use sp_runtime::{BoundedVec, Percent};

    pub mod old_storage {
        use super::*;
        use frame_support::{storage_alias, Blake2_128Concat, Identity};
        use sp_runtime::Percent;

        #[storage_alias]
        pub type DelegationFee<T: Config> =
            StorageDoubleMap<Pallet<T>, Identity, u16, Blake2_128Concat, AccountIdOf<T>, Percent>;

        #[storage_alias]
        pub type AdjustmentAlpha<T: Config> = StorageMap<Pallet<T>, Identity, u16, u64>;

        #[storage_alias]
        pub type TargetRegistrationsPerInterval<T: Config> =
            StorageMap<Pallet<T>, Identity, u16, u16>;

        #[storage_alias]
        pub type TargetRegistrationsInterval<T: Config> = StorageMap<Pallet<T>, Identity, u16, u16>;

        #[storage_alias]
        pub type MaxRegistrationsPerInterval<T: Config> = StorageMap<Pallet<T>, Identity, u16, u16>;
    }

    pub struct MigrateToV13<T>(sp_std::marker::PhantomData<T>);

    impl<T: Config> OnRuntimeUpgrade for MigrateToV13<T> {
        fn on_runtime_upgrade() -> Weight {
            let on_chain_version = StorageVersion::get::<Pallet<T>>();
            if on_chain_version != 12 {
                log::info!("Storage v13 already updated");
                return Weight::zero();
            }
            log::info!("Migrating storage to v13");

            log::info!("Migrating delegation fees to new storage");

            let old_delegation_fee_keys = old_storage::DelegationFee::<T>::iter()
                .map(|(_, key, _)| key)
                .collect::<BTreeSet<_>>();
            let _ = old_storage::DelegationFee::<T>::clear(u32::MAX, None);

            for key in old_delegation_fee_keys {
                DelegationFee::<T>::set(key, Percent::from_percent(5));
            }

            // Add metadata to existing subnets
            for (netuid, _) in SubnetNames::<T>::iter() {
                let metadata = match netuid {
                    3 => b"https://github.com/agicommies/synthia/".to_vec(),
                    5 => b"https://open.0xscope.com/home".to_vec(),
                    6 => b"https://github.com/Comtensor/comtensor".to_vec(),
                    7 => b"https://kaiwa.dev/".to_vec(),
                    9 => b"https://github.com/panthervis/prediction-subnet".to_vec(),
                    10 => b"https://github.com/Agent-Artificial/eden-subnet/".to_vec(),
                    12 => b"https://www.yogpt.ai/".to_vec(),
                    13 => b"https://github.com/nakamoto-ai/zangief".to_vec(),
                    14 => b"https://mosaicx.org/".to_vec(),
                    15 => b"https://github.com/smart-window/comchat-subnet".to_vec(),
                    16 => b"https://github.com/bit-current/dtune/blob/commune/docs/tutorial.md"
                        .to_vec(),
                    17 => b"https://marketcompass.ai/".to_vec(),
                    18 => b"https://github.com/nakamoto-ai/yama".to_vec(),
                    _ => continue,
                };

                if let Ok(bounded_metadata) = BoundedVec::<u8, ConstU32<59>>::try_from(metadata) {
                    SubnetMetadata::<T>::insert(netuid, bounded_metadata);
                }
            }

            // Metadata
            log::info!("Metadata for subnets:");

            for (netuid, metadata) in SubnetMetadata::<T>::iter() {
                let subnet_name = SubnetNames::<T>::get(netuid);
                if let Ok(subnet_name_str) = core::str::from_utf8(&subnet_name) {
                    if let Ok(metadata_str) = core::str::from_utf8(&metadata) {
                        log::info!(
                            "Subnet {}: {} - Metadata: {}",
                            netuid,
                            subnet_name_str,
                            metadata_str
                        );
                    } else {
                        log::info!(
                            "Subnet {}: {} - Metadata: {:?} (non-UTF8)",
                            netuid,
                            subnet_name_str,
                            metadata
                        );
                    }
                } else {
                    log::info!(
                        "Subnet {} - Metadata: {:?} (subnet name is non-UTF8)",
                        netuid,
                        metadata
                    );
                }
            }

            // Change the name of subnet 2 from "commune" to "General"
            SubnetNames::<T>::insert(2, b"General".to_vec());

            for key in N::<T>::iter_keys() {
                MinValidatorStake::<T>::insert(key, GetDefaultMinValidatorStake::<T>::get());
            }

            for netuid in N::<T>::iter_keys() {
                let burn_config: GeneralBurnConfiguration<T> = GeneralBurnConfiguration {
                    min_burn: T::DefaultModuleMinBurn::get(),
                    max_burn: 150_000_000_000,
                    adjustment_alpha: old_storage::AdjustmentAlpha::<T>::get(netuid).unwrap_or(0),
                    target_registrations_interval:
                        old_storage::TargetRegistrationsInterval::<T>::get(netuid).unwrap_or(142),
                    target_registrations_per_interval:
                        old_storage::TargetRegistrationsPerInterval::<T>::get(netuid).unwrap_or(3),
                    max_registrations_per_interval:
                        old_storage::MaxRegistrationsPerInterval::<T>::get(netuid)
                            .unwrap_or(T::DefaultMaxRegistrationsPerInterval::get()),
                    _pd: PhantomData,
                };
                match burn_config.apply_module_burn(netuid) {
                    Ok(_) => (),
                    Err(e) => {
                        log::error!("Failed to apply module burn: {:?}", e);
                        let default_config = GeneralBurnConfiguration::<T>::default();
                        if let Err(e) = default_config.apply_module_burn(netuid) {
                            log::error!("Failed to apply default module burn: {:?}", e);
                        } else {
                            log::info!("Applied default burn config for netuid {}", netuid);
                        }
                    }
                }
                log::info!(
                    "netuid {} has a burn config {:?}",
                    netuid,
                    ModuleBurnConfig::<T>::get(netuid)
                );
            }

            log::info!("Migrated storage to v13");

            StorageVersion::new(13).put::<Pallet<T>>();
            T::DbWeight::get().reads_writes(1, 1)
        }
    }
}
