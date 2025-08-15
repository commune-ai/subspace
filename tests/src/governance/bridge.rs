use crate::mock::*;
use frame_support::{assert_noop, assert_ok};
use pallet_governance::{BridgeEventBlock, BridgeLockups, Error as GovError};

#[test]
fn can_lock_and_unlock_before_event() {
    new_test_ext().execute_with(|| {
        let who = 1u32;
        let amount = to_nano(100);

        // fund account
        add_balance(who, amount + to_nano(1));

        // lock
        assert_ok!(pallet_governance::Pallet::<Test>::bridge_lock(
            get_origin(who),
            amount,
            10u64,
        ));

        // storage updated
        let positions = BridgeLockups::<Test>::get(&who);
        assert_eq!(positions.len(), 1);
        assert_eq!(positions[0].amount, amount);
        assert_eq!(positions[0].vesting_period, 10u64);

        // unlock all pre-event
        assert_ok!(pallet_governance::Pallet::<Test>::bridge_unlock_all(
            get_origin(who),
        ));
        let positions = BridgeLockups::<Test>::get(&who);
        assert!(positions.is_empty());
    });
}

#[test]
fn trigger_event_freezes_lock_unlock() {
    new_test_ext().execute_with(|| {
        let who = 1u32;
        let amount = to_nano(10);

        add_balance(who, amount + to_nano(1));
        assert_ok!(pallet_governance::Pallet::<Test>::bridge_lock(
            get_origin(who),
            amount,
            5u64,
        ));

        // trigger bridge
        assert_ok!(pallet_governance::Pallet::<Test>::trigger_bridge_event(
            RuntimeOrigin::root(),
        ));
        assert!(BridgeEventBlock::<Test>::get().is_some());

        // cannot lock more
        assert_noop!(
            pallet_governance::Pallet::<Test>::bridge_lock(get_origin(who), amount, 5u64),
            GovError::<Test>::BridgeAlreadyTriggered
        );

        // cannot unlock
        assert_noop!(
            pallet_governance::Pallet::<Test>::bridge_unlock_all(get_origin(who)),
            GovError::<Test>::BridgeAlreadyTriggered
        );
    });
}

// Claiming tests removed: claiming/vesting happens on the new chain.


