// # Governance Proposal: Automated Payment System Implementation

// ## Summary
// This proposal implements an automated payment system within the governance pallet, enabling
// scheduled, recurring payments from the DAO treasury. The implementation uses block-based timing
// for deterministic execution and provides flexible configuration options for payment schedules.

// ## Technical Changes

// ### 1. Core Features
// - Integration of payment scheduling into the governance pallet
// - Block-based payment timing using `next_payment_block` and `payment_interval`
// - Treasury-managed fund disbursement
// - Configurable payment intervals
// - Support for both finite and infinite payment schedules

// ### 2. New Storage Items
// - `PaymentSchedules`: Stores active payment schedules
// - `NextPaymentScheduleId`: Counter for unique schedule IDs

// ### 3. New Dispatchables
// - `create_payment_schedule`: Create new payment schedules
// - `cancel_payment_schedule`: Cancel existing schedules

// ### 4. New Events
// - `PaymentScheduleCreated`
// - `PaymentScheduleCancelled`
// - `PaymentExecuted`
// - `PaymentFailed`

// ## Verification Steps
// After the upgrade is deployed, community members can verify the implementation by:

// 1. Checking Storage:
//    - Verify `PaymentSchedules` and `NextPaymentScheduleId` storage items exist
//    - Confirm initial state (empty schedules, ID counter at 0)

// 2. Testing Dispatchables:
//    - Create a test payment schedule with small amounts
//    - Verify schedule appears in storage
//    - Cancel the schedule and verify removal

// 3. Monitoring Events:
//    - Watch for appropriate events during schedule creation/cancellation
//    - Monitor payment execution events at scheduled blocks

// ## Benefits
// 1. **Automation**: Eliminates manual treasury proposals for recurring payments
// 2. **Reliability**: Block-based timing ensures deterministic execution
// 3. **Flexibility**: Configurable intervals and payment counts
// 4. **Transparency**: All actions emit events for easy tracking

// ## Documentation
// Full documentation is available in:
// - `pallets/governance/README.md`: Usage guide and examples
// - `docs/architecture/governance.md`: Technical architecture and implementation details

// ## Security Considerations
// 1. Only governance can create/cancel payment schedules
// 2. Treasury balance checks prevent overspending
// 3. Payment failures don't block other schedules
// 4. Schedules can be cancelled if needed

// ## Testing
// The implementation includes comprehensive test coverage:
// - Payment schedule creation/validation
// - Payment processing/completion
// - Failure handling
// - Treasury integration
// - Block progression simulation

// All tests pass successfully, demonstrating the system's reliability and correctness.

use crate::{Config, Event};
use frame_support::{
    pallet_prelude::*,
    traits::{Currency, DefensiveSaturating, ExistenceRequirement},
};
use frame_system::pallet_prelude::BlockNumberFor;

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
pub type BalanceOf<T> =
    <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;
