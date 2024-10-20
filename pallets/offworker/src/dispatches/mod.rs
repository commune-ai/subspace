use frame_support::pallet_macros::pallet_section;

#[pallet_section]
pub mod dispatches {
    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::call_index(0)]
        #[pallet::weight((Weight::zero(), DispatchClass::Normal, Pays::No))]
        pub fn send_decrypted_weights(
            origin: OriginFor<T>,
            payload: DecryptedWeightsPayload<T::Public, BlockNumberFor<T>>,
            _signature: T::Signature,
        ) -> DispatchResultWithPostInfo {
            // Signature valiadation is performed by the validate unsigned function
            ensure_none(origin)?;

            let DecryptedWeightsPayload {
                subnet_id,
                decrypted_weights,
                delta,
                block_number,
                public,
            } = payload;

            let decryption_data = SubnetDecryptionData::<T>::get(subnet_id);

            if let Some(decryption_data) = decryption_data {
                ensure!(
                    decryption_data.node_id == public.into_account(),
                    Error::<T>::InvalidDecryptionKey
                );
            } else {
                return Err(Error::<T>::InvalidSubnetId.into());
            }

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
        #[pallet::weight((Weight::zero(), DispatchClass::Normal, Pays::No))]
        pub fn send_ping(
            origin: OriginFor<T>,
            payload: KeepAlivePayload<T::Public, BlockNumberFor<T>>,
            _signature: T::Signature,
        ) -> DispatchResultWithPostInfo {
            // Signature valiadation is performed by the validate unsigned function
            ensure_none(origin)?;

            let KeepAlivePayload {
                public_key,
                block_number,
                public: _,
            } = payload;

            pallet_subnet_emission::Pallet::<T>::handle_authority_node_ping(public_key);

            Self::deposit_event(Event::KeepAliveSent { block_number });
            Ok(().into())
        }

        #[pallet::call_index(2)]
        #[pallet::weight((Weight::zero(), DispatchClass::Normal, Pays::No))]
        pub fn add_authorities(
            origin: OriginFor<T>,
            new_authorities: Vec<(T::AccountId, PublicKey)>,
        ) -> DispatchResultWithPostInfo {
            ensure_root(origin)?;

            Authorities::<T>::try_mutate(|authorities| {
                new_authorities.into_iter().try_for_each(|(account_id, public_key)| {
                    authorities
                        .try_push((account_id, public_key))
                        .map_err(|_| Error::<T>::TooManyAuthorities)
                })
            })?;

            Self::deposit_event(Event::AuthoritiesAdded);
            Ok(().into())
        }
    }
}
