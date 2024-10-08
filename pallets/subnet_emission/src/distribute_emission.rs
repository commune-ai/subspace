use super::*;
use crate::subnet_consensus::{linear::LinearEpoch, treasury::TreasuryEpoch};

use crate::subnet_consensus::util::params::ConsensusParams;
use frame_support::storage::with_storage_layer;
use pallet_subnet_emission_api::SubnetConsensus;
use pallet_subspace::{Pallet as PalletSubspace, N};
use subnet_consensus::yuma::YumaEpoch;

/// Processes subnets by updating pending emissions and running epochs when due.
///
/// # Arguments
///
/// * `block_number` - The current block number.
/// * `subnets_emission_distribution` - A map of subnet IDs to their emission values.
///
/// This function iterates through all subnets, updates their pending emissions,
/// and runs an epoch if it's time for that subnet.
fn process_subnets<T: Config>(block_number: u64, subnets_emission_distribution: PricedSubnets) {
    for netuid in N::<T>::iter_keys() {
        update_pending_emission::<T>(
            netuid,
            subnets_emission_distribution.get(&netuid).unwrap_or(&0),
        );

        if pallet_subspace::Pallet::<T>::blocks_until_next_epoch(netuid, block_number) == 0 {
            run_epoch::<T>(netuid);
        }
    }
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
fn run_epoch<T: Config>(netuid: u16) {
    log::trace!("running epoch for subnet {netuid}");

    let emission_to_drain = PendingEmission::<T>::get(netuid);
    if emission_to_drain > 0 {
        match run_consensus_algorithm::<T>(netuid, emission_to_drain) {
            Ok(_) => {
                finalize_epoch::<T>(netuid);
            }
            Err(e) => {
                log::error!(
                    "Error running consensus algorithm for subnet {}: {:?}",
                    netuid,
                    e
                );
            }
        }
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
) -> Result<(), &'static str> {
    with_storage_layer(|| {
        let Some(consensus_type) = SubnetConsensusType::<T>::get(netuid) else {
            return Ok(());
        };

        match consensus_type {
            SubnetConsensus::Root => Ok(()),
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
pub fn run_linear_consensus<T: Config>(
    netuid: u16,
    emission_to_drain: u64,
) -> Result<(), &'static str> {
    let params = ConsensusParams::<T>::new(netuid, emission_to_drain)
        .map_err(|_| "Failed to create ConsensusParams")?;

    let run = LinearEpoch::<T>::new(netuid, params);

    let uids = pallet_subspace::Keys::<T>::iter_prefix(netuid).collect::<BTreeMap<_, _>>();
    let mut weights: BTreeMap<_, _> = Weights::<T>::iter_prefix(netuid).collect();
    let weights = uids.keys().map(|uid| (*uid, weights.remove(uid).unwrap_or_default())).collect();

    let consensus_output = run.run(weights).map_err(|_| "Failed to run consensus")?;
    consensus_output.apply();
    Ok(())
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
    let params = ConsensusParams::<T>::new(netuid, emission_to_drain)?;

    if !pallet_subspace::UseWeightsEncrytyption::<T>::get(netuid) {
        let output =
            match YumaEpoch::new(netuid, params).run(Weights::<T>::iter_prefix(netuid).collect()) {
                Ok(output) => output,
                Err(err) => {
                    log::error!("could not run yuma consensus for {netuid}: {err:?}");
                    return Err("could not run yuma consensus");
                }
            };

        output.apply();

        return Ok(());
    }

    let mut accumulated_emission: u64 = 0;
    if let Some(mut weights) = DecryptedWeights::<T>::get(netuid) {
        weights.sort_by_key(|(block, _)| *block);

        let last = weights.last().cloned();

        for (block, weights) in weights {
            let consensus_type =
                SubnetConsensusType::<T>::get(netuid).ok_or("Invalid network ID")?;

            if consensus_type != pallet_subnet_emission_api::SubnetConsensus::Yuma {
                return Err("Unsupported consensus type");
            }

            let Some(mut params) = ConsensusParameters::<T>::get(netuid, block) else {
                log::error!("no params found for netuid {netuid} block {block}");
                continue;
            };

            ConsensusParameters::<T>::remove(netuid, block);

            params.token_emission = params.token_emission.saturating_add(accumulated_emission);
            let new_emission = params.token_emission;
            accumulated_emission = 0;

            let output = match YumaEpoch::new(netuid, params).run(weights) {
                Ok(output) => output,
                Err(err) => {
                    log::error!("could not run yuma consensus for {netuid} block {block}: {err:?}");
                    accumulated_emission = new_emission;
                    continue;
                }
            };

            output.apply();
        }

        if let Some((_, last)) = last {
            for (uid, weights) in last {
                Weights::<T>::set(netuid, uid, Some(weights));
            }
        }

        let _ = DecryptedWeights::<T>::remove(netuid);
    };

    let block_number = PalletSubspace::<T>::get_current_block_number();

    ConsensusParameters::<T>::insert(netuid, block_number, params);
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
) -> Result<(), &'static str> {
    TreasuryEpoch::<T>::new(netuid, emission_to_drain)
        .run()
        .map(|_| ())
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
    pub fn process_emission_distribution(block_number: u64, emission_per_block: u64) {
        log::debug!("stepping block {block_number:?}");

        let subnets_emission_distribution = Self::get_subnet_pricing(emission_per_block);
        process_subnets::<T>(block_number, subnets_emission_distribution);
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
