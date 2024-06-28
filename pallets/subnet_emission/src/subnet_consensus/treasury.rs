use crate::EmissionError;
use core::marker::PhantomData;
use pallet_subspace::{Config, Pallet as PalletSubspace};

use super::yuma::AccountKey;
// Code structure to reflect other consensus types, this code is ready for additional features.
// Whenever needed.
pub struct TreasuryEpoch<T: Config> {
    founder_key: AccountKey<T>,
    founder_emission: u64,
    _pd: PhantomData<T>,
}

impl<T: Config> TreasuryEpoch<T> {
    pub fn new(_netuid: u16, founder_emission: u64) -> Self {
        let founder_key = T::get_dao_treasury_address();
        Self {
            founder_key: AccountKey(founder_key),
            founder_emission,
            _pd: PhantomData,
        }
    }

    pub fn run(&self) -> Result<(), EmissionError> {
        match PalletSubspace::<T>::u64_to_balance(self.founder_emission) {
            Some(balance) => {
                PalletSubspace::<T>::add_balance_to_account(&self.founder_key.0, balance);
                Ok(())
            }
            None => Err(EmissionError::BalanceConversionFailed),
        }
    }
}
