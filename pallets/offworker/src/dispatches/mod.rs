use frame_support::pallet_macros::pallet_section;

#[pallet_section]
pub mod dispatches {
    #[pallet::call]
    impl<T: Config> Pallet<T> {
        // TODO: step 3 v2 of DEW will involve offworker sending potential zk proofs of encryption
        // correctness (proof that he can not decrypt certain weights)

        // # References
        // - [CS03] Practical Verifiable Encryption and Decryption of Discrete Logarithms
        //   Jan Camenisch and Victor Shoup, CRYPTO 2003
        //   https://link.springer.com/content/pdf/10.1007/978-3-540-45146-4_8.pdf

        // - [BBBPWM] Bulletproofs: Short Proofs for Confidential Transactions and More
        //   Benedikt Bünz, Jonathan Bootle, Dan Boneh, Andrew Poelstra, Pieter Wuille and Greg Maxwell, IEEE
        //   https://eprint.iacr.org/2017/1066.pdf

        // # Implementation
        // S&P 2018 validaity https://github.com/ZenGo-X/dlog-verifiable-enc
        #[pallet::call_index(0)]
        #[pallet::weight((Weight::zero(), DispatchClass::Normal, Pays::No))]
        pub fn send_decrypted_weights(
            origin: OriginFor<T>,
            payload: DecryptedWeightsPayload<T::Public, BlockNumberFor<T>>, /* make payload accept zk proofs of wrong encryption */
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

            ensure!(
                !decrypted_weights.is_empty(),
                Error::<T>::EmptyDecryptedWeights
            );

            // Get all epochs for this subnet
            let epoch_count = ConsensusParameters::<T>::iter_prefix(subnet_id).count();

            // Check if the length matches
            ensure!(
                decrypted_weights.len() == epoch_count,
                Error::<T>::DecryptedWeightsLengthMismatch
            );

            let has_weights = decrypted_weights.iter().any(|(_, inner_vec)| {
                inner_vec.iter().any(|(_, weight_vec, _)| !weight_vec.is_empty())
            });

            ensure!(has_weights, Error::<T>::EmptyDecryptedWeights);

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
                public_key: _,
                block_number,
                public,
            } = payload;

            pallet_subnet_emission::Pallet::<T>::handle_authority_node_ping(public.into_account());

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
