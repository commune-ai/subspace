use super::*;
use crate::{global::BurnConfiguration, math::*};
use frame_support::{storage::with_storage_layer, weights::RuntimeDbWeight};
use sp_arithmetic::per_things::Percent;
use sp_core::Get;
use sp_std::vec;
use substrate_fixed::types::{I110F18, I32F32, I64F64};

pub mod yuma;

struct WeightCounter<T: Config> {
    weight: Weight,
    db_weight: RuntimeDbWeight,
    _pd: PhantomData<T>,
}

impl<T: Config> WeightCounter<T> {
    fn reads_writes(&mut self, reads: u64, writes: u64) {
        self.weight = self.weight.saturating_add(self.db_weight.reads_writes(reads, writes));
    }
}

impl<T: Config> Default for WeightCounter<T> {
    fn default() -> Self {
        Self {
            weight: Weight::zero(),
            db_weight: T::DbWeight::get(),
            _pd: Default::default(),
        }
    }
}

impl<T: Config> Pallet<T> {
    pub fn block_step(block_number: u64) -> Result<Weight, sp_runtime::DispatchError> {
        log::debug!("stepping block {block_number:?}");

        RegistrationsPerBlock::<T>::mutate(|val: &mut u16| *val = 0);

        let total_stake = Self::total_stake() as u128;
        let subnet_stake_threshold = SubnetStakeThreshold::<T>::get();

        let mut weight = WeightCounter::<T>::default();
        weight.reads_writes(4, 1);

        log::debug!("ticking subnets, total stake: {total_stake}, stake threshold: {subnet_stake_threshold:?}");

        for (netuid, tempo) in Tempo::<T>::iter() {
            Self::subnet_step(
                netuid,
                &mut weight,
                block_number,
                subnet_stake_threshold,
                tempo,
                total_stake,
            )?;
        }

        Ok(weight.weight)
    }

    fn subnet_step(
        netuid: u16,
        weight: &mut WeightCounter<T>,
        block_number: u64,
        subnet_stake_threshold: Percent,
        tempo: u16,
        total_stake: u128,
    ) -> Result<(), sp_runtime::DispatchError> {
        let registration_this_interval = RegistrationsThisInterval::<T>::get(netuid);
        let target_registrations_interval = TargetRegistrationsInterval::<T>::get(netuid);
        let target_registrations_per_interval = TargetRegistrationsPerInterval::<T>::get(netuid);
        weight.reads_writes(3, 0);

        Self::adjust_registration(
            netuid,
            block_number,
            registration_this_interval,
            target_registrations_interval,
            target_registrations_per_interval,
            weight,
        );

        let new_queued_emission: u64 =
            Self::calculate_network_emission(netuid, subnet_stake_threshold);
        let emission_to_drain = PendingEmission::<T>::mutate(netuid, |queued: &mut u64| {
            *queued = queued.saturating_add(new_queued_emission);
            *queued
        });
        weight.reads_writes(1, 1);
        log::trace!("subnet {netuid} total pending emission: {emission_to_drain}, increased {new_queued_emission}");

        if Self::blocks_until_next_epoch(netuid, tempo, block_number) > 0 {
            return Ok(());
        }
        log::trace!("running epoch for subnet {netuid}");

        let _ = SetWeightCallsPerEpoch::<T>::clear_prefix(netuid, u32::MAX, None);
        weight.reads_writes(0, 1);

        let has_enough_stake_for_yuma = || {
            let subnet_stake = Self::get_total_subnet_stake(netuid) as u128;

            if total_stake == 0 {
                false
            } else {
                let subnet_stake_percent = subnet_stake
                    .checked_mul(100)
                    .and_then(|x| x.checked_div(total_stake))
                    .unwrap_or(0);

                subnet_stake_threshold <= Percent::from_parts(subnet_stake_percent as u8)
            }
        };

        if netuid == 0 {
            weight.reads_writes(50, 100);
            Self::linear_epoch(netuid, emission_to_drain)?;
        } else if has_enough_stake_for_yuma() {
            weight.reads_writes(55, 100);
            let res = with_storage_layer(|| {
                let Err(err) = yuma::YumaCalc::<T>::new(netuid, emission_to_drain).run() else {
                    return Ok(());
                };

                log::error!(
                    "failed to run yuma consensus algorithm: {err:?}, skipping this block. \
{emission_to_drain} tokens will be emitted on the next epoch."
                );

                Err("yuma failed")
            });

            if res.is_err() {
                return Ok(());
            }
        }

        PendingEmission::<T>::insert(netuid, 0);

        weight.reads_writes(0, 1);

        Ok(())
    }

    /// This function acts as the main function of the entire blockchain reward distribution.
    /// It calculates the dividends, the incentive, the weights, the bonds,
    /// the trust and the emission for the epoch.
    pub fn linear_epoch(netuid: u16, token_emission: u64) -> Result<(), sp_runtime::DispatchError> {
        // get the network parameters
        let global_params = Self::global_params();
        let subnet_params = Self::subnet_params(netuid);

        // get the amount of modules
        let n: u16 = N::<T>::get(netuid);
        let current_block: u64 = Self::get_current_block_number();

        // if there are no modules, then return
        if n == 0 {
            return Ok(());
        }

        // FOUNDER DIVIDENDS
        let founder_key = Founder::<T>::get(netuid);
        let (token_emission, founder_emission) =
            Self::calculate_founder_emission(netuid, token_emission);

        // STAKE
        let uid_key_tuples: Vec<(u16, T::AccountId)> = Self::get_uid_key_tuples(netuid);
        let total_stake_u64: u64 = Self::get_total_subnet_stake(netuid).max(1);

        let stake_u64: Vec<u64> =
            uid_key_tuples.iter().map(|(_, key)| Stake::<T>::get(netuid, key)).collect();

        let stake_f64: Vec<I64F64> = stake_u64
            .iter()
            .map(|x| {
                I64F64::from_num(*x)
                    .checked_div(I64F64::from_num(total_stake_u64))
                    .unwrap_or(I64F64::from_num(0))
            })
            .collect();

        let mut stake: Vec<I32F32> = stake_f64.iter().map(|x| I32F32::from_num(*x)).collect();

        // Normalize stake.
        inplace_normalize(&mut stake);

        // WEIGHTS
        let weights: Vec<Vec<(u16, I32F32)>> = Self::process_weights(
            netuid,
            n,
            &global_params,
            &subnet_params,
            current_block,
            &stake_f64,
            total_stake_u64,
        )?;

        // INCENTIVE
        // see if this shit needs to be mut
        let mut incentive: Vec<I32F32> =
            Self::compute_incentive(&weights, &stake, &uid_key_tuples, n);

        // TRUST
        let trust_ratio: u16 = TrustRatio::<T>::get(netuid);
        if trust_ratio > 0 {
            let trust_share: I32F32 = I32F32::from_num(trust_ratio)
                .checked_div(I32F32::from_num(100))
                .unwrap_or(I32F32::from_num(0));
            let incentive_share: I32F32 = I32F32::from_num(1.0).saturating_sub(trust_share);
            let trust = Self::compute_trust(&weights, &stake, &subnet_params, n);

            incentive = incentive
                .iter()
                .zip(trust.iter())
                .map(|(inc, tru)| {
                    let incentive_part = inc.checked_mul(incentive_share).unwrap_or_default();
                    let trust_part = tru.checked_mul(trust_share).unwrap_or_default();
                    incentive_part.saturating_add(trust_part)
                })
                .collect();

            // save the trust into the trust vector
            Trust::<T>::insert(
                netuid,
                trust.iter().map(|xi| fixed_proportion_to_u16(*xi)).collect::<Vec<u16>>(),
            );
        }

        // store the incentive
        let cloned_incentive: Vec<u16> =
            incentive.iter().map(|xi| fixed_proportion_to_u16(*xi)).collect::<Vec<u16>>();
        Incentive::<T>::insert(netuid, cloned_incentive);

        //  BONDS
        let bonds: Vec<Vec<(u16, I32F32)>> = Self::compute_bonds_delta(&weights, &stake)?;

        // DIVIDENDS
        let (fixed_dividends, dividends) =
            Self::compute_dividends(&bonds, &incentive, &uid_key_tuples)?;
        Dividends::<T>::insert(netuid, fixed_dividends);

        // EMISSION
        Self::process_emission(
            &incentive,
            &dividends,
            token_emission,
            netuid,
            founder_emission,
            &founder_key,
            &uid_key_tuples,
        );

        Ok(())
    }

    fn calculate_emission_ratios(
        incentive: &[I32F32],
        dividends: &[I32F32],
        token_emission: u64,
        netuid: u16,
    ) -> (Vec<I64F64>, Vec<I64F64>) {
        let incentive_ratio: I64F64 = I64F64::from_num(IncentiveRatio::<T>::get(netuid) as u64)
            .checked_div(I64F64::from_num(100))
            .unwrap_or(I64F64::from_num(0));
        let dividend_ratio: I64F64 = I64F64::from_num(1.0).saturating_sub(incentive_ratio);

        let incentive_emission_float: Vec<I64F64> = incentive
            .iter()
            .map(|&x| {
                let x_float = I64F64::from_num(x);
                let token_emission_float = I64F64::from_num(token_emission);
                x_float
                    .checked_mul(token_emission_float)
                    .unwrap_or(I64F64::from_num(0))
                    .checked_mul(incentive_ratio)
                    .unwrap_or(I64F64::from_num(0))
            })
            .collect();

        let dividends_emission_float: Vec<I64F64> = dividends
            .iter()
            .map(|&x| {
                let x_float = I64F64::from_num(x);
                let token_emission_float = I64F64::from_num(token_emission);
                x_float
                    .checked_mul(token_emission_float)
                    .unwrap_or(I64F64::from_num(0))
                    .checked_mul(dividend_ratio)
                    .unwrap_or(I64F64::from_num(0))
            })
            .collect();

        (incentive_emission_float, dividends_emission_float)
    }

    fn calculate_emissions(
        incentive_emission_float: &[I64F64],
        dividends_emission_float: &[I64F64],
        founder_emission: u64,
        netuid: u16,
        founder_key: &T::AccountId,
        uid_key_tuples: &[(u16, T::AccountId)],
    ) -> Vec<u64> {
        let n = incentive_emission_float.len();
        let mut incentive_emission: Vec<u64> =
            incentive_emission_float.iter().map(|e| e.to_num::<u64>()).collect();
        let dividends_emission: Vec<u64> =
            dividends_emission_float.iter().map(|e| e.to_num::<u64>()).collect();

        if netuid != 0 {
            let founder_uid = Self::get_uid_for_key(netuid, founder_key);
            if let Some(founder_incentive) = incentive_emission.get_mut(founder_uid as usize) {
                *founder_incentive = founder_incentive.saturating_add(founder_emission);
            }
        }

        let mut emission: Vec<u64> = vec![0; n];
        let mut emitted = 0u64;

        for (module_uid, module_key) in uid_key_tuples.iter() {
            let owner_emission_incentive: u64 =
                *incentive_emission.get(*module_uid as usize).unwrap_or(&0);
            let mut owner_dividends_emission: u64 =
                *dividends_emission.get(*module_uid as usize).unwrap_or(&0);
            if let Some(emi) = emission.get_mut(*module_uid as usize) {
                *emi = owner_emission_incentive.saturating_add(owner_dividends_emission);
            }

            if owner_dividends_emission > 0 {
                let ownership_vector: Vec<(T::AccountId, I64F64)> =
                    Self::get_ownership_ratios(netuid, module_key);

                let delegation_fee = Self::get_delegation_fee(netuid, module_key);

                let total_owner_dividends_emission: u64 = owner_dividends_emission;
                for (delegate_key, delegate_ratio) in ownership_vector.iter() {
                    if delegate_key == module_key {
                        continue;
                    }

                    let dividends_from_delegate: u64 =
                        I64F64::from_num(total_owner_dividends_emission)
                            .checked_mul(*delegate_ratio)
                            .map(|result| result.to_num::<u64>())
                            .unwrap_or(0);
                    let to_module: u64 = delegation_fee.mul_floor(dividends_from_delegate);
                    let to_delegate: u64 = dividends_from_delegate.saturating_sub(to_module);
                    Self::increase_stake(netuid, delegate_key, module_key, to_delegate);
                    emitted = emitted.saturating_add(to_delegate);
                    owner_dividends_emission = owner_dividends_emission.saturating_sub(to_delegate);
                }
            }

            let owner_emission: u64 =
                owner_emission_incentive.saturating_add(owner_dividends_emission);
            if owner_emission > 0 {
                Self::increase_stake(netuid, module_key, module_key, owner_emission);
                emitted = emitted.saturating_add(owner_emission);
            }
        }

        let total_stake = Self::total_stake() as u128;
        let total_yuma_stake = total_stake.saturating_sub(Self::get_total_subnet_stake(0) as u128);
        let subnet_stake_threshold = SubnetStakeThreshold::<T>::get();

        if netuid == 0 && founder_emission > 0 {
            let mut founder_emission = founder_emission;

            let distribution = T::get_dao_treasury_distribution();

            if !distribution.is_zero() && total_yuma_stake > 0 {
                let to_distribute = distribution.mul_floor(founder_emission);
                founder_emission = founder_emission.saturating_sub(to_distribute);

                let stakes: BTreeMap<_, _> = TotalStake::<T>::iter()
                    .filter(|(n, _)| *n != 0)
                    .filter(|(_, s)| {
                        let total_stake_percentage = (*s as u128)
                            .checked_mul(100)
                            .unwrap_or(0)
                            .checked_div(total_stake)
                            .map(|result| Percent::from_parts(result as u8))
                            .unwrap_or_default();
                        total_stake_percentage >= subnet_stake_threshold
                    })
                    .collect();
                let total_yuma_stake = stakes.values().copied().sum::<u64>() as u128;

                for (netuid, founder_key) in Founder::<T>::iter().filter(|(n, _)| *n != 0) {
                    let Some(subnet_stake) = stakes.get(&netuid) else {
                        continue;
                    };
                    let yuma_stake_percentage = Percent::from_parts(
                        (*subnet_stake as u128)
                            .checked_mul(100)
                            .unwrap_or(0)
                            .checked_div(total_yuma_stake)
                            .unwrap_or(0) as u8,
                    );
                    let founder_distribution = yuma_stake_percentage.mul_floor(to_distribute);
                    Self::add_balance_to_account(
                        &founder_key,
                        Self::u64_to_balance(founder_distribution).unwrap_or_default(),
                    );
                }
            }

            // Update global treasure
            Self::add_balance_to_account(
                &T::get_dao_treasury_address(),
                Self::u64_to_balance(founder_emission).unwrap_or_default(),
            );
        }

        emission
    }

    fn process_emission(
        incentive: &[I32F32],
        dividends: &[I32F32],
        token_emission: u64,
        netuid: u16,
        founder_emission: u64,
        founder_key: &T::AccountId,
        uid_key_tuples: &[(u16, T::AccountId)],
    ) {
        let (incentive_emission_float, dividends_emission_float) =
            Self::calculate_emission_ratios(incentive, dividends, token_emission, netuid);

        let emission = Self::calculate_emissions(
            &incentive_emission_float,
            &dividends_emission_float,
            founder_emission,
            netuid,
            founder_key,
            uid_key_tuples,
        );

        Emission::<T>::insert(netuid, emission);
    }

    // TODO: disable this later, this function has proven to be correct
    fn compute_dividends(
        bonds: &[Vec<(u16, I32F32)>],
        incentive: &[I32F32],
        uid_key_tuples: &[(u16, T::AccountId)],
    ) -> Result<(Vec<u16>, Vec<I32F32>), sp_runtime::DispatchError> {
        let n = incentive.len();
        let mut dividends: Vec<I32F32> = vec![I32F32::from_num(0.0); n];

        for (i, sparse_row) in bonds.iter().enumerate() {
            for (j, value) in sparse_row.iter() {
                let dividends_i = match dividends.get(i) {
                    Some(value) => *value,
                    None => Err(Error::<T>::ExtrinsicPanicked)?,
                };
                let incentive_i = match dividends.get(i) {
                    Some(value) => *value,
                    None => Err(Error::<T>::ExtrinsicPanicked)?,
                };

                dividends.insert(
                    *j as usize,
                    dividends_i.saturating_add(value.saturating_mul(incentive_i)),
                );
            }
        }

        if dividends.iter().all(|&x| x == I32F32::from_num(0.0)) {
            for (uid_i, _) in uid_key_tuples.iter() {
                dividends.insert(*uid_i as usize, I32F32::from_num(1.0));
            }
        }

        inplace_normalize(&mut dividends);

        let fixed_dividends: Vec<u16> =
            dividends.iter().map(|xi| fixed_proportion_to_u16(*xi)).collect();

        Ok((fixed_dividends, dividends))
    }

    // Disable this later, this function has proven to be correct
    fn compute_bonds_delta(
        weights: &[Vec<(u16, I32F32)>],
        stake: &[I32F32],
    ) -> Result<Vec<Vec<(u16, I32F32)>>, sp_runtime::DispatchError> {
        let n = weights.len();
        let mut bonds: Vec<Vec<(u16, I32F32)>> = weights.to_vec();
        let mut col_sum: Vec<I32F32> = vec![I32F32::from_num(0.0); n];

        for (i, sparse_row) in bonds.iter_mut().enumerate() {
            for (j, value) in sparse_row.iter_mut() {
                *value = match stake.get(i) {
                    Some(v) => *v,
                    None => Err(Error::<T>::ExtrinsicPanicked)?,
                };
                col_sum.insert(*j as usize, *value);
            }
        }

        for sparse_row in bonds.iter_mut() {
            for (j, value) in sparse_row.iter_mut() {
                let zero = I32F32::from_num(0.0);
                let i = col_sum.get(*j as usize).unwrap_or(&zero);
                if i > &I32F32::from_num(0.0) {
                    *value = value.saturating_div(*i);
                }
            }
        }

        Ok(bonds)
    }

    fn compute_trust(
        weights: &[Vec<(u16, I32F32)>],
        stake: &[I32F32],
        subnet_params: &SubnetParams<T>,
        n: u16,
    ) -> Vec<I32F32> {
        let mut trust = vec![I32F32::from_num(0.0); n as usize];
        for (i, weights_i) in weights.iter().enumerate() {
            for (j, weight_ij) in weights_i.iter() {
                if let Some(stake_i) = stake.get(i) {
                    if let Some(trust_j) = trust.get_mut(*j as usize) {
                        if *weight_ij > 0 && *stake_i > I32F32::from_num(subnet_params.min_stake) {
                            *trust_j = trust_j.saturating_add(I32F32::from_num(1.0));
                        }
                    }
                }
            }
        }
        inplace_normalize(&mut trust);
        trust
    }

    fn compute_incentive(
        weights: &[Vec<(u16, I32F32)>],
        stake: &[I32F32],
        uid_key_tuples: &[(u16, T::AccountId)],
        n: u16,
    ) -> Vec<I32F32> {
        let mut incentive: Vec<I32F32> = vec![I32F32::from_num(0.0); n as usize];

        for (i, sparse_row) in weights.iter().enumerate() {
            let zero = I32F32::from_num(0.0);
            let stake_i = stake.get(i).unwrap_or(&zero);
            for (j, value) in sparse_row.iter() {
                if let Some(incentive_j) = incentive.get_mut(*j as usize) {
                    let result = stake_i.checked_mul(*value);
                    if let Some(product) = result {
                        *incentive_j = incentive_j.saturating_add(product)
                    }
                }
            }
        }

        if is_zero(&incentive) {
            for (uid_i, _key) in uid_key_tuples.iter() {
                if let Some(value) = incentive.get_mut(*uid_i as usize) {
                    *value = I32F32::from_num(1.0);
                }
            }
        }

        inplace_normalize(&mut incentive);
        incentive
    }

    fn get_current_weight_age(last_update_vector: &[u64], current_block: u64, uid_i: u16) -> u64 {
        last_update_vector
            .get(uid_i as usize)
            .copied()
            .map(|last_update| current_block.saturating_sub(last_update))
            .unwrap_or(0)
    }

    fn check_weight_validity(
        weight_age: u64,
        subnet_params: &SubnetParams<T>,
        weights_i: &[(u16, u16)],
        stake_f64: &[I64F64],
        total_stake_u64: u64,
        min_weight_stake_f64: I64F64,
        n: u16,
        uid_i: u16,
    ) -> Result<(bool, Vec<(u16, u16)>), sp_runtime::DispatchError> {
        let mut valid_weights = Vec::new();

        if weight_age > subnet_params.max_weight_age
            || weights_i.len() < subnet_params.min_allowed_weights as usize
        {
            return Ok((true, valid_weights));
        }

        for (pos, (uid_j, weight_ij)) in weights_i.iter().enumerate() {
            if (pos as u16) > subnet_params.max_allowed_weights || *uid_j >= n {
                return Ok((true, valid_weights));
            }

            let weight_f64 = I64F64::from_num(*weight_ij)
                .checked_div(I64F64::from_num(u16::MAX))
                .unwrap_or(I64F64::from_num(0));
            let weight_stake = stake_f64
                .get(uid_i as usize)
                .copied()
                .unwrap_or(I64F64::from_num(0))
                .checked_mul(weight_f64)
                .unwrap_or(I64F64::from_num(0))
                .checked_mul(I64F64::from_num(total_stake_u64))
                .unwrap_or(I64F64::from_num(0));
            if weight_stake > min_weight_stake_f64 {
                valid_weights.push((*uid_j, *weight_ij));
            } else {
                return Ok((true, valid_weights));
            }
        }

        Ok((false, valid_weights))
    }

    fn process_weights(
        netuid: u16,
        n: u16,
        global_params: &GlobalParams<T>,
        subnet_params: &SubnetParams<T>,
        current_block: u64,
        stake_f64: &[I64F64],
        total_stake_u64: u64,
    ) -> Result<Vec<Vec<(u16, I32F32)>>, sp_runtime::DispatchError> {
        let last_update_vector = LastUpdate::<T>::get(netuid);
        let min_weight_stake_f64 = I64F64::from_num(global_params.min_weight_stake);
        let mut weights: Vec<Vec<(u16, u16)>> = vec![vec![]; n as usize];

        for (uid_i, weights_i) in Weights::<T>::iter_prefix(netuid) {
            let weight_age =
                Self::get_current_weight_age(&last_update_vector, current_block, uid_i);
            let (weight_changed, valid_weights) = Self::check_weight_validity(
                weight_age,
                subnet_params,
                &weights_i,
                stake_f64,
                total_stake_u64,
                min_weight_stake_f64,
                n,
                uid_i,
            )?;

            let Some(weights) = weights.get_mut(uid_i as usize) else {
                continue;
            };
            *weights = valid_weights;

            if weight_changed {
                Weights::<T>::insert(netuid, uid_i, weights.clone());
            }
        }

        let mut weights: Vec<Vec<(u16, I32F32)>> = weights
            .iter()
            .map(|x| {
                x.iter().map(|(uid, weight)| (*uid, u16_proportion_to_fixed(*weight))).collect()
            })
            .collect();

        weights = mask_diag_sparse(&weights);
        inplace_row_normalize_sparse(&mut weights);

        Ok(weights)
    }

    fn calculate_founder_emission(netuid: u16, mut token_emission: u64) -> (u64, u64) {
        let founder_share: u16 = FounderShare::<T>::get(netuid).min(100);
        if founder_share == 0u16 {
            return (token_emission, 0);
        }

        let founder_emission_ratio: I64F64 = I64F64::from_num(founder_share.min(100))
            .checked_div(I64F64::from_num(100))
            .unwrap_or(I64F64::from_num(0));

        let founder_emission = founder_emission_ratio
            .checked_mul(I64F64::from_num(token_emission))
            .map(|result| result.to_num::<u64>())
            .unwrap_or(0);

        token_emission = token_emission.saturating_sub(founder_emission);

        (token_emission, founder_emission)
    }

    pub fn get_block_at_registration(netuid: u16) -> Vec<u64> {
        let n = N::<T>::get(netuid) as usize;
        let mut block_at_registration: Vec<u64> = vec![0; n];

        for (module_uid, block) in block_at_registration.iter_mut().enumerate() {
            let module_uid = module_uid as u16;

            if Keys::<T>::contains_key(netuid, module_uid) {
                *block = RegistrationBlock::<T>::get(netuid, module_uid);
            }
        }

        block_at_registration
    }

    pub fn blocks_until_next_epoch(netuid: u16, tempo: u16, block_number: u64) -> u64 {
        // Return 1000 on fail to prevent rewards from being distributed
        block_number
            .saturating_add(netuid as u64)
            .checked_rem(tempo as u64)
            .unwrap_or(1000)
    }

    pub fn get_ownership_ratios(
        netuid: u16,
        module_key: &T::AccountId,
    ) -> Vec<(T::AccountId, I64F64)> {
        let stake_from_vector = Self::get_stake_from_vector(netuid, module_key);
        let _uid = Self::get_uid_for_key(netuid, module_key);
        let mut total_stake_from: I64F64 = I64F64::from_num(0);

        let mut ownership_vector: Vec<(T::AccountId, I64F64)> = Vec::new();

        for (k, v) in stake_from_vector.clone().into_iter() {
            let ownership = I64F64::from_num(v);
            ownership_vector.push((k.clone(), ownership));
            total_stake_from = total_stake_from.saturating_add(ownership);
        }

        // add the module itself, if it has stake of its own
        if total_stake_from == I64F64::from_num(0) {
            ownership_vector.push((module_key.clone(), I64F64::from_num(0)));
        } else {
            ownership_vector = ownership_vector
                .into_iter()
                .map(|(k, v)| {
                    (
                        k,
                        v.checked_div(total_stake_from).unwrap_or(I64F64::from_num(0)),
                    )
                })
                .collect();
        }

        ownership_vector
    }

    fn adjust_registration(
        netuid: u16,
        block_number: u64,
        registrations_this_interval: u16,
        target_registrations_interval: u16,
        target_registrations_per_interval: u16,
        weight: &mut WeightCounter<T>,
    ) {
        let reached_interval = block_number
            .checked_rem(target_registrations_interval as u64)
            .is_some_and(|r| r == 0);
        if !reached_interval {
            return;
        };

        let current_burn = Burn::<T>::get(netuid);

        let adjusted_burn = Self::adjust_burn(
            netuid,
            current_burn,
            registrations_this_interval,
            target_registrations_per_interval,
        );

        Burn::<T>::insert(netuid, adjusted_burn);

        // reset the registrations
        RegistrationsThisInterval::<T>::insert(netuid, 0);

        weight.reads_writes(3, 2);
    }

    fn adjust_burn(
        netuid: u16,
        current_burn: u64,
        registrations_this_interval: u16,
        target_registrations_per_interval: u16,
    ) -> u64 {
        let updated_burn: I110F18 = I110F18::from_num(current_burn)
            .checked_mul(I110F18::from_num(
                registrations_this_interval.saturating_add(target_registrations_per_interval),
            ))
            .unwrap_or_default()
            .checked_div(I110F18::from_num(
                target_registrations_per_interval.saturating_add(target_registrations_per_interval),
            ))
            .unwrap_or_default();
        let adjustment_alpha = AdjustmentAlpha::<T>::get(netuid);
        let BurnConfiguration {
            min_burn, max_burn, ..
        } = BurnConfig::<T>::get();
        let alpha: I110F18 = I110F18::from_num(adjustment_alpha)
            .checked_div(I110F18::from_num(u64::MAX))
            .unwrap_or_else(|| I110F18::from_num(0));
        let next_value: I110F18 = alpha
            .checked_mul(I110F18::from_num(current_burn))
            .unwrap_or_else(|| I110F18::from_num(0))
            .saturating_add(
                I110F18::from_num(1.0)
                    .saturating_sub(alpha)
                    .checked_mul(updated_burn)
                    .unwrap_or_else(|| I110F18::from_num(0)),
            );
        if next_value >= I110F18::from_num(max_burn) {
            max_burn
        } else if next_value <= I110F18::from_num(min_burn) {
            min_burn
        } else {
            next_value.to_num::<u64>()
        }
    }

    // gets the overall stake value for a given account_id,
    // if netuid is present only the specific subnet will be used
    pub fn get_account_stake(account_id: &T::AccountId, netuid: Option<u16>) -> u64 {
        match netuid {
            Some(specific_netuid) => {
                StakeTo::<T>::get(specific_netuid, account_id).into_values().sum()
            }
            None => N::<T>::iter_keys()
                .filter_map(|netuid| StakeTo::<T>::try_get(netuid, account_id).ok())
                .flat_map(|entries| entries.into_values())
                .sum(),
        }
    }

    pub(crate) fn deregister_not_whitelisted_modules(mut remaining: Weight) -> Weight {
        use crate::weights::WeightInfo;

        const MAX_MODULES: usize = 5;

        let db_weight = T::DbWeight::get();

        let mut weight = db_weight.reads(2);

        let find_id_weight = db_weight.reads(1);
        let deregister_weight = crate::weights::SubstrateWeight::<T>::deregister();

        if !remaining
            .all_gte(weight.saturating_add(find_id_weight).saturating_add(deregister_weight))
        {
            log::info!("not enough weight remaining: {remaining:?}");
            return Weight::zero();
        }

        let s0_keys: BTreeSet<_> = Keys::<T>::iter_prefix_values(0).collect();
        let whitelisted = T::whitelisted_keys();

        let not_whitelisted = s0_keys.difference(&whitelisted);

        remaining = remaining.saturating_sub(weight);

        for not_whitelisted in not_whitelisted.take(MAX_MODULES) {
            log::info!("deregistering module {not_whitelisted:?}");

            // we'll need at least to read outbound lane state, kill a message and update lane state
            if !remaining.all_gte(find_id_weight.saturating_add(deregister_weight)) {
                log::info!("not enough weight remaining: {remaining:?}");
                break;
            }

            let uid = Uids::<T>::get(0, not_whitelisted);
            weight = weight.saturating_add(find_id_weight);
            remaining = remaining.saturating_sub(find_id_weight);

            if let Some(uid) = uid {
                let Err(err) = with_storage_layer(|| Self::remove_module(0, uid)) else {
                    weight = weight.saturating_add(deregister_weight);
                    remaining = remaining.saturating_sub(deregister_weight);
                    continue;
                };

                log::error!("failed to deregister module {uid} due to: {err:?}");
            }
        }

        weight
    }
}
