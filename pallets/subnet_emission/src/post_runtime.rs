use super::*;
use frame_support::{storage::with_storage_layer, traits::Get, weights::Weight};
use pallet_subnet_emission_api::SubnetConsensus;
use pallet_subspace::{MaxAllowedUids, Pallet as PalletS, N};

impl<T: Config> Pallet<T> {
    pub(crate) fn deregister_excess_modules(mut remaining: Weight) -> Weight {
        let netuid = Self::get_consensus_netuid(SubnetConsensus::Linear).unwrap_or(2);
        const MAX_MODULES_PER_ITERATION: usize = 5;
        const MAX_UIDS: u16 = 524;

        let db_weight = T::DbWeight::get();
        let mut weight = db_weight.reads(2);
        let find_id_weight = db_weight.reads(1);
        let deregister_weight = Weight::from_parts(300_495_000, 21587)
            .saturating_add(T::DbWeight::get().reads(34_u64))
            .saturating_add(T::DbWeight::get().writes(48_u64));

        if !remaining
            .all_gte(weight.saturating_add(find_id_weight).saturating_add(deregister_weight))
        {
            log::info!("not enough weight remaining: {remaining:?}");
            return Weight::zero();
        }

        remaining = remaining.saturating_sub(weight);

        let mut module_count = N::<T>::get(netuid);
        while module_count > MAX_UIDS {
            for _ in 0..MAX_MODULES_PER_ITERATION {
                if !remaining.all_gte(find_id_weight.saturating_add(deregister_weight)) {
                    log::info!("not enough weight remaining: {remaining:?}");
                    return weight;
                }

                if let Some(uid) = PalletS::<T>::get_lowest_uid(netuid, false) {
                    log::info!("deregistering module with uid {uid}");

                    weight = weight.saturating_add(find_id_weight);
                    remaining = remaining.saturating_sub(find_id_weight);

                    let result =
                        with_storage_layer(|| PalletS::<T>::remove_module(netuid, uid, true));
                    if result.is_ok() {
                        weight = weight.saturating_add(deregister_weight);
                        remaining = remaining.saturating_sub(deregister_weight);
                        module_count = module_count.saturating_sub(1);
                    } else {
                        log::error!(
                            "failed to deregister module {uid} due to: {:?}",
                            result.unwrap_err()
                        );
                    }
                } else {
                    // No more modules to deregister
                    break;
                }

                if module_count <= MAX_UIDS {
                    break;
                }
            }

            if module_count <= MAX_UIDS {
                break;
            }
        }

        MaxAllowedUids::<T>::set(netuid, MAX_UIDS);
        weight
    }
}
