mod mock;
use frame_support::assert_ok;

use pallet_subspace::Error;
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

        // Should fail because we are only setting a single value and its not the self weight.
        let weight_keys: Vec<u16> = vec![1]; // not weight.
        let weight_values: Vec<u16> = vec![88]; // random value.
        let result = SubspaceModule::set_weights(
            RuntimeOrigin::signed(account_id),
            netuid,
            weight_keys,
            weight_values,
        );
        assert_eq!(result, Err(Error::<Test>::NotSettingEnoughWeights.into()));

        // Shouldnt fail because we setting a single value but it is the self weight.

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
        let weight_keys: Vec<u16> = (0..max_allowed_uids).collect(); // not weight.
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

/// Check _falsey_ path for weights length
#[test]
fn test_is_self_weight_weights_length_not_one() {
    new_test_ext().execute_with(|| {
        let max_allowed: u16 = 3;

        let uids: Vec<u16> = Vec::from_iter((0..max_allowed).map(|id| id + 1));
        let uid: u16 = uids[0];
        let weights: Vec<u16> = Vec::from_iter((0..max_allowed).map(|id| id + 1));

        let expected = false;
        let result = SubspaceModule::is_self_weight(uid, &uids, &weights);

        assert_eq!(
            expected, result,
            "Failed get expected result when `weights.len() != 1`"
        );
    });
}

/// Check _falsey_ path for uid vs uids[0]
#[test]
fn test_is_self_weight_uid_not_in_uids() {
    new_test_ext().execute_with(|| {
        let max_allowed: u16 = 3;

        let uids: Vec<u16> = Vec::from_iter((0..max_allowed).map(|id| id + 1));
        let uid: u16 = uids[1];
        let weights: Vec<u16> = vec![0];

        let expected = false;
        let result = SubspaceModule::is_self_weight(uid, &uids, &weights);

        assert_eq!(
            expected, result,
            "Failed get expected result when `uid != uids[0]`"
        );
    });
}

/// Check _truthy_ path
/// @TODO: double-check if this really be desired behavior
#[test]
fn test_is_self_weight_uid_in_uids() {
    new_test_ext().execute_with(|| {
        let max_allowed: u16 = 1;

        let uids: Vec<u16> = Vec::from_iter((0..max_allowed).map(|id| id + 1));
        let uid: u16 = uids[0];
        let weights: Vec<u16> = vec![0];

        let expected = true;
        let result = SubspaceModule::is_self_weight(uid, &uids, &weights);

        assert_eq!(
            expected, result,
            "Failed get expected result when everything _should_ be fine"
        );
    });
}

/// Check _truthy_ path
#[test]
fn test_check_len_uids_within_allowed_within_network_pool() {
    new_test_ext().execute_with(|| {
        let netuid: u16 = 0;
        let _tempo: u16 = 13;

        SubspaceModule::set_max_registrations_per_block(100);

        /* @TODO: use a loop maybe */
        assert_ok!(register_module(netuid, U256::from(1), 1_000_000_000));
        assert_ok!(register_module(netuid, U256::from(3), 1_000_000_000));
        assert_ok!(register_module(netuid, U256::from(5), 1_000_000_000));
        let max_allowed: u16 = SubspaceModule::get_subnet_n(netuid);

        let uids: Vec<u16> = Vec::from_iter(0..max_allowed);

        let result = SubspaceModule::check_len_uids_within_allowed(netuid, &uids);
        assert!(result, "netuid network length and uids length incompatible");
    });
}

/// Check _falsey_ path
#[test]
fn test_check_len_uids_within_allowed_not_within_network_pool() {
    new_test_ext().execute_with(|| {
        let netuid: u16 = 0;

        let _tempo: u16 = 13;
        let _modality: u16 = 0;

        SubspaceModule::set_max_registrations_per_block(100);

        /* @TODO: use a loop maybe */
        assert_ok!(register_module(netuid, U256::from(1), 1_000_000_000));
        assert_ok!(register_module(netuid, U256::from(3), 1_000_000_000));
        assert_ok!(register_module(netuid, U256::from(5), 1_000_000_000));
        let max_allowed: u16 = SubspaceModule::get_subnet_n(netuid);

        SubspaceModule::set_max_allowed_uids(netuid, max_allowed);

        let uids: Vec<u16> = Vec::from_iter(0..(max_allowed + 1));

        let expected = false;
        let result = SubspaceModule::check_len_uids_within_allowed(netuid, &uids);
        assert_eq!(
            expected, result,
            "Failed to detect incompatible uids for network"
        );
    });
}
