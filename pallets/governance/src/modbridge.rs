#![allow(dead_code)]

use crate::*;
use frame_support::{
	dispatch::DispatchResult,
	pallet_prelude::*,
	traits::ReservableCurrency,
};
use frame_system::pallet_prelude::BlockNumberFor;
use parity_scale_codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use sp_runtime::traits::Zero;

/// Lockup position to be exported to the new chain (vesting applied there).
#[derive(Encode, Decode, Clone, PartialEq, Eq, TypeInfo, MaxEncodedLen)]
pub struct BridgeLockup<BlockNumber> {
	/// Amount locked (in u64 units of the chain balance)
	pub amount: u64,
    /// Vesting period length requested by user
	pub vesting_period: BlockNumber,
}

impl<T: pallet::Config> Pallet<T> {
	/// Internal: lock tokens for future vesting. Only allowed before the bridge event.
	pub fn do_bridge_lock(
		who: &T::AccountId,
		amount: u64,
		vesting_period: BlockNumberFor<T>,
	) -> DispatchResult {
		ensure!(amount > 0, Error::<T>::InvalidProposalCost); // reuse generic non-zero guard
		ensure!(!vesting_period.is_zero(), Error::<T>::InvalidPaymentInterval); // reuse >0 guard
		ensure!(BridgeEventBlock::<T>::get().is_none(), Error::<T>::BridgeAlreadyTriggered);

		let amount_as_balance = <
			<T as pallet::Config>::Currency as frame_support::traits::Currency<
				<T as frame_system::Config>::AccountId,
			>
		>::Balance::from(amount);
		<T as pallet::Config>::Currency::reserve(who, amount_as_balance)?;

        let push_res: Result<(), sp_runtime::DispatchError> = BridgeLockups::<T>::mutate(who, |positions| {
            positions
                .try_push(BridgeLockup::<BlockNumberFor<T>> { amount, vesting_period })
                .map_err(|_| Error::<T>::InternalError.into())
        });
		push_res?;

		Ok(())
	}

	/// Internal: unlock all user positions before the bridge event without penalty.
	pub fn do_bridge_unlock_all(who: &T::AccountId) -> DispatchResult {
		ensure!(BridgeEventBlock::<T>::get().is_none(), Error::<T>::BridgeAlreadyTriggered);
		let mut total: u128 = 0;
		BridgeLockups::<T>::mutate(who, |positions| {
			for p in positions.iter() {
				total = total.saturating_add(p.amount as u128);
			}
			positions.clear();
		});

		if total > 0 {
			let total_u64: u64 = core::cmp::min(total, u128::from(u64::MAX)) as u64;
			let unreserve = <
				<T as pallet::Config>::Currency as frame_support::traits::Currency<
					<T as frame_system::Config>::AccountId,
				>
			>::Balance::from(total_u64);
			let _ = <T as pallet::Config>::Currency::unreserve(who, unreserve);
		} else {
			return Err(Error::<T>::NoBridgeLockups.into());
		}

		Ok(())
	}

    // Claiming is not performed on this chain. Unlocking is allowed only pre-event.

	/// Internal: mark the bridge event as triggered (freezes lock/unlock)
	pub fn do_trigger_bridge_event() -> Result<BlockNumberFor<T>, DispatchError> {
		ensure!(BridgeEventBlock::<T>::get().is_none(), Error::<T>::BridgeAlreadyTriggered);
		let now = frame_system::Pallet::<T>::block_number();
		BridgeEventBlock::<T>::put(now);
		Ok(now)
	}
}


