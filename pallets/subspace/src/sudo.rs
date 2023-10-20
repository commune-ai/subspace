use super::*;
use frame_support::pallet_prelude::DispatchResult;
use sp_runtime::DispatchError;
use system::ensure_root;

impl<T: Config> Pallet<T> {
	pub fn do_sudo_set_unit_emission(
		origin: T::RuntimeOrigin,
		unit_emission: u64,
	) -> DispatchResult {
		ensure_root(origin)?;
		Self::set_unit_emission(unit_emission);
		log::info!("UnitEmissionSet( unit_emission: ${:?} )", unit_emission);
		Self::deposit_event(Event::UnitEmissionSet(unit_emission));
		Ok(())
	}

	pub fn do_sudo_set_tx_rate_limit(
		origin: T::RuntimeOrigin,
		tx_rate_limit: u64,
	) -> DispatchResult {
		ensure_root(origin)?;
		Self::set_tx_rate_limit(tx_rate_limit);
		log::info!("TxRateLimitSet( tx_rate_limit: ${:?} )", tx_rate_limit);
		Self::deposit_event(Event::TxRateLimitSet(tx_rate_limit));
		Ok(())
	}

	pub fn do_sudo_set_max_name_length(
		origin: T::RuntimeOrigin,
		max_name_length: u16,
	) -> DispatchResult {
		ensure_root(origin)?;
		Self::set_max_name_length(max_name_length);
		log::info!("MaxNameLengthSet( max_name_length: ${:?} )", max_name_length);
		Self::deposit_event(Event::MaxNameLengthSet(max_name_length));
		Ok(())
	}

	pub fn do_sudo_set_max_allowed_subnets(
		origin: T::RuntimeOrigin,
		max_allowed_subnets: u16,
	) -> DispatchResult {
		ensure_root(origin)?;
		Self::set_max_allowed_subnets(max_allowed_subnets);
		log::info!("MaxAllowedSubnetsSet ( max_allowed_subnets: ${:?} )", max_allowed_subnets);
		Self::deposit_event(Event::MaxAllowedSubnetsSet(max_allowed_subnets));
		Ok(())
	}

	pub fn do_sudo_set_max_allowed_modules(
		origin: T::RuntimeOrigin,
		max_allowed_modules: u16,
	) -> DispatchResult {
		ensure_root(origin)?;
		Self::set_max_allowed_modules(max_allowed_modules);
		log::info!("MaxAllowedModuelsSet ( max_allowed_modules: ${:?} )", max_allowed_modules);
		Self::deposit_event(Event::MaxAllowedModulesSet(max_allowed_modules));
		Ok(())
	}

    pub fn do_sudo_set_max_registrations_per_block(
		origin: T::RuntimeOrigin,
		max_registrations_per_block: u16,
	) -> DispatchResult {
		ensure_root(origin)?;
		Self::set_max_registrations_per_block(max_registrations_per_block);
		log::info!("MaxRegistrationsPerBlockSet ( max_registrations_per_block: ${:?} )", max_registrations_per_block);
		Self::deposit_event(Event::MaxRegistrationsPerBlockSet(max_registrations_per_block));
		Ok(())
	}
}
