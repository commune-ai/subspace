use super::*;

use frame_support::traits::{StorageInstance, StorageVersion};
use pallet_subnet_emission_api::SubnetConsensus;

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
pub mod v14 {
    use super::*;
    use frame_support::traits::OnRuntimeUpgrade;

    pub struct MigrateToV14<T>(sp_std::marker::PhantomData<T>);

    impl<T: Config> OnRuntimeUpgrade for MigrateToV14<T> {
        fn on_runtime_upgrade() -> Weight {
            let on_chain_version = StorageVersion::get::<Pallet<T>>();
            if on_chain_version != 13 {
                log::info!("Storage v14  already updated");
                return Weight::zero();
            }

            let general_subnet_netuid = 2;
            let onchain_netuid = T::get_consensus_netuid(SubnetConsensus::Linear).unwrap_or(2);

            // return early if there is not a match
            if general_subnet_netuid != onchain_netuid {
                log::info!("General subnet netuid does not match onchain netuid");
                return Weight::zero();
            }

            // Clear all of the current weights on subnet 2
            let _ = Weights::<T>::clear_prefix(general_subnet_netuid, u32::MAX, None);
            log::info!("Cleared all weights for subnet 2");

            // Make sure we allow just one weight for the general subnet
            MinAllowedWeights::<T>::set(general_subnet_netuid, 1);
            log::info!("Set min allowed weights for subnet 2");

            // Make sure max allowed weights are same as max allowed uids
            let max_allowed_uids = MaxAllowedUids::<T>::get(general_subnet_netuid);
            MaxAllowedWeights::<T>::set(general_subnet_netuid, max_allowed_uids);
            log::info!("Set max allowed weights for subnet 2");

            log::info!("Migrating storage to v14");
            StorageVersion::new(14).put::<Pallet<T>>();
            Weight::zero()
        }
    }
}
