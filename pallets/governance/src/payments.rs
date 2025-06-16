use frame_support::{
    pallet_prelude::*,
    traits::Currency,
};
use frame_system::pallet_prelude::BlockNumberFor;
use sp_runtime::traits::Zero;
use crate::Config;

/// Number of blocks per day (assuming ~12 second block time)
pub const BLOCKS_PER_DAY: u32 = 7200;

/// Payment window in blocks (±2 hours)
pub const PAYMENT_WINDOW: u32 = 600;

/// A scheduled payment that will be executed on a specific day of each month
#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
#[scale_info(skip_type_params(T))]
pub struct ScheduledPayment<T: Config> {
    /// Account that will receive the payment
    pub recipient: T::AccountId,
    /// Amount to be paid
    pub amount: u64,
    /// Day of the month to execute payment (1-30)
    pub target_day: u8,
    /// Number of payments remaining (0 means indefinite)
    pub remaining_payments: u32,
    /// Block number when the last payment was made
    pub last_payment_block: BlockNumberFor<T>,
}

impl<T: Config> ScheduledPayment<T> {
    /// Create a new scheduled payment
    pub fn new(
        recipient: T::AccountId,
        amount: u64,
        target_day: u8,
        remaining_payments: u32,
    ) -> Self {
        Self {
            recipient,
            amount,
            target_day,
            remaining_payments,
            last_payment_block: Zero::zero(),
        }
    }

    /// Check if payment is due based on current block number
    pub fn is_payment_due(&self, current_block: BlockNumberFor<T>) -> bool {
        // Only check if we're at least a day from last check
        if current_block <= self.last_payment_block + BLOCKS_PER_DAY.into() {
            return false;
        }

        // Estimate current day (1-30)
        let estimated_day = ((current_block / BLOCKS_PER_DAY.into()) % 30u32.into()) + 1u32.into();
        
        // Check if we're within payment window
        self.target_day == estimated_day.try_into().unwrap_or(0)
    }

    /// Process payment if due, returns true if payment was processed
    pub fn try_process_payment(
        &mut self,
        current_block: BlockNumberFor<T>,
        treasury: &T::AccountId,
    ) -> Result<bool, DispatchError> {
        if !self.is_payment_due(current_block) {
            return Ok(false);
        }

        // Execute payment from treasury to recipient
        <T as Config>::Currency::transfer(
            treasury,
            &self.recipient,
            self.amount.into(),
            frame_support::traits::ExistenceRequirement::KeepAlive,
        )?;

        // Update payment tracking
        self.last_payment_block = current_block;
        if self.remaining_payments > 0 {
            self.remaining_payments = self.remaining_payments.saturating_sub(1);
        }

        Ok(true)
    }
}

/// Type alias for the Currency balance type
pub type BalanceOf<T> = <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;
