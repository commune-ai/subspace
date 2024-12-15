use crate::*;
use scale_info::TypeInfo;

#[derive(Decode, Encode, PartialEq, Eq, Clone, Debug, TypeInfo)]
#[scale_info(skip_type_params(T))]
pub struct ModuleParams<T: Config> {
    pub name: Vec<u8>,
    pub address: Vec<u8>,
    pub fees: ValidatorFees,
    pub metadata: Option<Vec<u8>>,
    pub _pd: PhantomData<T>,
}

#[derive(Debug)]
pub struct ModuleChangeset<T: Config> {
    pub name: Option<Vec<u8>>,
    pub address: Option<Vec<u8>>,
    pub fees: Option<ValidatorFees>,
    pub metadata: Option<Vec<u8>>,
    pub _pd: PhantomData<T>,
}

impl<T: Config> ModuleChangeset<T> {
    #[must_use]
    pub fn new(
        name: Vec<u8>,
        address: Vec<u8>,
        fees: ValidatorFees,
        metadata: Option<Vec<u8>>,
    ) -> Self {
        Self {
            name: Some(name),
            address: Some(address),
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
        address: Vec<u8>,
        fees: Option<ValidatorFees>,
        metadata: Option<Vec<u8>>,
    ) -> Self {
        let ModuleParams {
            name: old_name,
            address: old_address,
            fees: _,
            metadata: _,
            _pd: _,
        } = params;

        Self {
            name: (name != *old_name).then_some(name),
            address: (address != *old_address).then_some(address),
            fees,
            metadata,
            _pd: PhantomData,
        }
    }

    #[deny(unused_variables)]
    pub fn validate(&self, netuid: u16) -> Result<(), sp_runtime::DispatchError> {
        let Self {
            name,
            address,
            fees,
            metadata,
            _pd: _,
        } = self;

        let max_length = MaxNameLength::<T>::get() as usize;
        let min_length = MinNameLength::<T>::get() as usize;

        if let Some(name) = name {
            ModuleValidator::validate_name::<T>(name, min_length, max_length, netuid)?;
        }

        if let Some(address) = address {
            ModuleValidator::validate_address::<T>(address, max_length)?;
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
        netuid: u16,
        key: T::AccountId,
        uid: u16,
    ) -> Result<(), sp_runtime::DispatchError> {
        self.validate(netuid)?;

        let Self {
            name,
            address,
            fees,
            metadata,
            _pd: _,
        } = self;

        if let Some(new_name) = name {
            Name::<T>::insert(netuid, uid, new_name);
        }

        if let Some(new_address) = address {
            Address::<T>::insert(netuid, uid, new_address);
        }

        if let Some(new_fees) = fees {
            ValidatorFeeConfig::<T>::insert(&key, new_fees);
        }

        if let Some(new_metadata) = metadata {
            Metadata::<T>::insert(netuid, &key, new_metadata);
        }

        Pallet::<T>::deposit_event(Event::ModuleUpdated(netuid, key));
        Ok(())
    }
}

pub struct ModuleValidator;

impl ModuleValidator {
    pub fn validate_name<T: Config>(
        name: &[u8],
        min_length: usize,
        max_length: usize,
        netuid: u16,
    ) -> Result<(), sp_runtime::DispatchError> {
        ensure!(!name.is_empty(), Error::<T>::InvalidModuleName);
        ensure!(name.len() <= max_length, Error::<T>::ModuleNameTooLong);
        ensure!(name.len() >= min_length, Error::<T>::ModuleNameTooShort);
        core::str::from_utf8(name).map_err(|_| Error::<T>::InvalidModuleName)?;
        ensure!(
            !Name::<T>::iter_prefix_values(netuid).any(|existing| existing == name),
            Error::<T>::ModuleNameAlreadyExists
        );
        Ok(())
    }

    pub fn validate_address<T: Config>(
        address: &[u8],
        max_length: usize,
    ) -> Result<(), sp_runtime::DispatchError> {
        ensure!(!address.is_empty(), Error::<T>::InvalidModuleAddress);
        ensure!(
            address.len() <= max_length,
            Error::<T>::ModuleAddressTooLong
        );
        core::str::from_utf8(address).map_err(|_| Error::<T>::InvalidModuleAddress)?;
        Ok(())
    }

    pub fn validate_metadata<T: Config>(metadata: &[u8]) -> Result<(), sp_runtime::DispatchError> {
        ensure!(!metadata.is_empty(), Error::<T>::InvalidModuleMetadata);
        ensure!(metadata.len() <= 120, Error::<T>::ModuleMetadataTooLong);
        core::str::from_utf8(metadata).map_err(|_| Error::<T>::InvalidModuleMetadata)?;
        Ok(())
    }

    pub fn validate_fees<T: Config>(fees: &ValidatorFees) -> Result<(), sp_runtime::DispatchError> {
        fees.validate::<T>().map_err(|_| Error::<T>::InvalidMinDelegationFee)?;
        Ok(())
    }
}

impl<T: Config> Pallet<T> {
    pub fn module_params(netuid: u16, key: &T::AccountId, uid: u16) -> ModuleParams<T> {
        ModuleParams {
            name: Name::<T>::get(netuid, uid),
            address: Address::<T>::get(netuid, uid),
            fees: ValidatorFeeConfig::<T>::get(key),
            metadata: Metadata::<T>::get(netuid, key),
            _pd: PhantomData,
        }
    }
}
