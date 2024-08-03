#![cfg_attr(not(feature = "std"), no_std)]

use frame_system::offchain::{SignedPayload, SigningTypes};
use parity_scale_codec::{Decode, Encode};
use sp_core::RuntimeDebug;
use sp_runtime::{traits::StaticLookup, KeyTypeId};

pub use pallet::*;

type AccountIdLookupOf<T> = <<T as frame_system::Config>::Lookup as StaticLookup>::Source;

#[frame_support::pallet]
pub mod pallet {
    #![allow(clippy::too_many_arguments)]

    use super::*;
    use frame_support::pallet_prelude::*;
    use frame_system::{
        offchain::CreateSignedTransaction,
        pallet_prelude::{BlockNumberFor, OriginFor},
    };
    pub use sp_std::{vec, vec::Vec};

    const STORAGE_VERSION: StorageVersion = StorageVersion::new(0);

    #[pallet::pallet]
    #[pallet::storage_version(STORAGE_VERSION)]
    pub struct Pallet<T>(_);

    #[pallet::config(with_default)]
    pub trait Config: CreateSignedTransaction<Call<Self>> + frame_system::Config {
        #[pallet::no_default_bounds]
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_initialize(block_number: BlockNumberFor<T>) -> Weight {
            log::info!("on_initialize: {block_number}");

            Weight::zero()
        }

        fn offchain_worker(block_number: BlockNumberFor<T>) {
            log::info!("offchain_worker: {block_number}");

            let decrypted = testthing::offworker::decrypt_weight(b"detpyrcne".to_vec());
            log::info!("{:?}", core::str::from_utf8(&decrypted));
        }
    }

    #[pallet::event]
    pub enum Event<T: Config> {}

    #[pallet::error]
    pub enum Error<T> {}

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        // TODO: we have to think of a way to initialize the first node authorities list

        #[pallet::call_index(0)]
        #[pallet::weight(10_000)]
        pub fn update_weights(
            origin: OriginFor<T>,
            payload: WeightsPayload<T::AccountId, BlockNumberFor<T>>,
            _signature: T::Signature,
        ) -> DispatchResultWithPostInfo {
            // Implementation goes here
            unimplemented!()
        }
    }

    #[pallet::validate_unsigned]
    impl<T: Config> ValidateUnsigned for Pallet<T> {
        type Call = Call<T>;

        /// Validate unsigned call to this module.
        ///
        /// By default unsigned transactions are disallowed, but implementing the validator
        /// here we make sure that some particular calls (the ones produced by offchain worker)
        /// are being whitelisted and marked as valid.
        fn validate_unsigned(_source: TransactionSource, call: &Self::Call) -> TransactionValidity {
            // Firstly let's check that we call the right function.
            if let Call::update_weights {
                payload,
                ref signature,
            } = call
            {
                let signature_valid =
                    SignedPayload::<T>::verify::<T::AuthorityId>(payload, signature.clone());
                if !signature_valid {
                    return InvalidTransaction::BadProof.into();
                }
                Self::validate_transaction_parameters(&payload.block_number, &payload.price)
            } else {
                InvalidTransaction::Call.into()
            }
        }
    }
}

/// Payload used by this example crate to hold price
/// data required to submit a transaction.
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, scale_info::TypeInfo)]
pub struct WeightsPayload<AccountId, BlockNumber> {
    subnet_id: u16,
    epoch: BlockNumber,
    module_key: AccountId,
    decrypted_weights: Vec<u8>,
}

impl<T: SigningTypes> SignedPayload<T> for WeightsPayload<T::Public, BlockNumberFor<T>> {
    fn public(&self) -> T::Public {
        self.public.clone()
    }
}

/// Defines application identifier for crypto keys of this module.
///
/// Every module that deals with signatures needs to declare its unique identifier for
/// its crypto keys.
/// When offchain worker is signing transactions it's going to request keys of type
/// `KeyTypeId` from the keystore and use the ones it finds to sign the transaction.
/// The keys can be inserted manually via RPC (see `author_insertKey`).
pub const KEY_TYPE: KeyTypeId = KeyTypeId(*b"btc!");

pub mod crypto {
    use super::KEY_TYPE;
    use sp_core::sr25519::Signature as Sr25519Signature;
    use sp_runtime::{
        app_crypto::{app_crypto, sr25519},
        traits::Verify,
        MultiSignature, MultiSigner,
    };
    app_crypto!(sr25519, KEY_TYPE);

    pub struct TestAuthId;

    impl frame_system::offchain::AppCrypto<MultiSigner, MultiSignature> for TestAuthId {
        type RuntimeAppPublic = Public;
        type GenericSignature = sp_core::sr25519::Signature;
        type GenericPublic = sp_core::sr25519::Public;
    }

    // implemented for mock runtime in test
    impl frame_system::offchain::AppCrypto<<Sr25519Signature as Verify>::Signer, Sr25519Signature>
        for TestAuthId
    {
        type RuntimeAppPublic = Public;
        type GenericSignature = sp_core::sr25519::Signature;
        type GenericPublic = sp_core::sr25519::Public;
    }
}
