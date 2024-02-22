use frame_support::pallet_prelude::DispatchResult;
use substrate_fixed::types::{I64F64, I96F32};

use super::*;

impl<T: Config> Pallet<T> {
	pub fn do_add_profit_shares(
		origin: T::RuntimeOrigin,
		keys: Vec<T::AccountId>,
		shares: Vec<u16>,
	) -> DispatchResult {
		let key = ensure_signed(origin)?;

		// needs to be registered as a network
		ensure!(Self::is_key_registered_on_any_network(&key), Error::<T>::NotRegistered);
		assert!(keys.len() > 0);
		assert!(keys.len() == shares.len()); // make sure the keys and shares are the same length

		// make sure the keys are unique and the shares are unique

		let total_shares: u32 = shares.iter().map(|x| *x as u32).sum();
		assert!(total_shares > 0);
		let mut normalized_shares_float: Vec<I64F64> = Vec::new();
		// normalize shares
		let mut total_normalized_length: u32 = 0;
		for share in shares.iter() {
			let normalized_share =
				(I64F64::from(*share) / I64F64::from(total_shares as u16)) * I64F64::from(u16::MAX);
			total_normalized_length = total_normalized_length + normalized_share.to_num::<u32>();
			normalized_shares_float.push(normalized_share);
		}
		// make sure the normalized shares add up to the unit
		// convert the normalized shares to u16
		let mut normalize_shares: Vec<u16> =
			normalized_shares_float.iter().map(|x| x.to_num::<u16>()).collect::<Vec<u16>>();

		let mut total_normalized_shares: u16 = normalize_shares.iter().sum::<u16>();

		// ensure the profit shares add up to the unit
		if total_normalized_shares < u16::MAX {
			let diff = u16::MAX - total_normalized_shares;
			for i in 0..diff {
				let idx = (i % normalize_shares.len() as u16) as usize;
				normalize_shares[idx] = normalize_shares[idx] + 1;
			}
			total_normalized_shares = normalize_shares.iter().sum::<u16>();
		}

		assert!(
			total_normalized_shares == u16::MAX,
			"normalized shares {} vs {} do not add up to the unit",
			total_normalized_shares,
			u16::MAX
		);

		// check tssat the normalized shares add up to the unit
		let _total_normalized_shares: u16 = normalize_shares.iter().sum::<u16>();

		// now send the normalized shares to the profit share pallet
		let profit_share_tuples: Vec<(T::AccountId, u16)> =
			keys.iter().zip(normalize_shares.iter()).map(|(x, y)| (x.clone(), *y)).collect();

		ProfitShares::<T>::insert(&key, profit_share_tuples.clone());

		assert!(
			ProfitShares::<T>::get(&key).len() == profit_share_tuples.len(),
			"profit shares not added"
		);

		Ok(())
	}

	pub fn get_profit_share_emissions(
		key: T::AccountId,
		emission: u64,
	) -> Vec<(T::AccountId, u64)> {
		let profit_shares = ProfitShares::<T>::get(&key);
		let mut emission_shares: Vec<(T::AccountId, u64)> = Vec::new();
		for (share_key, share_ratio) in profit_shares.iter() {
			let share_emission_float: I96F32 =
				I96F32::from(emission) * (I96F32::from(*share_ratio) / I96F32::from(u16::MAX));
			let share_emission: u64 = share_emission_float.to_num::<u64>();
			emission_shares.push((share_key.clone(), share_emission));
		}

		return emission_shares;
	}

	pub fn get_profit_shares(key: T::AccountId) -> Vec<(T::AccountId, u16)> {
		return ProfitShares::<T>::get(&key);
	}
}
