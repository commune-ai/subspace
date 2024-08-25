#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use std::collections::BTreeMap;

use alloc::vec::Vec;
use frame_support::traits::Get;
use frame_system::{
    self as system,
    offchain::{AppCrypto, CreateSignedTransaction, SignedPayload, Signer, SigningTypes},
    pallet_prelude::BlockNumberFor,
};
use parity_scale_codec::{Decode, Encode};
use sp_core::crypto::KeyTypeId;
use sp_runtime::{
    offchain::storage::{StorageRetrievalError, StorageValueRef},
    RuntimeDebug,
};

/// Defines application identifier for crypto keys of this module.
///
/// Every module that deals with signatures needs to declare its unique identifier for
/// its crypto keys.
/// When offchain worker is signing transactions it's going to request keys of type
/// `KeyTypeId` from the keystore and use the ones it finds to sign the transaction.
/// The keys can be inserted manually via RPC (see `author_insertKey`).
pub const KEY_TYPE: KeyTypeId = KeyTypeId(*b"btc!");

/// Based on the above `KeyTypeId` we need to generate a pallet-specific crypto type wrappers.
/// We can use from supported crypto kinds (`sr25519`, `ed25519` and `ecdsa`) and augment
/// the types with this pallet-specific identifier.
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

pub use pallet::*;
use substrate_fixed::types::I64F64;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;

    /// This pallet's configuration trait
    #[pallet::config]
    pub trait Config:
        CreateSignedTransaction<Call<Self>> + frame_system::Config + pallet_subspace::Config
    {
        /// The identifier type for an offchain worker.
        type AuthorityId: AppCrypto<Self::Public, Self::Signature>;

        /// The overarching event type.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        // Configuration parameters

        /// A grace period after we send transaction.
        ///
        /// To avoid sending too many transactions, we only attempt to send one
        /// every `GRACE_PERIOD` blocks. We use Local Storage to coordinate
        /// sending between distinct runs of this offchain worker.
        #[pallet::constant]
        type GracePeriod: Get<BlockNumberFor<Self>>;

        /// Number of blocks of cooldown after unsigned transaction is included.
        ///
        /// This ensures that we only accept unsigned transactions once, every `UnsignedInterval`
        /// blocks.
        #[pallet::constant]
        type UnsignedInterval: Get<BlockNumberFor<Self>>;

        /// A configuration for base priority of unsigned transactions.
        ///
        /// This is exposed so that it can be tuned for particular runtime, when
        /// multiple pallets send unsigned transactions.
        #[pallet::constant]
        type UnsignedPriority: Get<TransactionPriority>;

        /// Maximum number of prices.
        #[pallet::constant]
        type MaxPrices: Get<u32>;
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        /// Offchain Worker entry point.
        ///
        /// By implementing `fn offchain_worker` you declare a new offchain worker.
        /// This function will be called when the node is fully synced and a new best block is
        /// successfully imported.
        /// Note that it's not guaranteed for offchain workers to run on EVERY block, there might
        /// be cases where some blocks are skipped, or for some the worker runs twice (re-orgs),
        /// so the code should be able to handle that.
        /// You can use `Local Storage` API to coordinate runs of the worker.
        fn offchain_worker(block_number: BlockNumberFor<T>) {
            for subnet_id in [0u16; 0] {
                let last: LastYuma<T> = todo!();
                let current: CurrentYuma<T> = todo!();

                if is_still_profitable(last, current) {
                    continue;
                }

                // | 0 | 1 | 2 | 3 | 4 | 5 |
                //                       ^ choose node F
                //                   ^ choose node E
                //               ^ choose node D
                //           ^ choose node C
                //       ^ choose node B
                //   ^ choose node A
            }

            log::info!("Hello World from offchain workers!");

            // Since off-chain workers are just part of the runtime code, they have direct access
            // to the storage and other included pallets.
            //
            // We can easily import `frame_system` and retrieve a block hash of the parent block.
            let parent_hash = <system::Pallet<T>>::block_hash(block_number - 1u32.into());
            log::debug!(
                "Current block: {:?} (parent hash: {:?})",
                block_number,
                parent_hash
            );
        }
    }

    /// A public part of the pallet.
    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::call_index(2)]
        #[pallet::weight({0})]
        pub fn submit_price_unsigned_with_signed_payload(
            origin: OriginFor<T>,
            _price_payload: WeightsPayload<T::Public, T::AccountId, BlockNumberFor<T>>,
            _signature: T::Signature,
        ) -> DispatchResultWithPostInfo {
            // This ensures that the function can only be called via unsigned transaction.
            ensure_none(origin)?;

            // now increment the block number at which we expect next unsigned transaction.
            // let current_block = <system::Pallet<T>>::block_number();
            // <NextUnsignedAt<T>>::put(current_block + T::UnsignedInterval::get());
            Ok(().into())
        }
    }

    /// Events for the pallet.
    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Event generated when new price is accepted to contribute to the average.
        NewPrice {
            price: u32,
            maybe_who: Option<T::AccountId>,
        },
    }

    /// A vector of recently submitted prices.
    ///
    /// This is used to calculate average price, should have bounded size.
    #[pallet::storage]
    pub(super) type Prices<T: Config> = StorageValue<_, BoundedVec<u32, T::MaxPrices>, ValueQuery>;

    /// Defines the block when next unsigned transaction will be accepted.
    ///
    /// To prevent spam of unsigned (and unpaid!) transactions on the network,
    /// we only allow one transaction every `T::UnsignedInterval` blocks.
    /// This storage entry defines when new transaction is going to be accepted.
    #[pallet::storage]
    pub(super) type NextUnsignedAt<T: Config> = StorageValue<_, BlockNumberFor<T>, ValueQuery>;
}

/// Payload used by this example crate to hold price
/// data required to submit a transaction.
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, scale_info::TypeInfo)]
pub struct WeightsPayload<Public, AccountId, BlockNumber> {
    subnet_id: u16,
    epoch: BlockNumber,
    module_key: AccountId,
    decrypted_weights: Vec<u8>,
    public: Public,
}

impl<T: SigningTypes> SignedPayload<T>
    for WeightsPayload<T::Public, T::AccountId, BlockNumberFor<T>>
{
    fn public(&self) -> T::Public {
        self.public.clone()
    }
}

impl<T: Config> Pallet<T> {
    /// Chooses which transaction type to send.
    ///
    /// This function serves mostly to showcase `StorageValue` helper
    /// and local storage usage.
    ///
    /// Returns a type of transaction that should be produced in current run.
    fn local_storage(block_number: BlockNumberFor<T>) {
        /// A friendlier name for the error that is going to be returned in case we are in the grace
        /// period.
        const RECENTLY_SENT: () = ();

        // Start off by creating a reference to Local Storage value.
        // Since the local storage is common for all offchain workers, it's a good practice
        // to prepend your entry with the module name.
        let val = StorageValueRef::persistent(b"example_ocw::last_send");
        // The Local Storage is persisted and shared between runs of the offchain workers,
        // and offchain workers may run concurrently. We can use the `mutate` function, to
        // write a storage entry in an atomic fashion. Under the hood it uses `compare_and_set`
        // low-level method of local storage API, which means that only one worker
        // will be able to "acquire a lock" and send a transaction if multiple workers
        // happen to be executed concurrently.
        let _res = val.mutate(
            |last_send: Result<Option<BlockNumberFor<T>>, StorageRetrievalError>| {
                match last_send {
                    // If we already have a value in storage and the block number is recent enough
                    // we avoid sending another transaction at this time.
                    Ok(Some(block)) if block_number < block + T::GracePeriod::get() => {
                        Err(RECENTLY_SENT)
                    }
                    // In every other case we attempt to acquire the lock and send a transaction.
                    _ => Ok(block_number),
                }
            },
        );
    }

    /// A helper function to fetch the price and send signed transaction.
    fn fetch_price_and_send_signed() -> Result<(), &'static str> {
        let signer = Signer::<T, T::AuthorityId>::all_accounts();
        if !signer.can_sign() {
            return Err(
                "No local accounts available. Consider adding one via `author_insertKey` RPC.",
            );
        }

        // Using `send_signed_transaction` associated type we create and submit a transaction
        // representing the call, we've just created.
        // Submit signed will return a vector of results for all accounts that were found in the
        // local keystore with expected `KEY_TYPE`.
        // let results = signer.send_signed_transaction(|_account| {
        //     // Received price is wrapped into a call to `submit_price` public function of this
        //     // pallet. This means that the transaction, when executed, will simply call that
        //     // function passing `price` as an argument.
        //     Call::submit_price { price }
        // });

        // for (acc, res) in &results {
        //     match res {
        //         Ok(()) => log::info!("[{:?}] Submitted price of {} cents", acc.id, price),
        //         Err(e) => log::error!("[{:?}] Failed to submit transaction: {:?}", acc.id, e),
        //     }
        // }

        Ok(())
    }
}

struct LastYuma<T: pallet_subspace::Config> {
    emissions: BTreeMap<T::AccountId, I64F64>,
    stakes: BTreeMap<T::AccountId, I64F64>,
}

struct CurrentYuma<T: pallet_subspace::Config> {
    emissions: BTreeMap<T::AccountId, I64F64>,
    stakes: BTreeMap<T::AccountId, I64F64>,
}

fn is_still_profitable<T: pallet_subspace::Config>(
    last: LastYuma<T>,
    current: CurrentYuma<T>,
) -> bool {
    true
}
