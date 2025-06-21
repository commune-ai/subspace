mod mock;
use mock::*;
use pallet_governance::{payments::BLOCKS_PER_PAYMENT_CYCLE, Event, ScheduledPayment};

#[test]
fn test_scheduled_payment_creation() {
    new_test_ext().execute_with(|| {
        let recipient: u64 = 1;
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
        let treasury = 1;
        let recipient = 2;
        let amount = 100;
        let first_payment_in: BlockNumber = 1000;
        let payment_interval: BlockNumber = BLOCKS_PER_PAYMENT_CYCLE.into();
        let remaining_payments = 3;
        let current_block: BlockNumber = 500;

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
            schedule.process_if_due(current_block, &treasury, 0),
            Ok(None)
        );

        // Payment should process at next_payment_block
        let payment_block = current_block + first_payment_in;
        assert!(matches!(
            schedule.process_if_due(payment_block, &treasury, 0),
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
        let treasury = 1;
        let recipient = 2;
        let amount = 100;
        let first_payment_in: BlockNumber = 1000;
        let payment_interval: BlockNumber = BLOCKS_PER_PAYMENT_CYCLE.into();
        let remaining_payments = 1; // Only one payment
        let current_block: BlockNumber = 500;

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
            schedule.process_if_due(payment_block, &treasury, 0),
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
        let recipient = 2;
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
