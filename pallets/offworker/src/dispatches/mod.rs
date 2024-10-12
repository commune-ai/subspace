use frame_support::pallet_macros::pallet_section;

#[pallet_section]
pub mod dispatches {
    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::call_index(0)]
        #[pallet::weight((
        Weight::from_parts(0, 0)
        .saturating_add(T::DbWeight::get().reads(0))
        .saturating_add(T::DbWeight::get().writes(0)),
        DispatchClass::Operational,
        Pays::No
    ))]
        pub fn send_decrypted_weights(
            origin: OriginFor<T>,
            subnet_id: u16,
            decrypted_weights: Vec<(u64, Vec<(u16, Vec<(u16, u16)>)>)>,
            delta: I64F64,
        ) -> DispatchResultWithPostInfo {
            ensure_none(origin)?;

            IrrationalityDelta::<T>::set(subnet_id, delta);

            pallet_subnet_emission::Pallet::<T>::handle_decrypted_weights(
                subnet_id,
                decrypted_weights,
            );

            Ok(().into())
        }

        #[pallet::call_index(1)]
        #[pallet::weight((
        Weight::from_parts(0, 0)
        .saturating_add(T::DbWeight::get().reads(0))
        .saturating_add(T::DbWeight::get().writes(0)),
        DispatchClass::Operational,
        Pays::No
    ))]
        pub fn send_keep_alive(
            origin: OriginFor<T>,
            public_key: (Vec<u8>, Vec<u8>),
        ) -> DispatchResultWithPostInfo {
            ensure_none(origin)?;

            pallet_subnet_emission::Pallet::<T>::handle_authority_node_keep_alive(public_key);

            Ok(().into())
        }
    }
}
