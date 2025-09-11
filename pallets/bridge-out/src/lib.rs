#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
    use frame_support::{
        pallet_prelude::*,
        traits::{Currency, ExistenceRequirement::AllowDeath},
        PalletId,
    };
    use frame_system::pallet_prelude::*;
    use parity_scale_codec::MaxEncodedLen;
    use sp_core::H160;
    use sp_runtime::traits::{AccountIdConversion, Zero};

    pub type BalanceOf<T> =
        <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
        type Currency: Currency<Self::AccountId>;
        #[pallet::constant]
        type PalletId: Get<PalletId>;
        /// Nonce storage counter type
        type Nonce: Parameter + Default + Copy + From<u64> + Into<u64> + MaxEncodedLen;
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    #[pallet::storage]
    #[pallet::getter(fn next_nonce)]
    pub type NextNonce<T: Config> = StorageValue<_, <T as Config>::Nonce, ValueQuery>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        BridgeToL1Locked {
            who: T::AccountId,
            amount_native: BalanceOf<T>,
            l2_recipient: H160,
            nonce: <T as Config>::Nonce,
        },
    }

    #[pallet::error]
    pub enum Error<T> {
        AmountZero,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::call_index(0)]
        #[pallet::weight(T::DbWeight::get().reads_writes(1,1))]
        pub fn lock_for_base(origin: OriginFor<T>, amount: BalanceOf<T>, l2_recipient: H160) -> DispatchResult {
            let who = ensure_signed(origin)?;
            ensure!(!amount.is_zero(), Error::<T>::AmountZero);

            let reserve = Self::reserve_account();
            <T as Config>::Currency::transfer(&who, &reserve, amount, AllowDeath)?;

            let nonce = Self::next_nonce();
            let next: <T as Config>::Nonce = (nonce.into().saturating_add(1u64)).into();
            NextNonce::<T>::put(next);

            Self::deposit_event(Event::BridgeToL1Locked { who, amount_native: amount, l2_recipient, nonce });
            Ok(())
        }
    }

    impl<T: Config> Pallet<T> {
        pub fn reserve_account() -> T::AccountId {
            T::PalletId::get().into_account_truncating()
        }
    }
}
