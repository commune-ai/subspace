// This subnet pricing mechanism is well known from bittensor
// Commune uses a custom implemenentation:
// This version, makes participation more acessible, while also allowing setting decreasing subnet
// weights.

use core::marker::PhantomData;

use frame_system::Config;
use substrate_fixed::transcendental::exp;

use sp_std::{vec, vec::Vec};

use crate::PricedSubnets;
use substrate_fixed::types::{I32F32, I64F64};

pub struct RootPricing<T: Config + pallet_subspace::Config> {
    to_be_emitted: u64,
    _pd: PhantomData<T>,
}

impl<T: Config + pallet_subspace::Config> RootPricing<T> {
    pub fn new(to_be_emitted: u64) -> Self {
        Self {
            to_be_emitted,
            _pd: PhantomData,
        }
    }

    pub fn run(self) -> Result<PricedSubnets, sp_runtime::DispatchError> {
        let num_root_validators = pallet_subspace::ValidatorPermits::<T>::get(0)
            .into_iter()
            .filter(|b| *b)
            .count();
        if num_root_validators == 0 {
            return Err("Rootnet has no validators.".into());
        }

        let subnet_ids = RootPricing::<T>::get_subnet_ids();
        let num_subnet_ids = subnet_ids.len();
        if num_subnet_ids == 0 {
            return Err("No networks to validate.".into());
        }

        let emission = I64F64::from_num(self.to_be_emitted);

        let mut keys: Vec<(u16, T::AccountId)> = vec![];
        for (uid_i, key) in pallet_subspace::Keys::<T>::iter_prefix(0) {
            keys.push((uid_i, key));
        }

        let mut stake_i64: Vec<I64F64> = vec![I64F64::from_num(0.0); num_root_validators];
        for ((_, key), stake) in keys.iter().zip(&mut stake_i64) {
            *stake = I64F64::from_num(pallet_subspace::Stake::<T>::get(key));
        }
        pallet_subspace::math::inplace_normalize_64(&mut stake_i64);

        let mut weights: Vec<Vec<I64F64>> = RootPricing::<T>::get_root_weights();
        pallet_subspace::math::inplace_row_normalize_64(&mut weights);

        let ranks: Vec<I64F64> = pallet_subspace::math::matmul_64(&weights, &stake_i64);

        let total_networks = num_subnet_ids;
        let mut trust = vec![I64F64::from_num(0); total_networks];
        let mut total_stake: I64F64 = I64F64::from_num(0);
        for (weights, key_stake) in weights.iter().zip(stake_i64) {
            total_stake = total_stake.checked_add(key_stake).ok_or(
                sp_runtime::DispatchError::Other("Overflow occurred during stake addition"),
            )?;
            for (weight, trust_score) in weights.iter().zip(&mut trust) {
                if *weight > 0 {
                    *trust_score = trust_score.checked_add(key_stake).unwrap_or(*trust_score);
                }
            }
        }

        if total_stake == 0 {
            return Err("No stake on network".into());
        }

        for trust_score in trust.iter_mut() {
            if let Some(quotient) = trust_score.checked_div(total_stake) {
                *trust_score = quotient;
            }
        }

        let one = I64F64::from_num(1);
        let mut consensus = vec![I64F64::from_num(0); total_networks];
        for (trust_score, consensus_i) in trust.iter_mut().zip(&mut consensus) {
            let float_kappa = I32F32::from_num(pallet_subspace::Kappa::<T>::get())
                .checked_div(I32F32::from_num(u16::MAX))
                .unwrap_or_else(|| I32F32::from_num(0));

            let shifted_trust = trust_score
                .checked_sub(I64F64::from_num(float_kappa))
                .unwrap_or_else(|| I64F64::from_num(0));
            let temperatured_trust = shifted_trust
                .checked_mul(I64F64::from_num(pallet_subspace::Rho::<T>::get()))
                .unwrap_or(I64F64::from_num(0));
            let neg_trust = temperatured_trust
                .checked_neg()
                .ok_or(sp_runtime::DispatchError::Other("Negation failed"))?;

            let exponentiated_trust: I64F64 = exp(neg_trust).map_err(|_| {
                sp_runtime::DispatchError::Other("Failed to calculate exponentiated trust")
            })?;
            *consensus_i = one.checked_div(one.saturating_add(exponentiated_trust)).unwrap_or(one);
        }

        let mut weighted_emission = vec![I64F64::from_num(0); total_networks];
        for ((emission, consensus_i), rank) in
            weighted_emission.iter_mut().zip(&consensus).zip(&ranks)
        {
            *emission = consensus_i.saturating_mul(*rank);
        }
        pallet_subspace::math::inplace_normalize_64(&mut weighted_emission);

        let emission_as_com: Vec<I64F64> =
            weighted_emission.iter().map(|v: &I64F64| v.saturating_mul(emission)).collect();

        let emission_u64: Vec<u64> = pallet_subspace::math::vec_fixed64_to_u64(emission_as_com);

        let mut priced_subnets = PricedSubnets::new();
        emission_u64.into_iter().enumerate().for_each(|(index, emission)| {
            priced_subnets.insert(*subnet_ids.get(index).unwrap(), emission);
        });

        Ok(priced_subnets)
    }

    fn get_root_weights() -> Vec<Vec<I64F64>> {
        let num_root_validators = pallet_subspace::ValidatorPermits::<T>::get(0)
            .into_iter()
            .filter(|b| *b)
            .count();

        let subnet_ids = pallet_subspace::N::<T>::iter_keys().collect::<Vec<_>>();
        let num_subnet_ids = subnet_ids.len();

        let mut weights: Vec<Vec<I64F64>> =
            vec![vec![I64F64::from_num(0.0); num_subnet_ids]; num_root_validators];

        for (uid_i, weights_i) in pallet_subspace::Weights::<T>::iter_prefix(0) {
            for (netuid, weight_ij) in &weights_i {
                let idx = uid_i as usize;
                if let Some(weight) = weights.get_mut(idx) {
                    if let Some((w, _)) =
                        weight.iter_mut().zip(&subnet_ids).find(|(_, subnet)| *subnet == netuid)
                    {
                        *w = I64F64::from_num(*weight_ij);
                    }
                }
            }
        }

        weights
    }

    fn get_subnet_ids() -> Vec<u16> {
        pallet_subspace::N::<T>::iter_keys()
            .filter(|subnet_id| subnet_id != &0) //filter rootnet out
            .collect::<Vec<_>>()
    }
}
