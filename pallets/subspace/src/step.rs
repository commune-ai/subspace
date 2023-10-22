use super::*;
use crate::math::*;
use frame_support::{
	inherent::Vec,
	sp_std::vec,
	storage::{IterableStorageDoubleMap, IterableStorageMap},
};
use substrate_fixed::types::{I110F18, I32F32, I64F64, I96F32};

impl<T: Config> Pallet<T> {
	pub fn block_step() {
		let block_number: u64 = Self::get_current_block_as_u64();
		RegistrationsPerBlock::<T>::mutate(|val| *val = 0);
		log::debug!("block_step for block: {:?} ", block_number);
		for (netuid, tempo) in <Tempo<T> as IterableStorageMap<u16, u16>>::iter() {
			let new_queued_emission: u64 = Self::calculate_network_emission(netuid);
			PendingEmission::<T>::mutate(netuid, |queued| *queued += new_queued_emission);
			log::debug!("netuid_i: {:?} queued_emission: +{:?} ", netuid, new_queued_emission);
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
		let mut total_stake: I64F64 =
			I64F64::from_num(Self::get_total_subnet_stake(netuid).clone());
		if total_stake == I64F64::from_num(0.0) {
			total_stake = I64F64::from_num(1.0);
		}
		for (uid_i, key) in keys.iter() {
			stake_64[*uid_i as usize] =
				I64F64::from_num(Self::get_stake_for_key(netuid, key).clone()) / total_stake;
		}

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
		log::trace!("W (permit): {:?}", &weights);

		// Normalize remaining weights.
		inplace_row_normalize_sparse(&mut weights);
		log::trace!("W (mask+norm): {:?}", &weights);

		// =============================
		// ==  Incentive ==
		// =============================

		// Compute incentive: r_j = SUM(i) w_ij * s_i.
		let mut incentive: Vec<I32F32> = matmul_sparse(&weights, &stake, n);
		log::trace!("Incentive: {:?}", &incentive);

		// trust (normalized)
		let trust: Vec<I32F32> = Self::calculate_trust(&weights, &stake, n);
		let cloned_trust: Vec<u16> =
			trust.iter().map(|xi| fixed_proportion_to_u16(*xi)).collect::<Vec<u16>>();

		Trust::<T>::insert(netuid, cloned_trust);
		incentive = incentive.iter().zip(trust.iter()).map(|(inc, tru)| inc * tru).collect();

		// If emission is zero, do an even split.
		if is_zero(&incentive) {
			// no weights set
			for (uid_i, key) in keys.iter() {
				incentive[*uid_i as usize] = I32F32::from_num(1.0);
			}
		}
		inplace_normalize(&mut incentive); // range: I32F32(0, 1)

		let cloned_incentive: Vec<u16> =
			incentive.iter().map(|xi| fixed_proportion_to_u16(*xi)).collect::<Vec<u16>>();
		Incentive::<T>::insert(netuid, cloned_incentive);

		// =================================
		// == Bonds==
		// =================================

		// Compute bonds delta column normalized.
		let mut bonds: Vec<Vec<(u16, I32F32)>> = row_hadamard_sparse(&weights, &stake); // ΔB = W◦S (outdated W masked)
		log::trace!("ΔB: {:?}", &bonds);

		// Normalize bonds delta.
		inplace_col_normalize_sparse(&mut bonds, n); // sum_i b_ij = 1
		log::trace!("ΔB (norm): {:?}", &bonds);

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

		let cloned_dividends: Vec<u16> =
			dividends.iter().map(|xi| fixed_proportion_to_u16(*xi)).collect::<Vec<u16>>();
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

		// Emission tuples ( keys, u64 emission)
		for (uid_i, uid_key) in keys.iter() {
			if incentive_emission[*uid_i as usize] > 0 {
				// add the stake to the module
				Self::increase_stake(netuid, uid_key, uid_key, incentive_emission[*uid_i as usize]);
			}
			if dividends_emission[*uid_i as usize] > 0 {
				// get the ownership emission for this key
				let ownership_emission_for_key: Vec<(T::AccountId, u64)> =
					Self::get_ownership_ratios_emission(
						netuid,
						uid_key,
						dividends_emission[*uid_i as usize],
					);
				let delegation_fee = Self::get_delegation_fee(netuid, uid_key);
				// add the ownership
				for (owner_key, amount) in ownership_emission_for_key.iter() {
					let to_module = delegation_fee.mul_floor(*amount);
					let to_owner = amount.saturating_sub(to_module);
					Self::increase_stake(netuid, owner_key, uid_key, to_owner);
					Self::increase_stake(netuid, uid_key, uid_key, to_module);
				}
			}
		}

		// calculate the total emission
		let emission: Vec<u64> = incentive_emission
			.iter()
			.zip(dividends_emission.iter())
			.map(|(inc, div)| inc + div)
			.collect();
		Emission::<T>::insert(netuid, emission.clone());
	}

	pub fn stale_modules_outside_immunity(netuid: u16) -> Vec<u16> {
		// get the modules that are 0 and outside of the immunity period
		let mut uids: Vec<u16> = Vec::new();
		let block_number: u64 = Self::get_current_block_as_u64();
		let immunity_period: u16 = Self::get_immunity_period(netuid);
		let emission_vector: Vec<u64> = Self::get_emissions(netuid);
		for (uid, block_at_registration) in
			<RegistrationBlock<T> as IterableStorageDoubleMap<u16, u16, u64>>::iter_prefix(netuid)
		{
			if (block_at_registration + immunity_period as u64) < block_number {
				if emission_vector[uid as usize] == 0 {
					Self::remove_module(netuid, uid);
				}
			}
		}

		return uids
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
}
