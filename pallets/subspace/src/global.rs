use super::*;
use frame_support::pallet_prelude::DispatchResult;
use sp_runtime::DispatchError;
use system::ensure_root;

impl<T: Config> Pallet<T> {


	pub fn set_global_params(params: GlobalParams) {
		Self::set_max_name_length(params.max_name_length);
		Self::set_max_allowed_subnets(params.max_allowed_subnets);
		Self::set_max_allowed_modules(params.max_allowed_modules);
		Self::set_max_registrations_per_block(params.max_registrations_per_block);
		Self::set_unit_emission(params.unit_emission);
		Self::set_tx_rate_limit(params.tx_rate_limit);
		Self::set_global_vote_threshold(params.vote_threshold);
		Self::set_max_proposals(params.max_proposals);

	}

	pub fn set_max_proposals(max_proposals: u64) {
		MaxProposals::<T>::put(max_proposals);
	}

	pub fn get_max_proposals() -> u64 {
		MaxProposals::<T>::get()
	}

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
			vote_mode: Self::get_global_vote_mode(),
		}
	}

	pub fn get_global_vote_mode() -> Vec<u8> {
		return GlobalVoteMode::<T>::get();
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

	pub fn do_update_global(
		origin: T::RuntimeOrigin,
		params: GlobalParams,
	) -> DispatchResult {
		ensure_root(origin)?;
		Self::check_global_params(params.clone())?;
		Self::set_global_params(params.clone());
		Ok(())
	}


	pub fn get_total_global_stake(
        key: &T::AccountId,
    ) -> u64 {
		// get all of the stake to
        let total_networks: u16 = TotalSubnets::<T>::get();
        let mut total_stake_to = 0;

        for netuid in 0..total_networks {
            total_stake_to += Self::get_total_stake_to(netuid, key);
        }

        total_stake_to
    }


    pub fn check_global_params(params: GlobalParams) -> DispatchResult{
        // checks if params are valid

        // check if the name already exists
        ensure!(params.max_name_length > 0, "Invalid max_name_length");
        ensure!(params.max_allowed_subnets > 0, "Invalid max_allowed_subnets");
        ensure!(params.max_allowed_modules > 0, "Invalid max_allowed_modules");
        ensure!(params.max_registrations_per_block > 0, "Invalid max_registrations_per_block");
        ensure!(params.unit_emission > 0, "Invalid unit_emission");
        ensure!(params.tx_rate_limit > 0, "Invalid tx_rate_limit");
        Ok(())
    }




}
