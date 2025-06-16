# Automated Payment System Documentation

## Overview
The automated payment system is integrated into the governance pallet and enables recurring payments from the DAO treasury on specific calendar dates. The system uses block-based timing to estimate calendar days and includes a payment window to handle block time variations.

## Storage Items

### PaymentSchedules: StorageMap<u64, ScheduledPayment<T>>
- Maps schedule IDs to payment configurations
- Key: `u64` (schedule ID)
- Value: `ScheduledPayment<T>` struct containing:
  - `recipient: T::AccountId`
  - `amount: BalanceOf<T>`
  - `target_day: u8` (1-30)
  - `remaining_payments: u32` (0 means indefinite)
  - `last_payment_block: T::BlockNumber`

### NextPaymentScheduleId: StorageValue<u64>
- Counter for generating unique schedule IDs
- Increments with each new schedule creation

## Dispatchable Functions

### create_payment_schedule
```rust
fn create_payment_schedule(
    origin: OriginFor<T>,
    recipient: T::AccountId,
    amount: BalanceOf<T>,
    target_day: u8,
    remaining_payments: u32,
) -> DispatchResult
```
- Creates a new payment schedule
- Origin must be root or approved via governance
- Parameters:
  - `recipient`: Account to receive payments
  - `amount`: Amount to pay per cycle
  - `target_day`: Day of month for payment (1-30)
  - `remaining_payments`: Number of payments (0 for indefinite)
- Emits: `PaymentScheduleCreated`

### cancel_payment_schedule
```rust
fn cancel_payment_schedule(
    origin: OriginFor<T>,
    schedule_id: u64,
) -> DispatchResult
```
- Cancels an existing payment schedule
- Origin must be root or approved via governance
- Parameters:
  - `schedule_id`: ID of the schedule to cancel
- Emits: `PaymentScheduleCancelled`

## Events

### PaymentScheduleCreated
```rust
PaymentScheduleCreated {
    schedule_id: u64,
    recipient: T::AccountId,
    amount: BalanceOf<T>,
    target_day: u8,
    remaining_payments: u32,
}
```
- Emitted when a new payment schedule is created

### PaymentScheduleCancelled
```rust
PaymentScheduleCancelled {
    schedule_id: u64,
}
```
- Emitted when a payment schedule is cancelled

### PaymentExecuted
```rust
PaymentExecuted {
    schedule_id: u64,
    recipient: T::AccountId,
    amount: BalanceOf<T>,
}
```
- Emitted when a scheduled payment is successfully executed

### PaymentFailed
```rust
PaymentFailed {
    schedule_id: u64,
    recipient: T::AccountId,
    amount: BalanceOf<T>,
}
```
- Emitted when a scheduled payment fails (e.g., insufficient funds)

## Configuration and Management

### Creating Payment Schedules
1. Payment schedules can be created through:
   - Direct root call to `create_payment_schedule`
   - Governance proposal using `PaymentSchedule` proposal type
2. Required information:
   - Recipient address
   - Payment amount
   - Target day (1-30)
   - Number of payments (0 for indefinite)

### Managing Schedules
1. View active schedules:
   - Query `PaymentSchedules` storage
   - Monitor `PaymentExecuted` and `PaymentFailed` events
2. Cancel schedules:
   - Use `cancel_payment_schedule` call
   - Requires root or governance approval
3. Modify schedules:
   - Cancel existing schedule
   - Create new schedule with updated parameters

### Payment Processing
1. Timing:
   - Payments checked daily in `on_initialize`
   - Uses block-based day estimation
   - Payment window of ±2 hours around target time
2. Execution:
   - Transfers from DAO treasury to recipient
   - Updates `last_payment_block`
   - Decrements `remaining_payments`
   - Removes completed schedules
3. Error handling:
   - Failed payments emit `PaymentFailed` event
   - Schedule remains active for retry
   - Treasury balance should be monitored

### Best Practices
1. Treasury Management:
   - Maintain sufficient treasury balance
   - Monitor `PaymentFailed` events
   - Consider total monthly obligations
2. Schedule Creation:
   - Use different days for different recipients
   - Consider token price impact
   - Document schedule details
3. Monitoring:
   - Track payment events
   - Verify schedule parameters
   - Monitor treasury balance
