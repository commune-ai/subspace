use crate::*;
use frame_support::{
    pallet_prelude::*,
    traits::{Currency, ExistenceRequirement},
};
use frame_system::ensure_signed;
use scale_info::TypeInfo;
use sp_runtime::BoundedVec;
use sp_std::marker::PhantomData;
//. import config
use frame_system::Config;
use frame_support::traits::Get;
#[derive(Decode, Encode, PartialEq, Eq, Clone, Debug, TypeInfo)]
#[scale_info(skip_type_params(T))]
pub struct ModuleParams<T: Config> {
    pub name: Option<Vec<u8>>,
    pub url: Option<Vec<u8>>,
    pub metadata: Option<Vec<u8>>,
    pub _pd: PhantomData<T>,
}

impl<T: Config> ModuleParams<T> {
    #[must_use]
    pub fn new(
        name: Vec<u8>,
        url: Vec<u8>,
        metadata: Option<Vec<u8>>,
    ) -> Self {
        Self {
            name: Some(name),
            url: Some(url),
            metadata,
            _pd: PhantomData,
        }
    }

    #[deny(unused_variables)]
    #[must_use]
    pub fn update(
        params: &ModuleParams<T>,
        name: Option<Vec<u8>>,
        url: Option<Vec<u8>>,
        metadata: Option<Vec<u8>>,
    ) -> Self {
        let ModuleParams {
            name: old_name,
            url: old_url,
            metadata: _,
            _pd: _,
        } = params;

        Self {
            name: name.filter(|n| n != old_name.as_ref().unwrap()),
            url: url.filter(|u| u != old_url.as_ref().unwrap()),
            metadata,
            _pd: PhantomData,
        }
    }

    #[deny(unused_variables)]
    pub fn validate(&self) -> Result<(), sp_runtime::DispatchError> {
        let Self {
            name,
            url,
            metadata,
            _pd: _,
        } = self;

        let max_length = MaxNameLength::<T>::get() as usize;
        let min_length = MinNameLength::<T>::get() as usize;

        if let Some(name) = name {
            ModuleValidator::validate_name::<T>(name, min_length, max_length)?;
        }

        if let Some(url) = url {
            ModuleValidator::validate_url::<T>(url, max_length)?;
        }

        if let Some(metadata) = metadata {
            ModuleValidator::validate_metadata::<T>(metadata)?;
        }

        Ok(())
    }

    #[deny(unused_variables)]
    pub fn apply(
        self,
        key: T::AccountId,
    ) -> Result<(), sp_runtime::DispatchError> {

        let Self {
            name,
            url,
            metadata,
            _pd: _,
        } = self;

        if let Some(new_name) = name {
            Name::<T>::insert(&key, new_name);
        }

        if let Some(new_url) = url {
            Address::<T>::insert(&key, new_url);
        }

        if let Some(new_metadata) = metadata {
            Metadata::<T>::insert(&key, new_metadata);
        }

        Pallet::<T>::deposit_event(Event::ModuleUpdated( key));
        Ok(())
    }
}

pub struct ModuleValidator;

impl ModuleValidator {
    pub fn validate_name<T: Config>(
        name: &[u8],
        min_length: usize,
        max_length: usize,
    ) -> Result<(), sp_runtime::DispatchError> {
        ensure!(!name.is_empty(), Error::<T>::InvalidModuleName);
        ensure!(name.len() <= max_length, Error::<T>::ModuleNameTooLong);
        ensure!(name.len() >= min_length, Error::<T>::ModuleNameTooShort);
        core::str::from_utf8(name).map_err(|_| Error::<T>::InvalidModuleName)?;
        ensure!(
            !Name::<T>::iter_prefix_values().any(|existing| existing == name),
            Error::<T>::ModuleNameAlreadyExists
        );
        Ok(())
    }

    pub fn validate_url<T: Config>(
        url: &[u8],
        max_length: usize,
    ) -> Result<(), sp_runtime::DispatchError> {
        ensure!(!url.is_empty(), Error::<T>::InvalidModuleAddress);
        ensure!(
            url.len() <= max_length,
            Error::<T>::ModuleAddressTooLong
        );
        core::str::from_utf8(url).map_err(|_| Error::<T>::InvalidModuleAddress)?;
        Ok(())
    }

    pub fn validate_metadata<T: Config>(metadata: &[u8]) -> Result<(), sp_runtime::DispatchError> {
        ensure!(!metadata.is_empty(), Error::<T>::InvalidModuleMetadata);
        ensure!(metadata.len() <= 120, Error::<T>::ModuleMetadataTooLong);
        core::str::from_utf8(metadata).map_err(|_| Error::<T>::InvalidModuleMetadata)?;
        Ok(())
    }

}

impl<T: Config> Pallet<T> {


    pub fn update_module(
        origin: OriginFor<T>,
        name: Vec<u8>,
        url: Vec<u8>, 
        metadata: Option<Vec<u8>>,
    ) -> DispatchResult {
        let key = ensure_signed(origin.clone())?;
        ensure!(
            Self::is_registered(&key),
            Error::<T>::ModuleDoesNotExist
        );
        let params = Self::module_params(&key);
        let changeset = ModuleParams::update(&params, name, url, metadata);
        Self::do_update_module(origin, changeset)
    }

    pub fn do_update_module(
        origin: OriginFor<T>,
        changeset: ModuleParams<T>,
    ) -> DispatchResult {
        let key = ensure_signed(origin.clone())?;
        changeset.validate()?;
        changeset.apply(key)?;
        Ok(())
    }

    pub fn append_module(
        key: &T::AccountId,
        changeset: ModuleParams<T>,
    ) -> Result<u64, sp_runtime::DispatchError> {

        // -- Initialize All Storages ---
        // Make sure this overwrites the defaults (keep it second)
        changeset.apply(key.clone())?;

        // --- Update The Network Module Size ---
        N::<T>::mutate(|n| *n = n.saturating_add(1));

        // --- Initilaize Stake Storage ---
        Self::increase_stake(key, key, 0);

        Ok(N::<T>::get())
    }

    /// Replace the module under this uid.
    pub fn remove_module( key: &T::AccountId) -> DispatchResult {
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
        Self::remove_module( &key, true)?;
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
        let module_changeset = ModuleParams::new(name, url, metadata);
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
        let module_changeset = ModuleParams::new(name, url, metadata);
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
        Vec::new() // Return empty vector for now
    }
    /// returns the amount of total modules on the network
    pub fn n_modules() -> u16 {
        N::<T>::get()
    }

    // --- Util ---


}