use super::*;
use frame_support::pallet_prelude::DispatchResult;
use sp_runtime::DispatchError;
use system::ensure_root;

impl<T: Config> Pallet<T> {
	pub fn do_update_global(
		origin: T::RuntimeOrigin,
		max_name_length: u16,
		max_allowed_subnets: u16,
		max_allowed_modules: u16,
		max_registrations_per_block: u16,
		unit_emission: u64,
		tx_rate_limit: u64,
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
}
