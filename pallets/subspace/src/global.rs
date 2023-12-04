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
		}
	}

	fn get_global_vote_threshold() -> u16 {
		return 50;
	}


	




	pub fn do_update_global(
		origin: T::RuntimeOrigin,
		max_name_length: u16,
		max_allowed_subnets: u16,
		max_allowed_modules: u16,
		max_registrations_per_block: u16,
		unit_emission: u64,
		tx_rate_limit: u64
	) -> DispatchResult {
		ensure_root(origin)?;

		if max_name_length > 0 {
			Self::set_max_name_length(max_name_length);
		}
		if max_allowed_subnets > 0 {
			Self::set_max_allowed_subnets(max_allowed_subnets);
		}
		if max_allowed_modules > 0 {
			Self::set_max_allowed_modules(max_allowed_modules);
		}
		if max_registrations_per_block > 0 {
			Self::set_max_registrations_per_block(max_registrations_per_block);
		}
		if unit_emission > 0 {
			Self::set_unit_emission(unit_emission);
		}
		if tx_rate_limit > 0 {
			Self::set_tx_rate_limit(tx_rate_limit);
		}
		Self::deposit_event(Event::GlobalUpdate(
			max_name_length,
			max_allowed_subnets,
			max_allowed_modules,
			max_registrations_per_block,
			unit_emission,
			tx_rate_limit,
		));
		Ok(())
	}


	pub fn get_total_global_stake(
        key: &T::AccountId,
    ) -> u64 {
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
