use crate::*;
// use frame_support::storage::with_storage_layer;

use frame_support::{pallet_prelude::DispatchResult, sp_runtime::DispatchError};
use frame_system::ensure_signed;
use sp_core::Get;
use sp_runtime::BoundedVec;
use substrate_fixed::types::I110F18;

impl<T: Config> Pallet<T> {
    /// Default Rootnetwork subnet id
    /// Registers a module in a subnet.
    /// 5. Emit a `ModuleRegistered` event.
    pub fn do_register(
        origin: T::RuntimeOrigin,
        network_name: Vec<u8>,
        name: Vec<u8>,
        url: Vec<u8>,
        module_key: T::AccountId,
        metadata: Option<Vec<u8>>,
    ) -> DispatchResult {
        let key = ensure_signed(origin.clone())?;

        ensure!(
            RegistrationsPerBlock::<T>::get() < MaxRegistrationsPerBlock::<T>::get(),
            Error::<T>::TooManyRegistrationsPerBlock
        );
        Self::register_module( &module_key, name, url, metadata)?;
        Ok(())
    }
    ///
    /// Emits a `ModuleDeregistered` event when successful.
    pub fn do_deregister(origin: T::RuntimeOrigin) -> DispatchResult {
        // --- 1. Check that the caller has signed the transaction.
        let key = ensure_signed(origin)?;
        // --- 2. Check that the module exists in the subnet.
        // --- 3. Remove the module from the subnet.
        Self::remove_module( key, true)?;

        // ensure the key is registered
        ensure!( !Self::key_registered( &key), Error::<T>::StillRegistered);
        // --- 4. Deposit the event
        Self::deposit_event(Event::ModuleDeregistered( key));
        // --- 5. Ok and done.
        Ok(())
    }

    // --------------------------
    // Registration Utils
    // --------------------------

    fn register_module(
        key: &T::AccountId,
        name: Vec<u8>,
        url: Vec<u8>,
        metadata: Option<Vec<u8>>,
    ) -> Result<u16, DispatchError> {
        let fees = DefaultValidatorFees::<T>::get();
        let module_changeset = ModuleParams::new(name, url, fees, metadata);
        Self::append_module(key, module_changeset)
    }

    fn finalize_registration( module_key: &T::AccountId) -> DispatchResult {
        ensure!(
            Self::key_registered(module_key),
            Error::<T>::ModuleDoesNotExist
        );

        Self::deposit_event(Event::ModuleRegistered(uid, module_key.clone()));

        Ok(())
    }

    // --- Util ---

    pub fn get_block_at_registration() -> Vec<u64> {
    }

    /// returns the amount of total modules on the network
    pub fn n_modules() -> u16 {
        N::<T>::get()
    }
}

