use crate::*;
use scale_info::TypeInfo;

#[derive(Decode, Encode, PartialEq, Eq, Clone, Debug, TypeInfo)]
#[scale_info(skip_type_params(T))]
pub struct ModuleParams<T: Config> {
    pub name: Vec<u8>,
    pub url: Vec<u8>,
    pub fees: ModuleFees,
    pub metadata: Option<Vec<u8>>,
    pub _pd: PhantomData<T>,
}

impl<T: Config> ModuleParams<T> {
    #[must_use]
    pub fn new(
        name: Vec<u8>,
        url: Vec<u8>,
        fees: ModuleFees,
        metadata: Option<Vec<u8>>,
    ) -> Self {
        Self {
            name: Some(name),
            url: Some(url),
            fees: Some(fees),
            metadata,
            _pd: PhantomData,
        }
    }

    #[deny(unused_variables)]
    #[must_use]
    pub fn update(
        params: &ModuleParams<T>,
        name: Vec<u8>,
        url: Vec<u8>,
        fees: Option<ModuleFees>,
        metadata: Option<Vec<u8>>,
    ) -> Self {
        let ModuleParams {
            name: old_name,
            url: old_url,
            fees: _,
            metadata: _,
            _pd: _,
        } = params;

        Self {
            name: (name != *old_name).then_some(name),
            url: (url != *old_url).then_some(url),
            fees,
            metadata,
            _pd: PhantomData,
        }
    }

    #[deny(unused_variables)]
    pub fn validate(&self, netuid: u16) -> Result<(), sp_runtime::DispatchError> {
        let Self {
            name,
            url,
            fees,
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

        if let Some(fees) = fees {
            ModuleValidator::validate_fees::<T>(fees)?;
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
            fees,
            metadata,
            _pd: _,
        } = self;

        if let Some(new_name) = name {
            Name::<T>::insert(uid, new_name);
        }

        if let Some(new_url) = url {
            Address::<T>::insert(uid, new_url);
        }

        if let Some(new_fees) = fees {
            ValidatorFeeConfig::<T>::insert(&key, new_fees);
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

    pub fn validate_fees<T: Config>(fees: &ModuleFees) -> Result<(), sp_runtime::DispatchError> {
        fees.validate::<T>().map_err(|_| Error::<T>::InvalidMinDelegationFee)?;
        Ok(())
    }
}

impl<T: Config> Pallet<T> {
    pub fn module_params( key: &T::AccountId) -> ModuleParams<T> {

    }
}
