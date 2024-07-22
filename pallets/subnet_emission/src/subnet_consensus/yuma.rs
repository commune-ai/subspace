use crate::{EmissionError, Pallet};
use core::marker::PhantomData;
use frame_support::{ensure, DebugNoBound};
use pallet_subspace::{
    math::*, Active, Bonds, BondsMovingAverage, Config, Consensus, Dividends, Emission, Founder,
    Incentive, Kappa, Keys, LastUpdate, MaxAllowedValidators, MaxWeightAge,
    Pallet as PalletSubspace, PruningScores, Rank, StakeFrom, Trust, Uids, ValidatorPermits,
    ValidatorTrust, Vec, Weights, N,
};
use sp_std::vec;
use substrate_fixed::types::{I32F32, I64F64, I96F32};

use sp_std::{borrow::Cow, collections::btree_map::BTreeMap};

pub type EmissionMap<T> = BTreeMap<ModuleKey<T>, BTreeMap<AccountKey<T>, u64>>;

#[derive(DebugNoBound)]
pub struct YumaEpoch<T: Config> {
    /// The amount of modules on the subnet
    module_count: u16,
    /// The UID of the subnet
    netuid: u16,
    /// Consensus majority ratio, e.g. 51%.
    kappa: I32F32,

    founder_key: AccountKey<T>,
    founder_emission: u64,
    to_be_emitted: u64,

    current_block: u64,
    activity_cutoff: u64,
    last_update: Vec<u64>,
    block_at_registration: Vec<u64>,

    validator_permits: Vec<bool>,
    validator_forbids: Vec<bool>,
    max_allowed_validators: Option<u16>,

    _pd: PhantomData<T>,
}

impl<T: Config> YumaEpoch<T> {
    pub fn new(netuid: u16, to_be_emitted: u64) -> Self {
        let validator_permits = ValidatorPermits::<T>::get(netuid);
        let validator_forbids = validator_permits.iter().map(|&b| !b).collect();

        let founder_key = Founder::<T>::get(netuid);
        let (to_be_emitted, founder_emission) =
            PalletSubspace::<T>::calculate_founder_emission(netuid, to_be_emitted);

        Self {
            module_count: N::<T>::get(netuid),
            netuid,
            kappa: Pallet::<T>::get_float_kappa(),

            founder_key: AccountKey(founder_key),
            founder_emission,
            to_be_emitted,

            current_block: PalletSubspace::<T>::get_current_block_number(),
            activity_cutoff: MaxWeightAge::<T>::get(netuid),
            last_update: LastUpdate::<T>::get(netuid),
            block_at_registration: PalletSubspace::<T>::get_block_at_registration(netuid),

            validator_forbids,
            validator_permits,
            max_allowed_validators: MaxAllowedValidators::<T>::get(netuid),

            _pd: Default::default(),
        }
    }

    /// Runs the YUMA consensus calculation on the network and distributes the emissions. Returns a
    /// map of emissions distributed per module key.
    pub fn run(self) -> Result<EmissionMap<T>, EmissionError> {
        log::debug!(
            "running yuma for netuid {}, will emit {} modules and {} to founder",
            self.netuid,
            self.to_be_emitted,
            self.founder_emission
        );
        log::trace!("yuma for netuid {} parameters: {self:?}", self.netuid);

        let (inactive, active): (Vec<_>, Vec<_>) = self
            .last_update
            .iter()
            .zip(&self.block_at_registration)
            .map(|(updated, block_at_registration)| {
                let is_inactive = *updated <= *block_at_registration
                    || updated.saturating_add(self.activity_cutoff) < self.current_block;
                (is_inactive, !is_inactive)
            })
            .unzip();

        let mut weights = self
            .compute_weights()
            .ok_or(EmissionError::Other("weights storage is broken"))?;
        log::trace!("final weights: {weights:?}");

        let stake = self.compute_stake()?;
        log::trace!("final stake: {stake:?}");

        let new_permits: Vec<bool> = if let Some(max) = self.max_allowed_validators {
            is_topk(stake.as_ref(), max as usize)
        } else {
            vec![true; stake.as_ref().len()]
        };
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
            .compute_bonds_and_dividends(&weights, &active_stake, &incentives)
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

        Active::<T>::insert(self.netuid, active);
        Consensus::<T>::insert(self.netuid, consensus);
        Dividends::<T>::insert(self.netuid, dividends);
        Emission::<T>::insert(self.netuid, combined_emissions);
        Incentive::<T>::insert(self.netuid, incentives);
        PruningScores::<T>::insert(self.netuid, pruning_scores);
        Rank::<T>::insert(self.netuid, ranks);
        Trust::<T>::insert(self.netuid, trust);
        ValidatorPermits::<T>::insert(self.netuid, &new_permits);
        ValidatorTrust::<T>::insert(self.netuid, validator_trust);

        ensure!(
            new_permits.len() == self.module_count as usize,
            "unequal number of permits and modules"
        );
        ensure!(
            ema_bonds.len() == self.module_count as usize,
            "unequal number of bonds and modules"
        );
        ensure!(
            self.validator_permits.len() == self.module_count as usize,
            "unequal number of bonds and modules"
        );

        for i in 0..self.module_count as usize {
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
                Bonds::<T>::insert(self.netuid, i as u16, new_bonds_row);
            } else if self.max_allowed_validators.is_none()
                || *self.validator_permits.get(i).unwrap_or(&false)
            {
                // Only overwrite the intersection.
                let new_empty_bonds_row: Vec<(u16, u16)> = vec![];
                Bonds::<T>::insert(self.netuid, i as u16, new_empty_bonds_row);
            }
        }

        // Emission tuples ( key, server_emission, validator_emission )
        let mut result: Vec<(ModuleKey<T>, u64, u64)> = vec![];
        for (uid_i, module_key) in Keys::<T>::iter_prefix(self.netuid) {
            result.push((
                ModuleKey(module_key),
                *server_emissions.get(uid_i as usize).unwrap_or(&0),
                *validator_emissions.get(uid_i as usize).unwrap_or(&0),
            ));
        }

        let distribute_emissions = self.distribute_emissions(result);
        log::debug!(
            "finished yuma for {} with distributed: {distribute_emissions:?}",
            self.netuid
        );

        distribute_emissions
    }

    fn distribute_emissions(
        &self,
        result: Vec<(ModuleKey<T>, u64, u64)>,
    ) -> Result<EmissionMap<T>, EmissionError> {
        let mut emissions: EmissionMap<T> = Default::default();
        let mut emitted: u64 = 0;

        if self.founder_emission > 0 {
            match PalletSubspace::<T>::u64_to_balance(self.founder_emission) {
                Some(balance) => {
                    PalletSubspace::<T>::add_balance_to_account(&self.founder_key.0, balance);
                }
                None => return Err(EmissionError::BalanceConversionFailed),
            }
            emitted = emitted.saturating_add(self.founder_emission);
        }

        for (module_key, server_emission, mut validator_emission) in result {
            let mut increase_stake = |account_key: &AccountKey<T>, amount: u64| {
                PalletSubspace::<T>::increase_stake(&account_key.0, &module_key.0, amount);

                let stake = emissions
                    .entry(module_key.clone())
                    .or_default()
                    .entry(account_key.clone())
                    .or_default();
                *stake = stake.saturating_add(amount);

                emitted = emitted.saturating_add(amount);
            };

            if validator_emission > 0 {
                let ownership_vector =
                    PalletSubspace::<T>::get_ownership_ratios(self.netuid, &module_key.0);
                let delegation_fee =
                    PalletSubspace::<T>::get_delegation_fee(self.netuid, &module_key.0);

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

                    increase_stake(&AccountKey(delegate_key), to_delegate);

                    validator_emission = validator_emission
                        .checked_sub(to_delegate)
                        .ok_or("more validator emissions were done than expected")?;
                }
            }

            let remaining_emission = server_emission.saturating_add(validator_emission);
            if remaining_emission > 0 {
                increase_stake(&AccountKey(module_key.0.clone()), remaining_emission);
            }
        }

        ensure!(
            emitted <= self.founder_emission.saturating_add(self.to_be_emitted),
            EmissionError::EmittedMoreThanExpected {
                emitted,
                expected: self.founder_emission.saturating_add(self.to_be_emitted)
            }
        );

        log::trace!("emitted {emitted} tokens in total");

        Ok(emissions)
    }

    fn compute_weights(&self) -> Option<WeightsVal> {
        // Access network weights row unnormalized.
        let mut weights = Pallet::<T>::get_weights_sparse(self.netuid)?;
        log::trace!("  original weights: {weights:?}");

        if self.max_allowed_validators.is_some() {
            // Mask weights that are not from permitted validators.
            weights = mask_rows_sparse(&self.validator_forbids, &weights);
            log::trace!("  no forbidden validator weights: {weights:?}");
        }

        // Remove self-weight by masking diagonal.
        weights = mask_diag_sparse(&weights);
        log::trace!("  no self-weight weights: {weights:?}");

        // Remove weights referring to deregistered modules.
        weights = vec_mask_sparse_matrix(
            &weights,
            &self.last_update,
            &self.block_at_registration,
            |updated, registered| updated <= registered,
        )?;
        log::trace!("  no deregistered modules weights: {weights:?}");

        // Normalize remaining weights.
        inplace_row_normalize_sparse(&mut weights);
        log::trace!("  normalized weights: {weights:?}");

        Some(WeightsVal::unchecked_from_inner(weights))
    }

    fn compute_stake(&self) -> Result<StakeVal, &'static str> {
        let keys_map: BTreeMap<_, _> = Uids::<T>::iter_prefix(self.netuid).collect();
        let mut stake_map: BTreeMap<u16, I64F64> = BTreeMap::new();
        for (k1, _, value) in StakeFrom::<T>::iter() {
            if let Some(uid) = keys_map.get(&k1) {
                let stake = stake_map.entry(*uid).or_default();
                *stake = stake.saturating_add(I64F64::from_num(value));
            }
        }

        // let stake_map: BTreeMap<_, _> = StakeFrom::<T>::iter_key_prefix()
        //     .filter_map(|(k, v)| Some((keys_map.remove(&k)?, I64F64::from_num(v))))
        //     .collect();
        let mut stake: Vec<_> = stake_map.into_values().collect();

        ensure!(
            stake.len() == self.module_count as usize,
            "unequal number of stakes and modules"
        );
        log::trace!("  original stake: {stake:?}");

        inplace_normalize_64(&mut stake);
        log::trace!("  normalized stake: {stake:?}");

        Ok(StakeVal::unchecked_from_inner(vec_fixed64_to_fixed32(
            stake,
        ))) // range: I32F32(0, 1)
    }

    fn compute_active_stake(&self, inactive: &[bool], stake: &StakeVal) -> ActiveStake {
        let mut active_stake = stake.as_ref().clone();
        log::trace!("  original active stake: {active_stake:?}");

        // Remove inactive stake.
        inplace_mask_vector(inactive, &mut active_stake);
        log::trace!("  no inactive active stake: {active_stake:?}");

        if self.max_allowed_validators.is_some() {
            // Remove non-validator stake.
            inplace_mask_vector(&self.validator_forbids, &mut active_stake);
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
            self.module_count,
            self.kappa,
        );
        log::trace!("final consensus: {consensus:?}");

        // Compute preranks: r_j = SUM(i) w_ij * s_i
        let preranks = matmul_sparse(weights.as_ref(), active_stake.as_ref(), self.module_count);
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
        let ranks = matmul_sparse(weights.as_ref(), active_stake.as_ref(), self.module_count);
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

    fn compute_bonds_and_dividends(
        &self,
        weights: &WeightsVal,
        active_stake: &ActiveStake,
        incentives: &IncentivesVal,
    ) -> Option<BondsAndDividends> {
        // Access network bonds.
        let mut bonds = Pallet::<T>::get_bonds_sparse(self.netuid)?;
        log::trace!("  original bonds: {bonds:?}");

        // Remove bonds referring to deregistered modules.
        bonds = vec_mask_sparse_matrix(
            &bonds,
            &self.last_update,
            &self.block_at_registration,
            |updated, registered| updated <= registered,
        )?;
        log::trace!("  no deregistered modules bonds: {bonds:?}");

        // Normalize remaining bonds: sum_i b_ij = 1.
        inplace_col_normalize_sparse(&mut bonds, self.module_count);
        log::trace!("  normalized bonds: {bonds:?}");

        // Compute bonds delta column normalized.
        let mut bonds_delta = row_hadamard_sparse(weights.as_ref(), active_stake.as_ref()); // ΔB = W◦S (outdated W masked)
        log::trace!("  original bonds delta: {bonds_delta:?}");

        // Normalize bonds delta.
        inplace_col_normalize_sparse(&mut bonds_delta, self.module_count); // sum_i b_ij = 1
        log::trace!("  normalized bonds delta: {bonds_delta:?}");

        // Compute bonds moving average.
        let bonds_moving_average = I64F64::from_num(BondsMovingAverage::<T>::get(self.netuid))
            .checked_div(I64F64::from_num(1_000_000))
            .unwrap_or_default();
        log::trace!("  bonds moving average: {bonds_moving_average}");
        let alpha = I32F32::from_num(1).saturating_sub(I32F32::from_num(bonds_moving_average));
        let mut ema_bonds = mat_ema_sparse(&bonds_delta, &bonds, alpha);
        log::trace!("  original ema bonds: {ema_bonds:?}");

        // Normalize EMA bonds.
        inplace_col_normalize_sparse(&mut ema_bonds, self.module_count); // sum_i b_ij = 1
        log::trace!("  normalized ema bonds: {ema_bonds:?}");

        // Compute dividends: d_i = SUM(j) b_ij * inc_j.
        // range: I32F32(0, 1)
        let mut dividends = matmul_transpose_sparse(&ema_bonds, incentives.as_ref());
        log::trace!("  original dividends: {dividends:?}");

        inplace_normalize(&mut dividends);
        log::trace!("  normalized dividends: {dividends:?}");

        // Column max-upscale EMA bonds for storage: max_i w_ij = 1.
        inplace_col_max_upscale_sparse(&mut ema_bonds, self.module_count);
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
        let to_be_emitted = I96F32::from_num(self.to_be_emitted);
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

#[derive(Clone)]
pub struct ModuleKey<T: Config>(pub T::AccountId);

#[derive(Clone)]
pub struct AccountKey<T: Config>(pub T::AccountId);

macro_rules! impl_things {
    ($ty:ident) => {
        impl<T: Config> PartialEq for $ty<T> {
            fn eq(&self, other: &Self) -> bool {
                self.0 == other.0
            }
        }

        impl<T: Config> Eq for $ty<T> {}

        impl<T: Config> PartialOrd for $ty<T> {
            fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
                Some(self.cmp(other))
            }
        }

        impl<T: Config> Ord for $ty<T> {
            fn cmp(&self, other: &Self) -> scale_info::prelude::cmp::Ordering {
                self.0.cmp(&other.0)
            }
        }

        impl<T: Config> core::fmt::Debug for $ty<T> {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                f.write_fmt(format_args!("{}({:?})", stringify!($ty), self.0))
            }
        }
    };
}

impl_things!(ModuleKey);
impl_things!(AccountKey);

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

impl<T: Config> Pallet<T> {
    pub fn get_float_kappa() -> I32F32 {
        I32F32::from_num(Kappa::<T>::get())
            .checked_div(I32F32::from_num(u16::MAX))
            .unwrap_or_default()
    }

    fn get_weights_sparse(netuid: u16) -> Option<Vec<Vec<(u16, I32F32)>>> {
        let n = N::<T>::get(netuid) as usize;
        let mut weights: Vec<Vec<(u16, I32F32)>> = vec![vec![]; n];
        for (uid_i, weights_i) in
            Weights::<T>::iter_prefix(netuid).filter(|(uid, _)| *uid < n as u16)
        {
            for (uid_j, weight_ij) in weights_i.into_iter().filter(|(uid, _)| *uid < n as u16) {
                weights.get_mut(uid_i as usize)?.push((uid_j, I32F32::from_num(weight_ij)));
            }
        }
        Some(weights)
    }

    fn get_bonds_sparse(netuid: u16) -> Option<Vec<Vec<(u16, I32F32)>>> {
        let n: usize = N::<T>::get(netuid) as usize;
        let mut bonds: Vec<Vec<(u16, I32F32)>> = vec![vec![]; n];
        for (uid_i, bonds_i) in Bonds::<T>::iter_prefix(netuid).filter(|(uid, _)| *uid < n as u16) {
            for (uid_j, bonds_ij) in bonds_i.into_iter().filter(|(uid, _)| *uid < n as u16) {
                bonds.get_mut(uid_i as usize)?.push((uid_j, I32F32::from_num(bonds_ij)));
            }
        }
        Some(bonds)
    }
}
