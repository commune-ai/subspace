use frame_support::pallet_macros::pallet_section;

#[pallet_section]
pub mod dispatches {
    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::call_index(1)]
        #[pallet::weight((T::WeightInfo::add_stake(), DispatchClass::Normal, Pays::No))]
        pub fn add_stake(
            origin: OriginFor<T>,
            module_key: T::AccountId,
            amount: u64,
        ) -> DispatchResult {
            Self::do_add_stake(origin, module_key, amount)
        }

        #[pallet::call_index(2)]
        #[pallet::weight((T::WeightInfo::remove_stake(), DispatchClass::Normal, Pays::No))]
        pub fn remove_stake(
            origin: OriginFor<T>,
            module_key: T::AccountId,
            amount: u64,
        ) -> DispatchResult {
            Self::do_remove_stake(origin, module_key, amount)
        }

        #[pallet::call_index(3)]
        #[pallet::weight((T::WeightInfo::add_stake_multiple(), DispatchClass::Normal, Pays::No))]
        pub fn add_stake_multiple(
            origin: OriginFor<T>,
            module_keys: Vec<T::AccountId>,
            amounts: Vec<u64>,
        ) -> DispatchResult {
            Self::do_add_stake_multiple(origin, module_keys, amounts)
        }

        #[pallet::call_index(4)]
        #[pallet::weight((T::WeightInfo::remove_stake_multiple(), DispatchClass::Normal, Pays::No))]
        pub fn remove_stake_multiple(
            origin: OriginFor<T>,
            module_keys: Vec<T::AccountId>,
            amounts: Vec<u64>,
        ) -> DispatchResult {
            Self::do_remove_stake_multiple(origin, module_keys, amounts)
        }

        #[pallet::call_index(5)]
        #[pallet::weight((T::WeightInfo::transfer_stake(), DispatchClass::Normal, Pays::No))]
        pub fn transfer_stake(
            origin: OriginFor<T>,
            module_key: T::AccountId,
            new_module_key: T::AccountId,
            amount: u64,
        ) -> DispatchResult {
            Self::do_transfer_stake(origin, module_key, new_module_key, amount)
        }

        // todo transfer stake multiple

        #[pallet::call_index(6)]
        #[pallet::weight((T::WeightInfo::transfer_multiple(), DispatchClass::Normal, Pays::No))]
        pub fn transfer_multiple(
            origin: OriginFor<T>,
            destinations: Vec<T::AccountId>,
            amounts: Vec<u64>,
        ) -> DispatchResult {
            Self::do_transfer_multiple(origin, destinations, amounts)
        }

        #[pallet::call_index(7)]
        #[pallet::weight((T::WeightInfo::register(), DispatchClass::Normal, Pays::No))]
        pub fn register(
            origin: OriginFor<T>,
            network_name: Vec<u8>,
            name: Vec<u8>,
            url: Vec<u8>,
            module_key: T::AccountId,
            metadata: Option<Vec<u8>>,
        ) -> DispatchResult {
            Self::do_register(origin, network_name, name, url, module_key, metadata)
        }

        #[pallet::call_index(8)]
        #[pallet::weight((T::WeightInfo::deregister(), DispatchClass::Normal, Pays::No))]
        pub fn deregister(origin: OriginFor<T>) -> DispatchResult {
            Self::do_deregister(origin)
        }

        #[pallet::call_index(9)]
        #[pallet::weight((T::WeightInfo::deregister(), DispatchClass::Normal, Pays::No))]
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
            let params = Self::module_params(&key, uid);
            let changeset = ModuleParams::update(&params, name, url, metadata);
            Self::do_update_module(origin, changeset)
        }


}
}