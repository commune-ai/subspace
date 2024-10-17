use crate::*;
use frame_support::pallet_prelude::{BoundedVec, ConstU32, DispatchResult};
use frame_system::ensure_signed;
use pallet_subspace::Pallet as PalletSubspace;
use parity_scale_codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;

#[derive(Clone, Default, TypeInfo, Decode, Encode, MaxEncodedLen, frame_support::DebugNoBound)]
#[scale_info(skip_type_params(T))]
pub struct CuratorApplication<T: Config> {
    pub id: u64,
    pub user_id: T::AccountId,
    pub paying_for: T::AccountId,
    pub data: BoundedVec<u8, ConstU32<256>>,
    pub status: ApplicationStatus,
    pub application_cost: u64,
    pub block_number: u64,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, MaxEncodedLen, TypeInfo, Decode, Encode)]
pub enum ApplicationStatus {
    #[default]
    Pending,
    Accepted,
    Refused,
    Removed,
}

impl<T: Config> Pallet<T> {
    fn get_next_application_id() -> u64 {
        match CuratorApplications::<T>::iter_keys().max() {
            Some(id) => id.saturating_add(1),
            None => 0,
        }
    }

    #[must_use]
    fn can_add_application_status_based(key: &T::AccountId) -> bool {
        !CuratorApplications::<T>::iter().any(|(_, app)| app.user_id == *key)
    }

    pub fn add_application(
        key: T::AccountId,
        application_key: T::AccountId,
        data: Vec<u8>,
    ) -> DispatchResult {
        // make sure application isnt already whitelisted
        ensure!(
            !Self::is_in_legit_whitelist(&application_key),
            Error::<T>::AlreadyWhitelisted
        );
        // make sure application does not already exist
        ensure!(
            Self::can_add_application_status_based(&application_key),
            Error::<T>::ApplicationKeyAlreadyUsed
        );
        // check if the key has enough funds to file the application
        let application_cost = GeneralSubnetApplicationCost::<T>::get();
        ensure!(
            PalletSubspace::<T>::has_enough_balance(&key, application_cost),
            Error::<T>::NotEnoughBalanceToApply
        );
        // 1. a remove the balance from the account
        let Some(removed_balance_as_currency) =
            PalletSubspace::<T>::u64_to_balance(application_cost)
        else {
            return Err(Error::<T>::InvalidCurrencyConversionValue.into());
        };
        // add the application
        let application_id = Self::get_next_application_id();
        let current_block = PalletSubspace::<T>::get_current_block_number();

        let application = CuratorApplication {
            user_id: application_key,
            paying_for: key.clone(),
            id: application_id,
            data: BoundedVec::truncate_from(data),
            status: ApplicationStatus::Pending,
            application_cost,
            block_number: current_block,
        };

        // 1. b remove the balance from the account
        PalletSubspace::<T>::remove_balance_from_account(&key, removed_balance_as_currency)?;

        CuratorApplications::<T>::insert(application_id, application);

        Self::deposit_event(Event::ApplicationCreated(application_id));
        Ok(())
    }

    pub fn do_refuse_dao_application(
        origin: T::RuntimeOrigin,
        application_id: u64,
    ) -> DispatchResult {
        let key = ensure_signed(origin)?;

        // Ensure that the key is the curator multisig.
        ensure!(Curator::<T>::get() == key, Error::<T>::NotCurator);

        CuratorApplications::<T>::try_mutate(application_id, |application| match application {
            Some(app) if app.status == ApplicationStatus::Pending => {
                app.status = ApplicationStatus::Refused;
                Ok(())
            }
            Some(_) => Err(Error::<T>::ApplicationNotPending),
            None => Err(Error::<T>::ApplicationNotFound),
        })?;

        Ok(())
    }

    pub fn do_add_dao_application(
        origin: T::RuntimeOrigin,
        application_key: T::AccountId,
        data: Vec<u8>,
    ) -> DispatchResult {
        let key = ensure_signed(origin)?;
        ensure!(
            (1..=256).contains(&data.len()),
            Error::<T>::InvalidApplicationSize
        );
        ensure!(
            sp_std::str::from_utf8(&data).is_ok(),
            Error::<T>::InvalidApplication
        );

        Self::add_application(key, application_key, data)
    }

    pub fn execute_application(user_id: &T::AccountId) -> DispatchResult {
        // Perform actions based on the application data type
        // The owners will handle the off-chain logic

        let mut application = CuratorApplications::<T>::iter_values()
            .find(|app| app.user_id == *user_id)
            .ok_or(Error::<T>::ApplicationNotFound)?;

        // Give the proposer back his tokens, if the application passed
        PalletSubspace::<T>::add_balance_to_account(
            &application.paying_for,
            PalletSubspace::<T>::u64_to_balance(application.application_cost).unwrap(),
        );
        application.status = ApplicationStatus::Accepted;

        CuratorApplications::<T>::insert(application.id, application);

        Ok(())
    }

    pub fn do_add_to_whitelist(
        origin: T::RuntimeOrigin,
        module_key: T::AccountId,
    ) -> DispatchResult {
        let key = ensure_signed(origin)?;
        ensure!(Curator::<T>::get() == key, Error::<T>::NotCurator);

        // make sure application isnt already whitelisted
        ensure!(
            !Self::is_in_legit_whitelist(&module_key),
            Error::<T>::AlreadyWhitelisted
        );

        let application = CuratorApplications::<T>::iter_values()
            .find(|app| app.user_id == module_key)
            .ok_or(Error::<T>::ApplicationNotFound)?;

        ensure!(
            application.status == ApplicationStatus::Pending,
            Error::<T>::ApplicationNotPending
        );

        LegitWhitelist::<T>::insert(&module_key, ());

        T::execute_application(&module_key)?;

        Self::deposit_event(Event::WhitelistModuleAdded(module_key.clone()));

        Ok(())
    }

    pub fn do_remove_from_whitelist(
        origin: T::RuntimeOrigin,
        module_key: T::AccountId,
    ) -> DispatchResult {
        let key = ensure_signed(origin)?;
        ensure!(Curator::<T>::get() == key, Error::<T>::NotCurator);
        ensure!(
            Self::is_in_legit_whitelist(&module_key),
            Error::<T>::NotWhitelisted
        );

        LegitWhitelist::<T>::remove(&module_key);

        CuratorApplications::<T>::iter()
            .filter(|(_, app)| app.user_id == module_key)
            .for_each(|(id, mut app)| {
                app.status = ApplicationStatus::Removed;
                CuratorApplications::<T>::insert(id, app);
            });

        Self::deposit_event(Event::WhitelistModuleRemoved(module_key));

        Ok(())
    }

    // Util
    // ====

    pub fn curator_application_exists(module_key: &T::AccountId) -> bool {
        CuratorApplications::<T>::iter().any(|(_, application)| application.user_id == *module_key)
    }

    // Whitelist management
    pub fn is_in_legit_whitelist(account_id: &T::AccountId) -> bool {
        LegitWhitelist::<T>::contains_key(account_id)
    }
}
