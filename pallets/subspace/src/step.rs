use super::*;
use crate::math::*;
use frame_support::{
	storage::{IterableStorageDoubleMap, IterableStorageMap},
};
use substrate_fixed::types::{I110F18, I32F32, I64F64, I96F32};
use sp_std::vec;

impl<T: Config> Pallet<T> {
	pub fn block_step() {
		let block_number: u64 = Self::get_current_block_as_u64();
		RegistrationsPerBlock::<T>::mutate(|val| *val = 0);
		log::debug!("block_step for block: {:?} ", block_number);
		for (netuid, tempo) in <Tempo<T> as IterableStorageMap<u16, u16>>::iter() {

			let new_queued_emission: u64 = Self::calculate_network_emission(netuid);
			PendingEmission::<T>::mutate(netuid, |mut queued| *queued += new_queued_emission);
			log::debug!("netuid_i: {:?} queued_emission: +{:?} ", netuid, new_queued_emission);
			Self::deregister_pending_uid(netuid); // deregister any pending uids
			if Self::blocks_until_next_epoch(netuid, tempo, block_number) > 0 {
				continue
			}
			let emission_to_drain: u64 = PendingEmission::<T>::get(netuid).clone();
			Self::epoch(netuid, emission_to_drain);
			PendingEmission::<T>::insert(netuid, 0);
		}
	}

	pub fn epoch(netuid: u16, token_emission: u64) {
		// Get subnetwork size.
		let n: u16 = Self::get_subnet_n(netuid);
		log::trace!("n: {:?}", n);

		if n == 0 {
			return
		}

		// Get current block.
		let current_block: u64 = Self::get_current_block_as_u64();
		log::trace!("current_block: {:?}", current_block);

		// Block at registration vector (block when each module was most recently registered).
		let block_at_registration: Vec<u64> = Self::get_block_at_registration(netuid);
		log::trace!("Block at registration: {:?}", &block_at_registration);

		// ===========
		// == Stake ==
		// ===========

		let mut keys: Vec<(u16, T::AccountId)> = Self::get_uid_key_tuples(netuid);
		log::trace!("keys: {:?}", &keys);
		// Access network stake as normalized vector.
		let mut stake_64: Vec<I64F64> = vec![I64F64::from_num(0.0); n as usize];
		let mut total_stake_u64: u64 =Self::get_total_subnet_stake(netuid).clone();
		if total_stake_u64 == 0 {
			total_stake_u64 = 1;
		}
				// quadradic voting

		// let quadradic_voting: bool = Self::get_quadradic_voting(netuid);
		// if quadradic_voting {
		// 	// take a square root of the stake if its > 1

		// 	total_stake_u64 = total_stake_u64;
		// }

		for (uid_i, key) in keys.iter() {
			let mut stake_u64 = Self::get_stake_for_key(netuid, key).clone();
			stake_64[*uid_i as usize] = I64F64::from_num(stake_u64)/I64F64::from_num(total_stake_u64);
		}

		// if quadradic_voting && stake_u64 > 0{
		// 	stake_64 = stake_64.iter().map(|x| x.sqrt()).collect();
		// } 
		


		let mut stake: Vec<I32F32> = stake_64.iter().map(|x| I32F32::from_num(x.clone())).collect();

		// range: I32F32(0, 1)
		log::trace!("S: {:?}", &stake);

		// Normalize active stake.
		inplace_normalize(&mut stake);
		log::trace!("S (mask+norm): {:?}", &stake);

		// =============
		// == Weights (N x N) Sparsified ==
		// =============

		// Access network weights row normalized.
		let mut weights: Vec<Vec<(u16, I32F32)>> = Self::get_weights_sparse(netuid);

		// Normalize remaining weights.
		inplace_row_normalize_sparse(&mut weights);

		// =============================
		// ==  Incentive ==
		// =============================

		// Compute incentive: r_j = SUM(i) w_ij * s_i.
		let mut incentive: Vec<I32F32> = matmul_sparse(&weights, &stake, n);
		// If emission is zero, do an even split.
		if is_zero(&incentive) {
			// no weights set
			for (uid_i, key) in keys.iter() {
				incentive[*uid_i as usize] = I32F32::from_num(1.0);
			}
		}
		inplace_normalize(&mut incentive); // range: I32F32(0, 1)

		// =================================
		// == TRUST ==
		// =================================

		// trust that acts as a multiplier for the incentive
		let trust_ratio: u16 = Self::get_trust_ratio(netuid);
		if trust_ratio > 0 {
			let  trust_share : I32F32 = I32F32::from_num(trust_ratio)/I32F32::from_num(100);
			let incentive_share : I32F32 = I32F32::from_num(1.0) - trust_share;
			let mut trust: Vec<I32F32> = Self::calculate_trust(&weights, &stake, n);
			incentive = incentive.iter().zip(trust.iter()).map(|(inc, tru)| (inc * incentive_share) + (tru * trust_share)).collect();

			// save the trust into the trust vector
			Trust::<T>::insert(netuid, trust.iter().map(|xi| fixed_proportion_to_u16(*xi)).collect::<Vec<u16>>());
		}

		// store the incentive
		let cloned_incentive: Vec<u16> = incentive.iter().map(|xi| fixed_proportion_to_u16(*xi)).collect::<Vec<u16>>();
		Incentive::<T>::insert(netuid, cloned_incentive);


		// =================================
		// == Bonds==
		// =================================

		// Compute bonds delta column normalized.
		let mut bonds: Vec<Vec<(u16, I32F32)>> = row_hadamard_sparse(&weights, &stake); // ΔB = W◦S (outdated W masked)
		// Normalize bonds delta.
		inplace_col_normalize_sparse(&mut bonds, n); // sum_i b_ij = 1

		// Compute dividends: d_i = SUM(j) b_ij * inc_j.
		// range: I32F32(0, 1)
		let mut dividends: Vec<I32F32> = matmul_transpose_sparse(&bonds, &incentive).clone();
		// If emission is zero, do an even split.
		if is_zero(&dividends) {
			// no weights set
			for (uid_i, key) in keys.iter() {
				dividends[*uid_i as usize] = I32F32::from_num(1.0);
			}
		}

		inplace_normalize(&mut dividends);
		log::trace!("D: {:?}", &dividends);

		let cloned_dividends: Vec<u16> = dividends.iter().map(|xi| fixed_proportion_to_u16(*xi)).collect::<Vec<u16>>();
		Dividends::<T>::insert(netuid, cloned_dividends);

		// =================================
		// == Emission==
		// =================================


		let incentive_emission_float: Vec<I64F64> = incentive
			.clone()
			.iter()
			.map(|x| I64F64::from_num(x.clone()) * I64F64::from_num(token_emission / 2))
			.collect();
		let dividends_emission_float: Vec<I64F64> = dividends
			.clone()
			.iter()
			.map(|x| I64F64::from_num(x.clone()) * I64F64::from_num(token_emission / 2))
			.collect();

		let incentive_emission: Vec<u64> =
			incentive_emission_float.iter().map(|e: &I64F64| e.to_num::<u64>()).collect();
		let dividends_emission: Vec<u64> =
			dividends_emission_float.iter().map(|e: &I64F64| e.to_num::<u64>()).collect();


		let mut burn_amount_per_epoch: u64 = Self::get_burn_emission_per_epoch(netuid);
		let mut zero_stake_uids : Vec<u16> = Vec::new();
		let min_stake: u64 = Self::get_min_stake(netuid);

		// Emission tuples ( keys, u64 emission)
		for (module_uid, module_key) in keys.iter() {

			let mut owner_emission_incentive: u64 = incentive_emission[*module_uid as usize] + dividends_emission[*module_uid as usize];
			let mut owner_dividends_emission: u64 = dividends_emission[*module_uid as usize];
			
			// calculate the future
			let mut total_future_stake: u64 = stake_64[*module_uid as usize].to_num::<u64>();
			total_future_stake = total_future_stake.saturating_add(owner_emission_incentive);
			total_future_stake = total_future_stake.saturating_add(owner_dividends_emission);
			total_future_stake = total_future_stake.saturating_sub(burn_amount_per_epoch);
			if total_future_stake < min_stake {
				// if the stake is less than the burn amount, then deregister the module
				zero_stake_uids.push(*module_uid as u16);
			}
			// eat into dividends first and then into the incentive
			owner_dividends_emission = owner_dividends_emission.saturating_sub(burn_amount_per_epoch);
			burn_amount_per_epoch = burn_amount_per_epoch.saturating_sub(owner_dividends_emission);

			// if the owner emission is less than the burn amount
			if burn_amount_per_epoch > 0{
				// eat into the emissions
				// get the amount to burn
				if burn_amount_per_epoch > owner_emission_incentive  {

					let amount_to_burn: u64 = owner_emission_incentive - burn_amount_per_epoch;
					Self::decrease_stake(netuid, module_key, module_key, amount_to_burn);

				} 
				// burn the rest
				owner_emission_incentive = owner_emission_incentive.saturating_sub(owner_dividends_emission); 

			}
			
			// if the owner emission is less than the burn amount
			let mut owner_emission: u64 = owner_emission_incentive + owner_dividends_emission;

			if owner_dividends_emission > 0 {
				// get the ownership emission for this key

				let ownership_vector: Vec<(T::AccountId, I64F64)> = Self::get_ownership_ratios(netuid, module_key);
	
				let delegation_fee = Self::get_delegation_fee(netuid, module_key);
				
				// add the ownership
				for (delegate_key, delegate_ratio) in ownership_vector.iter() {

					let mut dividends_from_delegate : u64 = (delegate_ratio * I64F64::from_num(dividends_emission[*module_uid as usize])).to_num::<u64>();
					let to_module: u64 = delegation_fee.mul_floor(dividends_from_delegate);
					let to_delegate: u64 = dividends_from_delegate.saturating_sub(to_module);
					Self::increase_stake(netuid, delegate_key, module_key, to_delegate);
					owner_emission = owner_emission.saturating_sub(to_module);

				}
			}

			if owner_emission > 0 {
				// generate the profit shares
				let profit_share_emissions: Vec<(T::AccountId, u64)> = Self::get_profit_share_emissions(module_key.clone(), owner_emission);

				// if there are profit shares, then increase the balance of the profit share key
				if profit_share_emissions.len() > 0 {
					// if there are profit shares, then increase the balance of the profit share key
					for (profit_share_key, profit_share_emission) in profit_share_emissions.iter() {
						// increase the balance of the profit share key
						Self::increase_stake(netuid, profit_share_key, module_key, *profit_share_emission);
					}
				} else {
					// increase it to the module key
					Self::increase_stake(netuid, module_key, module_key, owner_emission);
				}
			}

		}
		if zero_stake_uids.len() > 0 {
			PendingDeregisterUids::<T>::insert(netuid, zero_stake_uids.clone());
		}
		// calculate the total emission
		let emission: Vec<u64> = incentive_emission
			.iter()
			.zip(dividends_emission.iter())
			.map(|(inc, div)| inc + div)
			.collect();
			
		Emission::<T>::insert(netuid, emission.clone());
	}

	pub fn get_block_at_registration(netuid: u16) -> Vec<u64> {
		let n: usize = Self::get_subnet_n(netuid) as usize;
		let mut block_at_registration: Vec<u64> = vec![0; n];
		for module_uid in 0..n {
			if Keys::<T>::contains_key(netuid, module_uid as u16) {
				block_at_registration[module_uid] =
					Self::get_module_registration_block(netuid, module_uid as u16);
			}
		}
		block_at_registration
	}

	pub fn get_weights_sparse(netuid: u16) -> Vec<Vec<(u16, I32F32)>> {
		let n: usize = Self::get_subnet_n(netuid) as usize;
		let mut weights: Vec<Vec<(u16, I32F32)>> = vec![vec![]; n];
		for (uid_i, weights_i) in
			<Weights<T> as IterableStorageDoubleMap<u16, u16, Vec<(u16, u16)>>>::iter_prefix(netuid)
		{
			for (uid_j, weight_ij) in weights_i.iter() {
				weights[uid_i as usize].push((*uid_j, u16_proportion_to_fixed(*weight_ij)));
			}
		}
		// Remove self-weight by masking diagonal.
		weights = mask_diag_sparse(&weights);
		return weights
	}

	pub fn blocks_until_next_epoch(netuid: u16, tempo: u16, block_number: u64) -> u64 {
		if tempo == 0 {
			return 0
		}
		return (block_number + netuid as u64) % (tempo as u64)
	}

	pub fn get_ownership_ratios_for_uid(netuid: u16, uid: u16) -> Vec<(T::AccountId, I64F64)> {
		return Self::get_ownership_ratios(netuid, &Self::get_key_for_uid(netuid, uid))
	}

	pub fn get_ownership_ratios(
		netuid: u16,
		module_key: &T::AccountId,
	) -> Vec<(T::AccountId, I64F64)> {
		let stake_from_vector: Vec<(T::AccountId, u64)> =
			Self::get_stake_from_vector(netuid, module_key);
		let uid = Self::get_uid_for_key(netuid, module_key);
		let mut total_stake_from: I64F64 = I64F64::from_num(0);

		let mut ownership_vector: Vec<(T::AccountId, I64F64)> = Vec::new();

		for (k, v) in stake_from_vector.clone().into_iter() {
			let ownership = I64F64::from_num(v);
			ownership_vector.push((k.clone(), ownership));
			total_stake_from += ownership;
		}
		// add the module itself, if it has stake of its own
		if total_stake_from == I64F64::from_num(0) {
			ownership_vector[0].1 = I64F64::from_num(1.0);
		} else {
			ownership_vector =
				ownership_vector.into_iter().map(|(k, v)| (k, v / total_stake_from)).collect();
		}

		return ownership_vector
	}

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

		return emission_vector
	}

	pub fn calculate_trust(
		weights: &Vec<Vec<(u16, I32F32)>>,
		stake_vector: &Vec<I32F32>,
		n: u16,
	) -> Vec<I32F32> {
		let mut trust: Vec<I32F32> = vec![I32F32::from_num(0.0); n as usize];
		for (i, w_row) in weights.iter().enumerate() {
			for (j, w_row_value) in w_row.iter() {
				// Compute trust scores: t_j = SUM(i) w_ij * s_i
				// result_j = SUM(i) vector_i * matrix_ij
				if *w_row_value > I32F32::from_num(0.0) && stake_vector[i] > I32F32::from_num(0.0) {
					trust[*j as usize] += I32F32::from_num(1.0);
				}
			}
		}

		inplace_normalize(&mut trust);
		trust
	}
	
	pub fn deregister_pending_uid(netuid: u16) {
		let mut pending_deregister_uids:  Vec<u16> = PendingDeregisterUids::<T>::get(netuid);
		if pending_deregister_uids.len() > 0 {
			let uid: u16 = pending_deregister_uids.remove(0);
			Self::remove_module(netuid,uid);
			PendingDeregisterUids::<T>::insert(netuid, pending_deregister_uids);
		}
	}
}
