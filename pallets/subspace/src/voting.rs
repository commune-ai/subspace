use super::*;
use frame_support::pallet_prelude::DispatchResult;

#[derive(Clone, Debug, TypeInfo, Decode, Encode)]
#[scale_info(skip_type_params(T))]
pub struct CuratorApplication<T: Config> {
    pub id: u64,
    pub user_id: T::AccountId,
    pub paying_for: T::AccountId,
    pub data: Vec<u8>,
    pub status: ApplicationStatus,
    pub application_cost: u64,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, TypeInfo, Decode, Encode)]
pub enum ApplicationStatus {
    #[default]
    Pending,
    Accepted,
    Refused,
}

impl<T: Config> Pallet<T> {
    fn get_next_application_id() -> u64 {
        match CuratorApplications::<T>::iter_keys().max() {
            Some(id) => id + 1,
            None => 0,
        }
    }

    pub fn add_application(
        key: T::AccountId,
        application_key: T::AccountId,
        data: Vec<u8>,
    ) -> DispatchResult {
        // Check if the proposer has enough balance
        // re use the same value as for proposals
        let application_cost = GeneralSubnetApplicationCost::<T>::get();

        ensure!(
            Self::has_enough_balance(&key, application_cost),
            Error::<T>::NotEnoughtBalnceToApply
        );

        let removed_balance_as_currency = Self::u64_to_balance(application_cost);
        ensure!(
            removed_balance_as_currency.is_some(),
            Error::<T>::CouldNotConvertToBalance
        );

        let application_id = Self::get_next_application_id();

        let application = CuratorApplication {
            user_id: application_key,
            paying_for: key.clone(),
            id: application_id,
            data,
            status: ApplicationStatus::Pending,
            application_cost,
        };

        // Burn the application cost from the proposer's balance
        Self::remove_balance_from_account(&key, removed_balance_as_currency.unwrap())?;

        CuratorApplications::<T>::insert(application_id, application);

        Self::deposit_event(Event::<T>::ApplicationCreated(application_id));
        Ok(())
    }

    pub fn do_refuse_dao_application(
        origin: T::RuntimeOrigin,
        application_id: u64,
    ) -> DispatchResult {
        let key = ensure_signed(origin)?;

        // --- 2. Ensure that the key is the curator multisig.
        ensure!(Curator::<T>::get() == key, Error::<T>::NotCurator);

        let mut application =
            CuratorApplications::<T>::get(application_id).ok_or(Error::<T>::ApplicationNotFound)?;

        ensure!(
            application.status == ApplicationStatus::Pending,
            Error::<T>::ApplicationNotPending
        );

        // Change the status of application to refused
        application.status = ApplicationStatus::Refused;

        CuratorApplications::<T>::insert(application_id, application);

        Ok(())
    }

    pub fn do_add_dao_application(
        origin: T::RuntimeOrigin,
        application_key: T::AccountId,
        data: Vec<u8>,
    ) -> DispatchResult {
        let key = ensure_signed(origin)?;
        ensure!(!data.is_empty(), Error::<T>::ApplicationTooSmall);
        ensure!(data.len() <= 256, Error::<T>::ApplicationTooLarge);
        sp_std::str::from_utf8(&data).map_err(|_| Error::<T>::InvalidApplication)?;

        Self::add_application(key, application_key, data)
    }

    pub fn execute_application(user_id: &T::AccountId) -> DispatchResult {
        // Perform actions based on the application data type
        // The owners will handle the off-chain logic

        let mut application = CuratorApplications::<T>::iter_values()
            .find(|app| app.user_id == *user_id)
            .ok_or(Error::<T>::ApplicationNotFound)?;

        // Give the proposer back his tokens, if the application passed
        Self::add_balance_to_account(
            &application.paying_for,
            Self::u64_to_balance(application.application_cost).unwrap(),
        );
        application.status = ApplicationStatus::Accepted;

        CuratorApplications::<T>::insert(application.id, application);

        Ok(())
    }
}
