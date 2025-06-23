use crate::mock::*;
use frame_support::{assert_err, assert_ok, traits::Hooks};
use pallet_governance::{payments::BLOCKS_PER_PAYMENT_CYCLE, Event, ScheduledPayment, *};

fn run_to_block(n: BlockNumber) {
    while System::block_number() < n {
        let current_block = System::block_number();
        System::on_finalize(current_block);
        GovernanceMod::on_finalize(current_block);
        System::set_block_number(current_block + 1);
        System::on_initialize(current_block + 1);
        GovernanceMod::on_initialize(current_block + 1);
    }
}

#[test]
fn test_scheduled_payment_creation() {
    new_test_ext().execute_with(|| {
        let recipient: <Test as frame_system::Config>::AccountId = 1;
        let amount: u64 = 100;
        let first_payment_in: BlockNumber = 1000;
        let payment_interval: BlockNumber = BLOCKS_PER_PAYMENT_CYCLE.into();
        let remaining_payments = 3;
        let current_block: BlockNumber = 500;

        let schedule = ScheduledPayment::<Test>::new(
            recipient,
            amount,
            first_payment_in,
            payment_interval,
            remaining_payments,
            current_block,
        );

        assert_eq!(schedule.recipient, recipient);
        assert_eq!(schedule.amount, amount);
        assert_eq!(
            schedule.next_payment_block,
            current_block + first_payment_in
        );
        assert_eq!(schedule.payment_interval, payment_interval);
        assert_eq!(schedule.remaining_payments, remaining_payments);
    });
}

#[test]
fn test_payment_processing() {
    new_test_ext().execute_with(|| {
        let recipient: <Test as frame_system::Config>::AccountId = 2;
        let amount = 100;
        let first_payment_in: BlockNumber = 1000;
        let payment_interval: BlockNumber = BLOCKS_PER_PAYMENT_CYCLE.into();
        let remaining_payments = 3;
        let current_block: BlockNumber = 500;
        
        // Create treasury address with 100,000 Tokens
        let treasury_address = DaoTreasuryAddress::<Test>::get();
        SubspaceMod::add_balance_to_account(&treasury_address, 100_000_000_000_000);

        let mut schedule = ScheduledPayment::<Test>::new(
            recipient,
            amount,
            first_payment_in,
            payment_interval,
            remaining_payments,
            current_block,
        );

        // Payment should not process before next_payment_block
        assert_eq!(
            schedule.process_if_due(current_block, &treasury_address, 0),
            Ok(None)
        );

        // Payment should process at next_payment_block
        let payment_block = current_block + first_payment_in;
        assert!(matches!(
            schedule.process_if_due(payment_block, &treasury_address, 0),
            Ok(Some(Event::PaymentExecuted { .. }))
        ));

        // Verify schedule state after payment
        assert_eq!(
            schedule.next_payment_block,
            payment_block + payment_interval
        );
        assert_eq!(schedule.remaining_payments, remaining_payments - 1);
    });
}

#[test]
fn test_payment_completion() {
    new_test_ext().execute_with(|| {
        let recipient: <Test as frame_system::Config>::AccountId = 2;
        let amount = 100;
        let first_payment_in: BlockNumber = 1000;
        let payment_interval: BlockNumber = BLOCKS_PER_PAYMENT_CYCLE.into();
        let remaining_payments = 1; // Only one payment
        let current_block: BlockNumber = 500;

        // Create treasury address with 100,000 Tokens
        let treasury_address = DaoTreasuryAddress::<Test>::get();
        SubspaceMod::add_balance_to_account(&treasury_address, 100_000_000_000_000);

        let mut schedule = ScheduledPayment::<Test>::new(
            recipient,
            amount,
            first_payment_in,
            payment_interval.into(),
            remaining_payments,
            current_block,
        );

        // Process the only payment
        let payment_block = current_block + first_payment_in;
        assert!(matches!(
            schedule.process_if_due(payment_block, &treasury_address, 0),
            Ok(Some(Event::PaymentExecuted { .. }))
        ));

        // Schedule should be completed
        assert!(schedule.is_completed());
    });
}

#[test]
fn test_payment_failure() {
    new_test_ext().execute_with(|| {
        let treasury = 1;
        let recipient: <Test as frame_system::Config>::AccountId = 2;
        let amount = u64::MAX; // More than treasury balance
        let first_payment_in: BlockNumber = 1000;
        let payment_interval: BlockNumber = BLOCKS_PER_PAYMENT_CYCLE.into();
        let remaining_payments = 3;
        let current_block: BlockNumber = 500;

        let mut schedule = ScheduledPayment::<Test>::new(
            recipient,
            amount,
            first_payment_in,
            payment_interval.into(),
            remaining_payments,
            current_block,
        );

        // Payment should fail due to insufficient funds
        let payment_block = current_block + first_payment_in;
        assert!(matches!(
            schedule.process_if_due(payment_block, &treasury, 0),
            Err(_)
        ));

        // Schedule state should remain unchanged after failure
        assert_eq!(schedule.next_payment_block, payment_block);
        assert_eq!(schedule.remaining_payments, remaining_payments);
    });
}

#[test]
fn test_create_payment_schedule() {
    new_test_ext().execute_with(|| {
        let recipient: <Test as frame_system::Config>::AccountId = 2;
        let amount = 100;
        let first_payment_in: BlockNumber = 1000;
        let payment_interval: BlockNumber = BLOCKS_PER_PAYMENT_CYCLE.into();
        let remaining_payments = 3;

        // Create payment schedule
        assert_ok!(GovernanceMod::create_payment_schedule(
            RuntimeOrigin::root(),
            recipient,
            amount,
            first_payment_in.into(),
            payment_interval.into(),
            remaining_payments,
        ));

        // Verify schedule was created correctly
        let schedule_id = 0; // First schedule should have ID 0
        let schedule = PaymentSchedules::<Test>::get(schedule_id).unwrap();
        assert_eq!(schedule.recipient, recipient);
        assert_eq!(schedule.amount, amount);
        assert_eq!(schedule.payment_interval, payment_interval);
        assert_eq!(schedule.remaining_payments, remaining_payments);
    });
}

#[test]
fn test_create_payment_schedule_invalid_interval() {
    new_test_ext().execute_with(|| {
        let recipient: <Test as frame_system::Config>::AccountId = 2;
        let amount = 100;
        let first_payment_in: BlockNumber = 1000;
        let payment_interval: BlockNumber = 0; // Invalid: must be > 0
        let remaining_payments = 3;

        // Attempt to create payment schedule with invalid interval
        assert_err!(
            GovernanceMod::create_payment_schedule(
                RuntimeOrigin::root(),
                recipient,
                amount,
                first_payment_in.into(),
                payment_interval.into(),
                remaining_payments,
            ),
            Error::<Test>::InvalidPaymentInterval
        );
    });
}

#[test]
fn test_cancel_payment_schedule() {
    new_test_ext().execute_with(|| {
        let recipient: <Test as frame_system::Config>::AccountId = 2;
        let amount = 100;
        let first_payment_in: BlockNumber = 1000;
        let payment_interval: BlockNumber = BLOCKS_PER_PAYMENT_CYCLE.into();
        let remaining_payments = 3;

        // Create payment schedule
        assert_ok!(GovernanceMod::create_payment_schedule(
            RuntimeOrigin::root(),
            recipient,
            amount,
            first_payment_in.into(),
            payment_interval.into(),
            remaining_payments,
        ));

        let schedule_id = 0;

        // Cancel the schedule
        assert_ok!(GovernanceMod::cancel_payment_schedule(
            RuntimeOrigin::root(),
            schedule_id,
        ));

        // Verify schedule was removed
        assert!(PaymentSchedules::<Test>::get(schedule_id).is_none());
    });
}

#[test]
fn test_cancel_nonexistent_payment_schedule() {
    new_test_ext().execute_with(|| {
        // Attempt to cancel a schedule that doesn't exist
        assert_err!(
            GovernanceMod::cancel_payment_schedule(RuntimeOrigin::root(), 0),
            Error::<Test>::PaymentScheduleNotFound
        );
    });
}

#[test]
fn test_payment_processing_in_on_initialize() {
    new_test_ext().execute_with(|| {
        let recipient: <Test as frame_system::Config>::AccountId = 2;
        let amount = 100;
        let first_payment_in: BlockNumber = 1000;
        let payment_interval: BlockNumber = BLOCKS_PER_PAYMENT_CYCLE.into();
        let remaining_payments = 3;
        
        // Create treasury address with 100,000 Tokens
        let treasury_address = DaoTreasuryAddress::<Test>::get();
        SubspaceMod::add_balance_to_account(&treasury_address, 100_000_000_000_000);

        // Create payment schedule
        assert_ok!(GovernanceMod::create_payment_schedule(
            RuntimeOrigin::root(),
            recipient,
            amount,
            first_payment_in.into(),
            payment_interval.into(),
            remaining_payments,
        ));

        let schedule_id = 0;
        let start_block = System::block_number();

        // Run to payment block
        run_to_block(start_block.saturating_add(first_payment_in));

        // Verify payment was processed
        let schedule = PaymentSchedules::<Test>::get(schedule_id).unwrap();
        assert_eq!(schedule.remaining_payments, remaining_payments - 1);
        assert_eq!(
            schedule.next_payment_block,
            start_block.saturating_add(first_payment_in).saturating_add(payment_interval)
        );

        // Run to next payment
        run_to_block(start_block.saturating_add(first_payment_in).saturating_add(payment_interval));

        // Verify second payment was processed
        let schedule = PaymentSchedules::<Test>::get(schedule_id).unwrap();
        assert_eq!(schedule.remaining_payments, remaining_payments - 2);
        assert_eq!(
            schedule.next_payment_block,
            start_block
                .saturating_add(first_payment_in)
                .saturating_add(payment_interval.saturating_mul(2))
        );

        // Run to final payment
        run_to_block(
            start_block
                .saturating_add(first_payment_in)
                .saturating_add(payment_interval.saturating_mul(2)),
        );

        // Verify schedule was removed after final payment
        assert!(PaymentSchedules::<Test>::get(schedule_id).is_none());
    });
}

#[test]
fn test_payment_failure_in_on_initialize() {
    new_test_ext().execute_with(|| {
        let recipient: <Test as frame_system::Config>::AccountId = 2;
        let amount = u64::MAX; // More than treasury balance
        let first_payment_in: BlockNumber = 1000;
        let payment_interval: BlockNumber = BLOCKS_PER_PAYMENT_CYCLE.into();
        let remaining_payments = 3;

        // Create payment schedule
        assert_ok!(GovernanceMod::create_payment_schedule(
            RuntimeOrigin::root(),
            recipient,
            amount,
            first_payment_in.into(),
            payment_interval.into(),
            remaining_payments,
        ));

        let schedule_id = 0;
        let start_block = System::block_number();

        // Run to payment block
        run_to_block(start_block.saturating_add(first_payment_in));

        // Verify schedule still exists but payment failed
        let schedule = PaymentSchedules::<Test>::get(schedule_id).unwrap();
        assert_eq!(schedule.remaining_payments, remaining_payments); // Unchanged
        assert_eq!(
            schedule.next_payment_block,
            start_block.saturating_add(first_payment_in)
        ); // Unchanged
    });
}
