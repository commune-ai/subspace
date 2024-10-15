use frame_support::pallet_macros::pallet_section;

#[pallet_section]
pub mod dispatches {
    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::call_index(0)]
        #[pallet::weight(Weight::zero())]
        pub fn send_decrypted_weights(
            origin: OriginFor<T>,
            payload: DecryptedWeightsPayload<T::Public, BlockNumberFor<T>>,
            _signature: T::Signature,
        ) -> DispatchResultWithPostInfo {
            ensure_none(origin)?;

            let DecryptedWeightsPayload {
                subnet_id,
                decrypted_weights,
                delta,
                block_number,
                public,
            } = payload;

            // Perform your existing logic here
            IrrationalityDelta::<T>::set(subnet_id, delta);
            pallet_subnet_emission::Pallet::<T>::handle_decrypted_weights(
                subnet_id,
                decrypted_weights,
            );

            Self::deposit_event(Event::DecryptedWeightsSent {
                subnet_id,
                block_number,
            });
            Ok(().into())
        }

        #[pallet::call_index(1)]
        #[pallet::weight(Weight::zero())]
        pub fn send_keep_alive(
            origin: OriginFor<T>,
            payload: KeepAlivePayload<T::Public, BlockNumberFor<T>>,
            _signature: T::Signature,
        ) -> DispatchResultWithPostInfo {
            ensure_none(origin)?;

            let KeepAlivePayload {
                public_key,
                block_number,
                public,
            } = payload;

            // Perform your existing logic here
            pallet_subnet_emission::Pallet::<T>::handle_authority_node_keep_alive(public_key);

            Self::deposit_event(Event::KeepAliveSent { block_number });
            Ok(().into())
        }

        #[pallet::call_index(2)]
        #[pallet::weight((
            Weight::zero(),
            DispatchClass::Operational,
            Pays::No
        ))]
        pub fn add_authorities(origin: OriginFor<T>) -> DispatchResultWithPostInfo {
            ensure_root(origin)?;
            // TODO:  // add an extrinsic that will insert a new authority
            Ok(().into())
        }
    }
}
