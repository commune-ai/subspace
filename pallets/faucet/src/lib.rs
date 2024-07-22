#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{dispatch::DispatchResult, ensure, LOG_TARGET};
use frame_system::{self as system, ensure_none, pallet_prelude::BlockNumberFor};
use pallet_subspace::Pallet as PalletSubspace;
use sp_core::{keccak_256, sha2_256, Get, H256, U256};
use sp_runtime::{traits::StaticLookup, DispatchError, MultiAddress};

pub use pallet::*;
use parity_scale_codec::Encode;

type AccountIdLookupOf<T> = <<T as frame_system::Config>::Lookup as StaticLookup>::Source;

#[frame_support::pallet]
pub mod pallet {
    #![allow(clippy::too_many_arguments)]

    use super::*;
    use frame_support::{pallet_prelude::*, traits::Currency};
    use frame_system::pallet_prelude::*;
    use pallet_subspace::N;
    pub use sp_std::{vec, vec::Vec};

    const STORAGE_VERSION: StorageVersion = StorageVersion::new(0);

    #[pallet::pallet]
    #[pallet::storage_version(STORAGE_VERSION)]
    pub struct Pallet<T>(_);

    #[pallet::config(with_default)]
    pub trait Config: frame_system::Config + pallet_subspace::Config {
        /// The events emitted on proposal changes.
        #[pallet::no_default_bounds]
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// Currency type that will be used to place deposits on modules
        type Currency: Currency<Self::AccountId> + Send + Sync;
    }

    #[pallet::validate_unsigned]
    impl<T: Config> ValidateUnsigned for Pallet<T> {
        type Call = Call<T>;

        fn validate_unsigned(_: TransactionSource, call: &Self::Call) -> TransactionValidity {
            #[allow(unused_variables)]
            let Call::faucet {
                block_number,
                nonce,
                work,
                key,
            } = call
            else {
                return InvalidTransaction::Call.into();
            };

            let key = T::Lookup::lookup(key.clone())?;

            let key_balance = PalletSubspace::<T>::get_balance_u64(&key);
            let key_stake: u64 = N::<T>::iter()
                .map(|_| pallet_subspace::Pallet::<T>::get_owned_stake(&key))
                .sum();
            let total_worth = key_balance.saturating_add(key_stake);
            if total_worth >= 50_000_000_000_000 {
                // if it's larger than 50k don't allow more funds
                return InvalidTransaction::Custom(0).into();
            }

            ValidTransaction::with_tag_prefix("RunFaucet")
                .priority(0) // Faucet, so low priority
                .longevity(5) // 5 blocks longevity to prevent too much spam
                .and_provides(key)
                .propagate(true)
                .build()
        }

        fn pre_dispatch(_: &Self::Call) -> Result<(), TransactionValidityError> {
            Ok(())
        }
    }

    pub type BalanceOf<T> =
        <<T as Config>::Currency as Currency<<T as system::Config>::AccountId>>::Balance;

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        // ---------------------------------
        // Testnet
        // ---------------------------------

        #[pallet::call_index(1)]
        #[pallet::weight((
            Weight::from_parts(85_000_000, 0)
            .saturating_add(T::DbWeight::get().reads(16))
            .saturating_add(T::DbWeight::get().writes(28)),
            DispatchClass::Operational,
            Pays::No
        ))]
        pub fn faucet(
            origin: OriginFor<T>,
            block_number: u64,
            nonce: u64,
            work: Vec<u8>,
            key: AccountIdLookupOf<T>,
        ) -> DispatchResult {
            Self::do_faucet(origin, block_number, nonce, work, key)
        }
    }

    // ---------------------------------
    // Events
    // ---------------------------------

    #[pallet::event]
    #[pallet::generate_deposit(pub(crate) fn deposit_event)]
    pub enum Event<T: Config> {
        // faucet
        Faucet(T::AccountId, u64), // (id, balance_to_add)
    }

    // ---------------------------------
    // Errors
    // ---------------------------------

    #[pallet::error]
    pub enum Error<T> {
        /// The work block provided is invalid.
        InvalidWorkBlock,
        /// The difficulty provided does not meet the required criteria.
        InvalidDifficulty,
        /// The seal provided is invalid or does not match the expected value.
        InvalidSeal,
    }
}

// ---------------------------------
// Pallet Implementation
// ---------------------------------

impl<T: Config> Pallet<T> {
    // Make sure this can never panic
    pub fn do_faucet(
        origin: T::RuntimeOrigin,
        block_number: u64,
        nonce: u64,
        work: Vec<u8>,
        key: AccountIdLookupOf<T>,
    ) -> DispatchResult {
        // --- 1. Validate unsigned
        ensure_none(origin)?;

        let key = T::Lookup::lookup(key)?;

        if <frame_system::Pallet<T>>::account(&key) == Default::default() {
            <frame_system::Pallet<T>>::inc_providers(&key);
        }

        log::info!(
            "do faucet with key: {key:?} and block number: {block_number} and nonce: {nonce}"
        );

        // --- 2. Ensure the passed block number is valid, not in the future or too old.
        // Work must have been done within 3 blocks (stops long range attacks).
        let current_block_number: u64 = PalletSubspace::<T>::get_current_block_number();
        ensure!(
            block_number <= current_block_number,
            Error::<T>::InvalidWorkBlock
        );
        ensure!(
            current_block_number.saturating_sub(block_number) < 3,
            Error::<T>::InvalidWorkBlock
        );

        // --- 3. Ensure the supplied work passes the difficulty.
        let difficulty: U256 = U256::from(1_000_000); // Base faucet difficulty.
        let work_hash: H256 = H256::from_slice(&work);
        ensure!(
            Self::hash_meets_difficulty(&work_hash, difficulty),
            Error::<T>::InvalidDifficulty
        ); // Check that the work meets difficulty.

        // --- 4. Check Work is the product of the nonce, the block number, and hotkey. Add this as
        // used work.
        let seal: H256 = Self::create_seal_hash(block_number, nonce, &key)?;
        ensure!(seal == work_hash, Error::<T>::InvalidSeal);

        // --- 5. Add Balance via faucet. 15 tokens
        let amount: u64 = 15_000_000_000;
        let balance_to_add = PalletSubspace::<T>::u64_to_balance(amount).unwrap();
        PalletSubspace::<T>::add_balance_to_account(&key, balance_to_add);

        // --- 6. Deposit successful event.
        log::info!("faucet done successfully with key: {key:?} and amount: {balance_to_add:?})");
        Self::deposit_event(Event::Faucet(key, amount));

        // --- 7. Ok and done.
        Ok(())
    }

    pub fn hash_block_and_key(
        block_hash_bytes: &[u8; 32],
        key: &T::AccountId,
    ) -> Result<H256, sp_runtime::DispatchError> {
        // Get the public key from the account id.
        let key_pubkey: MultiAddress<_, ()> = MultiAddress::Id(key.clone());
        let binding = key_pubkey.encode();
        // Skip extra 0th byte.
        let key_bytes = binding.get(1..).ok_or(pallet_subspace::Error::<T>::ExtrinsicPanicked)?;
        let mut full_bytes = [0u8; 64];
        let (first_half, second_half) = full_bytes.split_at_mut(32);
        first_half.copy_from_slice(block_hash_bytes);
        // Safe because Substrate guarantees that all AccountId types are at least 32 bytes
        second_half.copy_from_slice(
            key_bytes.get(..32).ok_or(pallet_subspace::Error::<T>::ExtrinsicPanicked)?,
        );
        let keccak_256_seal_hash_vec: [u8; 32] = keccak_256(&full_bytes[..]);

        Ok(H256::from_slice(&keccak_256_seal_hash_vec))
    }

    pub fn create_seal_hash(
        block_number_u64: u64,
        nonce_u64: u64,
        hotkey: &T::AccountId,
    ) -> Result<H256, DispatchError> {
        let nonce = nonce_u64.to_le_bytes();
        let block_hash_at_number: H256 = Self::get_block_hash_from_u64(block_number_u64);
        let block_hash_bytes: &[u8; 32] = block_hash_at_number.as_fixed_bytes();
        let binding = Self::hash_block_and_key(block_hash_bytes, hotkey)?;
        let block_and_hotkey_hash_bytes: &[u8; 32] = binding.as_fixed_bytes();

        let mut full_bytes = [0u8; 40];
        let (first_chunk, second_chunk) = full_bytes.split_at_mut(8);
        first_chunk.copy_from_slice(&nonce);
        second_chunk.copy_from_slice(block_and_hotkey_hash_bytes);
        let sha256_seal_hash_vec: [u8; 32] = sha2_256(&full_bytes[..]);
        let keccak_256_seal_hash_vec: [u8; 32] = keccak_256(&sha256_seal_hash_vec);
        let seal_hash: H256 = H256::from_slice(&keccak_256_seal_hash_vec);

        log::trace!(
            "hotkey:{hotkey:?} \nblock_number: {block_number_u64:?}, \nnonce_u64: {nonce_u64:?}, \nblock_hash: {block_hash_at_number:?}, \nfull_bytes: {full_bytes:?}, \nsha256_seal_hash_vec: {sha256_seal_hash_vec:?},  \nkeccak_256_seal_hash_vec: {keccak_256_seal_hash_vec:?}, \nseal_hash: {seal_hash:?}",
        );

        Ok(seal_hash)
    }

    pub fn get_block_hash_from_u64(block_number: u64) -> H256 {
        let block_number: BlockNumberFor<T> = block_number.try_into().unwrap_or_else(|_| {
            panic!("Block number {block_number} is too large to be converted to BlockNumberFor<T>")
        });
        let block_hash_at_number = frame_system::Pallet::<T>::block_hash(block_number);
        let vec_hash: Vec<u8> = block_hash_at_number.as_ref().to_vec();
        let real_hash: H256 = H256::from_slice(&vec_hash);

        log::trace!(
            target: LOG_TARGET,
            "block_number: vec_hash: {vec_hash:?}, real_hash: {real_hash:?}",
        );

        real_hash
    }

    // Determine whether the given hash satisfies the given difficulty.
    // The test is done by multiplying the two together. If the product
    // overflows the bounds of U256, then the product (and thus the hash)
    // was too high.
    pub fn hash_meets_difficulty(hash: &H256, difficulty: U256) -> bool {
        let bytes: &[u8] = hash.as_bytes();
        let num_hash: U256 = U256::from(bytes);
        let (value, overflowed) = num_hash.overflowing_mul(difficulty);

        log::trace!(
            target: LOG_TARGET,
            "Difficulty: hash: {hash:?}, hash_bytes: {bytes:?}, hash_as_num: {num_hash:?}, difficulty: {difficulty:?}, value: {value:?} overflowed: {overflowed:?}",
        );
        !overflowed
    }
}
