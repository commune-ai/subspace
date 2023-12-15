use super::*;
use crate::utils::is_vec_str;
use frame_support::pallet_prelude::DispatchResult;
use sp_runtime::DispatchError;
use system::ensure_root;

impl<T: Config> Pallet<T> {
	pub fn global_params() -> GlobalParams {
		GlobalParams {
			max_name_length: Self::get_max_name_length(),
			max_allowed_subnets: Self::get_max_allowed_subnets(),
			max_allowed_modules: Self::get_max_allowed_modules(),
			max_registrations_per_block: Self::get_max_registrations_per_block(),
			unit_emission: Self::get_unit_emission(),
			tx_rate_limit: Self::get_tx_rate_limit(),
			vote_threshold: Self::get_global_vote_threshold(),
			max_proposals: Self::get_max_proposals(),
			vote_mode: Self::get_vote_mode_global(),
			burn_rate: Self::get_burn_rate(),
			min_burn: Self::get_min_burn(),
		}
	}

	pub fn check_global_params(params: GlobalParams) -> DispatchResult {
		// checks if params are valid
		let og_params = Self::global_params();

		// check if the name already exists
		ensure!(params.max_name_length > 0, "Invalid max_name_length");

		ensure!(params.max_allowed_subnets > 0, "Invalid max_allowed_subnets");

		ensure!(params.max_allowed_modules > 0, "Invalid max_allowed_modules");

		ensure!(params.max_registrations_per_block > 0, "Invalid max_registrations_per_block");

		ensure!(params.vote_threshold < 100, "Invalid vote_threshold");

		ensure!(params.tx_rate_limit < 100, "Invalid tx_rate_limit");

		assert!(params.burn_rate <= 100, "Invalid burn_rate");

		assert!(params.min_burn <= 100, "Invalid vote_threshold");

		Ok(())
	}

	pub fn set_global_params(params: GlobalParams) {
		Self::set_max_name_length(params.max_name_length);

		Self::set_max_allowed_subnets(params.max_allowed_subnets);

		Self::set_max_allowed_modules(params.max_allowed_modules);

		Self::set_max_registrations_per_block(params.max_registrations_per_block);

		Self::set_unit_emission(params.unit_emission);

		Self::set_tx_rate_limit(params.tx_rate_limit);

		Self::set_global_vote_threshold(params.vote_threshold);

		Self::set_max_proposals(params.max_proposals);

		Self::set_vote_mode_global(params.vote_mode);

		Self::set_burn_rate(params.burn_rate);

		Self::set_min_burn(params.min_burn);
	}

	pub fn set_vote_mode_global(vote_mode: Vec<u8>) {
		VoteModeGlobal::<T>::put(vote_mode);
	}

	pub fn get_vote_mode_global() -> Vec<u8> {
		return VoteModeGlobal::<T>::get();
	}
	pub fn get_burn_rate() -> u16 {
		return BurnRate::<T>::get()
	}

	pub fn set_burn_rate(mut burn_rate: u16) {
		if burn_rate > 100 {
			burn_rate = 100;
		}
		BurnRate::<T>::put(burn_rate);
	}

	pub fn set_max_proposals(max_proposals: u64) {
		MaxProposals::<T>::put(max_proposals);
	}

	pub fn get_max_proposals() -> u64 {
		MaxProposals::<T>::get()
	}

	pub fn get_global_vote_threshold() -> u16 {
		return GlobalVoteThreshold::<T>::get();
	}
	pub fn set_global_vote_threshold(vote_threshold: u16) {
		GlobalVoteThreshold::<T>::put(vote_threshold);
	}
	pub fn get_max_registrations_per_block() -> u16 {
		MaxRegistrationsPerBlock::<T>::get()
	}
	pub fn get_max_name_length() -> u16 {
		return MaxNameLength::<T>::get();
	}

	pub fn set_max_name_length(max_name_length: u16) {
		MaxNameLength::<T>::put(max_name_length)
	}

	pub fn do_update_global(origin: T::RuntimeOrigin, params: GlobalParams) -> DispatchResult {
		ensure_root(origin)?;
		assert!(is_vec_str(params.vote_mode.clone(), "authority"));
		Self::check_global_params(params.clone())?;
		Self::set_global_params(params.clone());
		Ok(())
	}

	pub fn global_n() -> u16 {
		let mut global_n: u16 = 0;
		for netuid in Self::netuids() {
			global_n += N::<T>::get(netuid);
		}
		return global_n
	}

	pub fn get_global_stake_to(key: &T::AccountId) -> u64 {
		// get all of the stake to
		let total_networks: u16 = TotalSubnets::<T>::get();
		let mut total_stake_to = 0;

		for netuid in 0..total_networks {
			total_stake_to += Self::get_total_stake_to(netuid, key);
		}

		total_stake_to
	}

	// Configure tx rate limiting
	pub fn get_tx_rate_limit() -> u64 {
		TxRateLimit::<T>::get()
	}
	pub fn set_tx_rate_limit(tx_rate_limit: u64) {
		TxRateLimit::<T>::put(tx_rate_limit)
	}

	pub fn set_min_burn(min_burn: u64) {
		MinBurn::<T>::put(min_burn);
	}

	pub fn get_min_burn() -> u64 {
		MinBurn::<T>::get().into()
	}

	// ========================
	// ==== Rate Limiting =====
	// ========================
	pub fn get_last_tx_block(key: &T::AccountId) -> u64 {
		LastTxBlock::<T>::get(key)
	}
	pub fn set_last_tx_block(key: &T::AccountId, last_tx_block: u64) {
		LastTxBlock::<T>::insert(key, last_tx_block)
	}
}
