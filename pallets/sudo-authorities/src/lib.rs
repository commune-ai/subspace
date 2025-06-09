#![cfg_attr(not(feature = "std"), no_std)]

pub use sp_consensus_grandpa::AuthorityList;

/// Edit this file to define custom logic or remove it if it is not needed.
/// Learn more about FRAME and the core library of Substrate FRAME pallets:
/// <https://docs.substrate.io/reference/frame-pallets/>
pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: pallet_grandpa::Config + pallet_aura::Config + frame_system::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		AuraAuthoritiesChanged {
			new_authorities: BoundedVec<T::AuthorityId, <T as pallet_aura::Config>::MaxAuthorities>,
		},
		GrandpaAuthoritiesChanged {
			new_authorities: AuthorityList,
		},
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(0)]
		// use 50% of a block based on current BlockWeights
		#[pallet::weight({1000000000000})]
		pub fn change_aura_authorities(
			origin: OriginFor<T>,
			new_authorities: BoundedVec<T::AuthorityId, <T as pallet_aura::Config>::MaxAuthorities>,
		) -> DispatchResult {
			ensure_root(origin)?;
			pallet_aura::Pallet::<T>::change_authorities(new_authorities.clone());
			Self::deposit_event(Event::AuraAuthoritiesChanged { new_authorities });
			Ok(())
		}

		#[pallet::call_index(1)]
		// use 50% of a block based on current BlockWeights
		#[pallet::weight({1000000000000})]
		pub fn change_grandpa_authorities(
			origin: OriginFor<T>,
			new_authorities: AuthorityList,
		) -> DispatchResult {
			ensure_root(origin)?;
			let block: u32 = 0;
			pallet_grandpa::Pallet::<T>::schedule_change(
				new_authorities.clone(),
				block.into(),
				None,
			)?;
			Self::deposit_event(Event::GrandpaAuthoritiesChanged { new_authorities });
			Ok(())
		}
	}
}
