#![cfg(test)]

use super::*;
use frame_support::{
    assert_ok, parameter_types,
    traits::{ConstU32, ConstU64},
};
use sp_core::H256;
use sp_runtime::{
    traits::{BlakeTwo256, IdentityLookup},
    BuildStorage,
};

type Block = frame_system::mocking::MockBlock<Test>;

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
    pub enum Test
    {
        System: frame_system,
        Balances: pallet_balances,
        Governance: crate,
    }
);

parameter_types! {
    pub const BlockHashCount: u64 = 250;
    pub const MaximumBlockWeight: Weight = Weight::from_parts(1024, 0);
    pub const MaximumBlockLength: u32 = 2 * 1024;
    pub const AvailableBlockRatio: Perbill = Perbill::one();
    pub const ExistentialDeposit: u64 = 1;
}

impl frame_system::Config for Test {
    type BaseCallFilter = frame_support::traits::Everything;
    type BlockWeights = ();
    type BlockLength = ();
    type DbWeight = ();
    type RuntimeOrigin = RuntimeOrigin;
    type RuntimeCall = RuntimeCall;
    type Nonce = u64;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type AccountId = u64;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Block = Block;
    type RuntimeEvent = RuntimeEvent;
    type BlockHashCount = ConstU64<250>;
    type Version = ();
    type PalletInfo = PalletInfo;
    type AccountData = pallet_balances::AccountData<u64>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type SS58Prefix = ();
    type OnSetCode = ();
    type MaxConsumers = ConstU32<16>;
}

impl pallet_balances::Config for Test {
    type MaxLocks = ();
    type MaxReserves = ();
    type ReserveIdentifier = [u8; 8];
    type Balance = u64;
    type RuntimeEvent = RuntimeEvent;
    type DustRemoval = ();
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
    type WeightInfo = ();
    type FreezeIdentifier = ();
    type MaxFreezes = ();
    type RuntimeHoldReason = ();
    type MaxHolds = ();
}

impl Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = ();
    type Currency = Balances;
}

// Helper function to build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
    let mut t = frame_system::GenesisConfig::<Test>::default()
        .build_storage()
        .unwrap();

    pallet_balances::GenesisConfig::<Test> {
        balances: vec![(1, 100), (2, 100), (3, 100), (4, 100)],
    }
    .assimilate_storage(&mut t)
    .unwrap();

    let mut ext = sp_io::TestExternalities::new(t);
    ext.execute_with(|| System::set_block_number(1));
    ext
}

#[test]
fn test_scheduled_payment_creation() {
    new_test_ext().execute_with(|| {
        let recipient = 2;
        let amount = 50;
        let target_day = 15;
        let remaining_payments = 12;

        let payment = ScheduledPayment::<Test>::new(
            recipient,
            amount,
            target_day,
            remaining_payments,
        );

        assert_eq!(payment.recipient, recipient);
        assert_eq!(payment.amount, amount);
        assert_eq!(payment.target_day, target_day);
        assert_eq!(payment.remaining_payments, remaining_payments);
        assert_eq!(payment.last_payment_block, 0);
    });
}

#[test]
fn test_payment_due_timing() {
    new_test_ext().execute_with(|| {
        let payment = ScheduledPayment::<Test>::new(2, 50, 15, 12);
        let treasury = 1;

        // Not due on day 14
        let block_day_14 = payments::BLOCKS_PER_DAY as u64 * 14;
        assert!(!payment.is_payment_due(block_day_14));

        // Due on day 15
        let block_day_15 = payments::BLOCKS_PER_DAY as u64 * 15;
        assert!(payment.is_payment_due(block_day_15));

        // Not due on day 16
        let block_day_16 = payments::BLOCKS_PER_DAY as u64 * 16;
        assert!(!payment.is_payment_due(block_day_16));
    });
}

#[test]
fn test_payment_processing() {
    new_test_ext().execute_with(|| {
        let recipient = 2;
        let treasury = 1;
        let amount = 50;
        let mut payment = ScheduledPayment::<Test>::new(recipient, amount, 15, 2);

        // Process payment on day 15
        let block_day_15 = payments::BLOCKS_PER_DAY as u64 * 15;
        assert_ok!(payment.try_process_payment(block_day_15, &treasury));
        assert_eq!(payment.remaining_payments, 1);
        assert_eq!(payment.last_payment_block, block_day_15);

        // Should not process again until next month
        let block_day_16 = payments::BLOCKS_PER_DAY as u64 * 16;
        assert!(!payment.is_payment_due(block_day_16));

        // Should process again next month
        let block_next_month = payments::BLOCKS_PER_DAY as u64 * 45;
        assert_ok!(payment.try_process_payment(block_next_month, &treasury));
        assert_eq!(payment.remaining_payments, 0);
    });
}

#[test]
fn test_payment_schedule_management() {
    new_test_ext().execute_with(|| {
        let recipient = 2;
        let amount = 50;
        let target_day = 15;
        let remaining_payments = 12;

        // Create payment schedule
        assert_ok!(Governance::create_payment_schedule(
            RuntimeOrigin::root(),
            recipient,
            amount,
            target_day,
            remaining_payments
        ));

        // Schedule should be stored
        let schedule_id = 0;
        assert!(PaymentSchedules::<Test>::contains_key(schedule_id));

        // Cancel payment schedule
        assert_ok!(Governance::cancel_payment_schedule(
            RuntimeOrigin::root(),
            schedule_id
        ));

        // Schedule should be removed
        assert!(!PaymentSchedules::<Test>::contains_key(schedule_id));
    });
}

#[test]
fn test_payment_execution() {
    new_test_ext().execute_with(|| {
        let treasury = 1;
        let recipient = 2;
        let amount = 50;
        let target_day = 15;
        
        // Create payment schedule
        assert_ok!(Governance::create_payment_schedule(
            RuntimeOrigin::root(),
            recipient,
            amount,
            target_day,
            1
        ));

        // Initial balances
        let treasury_balance = Balances::free_balance(treasury);
        let recipient_balance = Balances::free_balance(recipient);

        // Advance to payment day
        let block_day_15 = payments::BLOCKS_PER_DAY as u64 * 15;
        System::set_block_number(block_day_15);

        // Process block (triggers payment)
        Governance::on_initialize(block_day_15);

        // Check balances
        assert_eq!(
            Balances::free_balance(treasury),
            treasury_balance - amount
        );
        assert_eq!(
            Balances::free_balance(recipient),
            recipient_balance + amount
        );

        // Schedule should be removed after single payment
        assert!(!PaymentSchedules::<Test>::contains_key(0));
    });
}
