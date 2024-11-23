// make sure that not only yuma subnets work
#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use alloc::vec::Vec;
use frame_support::{pallet_macros::import_section, sp_runtime::DispatchError, traits::Get};
use frame_system::{
    self as system,
    offchain::{
        AppCrypto, CreateSignedTransaction, SendUnsignedTransaction, SignedPayload, Signer,
        SigningTypes,
    },
    pallet_prelude::BlockNumberFor,
};
use pallet_subnet_emission::{
    subnet_consensus::{
        util::{
            consensus::ConsensusOutput,
            params::{ConsensusParams, ModuleKey, ModuleParams},
        },
        yuma::YumaEpoch,
    },
    types::{BlockWeights, PublicKey},
    Authorities, SubnetDecryptionData,
};

use sp_std::collections::btree_map::BTreeMap;

use pallet_subnet_emission::{ConsensusParameters, Weights};
use pallet_subspace::{
    math::{inplace_normalize_64, vec_fixed64_to_fixed32},
    Consensus, CopierMargin, MinFees,
};
use parity_scale_codec::{Decode, Encode};
use scale_info::prelude::marker::PhantomData;
use sp_core::{crypto::KeyTypeId, hexdisplay::HexDisplay};
use sp_runtime::{
    offchain::storage::StorageValueRef,
    traits::{BlakeTwo256, Hash, IdentifyAccount},
    transaction_validity::{InvalidTransaction, TransactionValidity, ValidTransaction},
    Percent,
};
use substrate_fixed::types::I32F32;
use types::{
    ConsensusSimulationResult, DecryptedWeightsPayload, KeepAlivePayload, ShouldDecryptResult,
};
use util::process_consensus_params;

mod dispatches;
mod process;
mod profitability;
pub mod types;
mod util;

/// Defines application identifier for crypto keys of this module.
///
/// Every module that deals with signatures needs to declare its unique identifier for
/// its crypto keys.
/// When offchain worker is signing transactions it's going to request keys of type
/// `KeyTypeId` from the keystore and use the ones it finds to sign the transaction.
/// The keys can be inserted manually via RPC (see `author_insertKey`).
pub const KEY_TYPE: KeyTypeId = KeyTypeId(*b"wcs!");

/// Cryptography configuration for pallet.
///
/// Based on the above `KeyTypeId` we need to generate a
/// pallet-specific crypto type wrappers.
/// We can use from supported crypto kinds
/// (`sr25519`, `ed25519` and `ecdsa`) and augment
/// the types with this pallet-specific identifier.
pub mod crypto {
    use super::KEY_TYPE;
    use alloc::string::String;
    use sp_core::sr25519::Signature as Sr25519Signature;
    use sp_runtime::{
        app_crypto::{app_crypto, sr25519},
        traits::Verify,
        MultiSignature, MultiSigner,
    };
    app_crypto!(sr25519, KEY_TYPE);

    pub struct AuthId;

    impl frame_system::offchain::AppCrypto<MultiSigner, MultiSignature> for AuthId {
        type RuntimeAppPublic = Public;
        type GenericSignature = sp_core::sr25519::Signature;
        type GenericPublic = sp_core::sr25519::Public;
    }

    impl frame_system::offchain::AppCrypto<<Sr25519Signature as Verify>::Signer, Sr25519Signature>
        for AuthId
    {
        type RuntimeAppPublic = Public;
        type GenericSignature = sp_core::sr25519::Signature;
        type GenericPublic = sp_core::sr25519::Public;
    }
}

pub use pallet::*;
use substrate_fixed::types::I64F64;

#[import_section(dispatches::dispatches)]
#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::{
        pallet_prelude::{ValueQuery, *},
        Identity,
    };
    use frame_system::pallet_prelude::*;
    /// This pallet's configuration trait
    #[pallet::config]
    pub trait Config:
        CreateSignedTransaction<Call<Self>>
        + frame_system::Config
        + pallet_subspace::Config
        + pallet_subnet_emission::Config
    {
        /// The identifier type for an offchain worker.
        type AuthorityId: AppCrypto<Self::Public, Self::Signature>;

        /// The overarching nevent type.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        #[pallet::constant]
        type UnsignedPriority: Get<TransactionPriority>;
    }

    #[pallet::validate_unsigned]
    impl<T: Config> ValidateUnsigned for Pallet<T> {
        type Call = Call<T>;

        fn validate_unsigned(_source: TransactionSource, call: &Self::Call) -> TransactionValidity {
            match call {
                Call::send_decrypted_weights { payload, signature } => {
                    let block_number = payload.block_number;
                    Self::validate_signature_and_authority(&block_number, payload, signature)?;
                    Self::validate_unsigned_transaction(&block_number, "DecryptedWeights")
                }
                Call::send_ping { payload, signature } => {
                    let block_number = payload.block_number;
                    Self::validate_signature_and_authority(&block_number, payload, signature)?;
                    Self::validate_unsigned_transaction(&block_number, "KeepAlive")
                }
                _ => InvalidTransaction::Call.into(),
            }
        }
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_initialize(_block_number: BlockNumberFor<T>) -> Weight {
            Weight::zero()
        }

        // ! This function is not actually guaranteed to run on every block
        fn offchain_worker(block_number: BlockNumberFor<T>) {
            log::info!("Offchain worker is running");

            if !ow_extensions::offworker::is_decryption_node() {
                return;
            }

            let block_number =
                block_number.try_into().ok().expect("blockchain won't pass 2^64 blocks");

            if let Err(e) = Self::do_send_ping(block_number) {
                log::error!("Error sending keep alive: {:?}", e);
                return;
            }
            // Make sure that we have encryption key available
            let Some(public) = ow_extensions::offworker::get_encryption_key() else {
                return;
            };

            log::info!(
                "Node public key - modulus: {}, exponent: {}",
                HexDisplay::from(&public.0),
                HexDisplay::from(&public.1)
            );

            #[cfg(feature = "testing-offworker")]
            let (valid_subnets, hanging_subnets) =
                pallet_subnet_emission::Pallet::<T>::get_valid_subnets(None);

            #[cfg(not(feature = "testing-offworker"))]
            let acc_id = match Signer::<T, T::AuthorityId>::any_account()
                .accounts_from_keys()
                .next()
                .map(|account| account.id)
            {
                Some(id) => id,
                None => {
                    log::error!("No local accounts available");
                    return;
                }
            };

            #[cfg(feature = "testing-offworker")]
            let acc_id =
                T::AccountId::decode(&mut sp_runtime::traits::TrailingZeroInput::zeroes()).unwrap();

            // The valid subnets are ones that have encrypted weights, with matching key of the
            // offchain worker. The hanging, are potential subnets that have turned off the
            // encryption in the middle of offchain worker process
            #[cfg(not(feature = "testing-offworker"))]
            let (valid_subnets, hanging_subnets) =
                pallet_subnet_emission::Pallet::<T>::get_valid_subnets(Some(&acc_id));

            // The runtime mimics this logic, by deleting all storages related to consenus
            // parameters and weights
            hanging_subnets.iter().for_each(|subnet_id| {
                log::warn!("Deleting subnet: {}", subnet_id);
                Self::delete_subnet_state(subnet_id);
            });

            log::info!("Valid subnets: {:?}", valid_subnets);
            let deregistered_subnets = Self::process_subnets(valid_subnets, acc_id, block_number);
            deregistered_subnets.iter().for_each(|subnet_id| {
                log::info!("Deregistered subnet: {}", subnet_id);
                Self::delete_subnet_state(subnet_id);
            });
        }
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Offchain worker sent decrypted weights
        DecryptedWeightsSent {
            subnet_id: u16,
            block_number: BlockNumberFor<T>,
        },
        /// Offchain worker sent keep_alive message
        KeepAliveSent { block_number: BlockNumberFor<T> },
        /// New authorities were successfully added
        AuthoritiesAdded,
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Decryption key is invalid for a given subnet
        InvalidDecryptionKey,
        /// Subnet ID is invalid
        InvalidSubnetId,
        /// Attempted to add more authorities than the maximum allowed
        TooManyAuthorities,
        /// Attempted to send empty decrypted weights
        EmptyDecryptedWeights,
        /// Send more decrypted weights than expected epoch count
        DecryptedWeightsLengthMismatch,
    }

    // 10 % of total active stake
    #[pallet::type_value]
    pub fn DefaultMeasuredStakeAmount<T: Config>() -> Percent {
        Percent::from_percent(10u8)
    }

    /// The amount of actual consensus sum stake. Used for a simulated consensus.
    #[pallet::storage]
    pub type MeasuredStakeAmount<T: Config> =
        StorageValue<_, Percent, ValueQuery, DefaultMeasuredStakeAmount<T>>;

    /// The amount of delta between comulative copier dividends and compulative delegator dividends.
    #[pallet::storage]
    pub type IrrationalityDelta<T: Config> = StorageMap<_, Identity, u16, I64F64, ValueQuery>;
}

impl<T: Config> Pallet<T> {
    fn validate_signature_and_authority<P: SignedPayload<T>>(
        block_number: &BlockNumberFor<T>,
        payload: &P,
        signature: &T::Signature,
    ) -> Result<(), InvalidTransaction> {
        let current_block = <system::Pallet<T>>::block_number();
        if &current_block < block_number {
            return Err(InvalidTransaction::Future);
        }

        log::info!("Validating signature and authority");
        // Verify the signature, this just ensures the signature matches the public key
        if !SignedPayload::<T>::verify::<T::AuthorityId>(payload, signature.clone()) {
            log::info!("Authority Signature is invalid");
            return Err(InvalidTransaction::BadProof);
        }

        // Check if the signer is a valid authority
        let account_id = payload.public().clone().into_account();
        let authorities = Authorities::<T>::get();
        if !authorities.iter().any(|(account, _)| account == &account_id) {
            log::info!("Signer is not an authority");
            return Err(InvalidTransaction::BadSigner);
        }

        Ok(())
    }

    fn validate_unsigned_transaction(
        block_number: &BlockNumberFor<T>,
        tag_prefix: &'static str,
    ) -> TransactionValidity {
        ValidTransaction::with_tag_prefix(tag_prefix)
            .priority(T::UnsignedPriority::get())
            .and_provides(block_number)
            .longevity(5)
            .propagate(true)
            .build()
    }

    /// Decrypts all subnet weights for a given netuid. Returns a vector of block weights,
    /// maintaining the length of consensus parameters. Failed decryptions result in empty vectors.
    /// V2 TODO: make more efficient in v2, we can be just caching this
    pub(crate) fn decrypt_all_subnet_weighs(netuid: u16) -> Vec<BlockWeights> {
        ConsensusParameters::<T>::iter_prefix(netuid)
            .map(|(block_number, consensus_params)| {
                let uid_weights: Vec<_> = consensus_params
                    .modules
                    .iter()
                    .map(|(_, module_params)| {
                        if !module_params.weight_encrypted.is_empty() {
                            ow_extensions::offworker::decrypt_weight(
                                module_params.weight_encrypted.clone(),
                            )
                            .map(|(weights, key)| (module_params.uid, weights, key))
                            .unwrap_or_else(|| (module_params.uid, Vec::new(), Vec::new()))
                        } else {
                            (module_params.uid, Vec::new(), Vec::new())
                        }
                    })
                    .collect();

                (block_number, uid_weights)
            })
            .collect()
    }

    fn do_send_weights(
        subnet_id: u16,
        delta: I64F64,
        forced_send_by_rotation: bool,
    ) -> Result<(), &'static str> {
        let signer = Signer::<T, T::AuthorityId>::all_accounts();
        if !signer.can_sign() {
            return Err(
                "No local accounts available. Consider adding one via `author_insertKey` RPC.",
            );
        }

        let decrypted_weights = Self::decrypt_all_subnet_weighs(subnet_id);

        log::info!("Sending decrypted weights to subnet {}", subnet_id);

        // Sends unsigned transaction with a signed payload
        let results = signer.send_unsigned_transaction(
            |account| DecryptedWeightsPayload {
                subnet_id,
                decrypted_weights: decrypted_weights.clone(),
                delta,
                block_number: <system::Pallet<T>>::block_number(),
                public: account.public.clone(),
                forced_send_by_rotation,
            },
            |payload, signature| Call::send_decrypted_weights { payload, signature },
        );

        for (_acc, res) in &results {
            let status = match res {
                Ok(()) => {
                    log::info!(
                        "Successfully sent decrypted weights to subnet {}",
                        subnet_id
                    );
                    "Successfully"
                }
                Err(e) => {
                    // This case hopefully never happens, the runtime should just accept it
                    log::error!(
                        "Failed to send decrypted weights to subnet {}: {:?}",
                        subnet_id,
                        e
                    );
                    "Failed"
                }
            };

            log::info!("{} sent decrypted weights to subnet {}", status, subnet_id);
            Self::delete_subnet_state(&subnet_id);
        }

        Ok(())
    }

    // Get this from onchain storage, this should not run on every block but `KeepAlive` interval
    fn do_send_ping(current_block: u64) -> Result<(), DispatchError> {
        let storage = StorageValueRef::persistent(b"last_keep_alive");

        if storage.get::<u64>().ok().flatten().map_or(true, |last| {
            current_block.saturating_sub(last) >= T::PingInterval::get()
        }) {
            let public_key = ow_extensions::offworker::get_encryption_key()
                .ok_or(DispatchError::Other("Failed to get encryption key"))?;

            let signer = Signer::<T, T::AuthorityId>::all_accounts();
            if !signer.can_sign() {
                return Err(DispatchError::Other(
                    "No local accounts available. Consider adding one via `author_insertKey` RPC.",
                ));
            }

            signer
                .send_unsigned_transaction(
                    |account| KeepAlivePayload {
                        public_key: public_key.clone(),
                        block_number: current_block.try_into().ok().unwrap_or_default(),
                        public: account.public.clone(),
                    },
                    |payload, signature| Call::send_ping { payload, signature },
                )
                .into_iter()
                .try_for_each(|(_, result)| {
                    result.map_err(|e| {
                        log::error!("Failed to send keep-alive transaction: {:?}", e);
                        DispatchError::Other("Failed to send keep-alive transaction")
                    })
                })?;
            storage.set(&current_block);
        }
        Ok(())
    }
}
