use crate::*;
use frame_support::pallet_prelude::MaxEncodedLen;
use scale_info::TypeInfo;
use sp_core::Get;
use sp_runtime::DispatchError;

/// This struct is used for both global (Subnet Burn) and MAP parameters (Module Burn)
#[derive(
    Clone, TypeInfo, Decode, Encode, PartialEq, Eq, frame_support::DebugNoBound, MaxEncodedLen,
)]
#[scale_info(skip_type_params(T))]
pub struct GeneralBurnConfiguration<T> {
    /// min burn the adjustment algorithm can set
    pub min_burn: u64,
    /// max burn the adjustment algorithm can set
    pub max_burn: u64,
    /// the steepness with which the burn curve will increase
    /// every interval
    pub adjustment_alpha: u64,
    /// interval in blocks for the burn to be adjusted
    pub target_registrations_interval: u16,
    /// the number of registrations expected per interval, if
    /// below, burn gets decreased, it is increased otherwise
    pub target_registrations_per_interval: u16,
    /// the maximum number of registrations accepted per interval
    pub max_registrations_per_interval: u16,
    pub _pd: PhantomData<T>,
}

pub enum BurnType {
    Subnet,
    Module,
}

impl<T: Config> Default for GeneralBurnConfiguration<T> {
    fn default() -> Self {
        Self::module_burn_default()
    }
}

impl<T: Config> GeneralBurnConfiguration<T> {
    fn subnet_burn_default() -> Self {
        Self {
            min_burn: T::DefaultSubnetMinBurn::get(),
            max_burn: 100_000_000_000_000,
            adjustment_alpha: u64::MAX / 2,
            target_registrations_interval: 5_400,
            target_registrations_per_interval: 1,
            max_registrations_per_interval: T::DefaultMaxSubnetRegistrationsPerInterval::get(),
            _pd: PhantomData,
        }
    }

    pub fn module_burn_default() -> Self {
        Self {
            min_burn: T::DefaultModuleMinBurn::get(),
            max_burn: 150_000_000_000,
            adjustment_alpha: u64::MAX / 2,
            target_registrations_interval: 142,
            target_registrations_per_interval: 3,
            max_registrations_per_interval: T::DefaultMaxRegistrationsPerInterval::get(),
            _pd: PhantomData,
        }
    }

    pub fn default_for(burn_type: BurnType) -> Self {
        match burn_type {
            BurnType::Subnet => Self::subnet_burn_default(),
            BurnType::Module => Self::module_burn_default(),
        }
    }

    #[deny(unused_variables)]
    pub fn apply_module_burn(self, netuid: u16) -> Result<(), DispatchError> {
        let Self {
            min_burn,
            max_burn,
            adjustment_alpha,
            target_registrations_interval,
            target_registrations_per_interval,
            max_registrations_per_interval,
            _pd: _,
        } = self;

        ensure!(
            min_burn >= T::DefaultModuleMinBurn::get(),
            Error::<T>::InvalidMinBurn
        );
        ensure!(max_burn > min_burn, Error::<T>::InvalidMaxBurn);
        ensure!(adjustment_alpha > 0, Error::<T>::InvalidAdjustmentAlpha);
        ensure!(
            target_registrations_interval >= 10,
            Error::<T>::InvalidTargetRegistrationsInterval
        );
        ensure!(
            target_registrations_per_interval >= 1,
            Error::<T>::InvalidTargetRegistrationsPerInterval
        );
        ensure!(
            max_registrations_per_interval >= 1,
            Error::<T>::InvalidMaxRegistrationsPerInterval
        );
        ensure!(
            max_registrations_per_interval >= target_registrations_per_interval,
            Error::<T>::InvalidMaxRegistrationsPerInterval
        );

        ModuleBurnConfig::<T>::set(netuid, self);

        Ok(())
    }
}
