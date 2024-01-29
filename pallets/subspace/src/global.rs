use super::*;
use frame_support::pallet_prelude::DispatchResult;
use sp_runtime::DispatchError;
use system::ensure_root;
use crate::utils::is_vec_str;
use sp_runtime::BoundedVec;

impl<T: Config> Pallet<T> {

	pub fn global_params() -> GlobalParams {
		GlobalParams {
			max_name_length: Self::get_global_max_name_length(),
			max_allowed_subnets: Self::get_global_max_allowed_subnets(),
			max_allowed_modules: Self::get_max_allowed_modules(),
			max_registrations_per_block: Self::get_max_registrations_per_block(),
			unit_emission: Self::get_unit_emission(),
			tx_rate_limit: Self::get_tx_rate_limit(),
			vote_threshold: Self::get_global_vote_threshold(),
			max_proposals: Self::get_max_proposals(),
			vote_mode: Self::get_vote_mode_global(),
			burn_rate: Self::get_burn_rate(),
			min_burn: Self::get_min_burn(),
			min_stake: Self::get_min_stake_global(),
			min_weight_stake: Self::get_min_weight_stake(),
			max_allowed_weights: Self::get_max_allowed_weights_global(),
		}
	}

    pub fn check_global_params(params: GlobalParams) -> DispatchResult{
        // checks if params are valid
		let og_params = Self::global_params();

        // check if the name already exists
        ensure!(params.max_name_length > 0, Error::<T>::InvalidMaxNameLength);
		
        ensure!(params.max_allowed_subnets > 0, Error::<T>::InvalidMaxAllowedSubnets);

		ensure!(params.max_allowed_modules > 0, Error::<T>::InvalidMaxAllowedModules);

		ensure!(params.max_registrations_per_block > 0, Error::<T>::InvalidMaxRegistrationsPerBlock);

		ensure!(params.vote_threshold < 100, Error::<T>::InvalidVoteThreshold);

		ensure!(params.max_proposals > 0, Error::<T>::InvalidMaxProposals);

		ensure!(params.unit_emission <= og_params.unit_emission, Error::<T>::InvalidUnitEmission);

		ensure!(params.tx_rate_limit > 0, Error::<T>::InvalidTxRateLimit);

		ensure!(params.burn_rate <= 100, Error::<T>::InvalidBurnRate);
				
		ensure!(params.min_burn <= 100, Error::<T>::InvalidMinBurn);


		
        Ok(())
    }


	pub fn set_global_params(params: GlobalParams) {

		Self::set_global_max_name_length(params.max_name_length);
		Self::set_global_max_allowed_subnets(params.max_allowed_subnets);
		Self::set_max_allowed_modules(params.max_allowed_modules);
		Self::set_max_registrations_per_block(params.max_registrations_per_block);
		Self::set_unit_emission(params.unit_emission);
		Self::set_tx_rate_limit(params.tx_rate_limit);
		Self::set_global_vote_threshold(params.vote_threshold);
		Self::set_max_proposals(params.max_proposals);
		Self::set_vote_mode_global(params.vote_mode);
		Self::set_burn_rate(params.burn_rate);
		Self::set_min_burn( params.min_burn);
		Self::set_min_weight_stake(params.min_weight_stake);
		Self::set_min_stake_global(params.min_stake);
	}

	pub fn get_min_weight_stake() -> u64 {
		GlobalStateStorage::<T>::get().min_weight_stake
	}
	pub fn set_min_weight_stake(min_weight_stake: u64)  {
		let mut global_state = GlobalStateStorage::<T>::get();

		global_state.min_weight_stake = min_weight_stake;

		GlobalStateStorage::<T>::put(global_state)
	}

	pub fn get_max_allowed_weights_global() -> u16 {
		GlobalStateStorage::<T>::get().max_allowed_weights
	}

	pub fn set_max_allowed_weights_global(max_allowed_weights: u16) {
		let mut global_state = GlobalStateStorage::<T>::get();

		global_state.max_allowed_weights = max_allowed_weights;

		GlobalStateStorage::<T>::put(global_state)
	}

	pub fn get_min_stake_global() -> u64 {
		GlobalStateStorage::<T>::get().min_stake
	}
	pub fn set_min_stake_global(min_stake: u64) {
		let mut global_state = GlobalStateStorage::<T>::get();

		global_state.min_stake = min_stake;

		GlobalStateStorage::<T>::put(global_state)
	}

	pub fn set_vote_mode_global(vote_mode: Vec<u8>) {
		let mut global_state = GlobalStateStorage::<T>::get();

		global_state.vote_mode = BoundedVec::<u8, ConstU32<32>>::try_from(vote_mode).expect("too long vote mode");

		GlobalStateStorage::<T>::put(global_state)
	}
	
	pub fn get_vote_mode_global() -> Vec<u8> {
		GlobalStateStorage::<T>::get().vote_mode.into_inner()
	}
	pub fn get_burn_rate() -> u16 {
		GlobalStateStorage::<T>::get().burn_rate
	}

	pub fn set_burn_rate(mut burn_rate: u16) {
		let mut global_state = GlobalStateStorage::<T>::get();

		global_state.burn_rate = burn_rate.min(100);

		GlobalStateStorage::<T>::put(global_state)
	}
	
	pub fn set_max_proposals(max_proposals: u64) {
		let mut global_state = GlobalStateStorage::<T>::get();

		global_state.max_proposals = max_proposals;

		GlobalStateStorage::<T>::put(global_state)
	}

	pub fn get_max_proposals() -> u64 {
		GlobalStateStorage::<T>::get().max_proposals
	}

	pub fn get_global_vote_threshold() -> u16 {
		GlobalStateStorage::<T>::get().vote_threshold
	}
	pub fn set_global_vote_threshold(vote_threshold: u16) {
		let mut global_state = GlobalStateStorage::<T>::get();

		global_state.vote_threshold = vote_threshold;

		GlobalStateStorage::<T>::put(global_state)
	}
	pub fn get_max_registrations_per_block() -> u16 {
		GlobalStateStorage::<T>::get().max_registrations_per_block
	}
	pub fn get_global_max_name_length() -> u16 {
		GlobalStateStorage::<T>::get().max_name_length
	}

	pub fn set_global_max_name_length(max_name_length: u16) {
		let mut global_state = GlobalStateStorage::<T>::get();

		global_state.max_name_length = max_name_length;

		GlobalStateStorage::<T>::put(global_state)
	}

	pub fn do_update_global(
		origin: T::RuntimeOrigin,
		params: GlobalParams,
	) -> DispatchResult {
		ensure_root(origin)?;

		ensure!(is_vec_str(Self::get_vote_mode_global(),"authority"), Error::<T>::InvalidVoteMode);
		
		Self::set_global_params(params.clone());
		
		Ok(())
	}

	pub fn global_n() -> u16 {
		let mut global_n : u16 = 0;

		for netuid in Self::netuids() {
			global_n += SubnetStateStorage::<T>::get(netuid).n;
		}

		return global_n
	}


	pub fn get_global_stake_to(
        key: &T::AccountId,
    ) -> u64 {
		// get all of the stake to
        let total_networks: u16 = GlobalStateStorage::<T>::get().total_subnets;
        let mut total_stake_to = 0;

        for netuid in 0..total_networks {
            total_stake_to += Self::get_total_stake_to(netuid, key);
        }

        total_stake_to
    }


	// Configure tx rate limiting
	pub fn get_tx_rate_limit() -> u64 {
		GlobalStateStorage::<T>::get().tx_rate_limit
	}
	pub fn set_tx_rate_limit(tx_rate_limit: u64) {
		let mut global_state = GlobalStateStorage::<T>::get();

		global_state.tx_rate_limit = tx_rate_limit;

		GlobalStateStorage::<T>::put(global_state)
	}

	pub fn set_min_burn( min_burn: u64) {
		let mut global_state = GlobalStateStorage::<T>::get();

		global_state.min_burn = min_burn;

		GlobalStateStorage::<T>::put(global_state)
	}

	pub fn get_min_burn() -> u64 {
		GlobalStateStorage::<T>::get().min_burn
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
