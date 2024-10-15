use super::*;
use crate::profitability::{get_copier_stake, is_copying_irrational};
use types::SimulationYumaParams;

pub fn process_consensus_params<T>(
    subnet_id: u16,
    consensus_params: Vec<(u64, ConsensusParams<T>)>,
    mut simulation_result: ConsensusSimulationResult<T>,
) -> (Vec<BlockWeights>, ShouldDecryptResult<T>)
where
    T: pallet_subspace::Config + pallet_subnet_emission::Config + pallet::Config,
{
    let mut epochs = Vec::new();
    let mut result = ShouldDecryptResult::<T> {
        should_decrypt: false,
        simulation_result: simulation_result.clone(),
        delta: I64F64::from_num(0),
    };

    log::info!("Processing consensus params for subnet {}", subnet_id);

    for (param_block, params) in &consensus_params {
        let decrypted_weights: Vec<_> = params
            .modules
            .iter()
            .filter_map(|(key, params)| {
                consensus_params.iter().find_map(|(_, consensus_param)| {
                    consensus_param
                        .modules
                        .iter()
                        .find(|(module_key, _)| module_key.0 == key.0)
                        .map(|(_, module_params)| (module_params.uid, params))
                })
            })
            .filter_map(|(uid, params)| {
                if params.weight_encrypted.is_empty() {
                    Some((uid, Vec::new(), Vec::new()))
                } else {
                    ow_extensions::offworker::decrypt_weight(params.weight_encrypted.clone())
                        .map(|(decrypted, key)| (uid, decrypted, key))
                }
            })
            .collect();

        let should_decrypt_result = should_decrypt_weights::<T>(
            &decrypted_weights
                .iter()
                .cloned()
                .map(|(uid, weights, _)| (uid, weights))
                .collect::<Vec<_>>(),
            params.clone(),
            subnet_id,
            simulation_result.clone(),
            *param_block,
        );

        simulation_result = should_decrypt_result.simulation_result.clone();

        if should_decrypt_result.should_decrypt {
            epochs.push((*param_block, decrypted_weights));
            result = should_decrypt_result;
        }
    }

    (epochs, result)
}

/// Returns
#[must_use]
pub fn should_decrypt_weights<T: Config>(
    decrypted_weights: &[(u16, Vec<(u16, u16)>)],
    latest_runtime_yuma_params: ConsensusParams<T>,
    subnet_id: u16,
    mut simulation_result: ConsensusSimulationResult<T>,
    block_number: u64,
) -> ShouldDecryptResult<T> {
    let SimulationYumaParams {
        uid: copier_uid,
        params: simulation_yuma_params,
        decrypted_weights_map,
    } = compute_simulation_yuma_params::<T>(
        decrypted_weights,
        latest_runtime_yuma_params,
        subnet_id,
    );

    // TODO: determine consensus dynamically
    let simulation_yuma_output = YumaEpoch::<T>::new(subnet_id, simulation_yuma_params)
        .run(decrypted_weights_map.into_iter().collect::<Vec<_>>())
        .unwrap();

    // This is fine to get from the runtime
    let delegation_fee = FloorDelegationFee::<T>::get();
    simulation_result.update(simulation_yuma_output, copier_uid, delegation_fee);

    let (is_irrational, delta) =
        is_copying_irrational::<T>(simulation_result.clone(), block_number);

    ShouldDecryptResult {
        should_decrypt: is_irrational,
        delta: delta.abs(), // Make sure to convert to positive
        simulation_result,
    }
}

/// Appends copier information to simulated consensus ConsensusParams
/// Overwrites onchain decrypted weights with the offchain workers' decrypted weights
pub fn compute_simulation_yuma_params<T: Config>(
    decrypted_weights: &[(u16, Vec<(u16, u16)>)],
    mut runtime_yuma_params: ConsensusParams<T>,
    subnet_id: u16,
) -> SimulationYumaParams<T> {
    let copier_uid: u16 = runtime_yuma_params.modules.len() as u16;

    // This **has** to be ontained from the runtime storage
    let consensus_weights = Consensus::<T>::get(subnet_id);
    let copier_weights: Vec<(u16, u16)> = consensus_weights
        .into_iter()
        .enumerate()
        .map(|(index, value)| (index as u16, value))
        .collect();

    runtime_yuma_params = add_copier_to_yuma_params(copier_uid, runtime_yuma_params, subnet_id);

    let mut onchain_weights: BTreeMap<u16, Vec<(u16, u16)>> =
        Weights::<T>::iter_prefix(subnet_id).collect();

    onchain_weights.extend(
        decrypted_weights
            .iter()
            .cloned()
            .chain(sp_std::iter::once((copier_uid, copier_weights))), /* HONZA TODO figure out
                                                                       * this validator key */
    );

    SimulationYumaParams {
        uid: copier_uid,
        params: runtime_yuma_params,
        decrypted_weights_map: onchain_weights,
    }
}

/// This will mutate ConsensusParams with copier information, ready for simulation
/// This function should run
pub fn add_copier_to_yuma_params<T: Config>(
    copier_uid: u16,
    mut runtime_yuma_params: ConsensusParams<T>,
    subnet_id: u16,
) -> ConsensusParams<T> {
    // It is fine to get the permits from the consensus params
    let total_active_stake: u64 = runtime_yuma_params
        .modules
        .values()
        .filter(|m| m.validator_permit)
        .map(|m| m.stake_original.to_num::<u64>())
        .sum();

    let copier_stake = get_copier_stake::<T>(total_active_stake);
    let current_block = runtime_yuma_params.current_block;

    // Collect original stakes, including the copier's stake
    let mut all_stakes: Vec<I64F64> = runtime_yuma_params
        .modules
        .values()
        .map(|m| m.stake_original)
        .chain(sp_std::iter::once(I64F64::from_num(copier_stake)))
        .collect();

    // Normalize all stakes using the provided function
    inplace_normalize_64(&mut all_stakes);

    // Convert normalized I64F64 stakes to I32F32
    let normalized_stakes = vec_fixed64_to_fixed32(all_stakes.clone());

    let copier_module = ModuleParams {
        uid: copier_uid,
        last_update: current_block,
        block_at_registration: current_block.saturating_sub(1),
        validator_permit: true,
        stake_normalized: *normalized_stakes.last().unwrap_or(&I32F32::from_num(0)),
        stake_original: I64F64::from_num(copier_stake),
        bonds: Vec::new(),
        weight_encrypted: Vec::new(),
        weight_hash: Vec::new(),
    };

    let seed = (b"copier", subnet_id, copier_uid).using_encoded(BlakeTwo256::hash);
    let copier_account_id = T::AccountId::decode(&mut seed.as_ref())
        .expect("32 bytes should be sufficient for any AccountId");

    let copier_key = ModuleKey(copier_account_id);

    // Update existing modules with new normalized stakes
    runtime_yuma_params.modules.values_mut().zip(normalized_stakes.iter()).for_each(
        |(module, &normalized)| {
            module.stake_normalized = normalized;
        },
    );

    // Insert the copier module
    runtime_yuma_params.modules.insert(copier_key, copier_module);

    runtime_yuma_params
}
