use core::ops::Add;

use frame_support::{pallet_prelude::DispatchResult};

use super::*;

impl<T: Config> Pallet<T> {
    pub fn add_profit_share(
        origin: T::RuntimeOrigin,
        keys: Vec<T::AccountId>,
        shares: Vec<u16>
        
    ) -> DispatchResult {
        let key = ensure_signed(origin)?;

        // needs to be registered as a network
        ensure!(
            Self::is_key_registered_on_any_network(&key),
            Error::<T>::NotRegistered
        );

        
    }

}