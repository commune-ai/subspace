use crate::EmissionError;
use core::marker::PhantomData;
use frame_support::{ensure, weights::Weight, DebugNoBound};
use pallet_subspace::{math::*, BalanceOf, Config, Pallet as PalletSubspace};
pub use params::{AccountKey, ModuleKey, YumaParams};
use parity_scale_codec::{Decode, Encode};
use scale_info::TypeInfo;
use sp_std::{borrow::Cow, collections::btree_map::BTreeMap, vec, vec::Vec};
use substrate_fixed::types::{I32F32, I64F64, I96F32};

use super::WeightCounter;
pub mod params;

pub type EmissionMap<AccountId> =
    BTreeMap<ModuleKey<AccountId>, BTreeMap<AccountKey<AccountId>, u64>>;
pub type YumaEmissionMap<AccountId> =
    BTreeMap<ModuleKey<AccountId>, BTreeMap<AccountKey<AccountId>, u64>>;

#[derive(DebugNoBound)]
pub struct YumaEpoch<T: Config> {
    /// The UID of the subnet
    subnet_id: u16,

    params: params::YumaParams<T>,
    modules: params::FlattenedModules<T::AccountId>,

    weight_counter: WeightCounter,

    _pd: PhantomData<T>,
}

impl<T: Config> YumaEpoch<T> {
    pub fn new(subnet_id: u16, mut params: params::YumaParams<T>) -> Self {
        let modules = sp_std::mem::take(&mut params.modules).into();
        let mut weight_counter = WeightCounter::new();

        let validator_permits = ValidatorPermits::<T>::get(netuid);
        weight_counter.read(1);
        let validator_forbids = validator_permits.iter().map(|&b| !b).collect();

        let founder_key = Founder::<T>::get(netuid);
        weight_counter.read(1);
        let (to_be_emitted, founder_emission) =
            PalletSubspace::<T>::calculate_founder_emission(netuid, to_be_emitted);
        weight_counter.read(1);

        weight_counter.read(7);
        Self {
            subnet_id,

            params,
            modules,

            weight_counter,

            _pd: Default::default(),
        }
    }

    #[inline]
    fn module_count<I: From<u16>>(&self) -> I {
        (self.modules.keys.len() as u16).into()
    }

    /// Runs the YUMA consensus calculation on the network and distributes the emissions. Returns a
    /// map of emissions distributed per module key.
    pub fn run(self) -> Result<YumaOutput<T>, EmissionError> {
        log::debug!(
            "running yuma for subnet_id {}, will emit {:?} modules and {:?} to founder",
            self.subnet_id,
            self.params.token_emission,
            self.params.founder_emission
        );
        log::trace!("yuma for subnet_id {} parameters: {self:?}", self.subnet_id);

        let (inactive, active): (Vec<_>, Vec<_>) = self
            .modules
            .last_update
            .iter()
            .zip(&self.modules.block_at_registration)
            .map(|(updated, block_at_registration)| {
                let is_inactive = *updated <= *block_at_registration
                    || updated.saturating_add(self.params.activity_cutoff)
                        < self.params.current_block;
                (is_inactive, !is_inactive)
            })
            .unzip();

        let mut weights =
            self.compute_weights().ok_or(EmissionError::Other("weights are broken"))?;

        let stake = StakeVal::unchecked_from_inner(self.modules.stake.clone());
        log::trace!("final stake: {stake:?}");

        let new_permits: Vec<bool> = if let Some(max) = self.params.max_allowed_validators {
            is_topk(stake.as_ref(), max as usize)
        } else {
            vec![true; stake.as_ref().len()]
        };
        log::trace!("new permis: {new_permits:?}");

        let mut sorted_indexed_stake: Vec<(u16, u64)> = (0u16..(stake.as_ref().len() as u16))
            .map(|idx| {
                self.weight_counter.read(1);
                let key = match PalletSubspace::<T>::get_key_for_uid(self.netuid, idx) {
                    Some(key) => key,
                    None => return Err(EmissionError::Other("module doesn't have a key")),
                };

                self.weight_counter.read(1);
                let stake = PalletSubspace::<T>::get_delegated_stake(&key);
                Ok((idx, stake))
            })
            .collect::<Result<Vec<_>, EmissionError>>()?;
        sorted_indexed_stake.sort_by_key(|(_, stake)| *stake);
        sorted_indexed_stake.reverse();

        let current_block = PalletSubspace::<T>::get_current_block_number();
        self.weight_counter.read(1);
        let min_stake = pallet_subspace::MinValidatorStake::<T>::get(self.netuid);
        self.weight_counter.read(1);
        let mut validator_count = 0;
        for (idx, stake) in sorted_indexed_stake {
            if max_validators.is_some_and(|max| max <= validator_count) {
                break;
            }

            if stake < min_stake {
                continue;
            }

            self.weight_counter.read(1);
            match pallet_subspace::WeightSetAt::<T>::get(self.netuid, idx) {
                Some(weight_block) => {
                    if current_block.saturating_sub(weight_block) > 7200 {
                        continue;
                    }
                }
                None => continue,
            }

            if let Some(permit) = new_permits.get_mut(idx as usize) {
                validator_count = validator_count.saturating_add(1);
                *permit = true;
            }
        }

        log::trace!("new permis: {new_permits:?}");
        let active_stake = self.compute_active_stake(&inactive, &stake);
        log::trace!("final active stake: {active_stake:?}");

        let ConsensusAndTrust {
            consensus,
            validator_trust,
            preranks,
        } = self.compute_consensus_and_trust(&mut weights, &active_stake);

        let IncentivesAndTrust {
            incentives,
            ranks,
            trust,
        } = self.compute_incentive_and_trust(&weights, &active_stake, &preranks);

        let BondsAndDividends {
            ema_bonds,
            dividends,
        } = self
            .compute_bonds_and_dividends(&consensus, &weights, &active_stake, &incentives)
            .ok_or(EmissionError::Other("bonds storage is broken"))?;

        let Emissions {
            pruning_scores,
            validator_emissions,
            server_emissions,
            combined_emissions,
        } = self.compute_emissions(&stake, &active_stake, &incentives, &dividends);

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
            new_permits.len() == self.module_count::<usize>(),
            "unequal number of permits and modules"
        );
        ensure!(
            ema_bonds.len() == self.module_count::<usize>(),
            "unequal number of bonds and modules"
        );
        ensure!(
            self.modules.validator_permit.len() == self.module_count::<usize>(),
            "unequal number of bonds and modules"
        );

        let has_max_validators = self.params.max_allowed_validators.is_none();
        let bonds: Vec<_> = (0..self.module_count())
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

                if has_max_validators || *self.modules.validator_permit.get(i).unwrap_or(&false) {
                    // Only overwrite the intersection.
                    return Some(vec![]);
                }

                None
            })
            .collect();

        // Emission tuples ( key, server_emission, validator_emission )
        let mut result = Vec::with_capacity(self.module_count());
        self.weight_counter.read(1);
        for (module_uid, module_key) in self.modules.keys.iter().enumerate() {
            result.push((
                ModuleKey(module_key.0.clone()),
                *server_emissions.get(module_uid).unwrap_or(&0),
                *validator_emissions.get(module_uid).unwrap_or(&0),
            ));
        }

        let (emission_map, total_emitted) = self.calculate_final_emissions(result)?;
        log::debug!(
            "finished yuma for {} with distributed: {emission_map:?}",
            self.subnet_id
        );

        Ok(YumaOutput {
            subnet_id: self.subnet_id,

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

            founder_emission: self.params.founder_emission,
            emission_map,
            total_emitted,

            params: self.params,
            weights: self.weight_counter.to_weights::<T>(),
        })
    }

    fn calculate_final_emissions(
        &self,
        result: Vec<(ModuleKey<T::AccountId>, u64, u64)>,
    ) -> Result<(EmissionMap<T::AccountId>, u64), EmissionError> {
        let mut emissions: EmissionMap<T::AccountId> = Default::default();
        let mut emitted: u64 = 0;

        if self.params.founder_emission > 0 {
            emitted = emitted.saturating_add(self.params.founder_emission);
        }

        for (module_key, server_emission, mut validator_emission) in result {
            let mut increase_stake = |account_key: AccountKey<T::AccountId>, amount: u64| {
                let stake = emissions
                    .entry(module_key.clone())
                    .or_default()
                    .entry(account_key)
                    .or_default();
                *stake = stake.saturating_add(amount);

                emitted = emitted.saturating_add(amount);
            };

            if validator_emission > 0 {
                let ownership_vector =
                    PalletSubspace::<T>::get_ownership_ratios(self.subnet_id, &module_key.0);
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

            let remaining_emission = server_emission.saturating_add(validator_emission);
            if remaining_emission > 0 {
                increase_stake(AccountKey(module_key.0.clone()), remaining_emission);
            }
        }

        Ok((emissions, emitted))
    }

    fn compute_weights(&self) -> Option<WeightsVal> {
        // Access network weights row unnormalized.
        let mut weights = self.modules.weights_unencrypted.clone();
        log::trace!("  original weights: {weights:?}");

        let validator_forbids: Vec<bool> =
            self.modules.validator_permit.iter().map(|&b| !b).collect();

        if self.params.max_allowed_validators.is_some() {
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
            &self.modules.last_update,
            &self.modules.block_at_registration,
            |updated, registered| updated <= registered,
        )?;
        log::trace!("  no deregistered modules weights: {weights:?}");

        // Normalize remaining weights.
        inplace_row_normalize_sparse(&mut weights);

        log::trace!("  normalized weights: {weights:?}");

        Some(WeightsVal::unchecked_from_inner(weights))
    }

    fn compute_active_stake(&self, inactive: &[bool], stake: &StakeVal) -> ActiveStake {
        let mut active_stake = stake.as_ref().clone();
        log::trace!("  original active stake: {active_stake:?}");

        // Remove inactive stake.
        inplace_mask_vector(inactive, &mut active_stake);
        log::trace!("  no inactive active stake: {active_stake:?}");

        if self.params.max_allowed_validators.is_some() {
            // Remove non-validator stake.
            inplace_mask_vector(&self.modules.validator_forbid, &mut active_stake);
            log::trace!("  no non-validator active stake: {active_stake:?}");
        }

        // Normalize active stake.
        inplace_normalize(&mut active_stake);
        log::trace!("  normalized active stake: {active_stake:?}");

        ActiveStake::unchecked_from_inner(active_stake)
    }

    fn compute_consensus_and_trust(
        &self,
        weights: &mut WeightsVal,
        active_stake: &ActiveStake,
    ) -> ConsensusAndTrust {
        // Clip weights at majority consensus
        let consensus = weighted_median_col_sparse(
            active_stake.as_ref(),
            weights.as_ref(),
            self.module_count(),
            self.params.kappa,
        );

        log::trace!("final consensus: {consensus:?}");

        // Compute preranks: r_j = SUM(i) w_ij * s_i
        let preranks = matmul_sparse(weights.as_ref(), active_stake.as_ref(), self.module_count());
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

    fn compute_incentive_and_trust(
        &self,
        weights: &WeightsVal,
        active_stake: &ActiveStake,
        preranks: &Preranks,
    ) -> IncentivesAndTrust {
        // Compute ranks: r_j = SUM(i) w_ij * s_i.
        let ranks = matmul_sparse(weights.as_ref(), active_stake.as_ref(), self.module_count());
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

    fn calculate_ema_bonds(
        &self,
        bonds_delta: &[Vec<(u16, I32F32)>],
        bonds: &[Vec<(u16, I32F32)>],
        consensus: &[I32F32],
    ) -> Vec<Vec<(u16, I32F32)>> {
        let bonds_moving_average = I64F64::from_num(self.params.bonds_moving_average)
            .checked_div(I64F64::from_num(1_000_000))
            .unwrap_or_default();
        let default_alpha =
            I32F32::from_num(1).saturating_sub(I32F32::from_num(bonds_moving_average));

        if !self.params.use_weights_encryption {
            return mat_ema_sparse(bonds_delta, bonds, default_alpha);
        }

        let consensus_high = quantile(consensus, 0.75);
        let consensus_low = quantile(consensus, 0.25);

        if consensus_high <= consensus_low && consensus_high == 0 && consensus_low >= 0 {
            return mat_ema_sparse(bonds_delta, bonds, default_alpha);
        }
        log::trace!("Using Liquid Alpha");
        let (alpha_low, alpha_high) = self.params.alpha_values;
        log::trace!("alpha_low: {:?} alpha_high: {:?}", alpha_low, alpha_high);

        let (a, b) =
            calculate_logistic_params(alpha_high, alpha_low, consensus_high, consensus_low);
        let alpha = compute_alpha_values(consensus, a, b);
        let clamped_alpha: Vec<I32F32> =
            alpha.into_iter().map(|a| a.clamp(alpha_low, alpha_high)).collect();

        mat_ema_alpha_vec_sparse(bonds_delta, bonds, &clamped_alpha)
    }

    fn compute_bonds_and_dividends(
        &self,
        consensus: &ConsensusVal,
        weights: &WeightsVal,
        active_stake: &ActiveStake,
        incentives: &IncentivesVal,
    ) -> Option<BondsAndDividends> {
        // Access network bonds.
        let mut bonds = self.modules.bonds.clone();
        log::trace!("  original bonds: {bonds:?}");

        // Remove bonds referring to deregistered modules.
        bonds = vec_mask_sparse_matrix(
            &bonds,
            &self.modules.last_update,
            &self.modules.block_at_registration,
            |updated, registered| updated <= registered,
        )?;

        log::trace!("  no deregistered modules bonds: {bonds:?}");

        // Normalize remaining bonds: sum_i b_ij = 1.
        inplace_col_normalize_sparse(&mut bonds, self.module_count());
        log::trace!("  normalized bonds: {bonds:?}");

        // Compute bonds delta column normalized.
        let mut bonds_delta = row_hadamard_sparse(weights.as_ref(), active_stake.as_ref()); // ΔB = W◦S (outdated W masked)
        log::trace!("  original bonds delta: {bonds_delta:?}");

        // Normalize bonds delta.
        inplace_col_normalize_sparse(&mut bonds_delta, self.module_count()); // sum_i b_ij = 1
        log::trace!("  normalized bonds delta: {bonds_delta:?}");

        // Compute bonds moving average.
        let mut ema_bonds =
            Self::calculate_ema_bonds(&self, &bonds_delta, &bonds, &consensus.clone().into_inner());

        log::trace!("  original ema bonds: {ema_bonds:?}");

        // Normalize EMA bonds.
        inplace_col_normalize_sparse(&mut ema_bonds, self.module_count()); // sum_i b_ij = 1
        log::trace!("  normalized ema bonds: {ema_bonds:?}");

        // Compute dividends: d_i = SUM(j) b_ij * inc_j.
        // range: I32F32(0, 1)
        let mut dividends = matmul_transpose_sparse(&ema_bonds, incentives.as_ref());
        log::trace!("  original dividends: {dividends:?}");

        inplace_normalize(&mut dividends);
        log::trace!("  normalized dividends: {dividends:?}");

        // Column max-upscale EMA bonds for storage: max_i w_ij = 1.
        inplace_col_max_upscale_sparse(&mut ema_bonds, self.module_count());
        log::trace!("  upscaled ema bonds: {ema_bonds:?}");

        Some(BondsAndDividends {
            ema_bonds,
            dividends: DividendsVal::unchecked_from_inner(dividends),
        })
    }

    fn compute_emissions<'a>(
        &self,
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

        log::trace!("  normalized miner emission: {normalized_miner_emission:?}");
        log::trace!("  normalized validator emission: {normalized_validator_emission:?}");
        log::trace!("  normalized combined emission: {normalized_combined_emission:?}");

        // Compute rao based emission scores. range: I96F32(0, rao_emission)
        let to_be_emitted =
            I96F32::from_num::<u64>(self.params.token_emission.try_into().unwrap_or_default());
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
            server_emissions: miner_emissions,
            combined_emissions,
        }
    }
}

#[derive(DebugNoBound, Clone, Encode, Decode, TypeInfo)]
#[scale_info(skip_type_params(T))]
pub struct YumaOutput<T: Config> {
    pub subnet_id: u16,
    pub params: YumaParams<T>,

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
    pub weights: Weight,
}

impl<T: Config> YumaOutput<T> {
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

        // TODO: ensure!(
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

struct ConsensusAndTrust {
    consensus: ConsensusVal,
    validator_trust: ValidatorTrustVal,
    preranks: Preranks,
}

struct BondsAndDividends {
    ema_bonds: Vec<Vec<(u16, I32F32)>>,
    dividends: DividendsVal,
}

struct IncentivesAndTrust {
    incentives: IncentivesVal,
    ranks: RanksVal,
    trust: TrustVal,
}

struct Emissions {
    pruning_scores: PruningScoresVal,
    validator_emissions: Vec<u64>,
    server_emissions: Vec<u64>,
    combined_emissions: Vec<u64>,
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
