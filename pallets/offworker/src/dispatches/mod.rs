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
            payload: DecryptedWeightsPayload<T::Public, BlockNumberFor<T>>,
            _signature: T::Signature,
        ) -> DispatchResultWithPostInfo {
            // Signature valiadation is performed by the validate unsigned function
            ensure_none(origin)?;

            let DecryptedWeightsPayload {
                subnet_id,
                decrypted_weights,
                delta: _,
                block_number,
                public,
                forced_send_by_rotation,
            } = payload;

            log::info!(
                "Decrypted weights for subnet {} received at block {:?}",
                subnet_id,
                block_number
            );

            let acc_id = public.into_account();

            // Modify this section to handle the forced rotation case
            SubnetDecryptionData::<T>::try_mutate(subnet_id, |maybe_data| -> DispatchResult {
                let decryption_data = maybe_data.as_mut().ok_or(Error::<T>::InvalidSubnetId)?;

                log::info!(
                    "checking if decryption key is correct at subnet {}",
                    subnet_id
                );

                // If this was a forced rotation send, clear the rotating_from field
                match forced_send_by_rotation {
                    true => {
                        ensure!(
                            matches!(decryption_data.rotating_from, Some(ref rotating_from) if acc_id == *rotating_from),
                            Error::<T>::InvalidDecryptionKey
                        );
                    }
                    false => {
                        ensure!(
                            decryption_data.node_id == acc_id,
                            Error::<T>::InvalidDecryptionKey
                        );
                    }
                }

                log::info!(
                    "checking if decryption key is rotating at subnet {}",
                    subnet_id
                );
                // If this was a forced rotation send, clear the rotating_from field
                if forced_send_by_rotation {
                    decryption_data.rotating_from = None;
                }

                Ok(())
            })?;

            log::info!("checking epoch count at subnet {subnet_id}");

            // Rest of the function remains the same
            let epoch_count = ConsensusParameters::<T>::iter_prefix(subnet_id).count();

            ensure!(
                decrypted_weights.len() == epoch_count,
                Error::<T>::DecryptedWeightsLengthMismatch
            );

            log::info!("setting irrationality delta to 0 at subnet {}", subnet_id);
            // TODO: make a periodical irrationality delta reseter that subnet owner can control
            // IrrationalityDelta::<T>::mutate(subnet_id, |current| {
            //     *current = current.saturating_add(delta)
            // });
            IrrationalityDelta::<T>::set(subnet_id, I64F64::from_num(0));

            log::info!("setting decrypted weights at subnet {}", subnet_id);
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
