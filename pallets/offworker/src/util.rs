use super::*;

/// TODO:
/// This is wrong, when `  if should_decrypt_result.should_decrypt {` is met,
/// we will decrypt the weights of literally all ConsensusParams for that given subnet.
/// so everything that was encrypted for that subnet will be decrypted.
pub fn process_consensus_params<T: Config>(
    subnet_id: u16,
    consensus_params: Vec<(u64, ConsensusParams<T>)>,
) -> (Vec<(u64, Vec<(UidT<T>, Vec<u8>)>)>, ShouldDecryptResult<T>)
where
    T: pallet_subspace::Config,
{
    let mut epochs = Vec::new();
    let mut result = SimulationResult::<T>::default();

    for (param_block, params) in consensus_params {
        let decrypted_weights = params
            .modules
            .iter()
            .filter_map(|(key, params)| {
                let Some(uid) = pallet_subspace::Pallet::<T>::get_uid_for_key(subnet_id, &key.0)
                else {
                    return None;
                };

                if params.weight_encrypted.is_empty() {
                    return Some((uid, Vec::new()));
                }

                ow_extensions::offworker::decrypt_weight(params.weight_encrypted.clone())
                    .map(|decrypted| (uid, decrypted))
            })
            .collect::<Vec<_>>();

        let should_decrypt_result: ShouldDecryptResult<T> =
            Self::should_decrypt_weights(&decrypted_weights, params, subnet_id, result.clone());

        result = should_decrypt_result.simulation_result;

        if should_decrypt_result.should_decrypt {
            epochs.push((param_block, decrypted_weights));
        }
    }

    (epochs, should_decrypt_result)
}

/// Returns
#[must_use]
pub fn should_decrypt_weights(
    decrypted_weights: &[(u16, Vec<(u16, u16)>)],
    latest_runtime_yuma_params: ConsensusParams<T>,
    subnet_id: u16,
    mut simulation_result: ConsensusSimulationResult<T>,
) -> ShouldDecryptResult<T> {
    // Now this will return struct X
    let (copier_uid, simulation_yuma_params) = Pallet::<T>::compute_simulation_yuma_params(
        decrypted_weights,
        latest_runtime_yuma_params,
        subnet_id,
    );

    // Run simulation
    /// TODO:
    /// use here the ` decrypted_weights` from the struct X instead of the one from the function arguments
    let simulation_yuma_output = YumaEpoch::<T>::new(subnet_id, simulation_yuma_params)
        .run(decrypted_weights.to_vec())
        .unwrap();

    // Update the simulation result
    let tempo = Tempo::<T>::get(subnet_id);
    let delegation_fee = FloorDelegationFee::<T>::get();
    simulation_result.update(simulation_yuma_output, tempo, copier_uid, delegation_fee);

    let (is_irrational, delta) = is_copying_irrational::<T>(simulation_result.clone());

    ShouldDecryptResult {
        should_decrypt: is_irrational,
        delta,
        simulation_result,
    }
}

/// Appends copier information to simulated consensus ConsensusParams
/// Overwrites onchain decrypted weights with the offchain workers' decrypted weights
///
/// TODO:
/// make this return a struct of u16, ConsensusParams<T>, decrypted_weights_map. Every of them seperately
pub fn compute_simulation_yuma_params(
    decrypted_weights: &[(u16, Vec<(u16, u16)>)],
    mut runtime_yuma_params: ConsensusParams<T>,
    subnet_id: u16,
    // Return copier uid and ConsensusParams
) -> (u16, ConsensusParams<T>) {
    let copier_uid: u16 = N::<T>::get(subnet_id);

    let consensus_weights = Consensus::<T>::get(subnet_id);
    let copier_weights: Vec<(u16, u16)> = consensus_weights
        .into_iter()
        .enumerate()
        .map(|(index, value)| (index as u16, value))
        .collect();

    // Overwrite the runtime yuma params with copier information
    runtime_yuma_params = Self::add_copier_to_yuma_params(
        copier_uid,
        runtime_yuma_params,
        subnet_id,
        copier_weights,
    );

    // Query the onchain weights for subnet_id
    /// TODO:
    /// Make sure the runtime really writes into the  `Weights` even for subnets that have the encryption on
    /// otherwise this whole logic is fucked !
    let onchain_weights: Vec<(u16, Vec<(u16, u16)>)> =
        Weights::<T>::iter_prefix(subnet_id).collect();

    // Create a map of uid to decrypted weights for easier lookup
    let decrypted_weights_map: BTreeMap<u16, Vec<(u16, u16)>> =
        decrypted_weights.iter().cloned().collect();

    (copier_uid, runtime_yuma_params)
}

/// This will mutate ConsensusParams with copier information, ready for simulation
pub fn add_copier_to_yuma_params(
    copier_uid: u16,
    mut runtime_yuma_params: ConsensusParams<T>,
    subnet_id: u16,
    weights: Vec<(u16, u16)>,
) -> ConsensusParams<T> {
    let copier_stake = get_copier_stake::<T>(subnet_id);
    let current_block = runtime_yuma_params.current_block;

    let mut all_stakes: Vec<I64F64> = runtime_yuma_params
        .modules
        .values()
        .map(|m| m.stake_original)
        .chain(std::iter::once(I64F64::from_num(copier_stake)))
        .collect();

    inplace_normalize_64(&mut all_stakes);

    let normalized_stakes = vec_fixed64_to_fixed32(all_stakes.clone());

    let copier_stake_normalized = normalized_stakes.last().cloned().unwrap_or_default();

    let copier_module = ModuleParams {
        uid: copier_uid,
        last_update: current_block,
        block_at_registration: current_block.saturating_sub(1),
        validator_permit: true,
        stake_normalized: copier_stake_normalized,
        stake_original: I64F64::from_num(copier_stake),
        bonds: Vec::new(),
        weight_encrypted: Vec::new(),
        weight_hash: Vec::new(),
    };

    let seed = (b"copier", subnet_id, copier_uid).using_encoded(BlakeTwo256::hash);
    let copier_account_id = T::AccountId::decode(&mut seed.as_ref())
        .expect("32 bytes should be sufficient for any AccountId");

    let copier_key = ModuleKey(copier_account_id);

    runtime_yuma_params.modules.insert(copier_key.clone(), copier_module);

    for (index, module) in runtime_yuma_params.modules.values_mut().enumerate() {
        module.stake_normalized =
            normalized_stakes.get(index).cloned().unwrap_or_else(|| I32F32::from_num(0));
        module.stake_original =
            all_stakes.get(index).cloned().unwrap_or_else(|| I64F64::from_num(0));
    }
    if let Some(copier_module) = runtime_yuma_params.modules.get_mut(&copier_key) {
        copier_module.stake_original = I64F64::from_num(copier_stake);
    }

    runtime_yuma_params
}
