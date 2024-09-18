use super::*;
use frame_support::{traits::Get, weights::Weight};
use pallet_subnet_emission_api::SubnetConsensus;
use pallet_subspace::{MaxAllowedUids, Pallet as PalletS, N};

impl<T: Config> Pallet<T> {
    pub(crate) fn deregister_excess_modules(mut remaining: Weight) -> Weight {
        let netuid = Self::get_consensus_netuid(SubnetConsensus::Linear).unwrap_or(2);
        const MAX_MODULES_PER_ITERATION: usize = 5;
        const MAX_UIDS: u16 = 524;

        log::info!("Deregistering excess modules for netuid: {}", netuid);
        let db_weight = T::DbWeight::get();
        let mut weight = db_weight.reads(2);
        let find_id_weight = db_weight.reads(1);
        let deregister_weight = Weight::from_parts(300_495_000, 21587)
            .saturating_add(db_weight.reads(34))
            .saturating_add(db_weight.writes(48));

        // Calculate the minimum required weight to proceed
        let min_required_weight =
            weight.saturating_add(find_id_weight).saturating_add(deregister_weight);

        if !remaining.all_gte(min_required_weight) {
            log::info!("Not enough weight remaining: {:?}", remaining);
            return Weight::zero();
        }

        remaining = remaining.saturating_sub(weight);

        let mut module_count = N::<T>::get(netuid);

        // Return early if no excess modules need to be deregistered
        if module_count <= MAX_UIDS {
            // Also set the max modules to this number
            MaxAllowedUids::<T>::set(netuid, MAX_UIDS);
            return weight;
        }

        for _ in 0..MAX_MODULES_PER_ITERATION {
            if module_count <= MAX_UIDS {
                break;
            }

            // Check if there's enough weight for finding the next module
            if !remaining.all_gte(find_id_weight) {
                log::info!(
                    "Not enough weight remaining for find_id_weight: {:?}",
                    remaining
                );
                break;
            }

            weight = weight.saturating_add(find_id_weight);
            remaining = remaining.saturating_sub(find_id_weight);

            if let Some(uid) = PalletS::<T>::get_lowest_uid(netuid, true) {
                // Check if there's enough weight for deregistration
                if !remaining.all_gte(deregister_weight) {
                    log::info!(
                        "Not enough weight remaining for deregister_weight: {:?}",
                        remaining
                    );
                    break;
                }
                log::info!("Deregistering module with UID: {}", uid);

                let _ = PalletS::<T>::remove_module(netuid, uid, false);
                module_count = module_count.saturating_sub(1);
                weight = weight.saturating_add(deregister_weight);
                remaining = remaining.saturating_sub(deregister_weight);
            } else {
                // No more modules to deregister
                break;
            }
        }
        weight
    }
}
