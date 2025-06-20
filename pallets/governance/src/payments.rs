use frame_support::{
    pallet_prelude::*,
    traits::{Currency, ExistenceRequirement, DefensiveSaturating},
};
use frame_system::pallet_prelude::BlockNumberFor;
use crate::{Config, Event};

/// Default payment interval in blocks (10 days worth of blocks at ~8s block time)
pub const BLOCKS_PER_PAYMENT_CYCLE: u32 = 108000;

/// A scheduled payment that will be executed at regular block intervals
#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
#[scale_info(skip_type_params(T))]
pub struct ScheduledPayment<T: Config> {
    /// Account that will receive the payment
    pub recipient: T::AccountId,
    /// Amount to be paid
    pub amount: u64,
    /// Block number when the next payment should be made
    pub next_payment_block: BlockNumberFor<T>,
    /// Number of blocks between payments
    pub payment_interval: BlockNumberFor<T>,
    /// Number of payments remaining (0 means indefinite)
    pub remaining_payments: u32,
}

impl<T: Config> ScheduledPayment<T> {
    /// Create a new scheduled payment
    pub fn new(
        recipient: T::AccountId,
        amount: u64,
        first_payment_in_blocks: BlockNumberFor<T>,
        payment_interval: BlockNumberFor<T>,
        remaining_payments: u32,
        current_block: BlockNumberFor<T>,
    ) -> Self {
        Self {
            recipient,
            amount,
            next_payment_block: current_block.defensive_saturating_add(first_payment_in_blocks),
            payment_interval,
            remaining_payments,
        }
    }

    /// Process payment if due, returns Some(Event) if payment was processed
    pub fn process_if_due(
        &mut self,
        current_block: BlockNumberFor<T>,
        treasury: &T::AccountId,
        schedule_id: u64,
    ) -> Result<Option<Event<T>>, DispatchError> {
        if current_block < self.next_payment_block {
            return Ok(None);
        }

        // Execute payment from treasury to recipient
        <T as Config>::Currency::transfer(
            treasury,
            &self.recipient,
            self.amount.into(),
            ExistenceRequirement::KeepAlive,
        )?;

        // Update next payment block
        self.next_payment_block = current_block.defensive_saturating_add(self.payment_interval);

        // Update remaining payments
        if self.remaining_payments > 0 {
            self.remaining_payments = self.remaining_payments.saturating_sub(1);
        }

        Ok(Some(Event::PaymentExecuted { 
            schedule_id,
            recipient: self.recipient.clone(),
            amount: self.amount,
            next_payment_block: self.next_payment_block,
        }))
    }

    /// Returns true if this schedule should be removed (all payments completed)
    pub fn is_completed(&self) -> bool {
        self.remaining_payments == 0
    }
}

/// Type alias for the Currency balance type
pub type BalanceOf<T> = <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;
