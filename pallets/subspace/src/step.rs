use super::*;
use crate::math::*;
use frame_support::storage::IterableStorageDoubleMap;
use sp_arithmetic::per_things::Percent;
use sp_std::vec;
use substrate_fixed::types::{I110F18, I32F32, I64F64};

pub mod yuma;

impl<T: Config> Pallet<T> {
    pub fn block_step() {
        let block_number: u64 = Self::get_current_block_number();
        RegistrationsPerBlock::<T>::mutate(|val: &mut u16| *val = 0);

        // Execute proposals if any should be executed, this is done every 100 blocks.
        if block_number % 100 == 0 {
            Self::resolve_proposals(block_number);
        }

        log::debug!("block_step for block: {block_number:?}");

        let subnet_stake_threshold: Percent = SubnetStakeThreshold::<T>::get();
        for (netuid, tempo) in Tempo::<T>::iter() {
            let registration_this_interval = Self::get_registrations_this_interval(netuid);

            // adjust registrations parameters
            Self::adjust_registration(netuid, block_number, registration_this_interval);

            let new_queued_emission: u64 =
                Self::calculate_network_emission(netuid, subnet_stake_threshold);
            PendingEmission::<T>::mutate(netuid, |queued: &mut u64| *queued += new_queued_emission);
            log::debug!("netuid_i: {netuid:?} queued_emission: +{new_queued_emission:?} ");

            if Self::blocks_until_next_epoch(netuid, tempo, block_number) > 0 {
                continue;
            }

            let emission_to_drain: u64 = PendingEmission::<T>::get(netuid);
            let has_enough_stake_for_yuma = || {
                let subnet_stake = Self::get_total_subnet_stake(netuid) as u128;
                let total_stake = Self::total_stake() as u128;

                if total_stake == 0 {
                    false
                } else {
                    let subnet_stake_percent = (subnet_stake * 100) / total_stake;
                    subnet_stake_threshold <= Percent::from_parts(subnet_stake_percent as u8)
                }
            };

            if netuid == 0 {
                Self::linear_epoch(netuid, emission_to_drain)
            } else if has_enough_stake_for_yuma() {
                yuma::YumaCalc::<T>::new(netuid, emission_to_drain).run();
            }

            PendingEmission::<T>::insert(netuid, 0);
        }
    }

    /// This function acts as the main function of the entire blockchain reward distribution.
    /// It calculates the dividends, the incentive, the weights, the bonds,
    /// the trust and the emission for the epoch.
    pub fn linear_epoch(netuid: u16, token_emission: u64) {
        // get the network parameters
        let global_params = Self::global_params();
        let subnet_params = Self::subnet_params(netuid);

        // get the amount of modules
        let n: u16 = Self::get_subnet_n(netuid);
        let current_block: u64 = Self::get_current_block_number();

        // if there are no modules, then return
        if n == 0 {
            return;
        }

        // FOUNDER DIVIDENDS
        let founder_key = Self::get_founder(netuid);
        let (token_emission, founder_emission) =
            Self::calculate_founder_emission(netuid, token_emission, &founder_key);

        // STAKE
        let uid_key_tuples: Vec<(u16, T::AccountId)> = Self::get_uid_key_tuples(netuid);
        let total_stake_u64: u64 = Self::get_total_subnet_stake(netuid).max(1);

        let max_stake = subnet_params.max_stake;

        let stake_u64: Vec<u64> = uid_key_tuples
            .iter()
            .map(|(_, key)| Self::get_stake_for_key(netuid, key).min(max_stake))
            .collect();

        // Clip it to the max stake
        let stake_f64: Vec<I64F64> = stake_u64
            .iter()
            .map(|x| I64F64::from_num(*x) / I64F64::from_num(total_stake_u64))
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
        );

        // INCENTIVE
        // see if this shit needs to be mut
        let mut incentive: Vec<I32F32> =
            Self::compute_incentive(&weights, &stake, &uid_key_tuples, n);

        // TRUST
        // trust that acts as a multiplier for the incentive
        let trust_ratio: u16 = Self::get_trust_ratio(netuid);
        if trust_ratio > 0 {
            let trust_share: I32F32 = I32F32::from_num(trust_ratio) / I32F32::from_num(100);
            let incentive_share: I32F32 = I32F32::from_num(1.0).saturating_sub(trust_share);
            let trust = Self::compute_trust(&weights, &stake, &subnet_params, n);

            incentive = incentive
                .iter()
                .zip(trust.iter())
                .map(|(inc, tru)| (inc * incentive_share) + (tru * trust_share))
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
        let bonds: Vec<Vec<(u16, I32F32)>> = Self::compute_bonds_delta(&weights, &stake);

        // DIVIDENDS
        let (fixed_dividends, dividends) =
            Self::compute_dividends(&bonds, &incentive, &uid_key_tuples);
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
    }

    fn calculate_emission_ratios(
        incentive: &[I32F32],
        dividends: &[I32F32],
        token_emission: u64,
        netuid: u16,
    ) -> (Vec<I64F64>, Vec<I64F64>) {
        let incentive_ratio: I64F64 =
            I64F64::from_num(Self::get_incentive_ratio(netuid) as u64) / I64F64::from_num(100);
        let dividend_ratio: I64F64 = I64F64::from_num(1.0) - incentive_ratio;

        let incentive_emission_float: Vec<I64F64> = incentive
            .iter()
            .map(|&x| I64F64::from_num(x) * I64F64::from_num(token_emission) * incentive_ratio)
            .collect();
        let dividends_emission_float: Vec<I64F64> = dividends
            .iter()
            .map(|&x| I64F64::from_num(x) * I64F64::from_num(token_emission) * dividend_ratio)
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

        let burn_amount_per_epoch: u64 = Self::get_burn_per_epoch(netuid);

        let founder_uid = Self::get_uid_for_key(netuid, founder_key);
        incentive_emission[founder_uid as usize] =
            incentive_emission[founder_uid as usize].saturating_add(founder_emission);

        let mut emission: Vec<u64> = vec![0; n];

        for (module_uid, module_key) in uid_key_tuples.iter() {
            let mut owner_emission_incentive: u64 = incentive_emission[*module_uid as usize];
            let mut owner_dividends_emission: u64 = dividends_emission[*module_uid as usize];
            let owner_emission: u64 = owner_emission_incentive + owner_dividends_emission;

            if burn_amount_per_epoch > owner_emission {
                let burn_into_stake: u64 = burn_amount_per_epoch.saturating_sub(owner_emission);

                if burn_into_stake > 0 {
                    Self::decrease_stake(netuid, module_key, module_key, burn_into_stake);
                }

                continue;
            }

            if burn_amount_per_epoch > owner_emission_incentive {
                owner_emission_incentive = 0;
                let left_burn_amount_per_epoch =
                    burn_amount_per_epoch.saturating_sub(owner_emission_incentive);
                owner_dividends_emission =
                    owner_dividends_emission.saturating_sub(left_burn_amount_per_epoch);
            } else {
                owner_emission_incentive =
                    owner_emission_incentive.saturating_sub(burn_amount_per_epoch);
            }

            emission[*module_uid as usize] = owner_emission_incentive + owner_dividends_emission;

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
                        (I64F64::from_num(total_owner_dividends_emission) * *delegate_ratio)
                            .to_num::<u64>();
                    let to_module: u64 = delegation_fee.mul_floor(dividends_from_delegate);
                    let to_delegate: u64 = dividends_from_delegate.saturating_sub(to_module);
                    Self::increase_stake(netuid, delegate_key, module_key, to_delegate);
                    owner_dividends_emission = owner_dividends_emission.saturating_sub(to_delegate);
                }
            }

            let owner_emission: u64 = owner_emission_incentive + owner_dividends_emission;
            if owner_emission > 0 {
                let profit_share_emissions: Vec<(T::AccountId, u64)> =
                    Self::get_profit_share_emissions(module_key, owner_emission);

                if !profit_share_emissions.is_empty() {
                    for (profit_share_key, profit_share_emission) in profit_share_emissions.iter() {
                        Self::increase_stake(
                            netuid,
                            profit_share_key,
                            module_key,
                            *profit_share_emission,
                        );
                    }
                } else {
                    Self::increase_stake(netuid, module_key, module_key, owner_emission);
                }
            }
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

    fn compute_dividends(
        bonds: &[Vec<(u16, I32F32)>],
        incentive: &[I32F32],
        uid_key_tuples: &[(u16, T::AccountId)],
    ) -> (Vec<u16>, Vec<I32F32>) {
        let n = incentive.len();
        let mut dividends: Vec<I32F32> = vec![I32F32::from_num(0.0); n];

        for (i, sparse_row) in bonds.iter().enumerate() {
            for (j, value) in sparse_row.iter() {
                dividends[i] += incentive[*j as usize] * *value;
            }
        }

        if dividends.iter().all(|&x| x == I32F32::from_num(0.0)) {
            for (uid_i, _) in uid_key_tuples.iter() {
                dividends[*uid_i as usize] = I32F32::from_num(1.0);
            }
        }

        inplace_normalize(&mut dividends);

        let fixed_dividends: Vec<u16> =
            dividends.iter().map(|xi| fixed_proportion_to_u16(*xi)).collect();

        (fixed_dividends, dividends)
    }

    fn compute_bonds_delta(
        weights: &[Vec<(u16, I32F32)>],
        stake: &[I32F32],
    ) -> Vec<Vec<(u16, I32F32)>> {
        let n = weights.len();
        let mut bonds: Vec<Vec<(u16, I32F32)>> = weights.to_vec();
        let mut col_sum: Vec<I32F32> = vec![I32F32::from_num(0.0); n];

        for (i, sparse_row) in bonds.iter_mut().enumerate() {
            for (j, value) in sparse_row.iter_mut() {
                *value *= stake[i];
                col_sum[*j as usize] += *value;
            }
        }

        for sparse_row in bonds.iter_mut() {
            for (j, value) in sparse_row.iter_mut() {
                if col_sum[*j as usize] > I32F32::from_num(0.0) {
                    *value /= col_sum[*j as usize];
                }
            }
        }

        bonds
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
                if *weight_ij > 0 && stake[i] > I32F32::from_num(subnet_params.min_stake) {
                    trust[*j as usize] += I32F32::from_num(1.0);
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
            for (j, value) in sparse_row.iter() {
                incentive[*j as usize] += stake[i] * value;
            }
        }

        if is_zero(&incentive) {
            for (uid_i, _key) in uid_key_tuples.iter() {
                incentive[*uid_i as usize] = I32F32::from_num(1.0);
            }
        }

        inplace_normalize(&mut incentive);
        incentive
    }

    fn get_current_weight_age(last_update_vector: &[u64], current_block: u64, uid_i: u16) -> u64 {
        current_block.saturating_sub(last_update_vector[uid_i as usize])
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

            let weight_f64 = I64F64::from_num(*weight_ij) / I64F64::from_num(u16::MAX);
            let weight_stake =
                (stake_f64[uid_i as usize] * weight_f64) * I64F64::from_num(total_stake_u64);

            if weight_stake > min_weight_stake_f64 {
                valid_weights.push((*uid_j, *weight_ij));
            } else {
                return (true, valid_weights);
            }
        }

        (false, valid_weights)
    }

    fn process_weights(
        netuid: u16,
        n: u16,
        global_params: &GlobalParams<T>,
        subnet_params: &SubnetParams<T>,
        current_block: u64,
        stake_f64: &[I64F64],
        total_stake_u64: u64,
    ) -> Vec<Vec<(u16, I32F32)>> {
        let last_update_vector = Self::get_last_update(netuid);
        let min_weight_stake_f64 = I64F64::from_num(global_params.min_weight_stake);
        let mut weights: Vec<Vec<(u16, u16)>> = vec![vec![]; n as usize];

        for (uid_i, weights_i) in
            <Weights<T> as IterableStorageDoubleMap<u16, u16, Vec<(u16, u16)>>>::iter_prefix(netuid)
        {
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

            weights[uid_i as usize] = valid_weights;
            if weight_changed {
                <Weights<T>>::insert(netuid, uid_i, weights[uid_i as usize].clone());
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

    fn calculate_founder_emission(
        netuid: u16,
        mut token_emission: u64,
        founder_key: &T::AccountId,
    ) -> (u64, u64) {
        let is_founder_registered = Self::key_registered(netuid, founder_key);
        if !is_founder_registered {
            return (token_emission, 0);
        }

        let founder_share: u16 = Self::get_founder_share(netuid);
        if founder_share == 0u16 {
            return (token_emission, 0);
        }

        let founder_emission_ratio: I64F64 =
            I64F64::from_num(founder_share.min(100)) / I64F64::from_num(100);
        let founder_emission =
            (founder_emission_ratio * I64F64::from_num(token_emission)).to_num::<u64>();
        token_emission = token_emission.saturating_sub(founder_emission);

        (token_emission, founder_emission)
    }

    pub fn get_block_at_registration(netuid: u16) -> Vec<u64> {
        let n = Self::get_subnet_n(netuid) as usize;
        let mut block_at_registration: Vec<u64> = vec![0; n];

        for (module_uid, block) in block_at_registration.iter_mut().enumerate() {
            let module_uid = module_uid as u16;

            if Keys::<T>::contains_key(netuid, module_uid) {
                *block = Self::get_module_registration_block(netuid, module_uid);
            }
        }

        block_at_registration
    }

    pub fn blocks_until_next_epoch(netuid: u16, tempo: u16, block_number: u64) -> u64 {
        if tempo == 0 {
            return 0;
        }
        (block_number + netuid as u64) % (tempo as u64)
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
            total_stake_from += ownership;
        }

        // add the module itself, if it has stake of its own
        if total_stake_from == I64F64::from_num(0) {
            ownership_vector.push((module_key.clone(), I64F64::from_num(0)));
        } else {
            ownership_vector =
                ownership_vector.into_iter().map(|(k, v)| (k, v / total_stake_from)).collect();
        }

        ownership_vector
    }

    #[cfg(debug_assertions)]
    pub fn get_ownership_ratios_emission(
        netuid: u16,
        module_key: &T::AccountId,
        emission: u64,
    ) -> Vec<(T::AccountId, u64)> {
        let ownership_vector: Vec<(T::AccountId, I64F64)> =
            Self::get_ownership_ratios(netuid, module_key);
        let mut emission_vector: Vec<(T::AccountId, u64)> = Vec::new();

        for (k, v) in ownership_vector {
            let emission_for_delegate = (v * I64F64::from_num(emission)).to_num::<u64>();
            emission_vector.push((k, emission_for_delegate));
        }

        emission_vector
    }

    pub fn get_burn_per_epoch(netuid: u16) -> u64 {
        let n = Self::get_subnet_n(netuid);
        let token_emission: u64 = PendingEmission::<T>::get(netuid);
        let burn_rate: u16 = Self::get_burn_rate().min(100);
        let mut burn_amount_per_epoch: u64 = 0;
        // get the float and convert to u64token_emission
        if burn_rate > 0 {
            let burn_rate_float: I64F64 = (I64F64::from_num(burn_rate) / I64F64::from_num(100))
                * (I64F64::from_num(token_emission) / I64F64::from_num(n));
            burn_amount_per_epoch = burn_rate_float.to_num::<u64>();
        }
        burn_amount_per_epoch
    }

    pub fn adjust_registration(netuid: u16, block_number: u64, registrations_this_interval: u16) {
        let target_registrations_interval = Self::get_target_registrations_interval();
        if block_number % u64::from(target_registrations_interval) == 0 {
            let current_burn = Self::get_burn(netuid);
            let target_registrations_per_interval = Self::get_target_registrations_per_interval();

            let adjusted_burn = Self::adjust_burn(
                current_burn,
                registrations_this_interval,
                target_registrations_per_interval,
            );

            Self::set_burn(netuid, adjusted_burn);

            // reset the registrations
            Self::set_registrations_this_interval(netuid, 0);
        }
    }

    pub fn adjust_burn(
        current_burn: u64,
        registrations_this_interval: u16,
        target_registrations_per_interval: u16,
    ) -> u64 {
        let updated_burn: I110F18 = I110F18::from_num(current_burn)
            * I110F18::from_num(registrations_this_interval + target_registrations_per_interval)
            / I110F18::from_num(
                target_registrations_per_interval + target_registrations_per_interval,
            );
        let alpha: I110F18 =
            I110F18::from_num(Self::get_adjustment_alpha()) / I110F18::from_num(u64::MAX);
        let next_value: I110F18 = alpha * I110F18::from_num(current_burn)
            + (I110F18::from_num(1.0) - alpha) * updated_burn;
        if next_value >= I110F18::from_num(Self::get_max_burn()) {
            Self::get_max_burn()
        } else if next_value <= I110F18::from_num(Self::get_min_burn()) {
            Self::get_min_burn()
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
            None => N::<T>::iter()
                .map(|(netuid, _)| netuid)
                .filter_map(|netuid| StakeTo::<T>::try_get(netuid, account_id).ok())
                .flat_map(|entries| entries.into_values())
                .sum(),
        }
    }
}
