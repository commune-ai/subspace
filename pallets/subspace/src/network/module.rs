use crate::*;

use frame_support::pallet_prelude::DispatchResult;

impl<T: Config> Pallet<T> {


    pub fn do_update_module(
        origin: T::RuntimeOrigin,
        module_params: ModuleParams<T>,
    ) -> DispatchResult {
        let key = ensure_signed(origin)?;
        Ok(())
    }

    pub fn append_module(
        key: &T::AccountId,
        changeset: ModuleParams<T>,
    ) -> Result<u16, sp_runtime::DispatchError> {
        // --- Get The Next Uid ---

        // -- Initialize All Storages ---
        // Make sure this overwrites the defaults (keep it second)
        changeset.apply(key.clone(), uid)?;

        // --- Update The Network Module Size ---
        N::<T>::mutate(|n| *n = n.saturating_add(1));

        // --- Initilaize Stake Storage ---
        Self::increase_stake(key, key, 0);

        Ok(uid)
    }

    /// Replace the module under this uid.
    pub fn remove_module( key: &T::AccountId,,) -> DispatchResult {
        // 1. Check if network has any modules
        let n = N::<T>::get();
        if n == 0 {
            return Ok(());
        }
        log::debug!(
            "remove_module(| key: {:?} ) ",
            key
        );

        // 9. Update network size
        let module_count = N::<T>::mutate(|v| {
            *v = v.saturating_sub(1);
            *v
        });

        Ok(())
    }
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
        Self::remove_module( uid, true)?;
        ensure!(
            !Self::key_registered( &key),
            Error::<T>::StillRegistered
        );

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


}