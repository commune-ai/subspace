use super::*;
use frame_support::pallet_prelude::DispatchResult;
use sp_runtime::DispatchError;
use system::ensure_root;

impl<T: Config> Pallet<T> {
	pub fn do_update_global(
		origin: T::RuntimeOrigin,
		max_name_length: Option<u16>,
		max_allowed_subnets: Option<u16>,
		max_allowed_modules: Option<u16>,
		max_registrations_per_block: Option<u16>,
		unit_emission: Option<u64>,
		tx_rate_limit: Option<u64>,
	) -> DispatchResult {
		ensure_root(origin)?;

		if max_name_length.is_some() {
			Self::set_max_name_length(max_name_length.unwrap());
		}
		if max_allowed_subnets.is_some() {
			Self::set_max_allowed_subnets(max_allowed_subnets.unwrap());
		}
		if max_allowed_modules.is_some() {
			Self::set_max_allowed_modules(max_allowed_modules.unwrap());
		}
		if max_registrations_per_block.is_some() {
			Self::set_max_registrations_per_block(max_registrations_per_block.unwrap());
		}
		if unit_emission.is_some() {
			Self::set_unit_emission(unit_emission.unwrap());
		}
		if tx_rate_limit.is_some() {
			Self::set_tx_rate_limit(tx_rate_limit.unwrap());
		}
		Self::deposit_event(Event::GlobalUpdate(
			Self::get_max_name_length(),
			Self::get_max_allowed_subnets(),
			Self::get_max_allowed_modules(),
			Self::get_max_registrations_per_block(),
			Self::get_unit_emission(),
			Self::get_tx_rate_limit(),
		));
		Ok(())
	}
}
