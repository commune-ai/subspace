use super::*;
use crate::subnet_consensus::{linear::LinearEpoch, treasury::TreasuryEpoch, yuma::YumaEpoch};

use frame_support::{storage::with_storage_layer, traits::Get, weights::Weight};
use pallet_subnet_emission_api::SubnetConsensus;
use pallet_subspace::N;

/// Processes subnets by updating pending emissions and running epochs when due.
///
/// # Arguments
///
/// * `block_number` - The current block number.
/// * `subnets_emission_distribution` - A map of subnet IDs to their emission values.
///
/// This function iterates through all subnets, updates their pending emissions,
/// and runs an epoch if it's time for that subnet.
fn process_subnets<T: Config>(
    block_number: u64,
    subnets_emission_distribution: PricedSubnets,
) -> Weight {
    let total_weight = N::<T>::iter_keys().fold(Weight::zero(), |acc_weight, netuid| {
        update_pending_emission::<T>(
            netuid,
            subnets_emission_distribution.get(&netuid).unwrap_or(&0),
        );
        let mut weight = acc_weight.saturating_add(T::DbWeight::get().writes(1));

        if pallet_subspace::Pallet::<T>::blocks_until_next_epoch(netuid, block_number) == 0 {
            weight = weight.saturating_add(run_epoch::<T>(netuid));
        }

        weight
    });

    total_weight
}

/// Updates the pending emission for a given subnet.
///
/// # Arguments
///
/// * `netuid` - The ID of the subnet.
/// * `new_queued_emission` - The new emission value to add to the pending emission.
///
/// This function adds the new emission value to the existing pending emission
/// for the specified subnet, and logs the updated total.
fn update_pending_emission<T: Config>(netuid: u16, new_queued_emission: &u64) {
    let emission_to_drain = PendingEmission::<T>::mutate(netuid, |queued: &mut u64| {
        *queued = queued.saturating_add(*new_queued_emission);
        *queued
    });
    log::trace!("subnet {netuid} total pending emission: {emission_to_drain}, increased {new_queued_emission}");
}

/// Runs an epoch for a given subnet.
///
/// # Arguments
///
/// * `netuid` - The ID of the subnet.
///
/// This function clears the set weight rate limiter, retrieves the pending emission,
/// and if there's emission to distribute, runs the consensus algorithm. If successful,
/// it finalizes the epoch. If an error occurs during consensus, it logs the error
fn run_epoch<T: Config>(netuid: u16) -> Weight {
    log::trace!("running epoch for subnet {netuid}");

    let mut weight = T::DbWeight::get().reads(1);

    let emission_to_drain = PendingEmission::<T>::get(netuid);
    if emission_to_drain > 0 {
        match run_consensus_algorithm::<T>(netuid, emission_to_drain) {
            Ok(consensus_weight) => {
                weight = weight.saturating_add(consensus_weight);
                finalize_epoch::<T>(netuid);
                weight
            }
            Err(e) => {
                log::error!(
                    "Error running consensus algorithm for subnet {}: {:?}",
                    netuid,
                    e
                );
                Weight::zero()
            }
        }
    } else {
        weight
    }
}

// ---------------------------------
// Consensus
// ---------------------------------

/// Runs the appropriate consensus algorithm for a given subnet.
///
/// # Arguments
///
/// * `netuid` - The ID of the subnet.
/// * `emission_to_drain` - The amount of emission to distribute in this epoch.
///
/// # Returns
///
/// A Result indicating success or failure of the consensus algorithm.
///
/// This function selects and runs either the linear or Yuma consensus algorithm
/// based on the subnet ID.
fn run_consensus_algorithm<T: Config>(
    netuid: u16,
    emission_to_drain: u64,
) -> Result<Weight, &'static str> {
    with_storage_layer(|| {
        let Some(consensus_type) = SubnetConsensusType::<T>::get(netuid) else {
            return Ok(T::DbWeight::get().reads(1));
        };

        match consensus_type {
            SubnetConsensus::Root => Ok(T::DbWeight::get().reads(1)),
            SubnetConsensus::Treasury => run_treasury_consensus::<T>(netuid, emission_to_drain),
            SubnetConsensus::Linear => run_linear_consensus::<T>(netuid, emission_to_drain),
            SubnetConsensus::Yuma => run_yuma_consensus::<T>(netuid, emission_to_drain),
        }
    })
}
/// Runs the linear consensus algorithm for subnet 0.
///
/// # Arguments
///
/// * `netuid` - The ID of the subnet (should be 0).
/// * `emission_to_drain` - The amount of emission to distribute in this epoch.
///
/// # Returns
///
/// A Result indicating success or failure of the linear consensus algorithm.
///
/// This function creates and runs a new LinearEpoch, logging any errors that occur.
fn run_linear_consensus<T: Config>(
    netuid: u16,
    emission_to_drain: u64,
) -> Result<Weight, &'static str> {
    LinearEpoch::<T>::new(netuid, emission_to_drain)
        .run()
        .map(|(_, weight)| weight)
        .map_err(|err| {
            log::error!(
                "Failed to run linear consensus algorithm: {err:?}, skipping this block. \
                {emission_to_drain} tokens will be emitted on the next epoch."
            );
            "linear failed"
        })
}

/// Runs the Yuma consensus algorithm for subnets other than 0.
///
/// # Arguments
///
/// * `netuid` - The ID of the subnet (should not be 0).
/// * `emission_to_drain` - The amount of emission to distribute in this epoch.
///
/// # Returns
///
/// A Result indicating success or failure of the Yuma consensus algorithm.
///
/// This function creates and runs a new YumaEpoch, logging any errors that occur.
fn run_yuma_consensus<T: Config>(netuid: u16, emission_to_drain: u64) -> Result<(), &'static str> {
    let params = subnet_consensus::yuma::params::YumaParams::<T>::new(netuid, emission_to_drain)?;

    let output = YumaEpoch::<T>::new(netuid, params).run().map_err(|err| {
        log::error!(
            "Failed to run yuma consensus algorithm: {err:?}, skipping this block. \
            {emission_to_drain} tokens will be emitted on the next epoch."
        );
        "yuma failed"
    })?;

    output.apply();

    Ok(())
}

/// Runs the treasury consensus algorithm for a given network and emission amount.
///
/// # Arguments
///
/// * `netuid` - The unique identifier for the network.
/// * `emission_to_drain` - The amount of tokens to be emitted/drained.
///
/// # Returns
///
/// * `Ok(())` if the treasury consensus runs successfully.
/// * `Err(&'static str)` with an error message if the consensus fails.
fn run_treasury_consensus<T: Config>(
    netuid: u16,
    emission_to_drain: u64,
) -> Result<Weight, &'static str> {
    TreasuryEpoch::<T>::new(netuid, emission_to_drain)
        .run()
        .map(|_| T::DbWeight::get().reads_writes(1, 1))
        .map_err(|err| {
            log::error!(
                "Failed to run treasury consensus algorithm: {err:?}, skipping this block. \
                {emission_to_drain} tokens will be emitted on the next epoch."
            );
            "treasury failed"
        })
}

/// Runs the treasury consensus algorithm for subnet 1.

// ---------------------------------
// Epoch utils
// ---------------------------------

/// Finalizes an epoch for a given subnet.
///
/// # Arguments
///
/// * `netuid` - The ID of the subnet.
///
/// This function resets the pending emission for the subnet to 0 and
/// emits an EpochFinished event.
fn finalize_epoch<T: Config>(netuid: u16) {
    PendingEmission::<T>::insert(netuid, 0);
    Pallet::<T>::deposit_event(Event::<T>::EpochFinished(netuid));
}

impl<T: Config> Pallet<T> {
    /// Processes the emission distribution for the entire blockchain.
    ///
    /// # Arguments
    ///
    /// * `block_number` - The current block number.
    /// * `emission_per_block` - The total emission to be distributed per block.
    ///
    /// This function calculates the emission distribution across subnets and
    /// processes each subnet accordingly.
    pub fn process_emission_distribution(block_number: u64, emission_per_block: u64) -> Weight {
        log::debug!("stepping block {block_number:?}");

        let subnets_emission_distribution = Self::get_subnet_pricing(emission_per_block);
        process_subnets::<T>(block_number, subnets_emission_distribution)
    }

    // ---------------------------------
    // Subnet Emission Pallet Api Utils
    // ---------------------------------

    /// Gets the subnet with the lowest emission.
    ///
    /// # Returns
    ///
    /// An Option containing the ID of the subnet with the lowest emission,
    /// or None if there are no subnets.
    pub fn get_lowest_emission_netuid(ignore_subnet_immunity: bool) -> Option<u16> {
        let current_block = pallet_subspace::Pallet::<T>::get_current_block_number();
        let immunity_period = pallet_subspace::SubnetImmunityPeriod::<T>::get();

        SubnetEmission::<T>::iter()
            .filter(|(netuid, _)| Self::can_remove_subnet(*netuid))
            .filter(|(netuid, _)| pallet_subspace::N::<T>::get(netuid) > 0)
            .filter(|(netuid, _)| {
                ignore_subnet_immunity
                    || !pallet_subspace::SubnetRegistrationBlock::<T>::get(netuid)
                        .is_some_and(|block| current_block.saturating_sub(block) < immunity_period)
            })
            .min_by_key(|(_, emission)| *emission)
            .map(|(netuid, _)| netuid)
    }
    /// Removes the emission storage for a given subnet.
    ///
    /// # Arguments
    ///
    /// * `netuid` - The ID of the subnet to remove from storage.
    pub fn remove_subnet_emission_storage(netuid: u16) {
        SubnetEmission::<T>::remove(netuid);
    }

    /// Sets the emission storage for a given subnet.
    ///
    /// # Arguments
    ///
    /// * `netuid` - The ID of the subnet.
    /// * `emission` - The emission value to set for the subnet.
    pub fn set_subnet_emission_storage(netuid: u16, emission: u64) {
        SubnetEmission::<T>::insert(netuid, emission);
    }

    pub fn create_yuma_subnet(netuid: u16) {
        SubnetConsensusType::<T>::set(netuid, Some(SubnetConsensus::Yuma));
    }

    pub fn remove_yuma_subnet(netuid: u16) {
        if Self::can_remove_subnet(netuid) {
            SubnetConsensusType::<T>::remove(netuid);
        }
    }

    pub fn can_remove_subnet(netuid: u16) -> bool {
        matches!(
            SubnetConsensusType::<T>::get(netuid),
            Some(SubnetConsensus::Yuma)
        )
    }

    // Subnet is minable, if it's consensus isn't root or treasury
    pub fn is_mineable_subnet(netuid: u16) -> bool {
        matches!(
            SubnetConsensusType::<T>::get(netuid),
            Some(SubnetConsensus::Linear) | Some(SubnetConsensus::Yuma)
        )
    }

    // Gets consensus running id by iterating through consensus, until we find root consensus
    pub fn get_consensus_netuid(subnet_consensus: SubnetConsensus) -> Option<u16> {
        SubnetConsensusType::<T>::iter().find_map(|(netuid, consensus)| {
            if consensus == subnet_consensus {
                Some(netuid)
            } else {
                None
            }
        })
    }
}
