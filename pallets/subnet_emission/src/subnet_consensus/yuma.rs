use crate::EmissionError;
use core::marker::PhantomData;
use frame_support::{ensure, weights::Weight, DebugNoBound};
use pallet_subspace::{math::*, Config, Pallet as PalletSubspace};
use sp_std::vec;
use substrate_fixed::types::{I32F32, I64F64, I96F32};

use sp_std::{borrow::Cow, collections::btree_map::BTreeMap, vec::Vec};

use super::WeightCounter;

pub type EmissionMap<T> = BTreeMap<ModuleKey<T>, BTreeMap<AccountKey<T>, u64>>;

pub mod params {
    use sp_std::collections::btree_map::BTreeMap;

    use super::{AccountKey, ModuleKey};

    use frame_support::DebugNoBound;
    use pallet_subspace::{
        math::*, Bonds, BondsMovingAverage, Config, Founder, Kappa, Keys, LastUpdate,
        MaxAllowedValidators, MaxWeightAge, Pallet as PalletSubspace, ValidatorPermits, Vec,
    };
    use substrate_fixed::types::{I32F32, I64F64};

    // StorageDoubleMap<SubnetId, TempoIndex = epoch block number, YumaParams>;

    #[derive(DebugNoBound)]
    pub struct YumaParams<T: Config> {
        pub subnet_id: u16,
        pub token_emission: u64,

        pub modules: BTreeMap<ModuleKey<T>, ModuleParams>,
        pub kappa: I32F32,

        pub founder_key: AccountKey<T>,
        pub founder_emission: u64,

        pub current_block: u64,
        pub activity_cutoff: u64,
        pub max_allowed_validators: Option<u16>,
        pub bonds_moving_average: u64,
    }

    #[derive(DebugNoBound)]
    pub struct ModuleParams {
        pub uid: u16,
        pub last_update: u64,
        pub block_at_registration: u64,
        pub validator_permit: bool,
        pub stake: I32F32,
        pub bonds: Vec<(u16, u16)>,
        pub weight_unencrypted_hash: Vec<u8>,
        pub weight_encrypted: Vec<u8>,
    }

    #[derive(DebugNoBound)]
    pub(super) struct FlattenedModules<T: Config> {
        pub keys: Vec<ModuleKey<T>>,
        pub last_update: Vec<u64>,
        pub block_at_registration: Vec<u64>,
        pub validator_permit: Vec<bool>,
        pub validator_forbid: Vec<bool>,
        pub stake: Vec<I32F32>,
        pub bonds: Vec<Vec<(u16, I32F32)>>,
        pub weight_unencrypted_hash: Vec<Vec<u8>>,
        pub weight_encrypted: Vec<Vec<u8>>,
    }

    impl<T: Config> From<BTreeMap<ModuleKey<T>, ModuleParams>> for FlattenedModules<T> {
        fn from(value: BTreeMap<ModuleKey<T>, ModuleParams>) -> Self {
            let mut modules = FlattenedModules {
                keys: Default::default(),
                last_update: Default::default(),
                block_at_registration: Default::default(),
                validator_permit: Default::default(),
                validator_forbid: Default::default(),
                stake: Default::default(),
                bonds: Default::default(),
                weight_unencrypted_hash: Default::default(),
                weight_encrypted: Default::default(),
            };

            for (key, module) in value {
                modules.keys.push(key);
                modules.last_update.push(module.last_update);
                modules.block_at_registration.push(module.block_at_registration);
                modules.validator_permit.push(module.validator_permit);
                modules.validator_forbid.push(!module.validator_permit);
                modules.stake.push(module.stake);
                modules.bonds.push(
                    module.bonds.into_iter().map(|(k, m)| (k, I32F32::from_num(m))).collect(),
                );
                modules.weight_unencrypted_hash.push(module.weight_unencrypted_hash);
                modules.weight_encrypted.push(module.weight_encrypted);
            }

            modules
        }
    }

    impl<T: Config> YumaParams<T> {
        pub fn new(subnet_id: u16, token_emission: u64) -> Result<Self, &'static str> {
            let uids: BTreeMap<_, _> = Keys::<T>::iter_prefix(subnet_id).collect();

            let stake = Self::compute_stake(&uids);
            let bonds = Self::compute_bonds(subnet_id, &uids);

            let last_update = LastUpdate::<T>::get(subnet_id);
            let block_at_registration = PalletSubspace::<T>::get_block_at_registration(subnet_id);
            let validator_permits = ValidatorPermits::<T>::get(subnet_id);

            let modules = uids
                .into_iter()
                .zip(stake)
                .zip(bonds)
                .map(|(((uid, key), stake), bonds)| {
                    let uid = uid as usize;
                    let last_update =
                        last_update.get(uid).copied().ok_or("LastUpdate storage is broken")?;
                    let block_at_registration = block_at_registration
                        .get(uid)
                        .copied()
                        .ok_or("RegistrationBlock storage is broken")?;
                    let validator_permit = validator_permits
                        .get(uid)
                        .copied()
                        .ok_or("ValidatorPermits storage is broken")?;

                    let module = ModuleParams {
                        uid: uid as u16,
                        last_update,
                        block_at_registration,
                        validator_permit,
                        stake,
                        bonds,
                        // TODO: implement weights
                        weight_unencrypted_hash: Default::default(),
                        // TODO: implement weights
                        weight_encrypted: Default::default(),
                    };

                    Result::<_, &'static str>::Ok((ModuleKey(key), module))
                })
                .collect::<Result<_, _>>()?;

            let founder_key = AccountKey(Founder::<T>::get(subnet_id));
            let (token_emission, founder_emission) =
                PalletSubspace::<T>::calculate_founder_emission(subnet_id, token_emission);

            Ok(Self {
                subnet_id,
                token_emission,

                modules,

                kappa: I32F32::from_num(Kappa::<T>::get())
                    .checked_div(I32F32::from_num(u16::MAX))
                    .unwrap_or_default(),
                founder_key,
                founder_emission,

                current_block: PalletSubspace::<T>::get_current_block_number(),
                activity_cutoff: MaxWeightAge::<T>::get(subnet_id),
                max_allowed_validators: MaxAllowedValidators::<T>::get(subnet_id),
                bonds_moving_average: BondsMovingAverage::<T>::get(subnet_id),
            })
        }

        fn compute_stake(uids: &BTreeMap<u16, T::AccountId>) -> Vec<I32F32> {
            // BTreeMap provides natural order, so iterating and collecting
            // will result in a vector with the same order as the uid map.
            let mut stake: Vec<_> = uids
                .values()
                .map(PalletSubspace::<T>::get_delegated_stake)
                .map(I64F64::from_num)
                .collect();
            log::trace!(target: "stake", "original: {stake:?}");

            inplace_normalize_64(&mut stake);
            log::trace!(target: "stake", "normalized: {stake:?}");

            vec_fixed64_to_fixed32(stake)
        }

        fn compute_bonds(
            subnet_id: u16,
            uids: &BTreeMap<u16, T::AccountId>,
        ) -> Vec<Vec<(u16, u16)>> {
            let mut bonds: BTreeMap<_, _> = Bonds::<T>::iter_prefix(subnet_id).collect();
            // BTreeMap provides natural order, so iterating and collecting
            // will result in a vector with the same order as the uid map.
            uids.keys().map(|uid| bonds.remove(uid).unwrap_or_default()).collect()
        }
    }
}

#[derive(DebugNoBound)]
pub struct YumaEpoch<T: Config> {
    /// The UID of the subnet
    subnet_id: u16,

    params: params::YumaParams<T>,
    modules: params::FlattenedModules<T>,

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
    pub fn run(mut self) -> Result<(EmissionMap<T>, Weight), EmissionError> {
        log::debug!(
            "running yuma for subnet_id {}, will emit {} modules and {} to founder",
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

        let mut weights = WeightsVal::default();

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

        pallet_subspace::Active::<T>::insert(self.subnet_id, active);
        pallet_subspace::Consensus::<T>::insert(self.subnet_id, consensus);
        pallet_subspace::Dividends::<T>::insert(self.subnet_id, dividends);
        pallet_subspace::Emission::<T>::insert(self.subnet_id, combined_emissions);
        pallet_subspace::Incentive::<T>::insert(self.subnet_id, incentives);
        pallet_subspace::PruningScores::<T>::insert(self.subnet_id, pruning_scores);
        pallet_subspace::Rank::<T>::insert(self.subnet_id, ranks);
        pallet_subspace::Trust::<T>::insert(self.subnet_id, trust);
        pallet_subspace::ValidatorPermits::<T>::insert(self.subnet_id, &new_permits);
        pallet_subspace::ValidatorTrust::<T>::insert(self.subnet_id, validator_trust);
        self.weight_counter.wrote(10);

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

        for i in 0..self.module_count() {
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
                pallet_subspace::Bonds::<T>::insert(self.subnet_id, i as u16, new_bonds_row);
                self.weight_counter.wrote(1);
            } else if self.params.max_allowed_validators.is_none()
                || *self.modules.validator_permit.get(i).unwrap_or(&false)
            {
                // Only overwrite the intersection.
                let new_empty_bonds_row: Vec<(u16, u16)> = vec![];
                pallet_subspace::Bonds::<T>::insert(self.subnet_id, i as u16, new_empty_bonds_row);
                self.weight_counter.wrote(1);
            }
        }

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

        let distribute_emissions = self.distribute_emissions(result);
        log::debug!(
            "finished yuma for {} with distributed: {distribute_emissions:?}",
            self.subnet_id
        );

        Ok((distribute_emissions?, self.weight_counter.to_weights::<T>()))
    }

    fn distribute_emissions(
        &self,
        result: Vec<(ModuleKey<T>, u64, u64)>,
    ) -> Result<EmissionMap<T>, EmissionError> {
        let mut emissions: EmissionMap<T> = Default::default();
        let mut emitted: u64 = 0;

        if self.params.founder_emission > 0 {
            match PalletSubspace::<T>::u64_to_balance(self.params.founder_emission) {
                Some(balance) => {
                    PalletSubspace::<T>::add_balance_to_account(
                        &self.params.founder_key.0,
                        balance,
                    );
                }
                None => return Err(EmissionError::BalanceConversionFailed),
            }
            emitted = emitted.saturating_add(self.params.founder_emission);
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
            emitted <= self.params.founder_emission.saturating_add(self.params.token_emission),
            EmissionError::EmittedMoreThanExpected {
                emitted,
                expected: self.params.founder_emission.saturating_add(self.params.token_emission)
            }
        );

        log::trace!("emitted {emitted} tokens in total");

        Ok(emissions)
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

    fn compute_bonds_and_dividends(
        &self,
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
        let bonds_moving_average = I64F64::from_num(self.params.bonds_moving_average)
            .checked_div(I64F64::from_num(1_000_000))
            .unwrap_or_default();
        log::trace!("  bonds moving average: {bonds_moving_average}");
        let alpha = I32F32::from_num(1).saturating_sub(I32F32::from_num(bonds_moving_average));
        let mut ema_bonds = mat_ema_sparse(&bonds_delta, &bonds, alpha);
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
        let to_be_emitted = I96F32::from_num(self.params.token_emission);
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
