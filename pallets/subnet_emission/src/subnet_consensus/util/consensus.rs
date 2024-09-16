use super::params::{AccountKey, ConsensusParams, FlattenedModules, ModuleKey};
use crate::EmissionError;
use frame_support::{ensure, DebugNoBound};
use pallet_subspace::{math::*, vec, BalanceOf, Pallet as PalletSubspace};
use parity_scale_codec::{Decode, Encode};
use scale_info::TypeInfo;
use sp_std::{borrow::Cow, collections::btree_map::BTreeMap, vec::Vec};
use substrate_fixed::types::{I32F32, I64F64, I96F32};

use crate::Config;
pub type EmissionMap<AccountId> =
    BTreeMap<ModuleKey<AccountId>, BTreeMap<AccountKey<AccountId>, u64>>;

pub fn split_modules_by_activity(
    last_update: &[u64],
    block_at_registration: &[u64],
    activity_cutoff: u64,
    current_block: u64,
) -> (Vec<bool>, Vec<bool>) {
    last_update
        .iter()
        .zip(block_at_registration)
        .map(|(updated, block_at_registration)| {
            let is_inactive = *updated <= *block_at_registration
                || updated.saturating_add(activity_cutoff) < current_block;
            (is_inactive, !is_inactive)
        })
        .unzip()
}

pub fn extract_bonds<T: Config>(
    module_count: u16,
    new_permits: &[bool],
    ema_bonds: &[Vec<(u16, I32F32)>],
    has_max_validators: bool,
    existing_validator_permits: &[bool],
) -> Vec<Option<Vec<(u16, u16)>>> {
    (0..module_count as usize)
        .map(|i| {
            // Set bonds only if uid retains validator permit, otherwise clear bonds.
            if *new_permits.get(i).unwrap_or(&false) {
                let new_bonds_row: Vec<(u16, u16)> = ema_bonds
                    .get(i)
                    .map(|bonds_row| {
                        bonds_row
                            .iter()
                            .map(|(j, value)| (*j, fixed_proportion_to_u16(*value)))
                            .collect()
                    })
                    .unwrap_or_default();

                return Some(new_bonds_row);
            }

            if has_max_validators || *existing_validator_permits.get(i).unwrap_or(&false) {
                // Only overwrite the intersection.
                return Some(vec![]);
            }

            None
        })
        .collect()
}

pub fn calculate_final_emissions<T: Config>(
    founder_emission: u64,
    subnet_id: u16,
    result: Vec<(ModuleKey<T::AccountId>, u64, u64)>,
) -> Result<(EmissionMap<T::AccountId>, u64), EmissionError> {
    let mut emissions: EmissionMap<T::AccountId> = Default::default();
    let mut emitted: u64 = 0;

    if founder_emission > 0 {
        emitted = emitted.saturating_add(founder_emission);
    }

    for (module_key, miner_emisison, mut validator_emission) in result {
        let mut increase_stake = |account_key: AccountKey<T::AccountId>, amount: u64| {
            let stake =
                emissions.entry(module_key.clone()).or_default().entry(account_key).or_default();
            *stake = stake.saturating_add(amount);

            emitted = emitted.saturating_add(amount);
        };

        if validator_emission > 0 {
            let ownership_vector =
                PalletSubspace::<T>::get_ownership_ratios(subnet_id, &module_key.0);
            let delegation_fee = PalletSubspace::<T>::get_delegation_fee(&module_key.0);

            let total_validator_emission = I64F64::from_num(validator_emission);
            for (delegate_key, delegate_ratio) in ownership_vector {
                if delegate_key == module_key.0 {
                    continue;
                }

                let dividends_from_delegate: u64 = total_validator_emission
                    .checked_mul(delegate_ratio)
                    .unwrap_or_default()
                    .to_num::<u64>();

                let to_module: u64 = delegation_fee.mul_floor(dividends_from_delegate);
                let to_delegate: u64 = dividends_from_delegate.saturating_sub(to_module);

                increase_stake(AccountKey(delegate_key), to_delegate);

                validator_emission = validator_emission
                    .checked_sub(to_delegate)
                    .ok_or("more validator emissions were done than expected")?;
            }
        }

        let remaining_emission = miner_emisison.saturating_add(validator_emission);
        if remaining_emission > 0 {
            increase_stake(AccountKey(module_key.0.clone()), remaining_emission);
        }
    }

    Ok((emissions, emitted))
}

pub fn compute_weights<T: Config>(
    modules: &FlattenedModules<T::AccountId>,
    params: &ConsensusParams<T>,
) -> Option<WeightsVal> {
    // Access network weights row unnormalized.
    let mut weights = modules.weights_unencrypted.clone();
    log::trace!("  original weights: {weights:?}");

    let validator_forbids: Vec<bool> = modules.validator_permit.iter().map(|&b| !b).collect();

    if params.max_allowed_validators.is_some() {
        // Mask weights that are not from permitted validators.
        weights = mask_rows_sparse(&validator_forbids, &weights);
        log::trace!("  no forbidden validator weights: {weights:?}");
    }

    // Remove self-weight by masking diagonal.
    weights = mask_diag_sparse(&weights);
    log::trace!("  no self-weight weights: {weights:?}");

    // Remove weights referring to deregistered modules.
    weights = vec_mask_sparse_matrix(
        &weights,
        &modules.last_update,
        &modules.block_at_registration,
        |updated, registered| updated <= registered,
    )?;
    log::trace!("  no deregistered modules weights: {weights:?}");

    // Normalize remaining weights.
    inplace_row_normalize_sparse(&mut weights);

    log::trace!("  normalized weights: {weights:?}");

    Some(WeightsVal::unchecked_from_inner(weights))
}

pub fn compute_active_stake<T: Config>(
    modules: &FlattenedModules<T::AccountId>,
    params: &ConsensusParams<T>,
    inactive: &[bool],
    stake: &StakeVal,
) -> ActiveStake {
    let mut active_stake = stake.as_ref().clone();
    log::trace!("  original active stake: {active_stake:?}");

    // Remove inactive stake.
    inplace_mask_vector(inactive, &mut active_stake);
    log::trace!("  no inactive active stake: {active_stake:?}");

    if params.max_allowed_validators.is_some() {
        // Remove non-validator stake.
        inplace_mask_vector(&modules.validator_forbid, &mut active_stake);
        log::trace!("  no non-validator active stake: {active_stake:?}");
    }

    // Normalize active stake.
    inplace_normalize(&mut active_stake);
    log::trace!("  normalized active stake: {active_stake:?}");

    ActiveStake::unchecked_from_inner(active_stake)
}

/// There is no modulation of weights. Linear relationship between stake and weights.
pub fn compute_consensus_and_trust_linear<T: Config>(
    modules: &FlattenedModules<T::AccountId>,
    active_stake: &ActiveStake,
    weights: &WeightsVal,
) -> ConsensusAndTrust {
    let total_stake: I32F32 = active_stake
        .as_ref()
        .iter()
        .fold(I32F32::from_num(0), |acc, &x| acc.saturating_add(x));

    // Compute consensus as a weighted sum of validator weights
    let mut consensus = vec![I32F32::from_num(0); modules.module_count()];
    for (validator_idx, validator_weights) in weights.as_ref().iter().enumerate() {
        let stake = active_stake.as_ref().get(validator_idx).cloned().unwrap_or_default();
        let stake_ratio = stake.checked_div(total_stake).unwrap_or_default();

        for &(uid, weight) in validator_weights {
            let uid_usize = uid as usize;
            if let Some(consensus_weight) = consensus.get_mut(uid_usize) {
                *consensus_weight = consensus_weight
                    .saturating_add(weight.checked_mul(stake_ratio).unwrap_or_default());
            }
        }
    }

    // Computes preranks as stake-scaled weights for each module
    let preranks = matmul_sparse(
        weights.as_ref(),
        active_stake.as_ref(),
        modules.module_count(),
    );

    // Compute validator trust as the sum of their weights
    let validator_trust = row_sum_sparse(weights.as_ref());

    ConsensusAndTrust {
        consensus: ConsensusVal::unchecked_from_inner(consensus),
        validator_trust: ValidatorTrustVal::unchecked_from_inner(validator_trust),
        preranks: Preranks::unchecked_from_inner(preranks),
    }
}

pub fn compute_consensus_and_trust_yuma<T: Config>(
    modules: &FlattenedModules<T::AccountId>,
    params: &ConsensusParams<T>,
    weights: &mut WeightsVal,
    active_stake: &ActiveStake,
) -> ConsensusAndTrust {
    // Clip weights at majority consensus
    let consensus = weighted_median_col_sparse(
        active_stake.as_ref(),
        weights.as_ref(),
        modules.module_count(),
        params.kappa,
    );

    log::trace!("final consensus: {consensus:?}");

    // Compute preranks: r_j = SUM(i) w_ij * s_i
    let preranks = matmul_sparse(
        weights.as_ref(),
        active_stake.as_ref(),
        modules.module_count(),
    );
    log::trace!("final preranks: {preranks:?}");

    *weights = WeightsVal::unchecked_from_inner(col_clip_sparse(weights.as_ref(), &consensus));

    log::trace!("final consensus weights: {weights:?}");

    let validator_trust = row_sum_sparse(weights.as_ref());
    log::trace!("final validator trust: {validator_trust:?}");

    ConsensusAndTrust {
        consensus: ConsensusVal::unchecked_from_inner(consensus),
        validator_trust: ValidatorTrustVal::unchecked_from_inner(validator_trust),
        preranks: Preranks::unchecked_from_inner(preranks),
    }
}

pub fn compute_emissions<'a>(
    token_emission: u64,
    stake: &'a StakeVal,
    active_stake: &'a ActiveStake,
    incentives: &IncentivesVal,
    dividends: &DividendsVal,
) -> Emissions {
    let stake = stake.as_ref();
    let active_stake = active_stake.as_ref();

    // Compute normalized emission scores. range: I32F32(0, 1)
    let combined_emission: Vec<I32F32> = incentives
        .as_ref()
        .iter()
        .zip(dividends.as_ref().iter())
        .map(|(ii, di)| ii.saturating_add(*di))
        .collect();
    log::trace!("  original combined emissions: {combined_emission:?}");
    let emission_sum: I32F32 = combined_emission.iter().sum();
    log::trace!("  emission sum: {emission_sum:?}");

    let mut normalized_miner_emission = incentives.as_ref().clone(); // Servers get incentive.
    inplace_normalize_using_sum(&mut normalized_miner_emission, emission_sum);

    let normalized_validator_emission: Cow<'a, [I32F32]>;
    let normalized_combined_emission: Cow<'a, [I32F32]>;

    // If emission is zero, replace emission with normalized stake.
    if emission_sum == I32F32::from_num(0) {
        // no weights set | outdated weights | self_weights
        if is_zero(active_stake) {
            // no active stake
            // do not mask inactive, assumes stake is normalized
            normalized_validator_emission = Cow::Borrowed(stake);
            normalized_combined_emission = Cow::Borrowed(stake);
        } else {
            // emission proportional to inactive-masked normalized stake
            normalized_validator_emission = Cow::Borrowed(active_stake);
            normalized_combined_emission = Cow::Borrowed(active_stake);
        }
    } else {
        let mut validator_emission = dividends.as_ref().clone(); // Validators get dividends.
        inplace_normalize_using_sum(&mut validator_emission, emission_sum);
        normalized_validator_emission = Cow::Owned(validator_emission);

        let mut combined_emission = combined_emission;
        inplace_normalize(&mut combined_emission);
        normalized_combined_emission = Cow::Owned(combined_emission);
    }

    log::trace!("normalized miner emission: {normalized_miner_emission:?}");
    log::trace!("normalized validator emission: {normalized_validator_emission:?}");
    log::trace!("normalized combined emission: {normalized_combined_emission:?}");

    let to_be_emitted = I96F32::from_num::<u64>(token_emission);
    log::trace!("  to be emitted: {to_be_emitted}");

    let miner_emissions: Vec<u64> = normalized_miner_emission
        .iter()
        .map(|&se| I96F32::from_num(se).checked_mul(to_be_emitted).unwrap_or_default())
        .map(I96F32::to_num)
        .collect();
    log::trace!("  miners emissions: {miner_emissions:?}");

    let validator_emissions: Vec<u64> = normalized_validator_emission
        .iter()
        .map(|&ve| I96F32::from_num(ve).checked_mul(to_be_emitted).unwrap_or_default())
        .map(I96F32::to_num)
        .collect();
    log::trace!("  validator_emissions: {validator_emissions:?}");

    // Only used to track emission in storage.
    let combined_emissions: Vec<u64> = normalized_combined_emission
        .iter()
        .map(|&ce| I96F32::from_num(ce).checked_mul(to_be_emitted).unwrap_or_default())
        .map(I96F32::to_num)
        .collect();
    log::trace!("  combined_emissions: {combined_emissions:?}");

    // Set pruning scores using combined emission scores.
    let pruning_scores = normalized_combined_emission.into_owned();

    Emissions {
        pruning_scores: PruningScoresVal::unchecked_from_inner(pruning_scores),
        validator_emissions,
        miner_emisisons: miner_emissions,
        combined_emissions,
    }
}

pub fn compute_bonds_and_dividends_linear<T: Config>(
    modules: &FlattenedModules<T::AccountId>,
    weights: &WeightsVal,
    active_stake: &ActiveStake,
    incentives: &IncentivesVal,
) -> Option<BondsAndDividends> {
    // Access network bonds.
    let mut bonds = modules.bonds.clone();
    log::trace!("  original bonds: {bonds:?}");

    // Remove bonds referring to deregistered modules.
    bonds = vec_mask_sparse_matrix(
        &bonds,
        &modules.last_update,
        &modules.block_at_registration,
        |updated, registered| updated <= registered,
    )?;

    log::trace!("  no deregistered modules bonds: {bonds:?}");

    // Normalize remaining bonds: sum_i b_ij = 1.
    inplace_col_normalize_sparse(&mut bonds, modules.module_count());
    log::trace!("  normalized bonds: {bonds:?}");

    // Compute bonds delta column normalized.
    let mut bonds_delta = row_hadamard_sparse(weights.as_ref(), active_stake.as_ref()); // ΔB = W◦S (outdated W masked)
    log::trace!("  original bonds delta: {bonds_delta:?}");

    // Normalize bonds delta.
    inplace_col_normalize_sparse(&mut bonds_delta, modules.module_count()); // sum_i b_ij = 1
    log::trace!("  normalized bonds delta: {bonds_delta:?}");

    // Compute dividends: d_i = SUM(j) b_ij * inc_j.
    // range: I32F32(0, 1)
    let mut dividends = matmul_transpose_sparse(&bonds_delta, incentives.as_ref());
    log::trace!("  original dividends: {dividends:?}");

    inplace_normalize(&mut dividends);
    log::trace!("  normalized dividends: {dividends:?}");

    Some(BondsAndDividends {
        ema_bonds: bonds_delta, // Use bonds_delta instead of ema_bonds
        dividends: DividendsVal::unchecked_from_inner(dividends),
    })
}

pub fn compute_bonds_and_dividends_yuma<T: Config>(
    params: &ConsensusParams<T>,
    modules: &FlattenedModules<T::AccountId>,
    consensus: &ConsensusVal,
    weights: &WeightsVal,
    active_stake: &ActiveStake,
    incentives: &IncentivesVal,
) -> Option<BondsAndDividends> {
    // Access network bonds.
    let mut bonds = modules.bonds.clone();
    log::trace!("  original bonds: {bonds:?}");

    // Remove bonds referring to deregistered modules.
    bonds = vec_mask_sparse_matrix(
        &bonds,
        &modules.last_update,
        &modules.block_at_registration,
        |updated, registered| updated <= registered,
    )?;

    log::trace!("  no deregistered modules bonds: {bonds:?}");

    // Normalize remaining bonds: sum_i b_ij = 1.
    inplace_col_normalize_sparse(&mut bonds, modules.module_count());
    log::trace!("  normalized bonds: {bonds:?}");

    // Compute bonds delta column normalized.
    let mut bonds_delta = row_hadamard_sparse(weights.as_ref(), active_stake.as_ref()); // ΔB = W◦S (outdated W masked)
    log::trace!("  original bonds delta: {bonds_delta:?}");

    // Normalize bonds delta.
    inplace_col_normalize_sparse(&mut bonds_delta, modules.module_count()); // sum_i b_ij = 1
    log::trace!("  normalized bonds delta: {bonds_delta:?}");

    // Compute bonds moving average.
    let mut ema_bonds = calculate_ema_bonds(
        params,
        &bonds_delta,
        &bonds,
        &consensus.clone().into_inner(),
    );

    log::trace!("  original ema bonds: {ema_bonds:?}");

    // Normalize EMA bonds.
    inplace_col_normalize_sparse(&mut ema_bonds, modules.module_count()); // sum_i b_ij = 1
    log::trace!("  normalized ema bonds: {ema_bonds:?}");

    // Compute dividends: d_i = SUM(j) b_ij * inc_j.
    // range: I32F32(0, 1)
    let mut dividends = matmul_transpose_sparse(&ema_bonds, incentives.as_ref());
    log::trace!("  original dividends: {dividends:?}");

    inplace_normalize(&mut dividends);
    log::trace!("  normalized dividends: {dividends:?}");

    // Column max-upscale EMA bonds for storage: max_i w_ij = 1.
    inplace_col_max_upscale_sparse(&mut ema_bonds, modules.module_count());
    log::trace!("  upscaled ema bonds: {ema_bonds:?}");

    Some(BondsAndDividends {
        ema_bonds,
        dividends: DividendsVal::unchecked_from_inner(dividends),
    })
}

pub fn calculate_ema_bonds<T: Config>(
    params: &ConsensusParams<T>,
    bonds_delta: &[Vec<(u16, I32F32)>],
    bonds: &[Vec<(u16, I32F32)>],
    consensus: &[I32F32],
) -> Vec<Vec<(u16, I32F32)>> {
    let bonds_moving_average = I64F64::from_num(params.bonds_moving_average)
        .checked_div(I64F64::from_num(1_000_000))
        .unwrap_or_default();
    let default_alpha = I32F32::from_num(1).saturating_sub(I32F32::from_num(bonds_moving_average));

    if !params.use_weights_encryption {
        return mat_ema_sparse(bonds_delta, bonds, default_alpha);
    }

    let consensus_high = quantile(consensus, 0.75);
    let consensus_low = quantile(consensus, 0.25);

    if consensus_high <= consensus_low && consensus_high == 0 && consensus_low >= 0 {
        return mat_ema_sparse(bonds_delta, bonds, default_alpha);
    }
    log::trace!("Using Liquid Alpha");
    let (alpha_low, alpha_high) = params.alpha_values;
    log::trace!("alpha_low: {:?} alpha_high: {:?}", alpha_low, alpha_high);

    let (a, b) = calculate_logistic_params(alpha_high, alpha_low, consensus_high, consensus_low);
    let alpha = compute_alpha_values(consensus, a, b);
    let clamped_alpha: Vec<I32F32> =
        alpha.into_iter().map(|a| a.clamp(alpha_low, alpha_high)).collect();

    mat_ema_alpha_vec_sparse(bonds_delta, bonds, &clamped_alpha)
}

pub fn compute_incentive_and_trust<T: Config>(
    modules: &FlattenedModules<T::AccountId>,
    weights: &WeightsVal,
    active_stake: &ActiveStake,
    preranks: &Preranks,
) -> IncentivesAndTrust {
    // Compute ranks: r_j = SUM(i) w_ij * s_i.
    let ranks = matmul_sparse(
        weights.as_ref(),
        active_stake.as_ref(),
        modules.module_count(),
    );
    log::trace!("final ranks: {ranks:?}");

    // Compute miner trust: ratio of rank after vs. rank before.
    let trust = vecdiv(&ranks, preranks.as_ref()); // range: I32F32(0, 1)
    log::trace!("final trust: {ranks:?}");

    let mut incentives = ranks.clone();
    log::trace!("  original incentives: {incentives:?}");

    inplace_normalize(&mut incentives); // range: I32F32(0, 1)
    log::trace!("  normalized incentives: {incentives:?}");

    IncentivesAndTrust {
        incentives: IncentivesVal::unchecked_from_inner(incentives),
        ranks: RanksVal::unchecked_from_inner(ranks),
        trust: TrustVal::unchecked_from_inner(trust),
    }
}

pub fn process_consensus_output<T: Config>(
    params: &ConsensusParams<T>,
    modules: &FlattenedModules<T::AccountId>,
    stake: StakeVal,
    active_stake: ActiveStake,
    consensus: ConsensusVal,
    incentives: IncentivesVal,
    dividends: DividendsVal,
    trust: TrustVal,
    ranks: RanksVal,
    active: Vec<bool>,
    validator_trust: ValidatorTrustVal,
    new_permits: Vec<bool>,
    ema_bonds: &[Vec<(u16, I32F32)>],
) -> Result<ConsensusOutput<T>, EmissionError> {
    let subnet_id = params.subnet_id;
    let Emissions {
        pruning_scores,
        validator_emissions,
        miner_emisisons,
        combined_emissions,
    } = compute_emissions(
        params.token_emission.try_into().unwrap_or_default(),
        &stake,
        &active_stake,
        &incentives,
        &dividends,
    );

    let consensus: Vec<_> =
        consensus.into_inner().into_iter().map(fixed_proportion_to_u16).collect();
    let incentives: Vec<_> =
        incentives.into_inner().into_iter().map(fixed_proportion_to_u16).collect();
    let dividends: Vec<_> =
        dividends.into_inner().into_iter().map(fixed_proportion_to_u16).collect();
    let trust: Vec<_> = trust.into_inner().into_iter().map(fixed_proportion_to_u16).collect();
    let ranks: Vec<_> = ranks.into_inner().into_iter().map(fixed_proportion_to_u16).collect();
    let pruning_scores = vec_max_upscale_to_u16(pruning_scores.as_ref());
    let validator_trust: Vec<_> =
        validator_trust.into_inner().into_iter().map(fixed_proportion_to_u16).collect();

    ensure!(
        new_permits.len() == modules.module_count::<usize>(),
        "unequal number of permits and modules"
    );
    ensure!(
        ema_bonds.len() == modules.module_count::<usize>(),
        "unequal number of bonds and modules"
    );
    ensure!(
        modules.validator_permit.len() == modules.module_count::<usize>(),
        "unequal number of bonds and modules"
    );

    let has_max_validators = params.max_allowed_validators.is_none();

    let bonds = extract_bonds::<T>(
        modules.module_count(),
        &new_permits,
        &ema_bonds,
        has_max_validators,
        &modules.validator_permit,
    );

    // Emission tuples ( key, miner_emisison, validator_emission )
    let mut result = Vec::with_capacity(modules.module_count());
    for (module_uid, module_key) in modules.keys.iter().enumerate() {
        result.push((
            ModuleKey(module_key.0.clone()),
            *miner_emisisons.get(module_uid).unwrap_or(&0),
            *validator_emissions.get(module_uid).unwrap_or(&0),
        ));
    }

    let (emission_map, total_emitted) =
        calculate_final_emissions::<T>(params.founder_emission, subnet_id, result)?;
    log::debug!(
        "finished yuma for {} with distributed: {emission_map:?}",
        subnet_id
    );

    Ok(ConsensusOutput {
        subnet_id,

        active,
        consensus,
        dividends,
        combined_emissions,
        incentives,
        pruning_scores,
        ranks,
        trust,
        validator_permits: new_permits,
        validator_trust,
        bonds,

        founder_emission: params.founder_emission,
        emission_map,
        total_emitted,

        params: params.clone(),
    })
}

#[derive(DebugNoBound, Clone, Encode, Decode, TypeInfo)]
#[scale_info(skip_type_params(T))]
pub struct ConsensusOutput<T: Config> {
    pub subnet_id: u16,
    pub params: ConsensusParams<T>,

    pub active: Vec<bool>,
    pub consensus: Vec<u16>,
    pub dividends: Vec<u16>,
    pub combined_emissions: Vec<u64>,
    pub incentives: Vec<u16>,
    pub pruning_scores: Vec<u16>,
    pub ranks: Vec<u16>,
    pub trust: Vec<u16>,
    pub validator_permits: Vec<bool>,
    pub validator_trust: Vec<u16>,
    pub bonds: Vec<Option<Vec<(u16, u16)>>>,

    pub founder_emission: BalanceOf<T>,
    pub emission_map: EmissionMap<T::AccountId>,
    pub total_emitted: u64,
}

impl<T: Config> ConsensusOutput<T> {
    pub fn apply(self) {
        use pallet_subspace::*;

        let Self {
            subnet_id,
            active,
            consensus,
            dividends,
            combined_emissions,
            incentives,
            pruning_scores,
            ranks,
            trust,
            validator_permits,
            validator_trust,
            bonds,
            ..
        } = self;

        Active::<T>::insert(subnet_id, active);
        Consensus::<T>::insert(subnet_id, consensus);
        Dividends::<T>::insert(subnet_id, dividends);
        Emission::<T>::insert(subnet_id, combined_emissions);
        Incentive::<T>::insert(subnet_id, incentives);
        PruningScores::<T>::insert(subnet_id, pruning_scores);
        Rank::<T>::insert(subnet_id, ranks);
        Trust::<T>::insert(subnet_id, trust);
        ValidatorPermits::<T>::insert(subnet_id, validator_permits);
        ValidatorTrust::<T>::insert(subnet_id, validator_trust);

        for (module_uid, bonds) in bonds.into_iter().enumerate() {
            let Some(bonds) = bonds else {
                continue;
            };

            Bonds::<T>::insert(subnet_id, module_uid as u16, bonds);
        }

        // TODO:  why is this commented out ?
        // ensure!(
        //     self.total_emitted <= self.founder_emission.saturating_add(params.token_emission),
        //     EmissionError::EmittedMoreThanExpected {
        //         emitted,
        //         expected: self.params.founder_emission.saturating_add(self.params.token_emission)
        //     }
        // );

        log::trace!("emitted {:?} tokens in total", self.total_emitted);

        PalletSubspace::<T>::add_balance_to_account(
            &self.params.founder_key.0,
            self.founder_emission,
        );

        for (module_key, emitted_to) in self.emission_map {
            for (account_key, emission) in emitted_to {
                PalletSubspace::<T>::increase_stake(&account_key.0, &module_key.0, emission);
            }
        }
    }
}

pub struct ConsensusAndTrust {
    pub consensus: ConsensusVal,
    pub validator_trust: ValidatorTrustVal,
    pub preranks: Preranks,
}

pub struct BondsAndDividends {
    pub ema_bonds: Vec<Vec<(u16, I32F32)>>,
    pub dividends: DividendsVal,
}

pub struct IncentivesAndTrust {
    pub incentives: IncentivesVal,
    pub ranks: RanksVal,
    pub trust: TrustVal,
}

pub struct Emissions {
    pub pruning_scores: PruningScoresVal,
    pub validator_emissions: Vec<u64>,
    pub miner_emisisons: Vec<u64>,
    pub combined_emissions: Vec<u64>,
}

bty::brand! {
    pub type ActiveStake = Vec<I32F32>;
    pub type ConsensusVal = Vec<I32F32>;
    pub type DividendsVal = Vec<I32F32>;
    pub type IncentivesVal = Vec<I32F32>;
    pub type Preranks = Vec<I32F32>;
    pub type PruningScoresVal = Vec<I32F32>;
    pub type RanksVal = Vec<I32F32>;
    pub type StakeVal = Vec<I32F32>;
    pub type TrustVal = Vec<I32F32>;
    pub type ValidatorTrustVal = Vec<I32F32>;
    pub type WeightsVal = Vec<Vec<(u16, I32F32)>>;
}
