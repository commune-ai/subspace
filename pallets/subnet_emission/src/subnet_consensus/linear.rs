use crate::{pallet, EmissionError, Pallet};

use core::marker::PhantomData;
use pallet_subnet_emission_api::SubnetConsensus;
// use frame_support::{pallet_prelude::Weight, weights::RuntimeDbWeight};
use pallet_subspace::{
    math::*, Config, Dividends, Emission, Founder, GlobalParams, Incentive, IncentiveRatio,
    LastUpdate, Pallet as PalletSubspace, SubnetParams, Trust, TrustRatio, Vec, Weights, N,
};
// use sp_core::Get;
use sp_std::vec;
use substrate_fixed::types::{I32F32, I64F64};

// struct WeightCounter<T: Config> {
//     weight: Weight,
//     db_weight: RuntimeDbWeight,
//     _pd: PhantomData<T>,
// }

// impl<T: Config> WeightCounter<T> {
//     fn reads_writes(&mut self, reads: u64, writes: u64) {
//         self.weight = self.weight.saturating_add(self.db_weight.reads_writes(reads, writes));
//     }
// }

// impl<T: Config> Default for WeightCounter<T> {
//     fn default() -> Self {
//         Self {
//             weight: Weight::zero(),
//             db_weight: T::DbWeight::get(),
//             _pd: Default::default(),
//         }
//     }
// }

pub struct LinearEpoch<T: Config + pallet::Config> {
    module_count: u16,
    netuid: u16,
    founder_key: T::AccountId,
    founder_emission: u64,
    to_be_emitted: u64,
    current_block: u64,
    last_update: Vec<u64>,
    global_params: GlobalParams<T>,
    subnet_params: SubnetParams<T>,
    linear_netuid: u16,
    _pd: PhantomData<T>,
}

/// This function acts as the main function of the entire blockchain reward distribution.
/// It calculates the dividends, the incentive, the weights, the bonds,
/// the trust and the emission for the epoch.
impl<T: Config + pallet::Config> LinearEpoch<T> {
    pub fn new(netuid: u16, to_be_emitted: u64) -> Self {
        let founder_key = Founder::<T>::get(netuid);
        let (to_be_emitted, founder_emission) =
            PalletSubspace::<T>::calculate_founder_emission(netuid, to_be_emitted);
        let global_params = PalletSubspace::<T>::global_params();
        let subnet_params = PalletSubspace::<T>::subnet_params(netuid);

        Self {
            module_count: N::<T>::get(netuid),
            netuid,

            founder_key,
            founder_emission,
            to_be_emitted,

            current_block: PalletSubspace::<T>::get_current_block_number(),
            last_update: LastUpdate::<T>::get(netuid),

            global_params,
            subnet_params,

            linear_netuid: Pallet::<T>::get_consensus_netuid(SubnetConsensus::Linear).unwrap_or(2),

            _pd: Default::default(),
        }
    }

    /// This function acts as the main function of the entire blockchain reward distribution.
    /// It calculates the dividends, the incentive, the weights, the bonds,
    /// the trust and the emission for the epoch.
    pub fn run(self) -> Result<(), EmissionError> {
        if self.module_count == 0 {
            return Ok(());
        }

        // STAKE
        let uid_key_tuples: Vec<(u16, T::AccountId)> =
            PalletSubspace::<T>::get_uid_key_tuples(self.netuid);
        let total_stake_u64: u64 = PalletSubspace::<T>::get_total_subnet_stake(self.netuid).max(1);

        let stake_u64: Vec<u64> = uid_key_tuples
            .iter()
            .map(|(_, key)| pallet_subspace::Pallet::<T>::get_delegated_stake(key))
            .collect();

        let stake_f64: Vec<I64F64> = stake_u64
            .iter()
            .map(|x| {
                I64F64::from_num(*x)
                    .checked_div(I64F64::from_num(total_stake_u64))
                    .unwrap_or_default()
            })
            .collect();

        let mut stake: Vec<I32F32> = stake_f64.iter().map(|x| I32F32::from_num(*x)).collect();

        // Normalize stake.
        inplace_normalize(&mut stake);

        // WEIGHTS
        let weights: Vec<Vec<(u16, I32F32)>> = Self::process_weights(
            self.netuid,
            self.module_count,
            &self.global_params,
            &self.subnet_params,
            self.current_block,
            &stake_f64,
            total_stake_u64,
            self.last_update,
        );

        // INCENTIVE
        let mut incentive: Vec<I32F32> =
            Self::compute_incentive(&weights, &stake, &uid_key_tuples, self.module_count);

        // TRUST
        // trust that acts as a multiplier for the incentive
        let trust_ratio: u16 = TrustRatio::<T>::get(self.netuid);
        if trust_ratio > 0 {
            let trust_share: I32F32 = I32F32::from_num(trust_ratio)
                .checked_div(I32F32::from_num(100))
                .unwrap_or_default();
            let incentive_share: I32F32 = I32F32::from_num(1.0).saturating_sub(trust_share);
            let trust = Self::compute_trust(&weights, self.module_count);

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
                self.netuid,
                trust.iter().map(|xi| fixed_proportion_to_u16(*xi)).collect::<Vec<u16>>(),
            );
        }

        // store the incentive
        let cloned_incentive: Vec<u16> =
            incentive.iter().map(|xi| fixed_proportion_to_u16(*xi)).collect::<Vec<u16>>();
        Incentive::<T>::insert(self.netuid, cloned_incentive);

        //  BONDS
        let bonds: Vec<Vec<(u16, I32F32)>> = Self::compute_bonds_delta(&weights, &stake)?;

        // DIVIDENDS
        let (fixed_dividends, dividends) =
            Self::compute_dividends(&bonds, &incentive, &uid_key_tuples)?;
        Dividends::<T>::insert(self.netuid, fixed_dividends);

        // EMISSION
        Self::process_emission(
            &incentive,
            &dividends,
            self.to_be_emitted,
            self.netuid,
            self.founder_emission,
            &self.founder_key,
            &uid_key_tuples,
            self.linear_netuid,
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
            .unwrap_or_default();
        let dividend_ratio: I64F64 = I64F64::from_num(1.0).saturating_sub(incentive_ratio);

        let incentive_emission_float: Vec<I64F64> = incentive
            .iter()
            .map(|&x| {
                let x_float = I64F64::from_num(x);
                let token_emission_float = I64F64::from_num(token_emission);
                x_float
                    .checked_mul(token_emission_float)
                    .unwrap_or_default()
                    .checked_mul(incentive_ratio)
                    .unwrap_or_default()
            })
            .collect();

        let dividends_emission_float: Vec<I64F64> = dividends
            .iter()
            .map(|&x| {
                let x_float = I64F64::from_num(x);
                let token_emission_float = I64F64::from_num(token_emission);
                x_float
                    .checked_mul(token_emission_float)
                    .unwrap_or_default()
                    .checked_mul(dividend_ratio)
                    .unwrap_or_default()
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
        linear_netuid: u16,
    ) -> Vec<u64> {
        let n = incentive_emission_float.len();
        let mut incentive_emission: Vec<u64> =
            incentive_emission_float.iter().map(|e| e.to_num::<u64>()).collect();
        let dividends_emission: Vec<u64> =
            dividends_emission_float.iter().map(|e| e.to_num::<u64>()).collect();

        if netuid != linear_netuid {
            if let Some(founder_incentive) =
                PalletSubspace::<T>::get_uid_for_key(netuid, founder_key)
                    .and_then(|founder_uid| incentive_emission.get_mut(founder_uid as usize))
            {
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
                    PalletSubspace::<T>::get_ownership_ratios(netuid, module_key);

                let delegation_fee = PalletSubspace::<T>::get_delegation_fee(netuid, module_key);

                let total_owner_dividends_emission: u64 = owner_dividends_emission;
                for (delegate_key, delegate_ratio) in ownership_vector.iter() {
                    if delegate_key == module_key {
                        continue;
                    }

                    let dividends_from_delegate: u64 =
                        I64F64::from_num(total_owner_dividends_emission)
                            .checked_mul(*delegate_ratio)
                            .map(|result| result.to_num::<u64>())
                            .unwrap_or_default();
                    let to_module: u64 = delegation_fee.mul_floor(dividends_from_delegate);
                    let to_delegate: u64 = dividends_from_delegate.saturating_sub(to_module);
                    PalletSubspace::<T>::increase_stake(delegate_key, module_key, to_delegate);
                    emitted = emitted.saturating_add(to_delegate);
                    owner_dividends_emission = owner_dividends_emission.saturating_sub(to_delegate);
                }
            }

            let owner_emission: u64 =
                owner_emission_incentive.saturating_add(owner_dividends_emission);
            if owner_emission > 0 {
                PalletSubspace::<T>::increase_stake(module_key, module_key, owner_emission);
                emitted = emitted.saturating_add(owner_emission);
            }
        }

        if netuid == linear_netuid && founder_emission > 0 {
            // Update global treasure
            PalletSubspace::<T>::add_balance_to_account(
                &T::get_dao_treasury_address(),
                PalletSubspace::<T>::u64_to_balance(founder_emission).unwrap_or_default(),
            );
        }

        emission
    }

    #[allow(clippy::too_many_arguments)]
    fn process_emission(
        incentive: &[I32F32],
        dividends: &[I32F32],
        to_be_emitted: u64,
        netuid: u16,
        founder_emission: u64,
        founder_key: &T::AccountId,
        uid_key_tuples: &[(u16, T::AccountId)],
        linear_netuid: u16,
    ) {
        let (incentive_emission_float, dividends_emission_float) =
            Self::calculate_emission_ratios(incentive, dividends, to_be_emitted, netuid);

        let emission = Self::calculate_emissions(
            &incentive_emission_float,
            &dividends_emission_float,
            founder_emission,
            netuid,
            founder_key,
            uid_key_tuples,
            linear_netuid,
        );

        Emission::<T>::insert(netuid, emission);
    }

    fn compute_dividends(
        bonds: &[Vec<(u16, I32F32)>],
        incentive: &[I32F32],
        uid_key_tuples: &[(u16, T::AccountId)],
    ) -> Result<(Vec<u16>, Vec<I32F32>), EmissionError> {
        let n = incentive.len();
        let mut dividends: Vec<I32F32> = vec![I32F32::from_num(0.0); n];

        for (i, sparse_row) in bonds.iter().enumerate() {
            for (j, value) in sparse_row.iter() {
                let incentive_i = match incentive.get(*j as usize) {
                    Some(value) => *value,
                    None => {
                        return Err(EmissionError::Other(
                            "linear step panicked in dividends calculation",
                        ))
                    }
                };

                if let Some(target) = dividends.get_mut(i) {
                    *target = target.saturating_add(value.saturating_mul(incentive_i))
                };
            }
        }

        if dividends.iter().all(|&x| x == I32F32::from_num(0.0)) {
            for (uid_i, _) in uid_key_tuples.iter() {
                if let Some(target) = dividends.get_mut(*uid_i as usize) {
                    *target = I32F32::from_num(1.0);
                }
            }
        }

        inplace_normalize(&mut dividends);

        let fixed_dividends: Vec<u16> =
            dividends.iter().map(|xi| fixed_proportion_to_u16(*xi)).collect();

        Ok((fixed_dividends, dividends))
    }

    fn compute_bonds_delta(
        weights: &[Vec<(u16, I32F32)>],
        stake: &[I32F32],
    ) -> Result<Vec<Vec<(u16, I32F32)>>, EmissionError> {
        let n = weights.len();
        let mut bonds: Vec<Vec<(u16, I32F32)>> = weights.to_vec();
        let mut col_sum: Vec<I32F32> = vec![I32F32::from_num(0.0); n];

        for (i, sparse_row) in bonds.iter_mut().enumerate() {
            for (j, value) in sparse_row.iter_mut() {
                *value = match stake.get(i) {
                    Some(v) => value.saturating_mul(*v),
                    None => {
                        return Err(EmissionError::Other(
                            "linear step panicked in bonds calculation",
                        ))
                    }
                };
                if let Some(col_sum_j) = col_sum.get_mut(*j as usize) {
                    *col_sum_j = col_sum_j.saturating_add(*value);
                }
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

    fn compute_trust(weights: &[Vec<(u16, I32F32)>], n: u16) -> Vec<I32F32> {
        let mut trust = vec![I32F32::from_num(0.0); n as usize];
        for weights_i in weights.iter() {
            for (j, weight_ij) in weights_i.iter() {
                if let Some(trust_j) = trust.get_mut(*j as usize) {
                    if *weight_ij > 0 {
                        *trust_j = trust_j.saturating_add(I32F32::from_num(1.0));
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
            .unwrap_or_default()
    }

    #[allow(clippy::too_many_arguments)]
    fn check_weight_validity(
        weight_age: u64,
        subnet_params: &SubnetParams<T>,
        weights_i: &[(u16, u16)],
        stake_f64: &[I64F64],
        total_stake_u64: u64,
        min_weight_stake_f64: I64F64,
        n: u16,
        uid_i: u16,
    ) -> (bool, Vec<(u16, u16)>) {
        let mut valid_weights = Vec::new();

        if weight_age > subnet_params.max_weight_age
            || weights_i.len() < subnet_params.min_allowed_weights as usize
        {
            return (true, valid_weights);
        }

        for (pos, (uid_j, weight_ij)) in weights_i.iter().enumerate() {
            if (pos as u16) > subnet_params.max_allowed_weights || *uid_j >= n {
                return (true, valid_weights);
            }

            let weight_f64 = I64F64::from_num(*weight_ij)
                .checked_div(I64F64::from_num(u16::MAX))
                .unwrap_or_default();
            let weight_stake = stake_f64
                .get(uid_i as usize)
                .copied()
                .unwrap_or_default()
                .checked_mul(weight_f64)
                .unwrap_or_default()
                .checked_mul(I64F64::from_num(total_stake_u64))
                .unwrap_or_default();
            if weight_stake > min_weight_stake_f64 {
                valid_weights.push((*uid_j, *weight_ij));
            } else {
                return (true, valid_weights);
            }
        }

        (false, valid_weights)
    }

    #[allow(clippy::too_many_arguments)]
    fn process_weights(
        netuid: u16,
        n: u16,
        global_params: &GlobalParams<T>,
        subnet_params: &SubnetParams<T>,
        current_block: u64,
        stake_f64: &[I64F64],
        total_stake_u64: u64,
        last_update_vector: Vec<u64>,
    ) -> Vec<Vec<(u16, I32F32)>> {
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
            );

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

        weights
    }
}
