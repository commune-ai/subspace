#![cfg(test)]

use super::*;
use frame_support::{assert_noop, assert_ok};
use mock::*;

mod mock {
    use super::*;
    use frame_support::traits::ConstU32;
    use sp_core::H256;
    use sp_runtime::{
        traits::{BlakeTwo256, IdentityLookup},
        BuildStorage,
    };

    type Block = frame_system::mocking::MockBlock<Test>;

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
        type BlockHashCount = BlockHashCount;
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

    pub fn new_test_ext() -> sp_io::TestExternalities {
        let mut t = frame_system::GenesisConfig::<Test>::default()
            .build_storage()
            .unwrap();

        pallet_balances::GenesisConfig::<Test> {
            balances: vec![
                (1, 1_000_000), // Treasury
                (2, 100),       // Recipient 1
                (3, 100),       // Recipient 2
                (4, 100),       // Recipient 3
            ],
        }
        .assimilate_storage(&mut t)
        .unwrap();

        let mut ext = sp_io::TestExternalities::new(t);
        ext.execute_with(|| System::set_block_number(1));
        ext
    }
}

#[test]
fn test_governance_proposal_payment_schedule() {
    new_test_ext().execute_with(|| {
        let recipient = 2;
        let amount = 50;
        let target_day = 15;
        let remaining_payments = 12;

        // Create proposal for payment schedule
        let proposal_data = ProposalData::PaymentSchedule {
            recipient,
            amount,
            target_day,
            remaining_payments,
        };

        // Submit proposal through governance
        assert_ok!(Governance::add_proposal(
            1,
            BoundedVec::try_from(b"Payment schedule proposal".to_vec()).unwrap(),
            proposal_data.clone(),
        ));

        // Accept proposal
        let proposal_id = 0;
        assert_ok!(Governance::vote_for(RuntimeOrigin::signed(1), proposal_id));
        
        // Verify schedule was created
        assert!(PaymentSchedules::<Test>::contains_key(0));
        
        let schedule = PaymentSchedules::<Test>::get(0).unwrap();
        assert_eq!(schedule.recipient, recipient);
        assert_eq!(schedule.amount, amount);
        assert_eq!(schedule.target_day, target_day);
        assert_eq!(schedule.remaining_payments, remaining_payments);
    });
}

#[test]
fn test_multi_month_payments() {
    new_test_ext().execute_with(|| {
        let treasury = 1;
        let recipient = 2;
        let amount = 50;
        let target_day = 15;
        let payments = 3;

        // Create payment schedule
        assert_ok!(Governance::create_payment_schedule(
            RuntimeOrigin::root(),
            recipient,
            amount,
            target_day,
            payments
        ));

        // Initial balances
        let initial_treasury = Balances::free_balance(treasury);
        let initial_recipient = Balances::free_balance(recipient);

        // Simulate three months
        for month in 0..3 {
            // Skip to payment day
            let block = payments::BLOCKS_PER_DAY as u64 * (30 * month + target_day);
            System::set_block_number(block);
            
            // Process block
            Governance::on_initialize(block);

            // Verify balances after payment
            assert_eq!(
                Balances::free_balance(treasury),
                initial_treasury - amount * (month + 1)
            );
            assert_eq!(
                Balances::free_balance(recipient),
                initial_recipient + amount * (month + 1)
            );

            // Verify schedule state
            if month < 2 {
                let schedule = PaymentSchedules::<Test>::get(0).unwrap();
                assert_eq!(schedule.remaining_payments as u64, payments - (month + 1));
            } else {
                // After last payment, schedule should be removed
                assert!(!PaymentSchedules::<Test>::contains_key(0));
            }

            // Verify events
            System::assert_has_event(RuntimeEvent::Governance(Event::PaymentExecuted {
                schedule_id: 0,
                recipient,
                amount,
            }));
        }
    });
}

#[test]
fn test_concurrent_payment_schedules() {
    new_test_ext().execute_with(|| {
        let treasury = 1;
        // Create three payment schedules for different days
        assert_ok!(Governance::create_payment_schedule(
            RuntimeOrigin::root(),
            2,
            50,
            5,
            2
        ));
        assert_ok!(Governance::create_payment_schedule(
            RuntimeOrigin::root(),
            3,
            75,
            15,
            2
        ));
        assert_ok!(Governance::create_payment_schedule(
            RuntimeOrigin::root(),
            4,
            100,
            25,
            2
        ));

        // Initial balances
        let initial_balances = vec![
            (2, Balances::free_balance(2)),
            (3, Balances::free_balance(3)),
            (4, Balances::free_balance(4)),
        ];

        // Test one month cycle
        for day in 1..=30 {
            let block = payments::BLOCKS_PER_DAY as u64 * day;
            System::set_block_number(block);
            Governance::on_initialize(block);

            // Check payments on scheduled days
            match day {
                5 => {
                    assert_eq!(
                        Balances::free_balance(2),
                        initial_balances[0].1 + 50
                    );
                    System::assert_has_event(RuntimeEvent::Governance(Event::PaymentExecuted {
                        schedule_id: 0,
                        recipient: 2,
                        amount: 50,
                    }));
                }
                15 => {
                    assert_eq!(
                        Balances::free_balance(3),
                        initial_balances[1].1 + 75
                    );
                    System::assert_has_event(RuntimeEvent::Governance(Event::PaymentExecuted {
                        schedule_id: 1,
                        recipient: 3,
                        amount: 75,
                    }));
                }
                25 => {
                    assert_eq!(
                        Balances::free_balance(4),
                        initial_balances[2].1 + 100
                    );
                    System::assert_has_event(RuntimeEvent::Governance(Event::PaymentExecuted {
                        schedule_id: 2,
                        recipient: 4,
                        amount: 100,
                    }));
                }
                _ => {}
            }
        }

        // All schedules should still exist with one payment remaining
        for id in 0..3 {
            assert!(PaymentSchedules::<Test>::contains_key(id));
            assert_eq!(PaymentSchedules::<Test>::get(id).unwrap().remaining_payments, 1);
        }
    });
}

#[test]
fn test_insufficient_funds() {
    new_test_ext().execute_with(|| {
        let treasury = 1;
        let recipient = 2;
        
        // Set treasury balance low
        let _ = Balances::set_balance(treasury, 40);
        
        // Try to create a schedule for more than available
        assert_ok!(Governance::create_payment_schedule(
            RuntimeOrigin::root(),
            recipient,
            50,
            15,
            1
        ));

        // Advance to payment day
        let block = payments::BLOCKS_PER_DAY as u64 * 15;
        System::set_block_number(block);
        
        // Process block
        Governance::on_initialize(block);

        // Verify payment failed event
        System::assert_has_event(RuntimeEvent::Governance(Event::PaymentFailed {
            schedule_id: 0,
            recipient,
            amount: 50,
        }));

        // Balances should be unchanged
        assert_eq!(Balances::free_balance(treasury), 40);
        assert_eq!(Balances::free_balance(recipient), 100);

        // Schedule should still exist
        assert!(PaymentSchedules::<Test>::contains_key(0));
    });
}
