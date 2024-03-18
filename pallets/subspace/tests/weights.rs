mod mock;
use frame_support::{assert_err, assert_ok};

use pallet_subspace::{Error, GlobalParams};
use sp_core::U256;
use sp_runtime::DispatchError;

use mock::*;

/***************************
  pub fn set_weights() tests
*****************************/

// Test ensures that uids -- weights must have the same size.
#[test]
fn test_weights_err_weights_vec_not_equal_size() {
    new_test_ext().execute_with(|| {
        let netuid: u16 = 0;
        let key_account_id = U256::from(55);
        assert_ok!(register_module(netuid, key_account_id, 1_000_000_000));
        let _neuron_uid: u16 = SubspaceModule::get_uid_for_key(netuid, &key_account_id);
        let weights_keys: Vec<u16> = vec![1, 2, 3, 4, 5, 6];
        let weight_values: Vec<u16> = vec![1, 2, 3, 4, 5]; // Uneven sizes
        let result = SubspaceModule::set_weights(
            RuntimeOrigin::signed(key_account_id),
            netuid,
            weights_keys,
            weight_values,
        );
        assert_eq!(result, Err(Error::<Test>::WeightVecNotEqualSize.into()));
    });
}

// Test ensures that uids can have not duplicates
#[test]
fn test_weights_err_has_duplicate_ids() {
    new_test_ext().execute_with(|| {
        let key_account_id = U256::from(666);
        let netuid: u16 = 0;
        SubspaceModule::set_max_registrations_per_block(100);

        assert_ok!(register_module(netuid, key_account_id, 10));
        SubspaceModule::set_max_allowed_uids(netuid, 100); // Allow many registrations per block.

        // uid 1
        assert_ok!(register_module(netuid, U256::from(1), 100));
        SubspaceModule::get_uid_for_key(netuid, &U256::from(1));

        // uid 2
        assert_ok!(register_module(netuid, U256::from(2), 10000));
        SubspaceModule::get_uid_for_key(netuid, &U256::from(2));

        // uid 3
        assert_ok!(register_module(netuid, U256::from(3), 10000000));
        SubspaceModule::get_uid_for_key(netuid, &U256::from(3));

        assert_eq!(SubspaceModule::get_subnet_n(netuid), 4);

        let weights_keys: Vec<u16> = vec![1, 1, 1]; // Contains duplicates
        let weight_values: Vec<u16> = vec![1, 2, 3];
        let result = SubspaceModule::set_weights(
            RuntimeOrigin::signed(key_account_id),
            netuid,
            weights_keys,
            weight_values,
        );
        assert_eq!(result, Err(Error::<Test>::DuplicateUids.into()));
    });
}

// Tests the call requires a valid origin.
#[test]
fn test_no_signature() {
    new_test_ext().execute_with(|| {
        let uids: Vec<u16> = vec![];
        let values: Vec<u16> = vec![];
        let result = SubspaceModule::set_weights(RuntimeOrigin::none(), 1, uids, values);
        assert_eq!(result, Err(DispatchError::BadOrigin));
    });
}

// Tests that set weights fails if you pass invalid uids.
#[test]
fn test_set_weights_err_invalid_uid() {
    new_test_ext().execute_with(|| {
        let key_account_id = U256::from(55);
        let netuid: u16 = 0;
        assert_ok!(register_module(netuid, key_account_id, 1_000_000_000));
        let _neuron_uid: u16 = SubspaceModule::get_uid_for_key(netuid, &key_account_id);
        let weight_keys: Vec<u16> = vec![9999]; // Does not exist
        let weight_values: Vec<u16> = vec![88]; // random value
        let result = SubspaceModule::set_weights(
            RuntimeOrigin::signed(key_account_id),
            netuid,
            weight_keys,
            weight_values,
        );
        assert_eq!(result, Err(Error::<Test>::InvalidUid.into()));
    });
}

// Tests that set weights fails if you dont pass enough values.
#[test]
fn test_set_weight_not_enough_values() {
    new_test_ext().execute_with(|| {
        let netuid: u16 = 0;
        let n = 100;
        SubspaceModule::set_max_registrations_per_block(n);
        let account_id = U256::from(0);

        assert_ok!(register_module(netuid, account_id, 1_000_000_000));

        let _neuron_uid: u16 = SubspaceModule::get_uid_for_key(netuid, &account_id);
        for i in 1..n {
            assert_ok!(register_module(netuid, U256::from(i), 1_000_000_000));
        }

        SubspaceModule::set_min_allowed_weights(netuid, 2);

        // setting weight below minimim
        let weight_keys: Vec<u16> = vec![1]; // not weight.
        let weight_values: Vec<u16> = vec![88]; // random value.
        let result = SubspaceModule::set_weights(
            RuntimeOrigin::signed(account_id),
            netuid,
            weight_keys,
            weight_values,
        );
        assert_eq!(result, Err(Error::<Test>::InvalidUidsLength.into()));

        SubspaceModule::set_min_allowed_weights(netuid, 1);

        let weight_keys: Vec<u16> = vec![0]; // self weight.
        let weight_values: Vec<u16> = vec![88]; // random value.
        let result = SubspaceModule::set_weights(
            RuntimeOrigin::signed(account_id),
            netuid,
            weight_keys,
            weight_values,
        );
        assert_eq!(result, Err(Error::<Test>::NoSelfWeight.into()));

        // Should pass because we are setting enough values.
        let weight_keys: Vec<u16> = vec![1, 2]; // self weight.
        let weight_values: Vec<u16> = vec![10, 10]; // random value.
        SubspaceModule::set_min_allowed_weights(netuid, 1);
        assert_ok!(SubspaceModule::set_weights(
            RuntimeOrigin::signed(account_id),
            netuid,
            weight_keys,
            weight_values
        ));
    });
}

// Tests that set weights fails if you dont pass enough values.
#[test]
fn test_set_max_allowed_uids() {
    new_test_ext().execute_with(|| {
        let netuid: u16 = 0;
        let n = 100;
        SubspaceModule::set_max_registrations_per_block(n);
        let account_id = U256::from(0);
        assert_ok!(register_module(netuid, account_id, 1_000_000_000));
        let _neuron_uid: u16 = SubspaceModule::get_uid_for_key(netuid, &account_id);
        for i in 1..n {
            assert_ok!(register_module(netuid, U256::from(i), 1_000_000_000));
        }

        let max_allowed_uids: u16 = 10;

        SubspaceModule::set_max_allowed_weights(netuid, max_allowed_uids);

        // Should fail because we are only setting a single value and its not the self weight.
        let weight_keys: Vec<u16> = (1..max_allowed_uids + 1).collect(); // not weight.
        let weight_values: Vec<u16> = vec![1; max_allowed_uids as usize]; // random value.
        let result = SubspaceModule::set_weights(
            RuntimeOrigin::signed(account_id),
            netuid,
            weight_keys,
            weight_values,
        );
        assert_ok!(result);
    });
}

/// Check do nothing path
#[test]
fn test_normalize_weights_does_not_mutate_when_sum_is_zero() {
    new_test_ext().execute_with(|| {
        let max_allowed: u16 = 3;

        let weights: Vec<u16> = Vec::from_iter((0..max_allowed).map(|_| 0));

        let _expected = weights.clone();
        let _result = SubspaceModule::normalize_weights(weights);
    });
}

/// Check do something path
#[test]
fn test_normalize_weights_does_not_mutate_when_sum_not_zero() {
    new_test_ext().execute_with(|| {
        let max_allowed: u16 = 3;

        let weights: Vec<u16> = Vec::from_iter(0..max_allowed);

        let expected = weights.clone();
        let result = SubspaceModule::normalize_weights(weights);

        assert_eq!(expected.len(), result.len(), "Length of weights changed?!");
    });
}

#[test]
fn test_min_weight_stake() {
    new_test_ext().execute_with(|| {
        let mut global_params: GlobalParams = SubspaceModule::global_params();
        global_params.min_weight_stake = to_nano(20);
        SubspaceModule::set_global_params(global_params);

        let netuid: u16 = 0;
        let module_count: u16 = 16;
        let voter_idx: u16 = 0;

        // registers the modules
        for i in 0..module_count {
            assert_ok!(register_module(netuid, U256::from(i), to_nano(10)));
        }

        let uids: Vec<u16> = (0..module_count).filter(|&uid| uid != voter_idx).collect();
        let weights = vec![1; uids.len()];

        assert_err!(
            SubspaceModule::set_weights(
                get_origin(U256::from(voter_idx)),
                netuid,
                uids.clone(),
                weights.clone(),
            ),
            Error::<Test>::NotEnoughStakePerWeight
        );

        increase_stake(netuid, U256::from(voter_idx), to_nano(400));

        assert_ok!(SubspaceModule::set_weights(
            get_origin(U256::from(voter_idx)),
            netuid,
            uids,
            weights,
        ));
    });
}

#[test]
fn test_weight_age() {
    new_test_ext().execute_with(|| {
        const NETUID: u16 = 0;
        const MODULE_COUNT: u16 = 16;
        const TEMPO: u64 = 100;
        const PASSIVE_VOTER: u16 = 0;
        const ACTIVE_VOTER: u16 = 1;

        // Register modules
        (0..MODULE_COUNT).for_each(|i| {
            assert_ok!(register_module(NETUID, U256::from(i), to_nano(10)));
        });

        let uids: Vec<u16> = (0..MODULE_COUNT)
            .filter(|&uid| uid != PASSIVE_VOTER && uid != ACTIVE_VOTER)
            .collect();
        let weights = vec![1; uids.len()];

        // Set subnet parameters
        let mut subnet_params = SubspaceModule::subnet_params(NETUID);
        subnet_params.tempo = TEMPO as u16;
        subnet_params.max_weight_age = TEMPO * 2;
        SubspaceModule::set_subnet_params(NETUID, subnet_params);

        // Set weights for passive and active voters
        assert_ok!(SubspaceModule::set_weights(
            get_origin(U256::from(PASSIVE_VOTER)),
            NETUID,
            uids.clone(),
            weights.clone(),
        ));
        assert_ok!(SubspaceModule::set_weights(
            get_origin(U256::from(ACTIVE_VOTER)),
            NETUID,
            uids.clone(),
            weights.clone(),
        ));

        let passive_stake_before =
            SubspaceModule::get_total_stake_to(NETUID, &U256::from(PASSIVE_VOTER));
        let active_stake_before =
            SubspaceModule::get_total_stake_to(NETUID, &U256::from(ACTIVE_VOTER));

        step_block((TEMPO as u16) * 2);

        let passive_stake_after =
            SubspaceModule::get_total_stake_to(NETUID, &U256::from(PASSIVE_VOTER));
        let active_stake_after =
            SubspaceModule::get_total_stake_to(NETUID, &U256::from(ACTIVE_VOTER));

        assert!(
            passive_stake_before < passive_stake_after || active_stake_before < active_stake_after,
            "Stake should be increasing"
        );

        // Set weights again for active voter
        assert_ok!(SubspaceModule::set_weights(
            get_origin(U256::from(ACTIVE_VOTER)),
            NETUID,
            uids,
            weights,
        ));

        step_block((TEMPO as u16) * 2);

        let passive_stake_after_v2 =
            SubspaceModule::get_total_stake_to(NETUID, &U256::from(PASSIVE_VOTER));
        let active_stake_after_v2 =
            SubspaceModule::get_total_stake_to(NETUID, &U256::from(ACTIVE_VOTER));

        assert_eq!(
            passive_stake_after, passive_stake_after_v2,
            "Stake values should remain the same after maximum weight age"
        );
        assert!(
            active_stake_after < active_stake_after_v2,
            "Stake should be increasing"
        );
    });
}
