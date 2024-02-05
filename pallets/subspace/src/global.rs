use super::*;
use frame_support::pallet_prelude::DispatchResult;
use sp_runtime::DispatchError;
use system::ensure_root;
use crate::utils::is_vec_str;
use sp_runtime::BoundedVec;

impl<T: Config> Pallet<T> {

	pub fn set_global_state(global_state: GlobalState) {
		GlobalStateStorage::<T>::put(global_state);
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
		GlobalParamsStorage::<T>::put(params);
	}

	pub fn get_min_weight_stake() -> u64 {
		Self::global_params().min_weight_stake
	}

	pub fn get_max_allowed_weights_global() -> u16 {
		Self::global_params().max_allowed_weights
	}


	pub fn get_vote_mode_global() -> Vec<u8> {
		Self::global_params().vote_mode
	}
	pub fn get_burn_rate() -> u16 {
		Self::global_params().burn_rate
	}

	pub fn get_max_proposals() -> u64 {
		Self::global_params().max_proposals
	}

	pub fn get_global_vote_threshold() -> u16 {
		Self::global_params().vote_threshold
	}
	pub fn get_max_registrations_per_block() -> u16 {
		Self::global_params().max_registrations_per_block
	}
	pub fn get_global_max_name_length() -> u16 {
		Self::global_params().max_name_length
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
			global_n += Self::subnet_state(netuid).n_uids;
		}

		return global_n
	}

	pub fn get_global_stake_to(
        key: &T::AccountId,
    ) -> u64 {
		// get all of the stake to
        let total_networks: u16 = Self::global_state().total_subnets;
        let mut total_stake_to = 0;

        for netuid in 0..total_networks {
            total_stake_to += Self::get_total_stake_to(netuid, key);
        }

        total_stake_to
    }

}
