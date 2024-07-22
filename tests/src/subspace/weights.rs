use crate::mock::*;
use frame_support::assert_err;
use pallet_subnet_emission_api::{SubnetConsensus, SubnetEmissionApi};
use pallet_subspace::*;
use sp_runtime::DispatchError;

#[test]
fn set_weights_call_must_fail_with_keys_and_values_are_not_the_same_length() {
    new_test_ext().execute_with(|| {
        zero_min_burn();
        MinimumAllowedStake::<Test>::set(0);

        assert_ok!(register_module(0, 0, 1, false));

        let weights_keys = vec![1, 2, 3, 4, 5, 6];
        let weight_values = vec![1, 2, 3, 4, 5];

        let result =
            SubspaceMod::set_weights(RuntimeOrigin::signed(0), 0, weights_keys, weight_values);

        assert_err!(result, Error::<Test>::WeightVecNotEqualSize);
    });
}

#[test]
fn cannot_set_weights_with_duplicate_keys() {
    new_test_ext().execute_with(|| {
        zero_min_burn();
        MinimumAllowedStake::<Test>::set(0);

        let netuid = 0;

        MaxRegistrationsPerBlock::<Test>::set(100);

        assert_ok!(register_module(netuid, 0, 10, false));
        update_params!(netuid => { max_allowed_uids: 100 });

        assert_ok!(register_module(netuid, 1, 100, false));
        assert_ok!(register_module(netuid, 2, 10000, false));
        assert_ok!(register_module(netuid, 3, 10000000, false));

        assert_eq!(N::<Test>::get(netuid), 4);

        let duplicated_weights_keys: Vec<u16> = vec![1, 1, 1];
        let weight_values: Vec<u16> = vec![1, 2, 3];
        let result = SubspaceMod::set_weights(
            get_origin(0),
            netuid,
            duplicated_weights_keys,
            weight_values,
        );
        assert_err!(result, Error::<Test>::DuplicateUids);
    });
}

#[test]
fn set_weights_requires_signature() {
    new_test_ext().execute_with(|| {
        let uids: Vec<u16> = vec![];
        let values: Vec<u16> = vec![];
        let result = SubspaceMod::set_weights(RuntimeOrigin::none(), 1, uids, values);
        assert_err!(result, DispatchError::BadOrigin);
    });
}

#[test]
fn set_weights_only_accepts_existing_keys() {
    new_test_ext().execute_with(|| {
        zero_min_burn();
        MinimumAllowedStake::<Test>::set(0);

        assert_ok!(register_module(0, 0, 1, false));

        let invalid_weight_keys: Vec<u16> = vec![9999];
        let weight_values: Vec<u16> = vec![88];
        let result = SubspaceMod::set_weights(
            RuntimeOrigin::signed(0),
            0,
            invalid_weight_keys,
            weight_values,
        );
        assert_err!(result, Error::<Test>::InvalidUid);
    });
}

#[test]
fn set_weights_call_respects_rate_limit() {
    new_test_ext().execute_with(|| {
        zero_min_burn();
        MinimumAllowedStake::<Test>::set(0);

        assert_ok!(register_module(0, 0, 1, false));
        assert_ok!(register_module(1, 0, 1, false));
        assert_ok!(register_module(1, 1, 1, false));

        Tempo::<Test>::set(1, 5);

        MaximumSetWeightCallsPerEpoch::<Test>::set(1, Some(1));

        let set_weights = || SubspaceMod::set_weights(get_origin(0), 1, vec![1], vec![10]);

        assert_ok!(set_weights());
        assert_err!(set_weights(), Error::<Test>::MaxSetWeightsPerEpochReached);

        step_block(5);

        eprintln!("foo");

        assert_ok!(set_weights());
        assert_err!(set_weights(), Error::<Test>::MaxSetWeightsPerEpochReached);

        MaximumSetWeightCallsPerEpoch::<Test>::set(1, None);
        assert_ok!(set_weights());
        assert_ok!(set_weights());

        MaximumSetWeightCallsPerEpoch::<Test>::set(1, Some(0));
        assert_ok!(set_weights());
        assert_ok!(set_weights());
    });
}

#[test]
fn set_weights_call_respects_rootnet_weight_limit() {
    new_test_ext().execute_with(|| {
        zero_min_burn();
        MinimumAllowedStake::<Test>::set(0);

        assert_ok!(register_named_subnet(u32::MAX, 0, "Rootnet"));
        Test::set_subnet_consensus_type(0, Some(SubnetConsensus::Root));

        assert_ok!(register_root_validator(0, 1));
        assert_ok!(register_module(0, 1, 1, false));
        assert_ok!(register_module(1, 1, 1, false));

        let set_weights = || SubspaceMod::set_weights(get_origin(0), 0, vec![1], vec![10]);

        assert_ok!(set_weights());
        assert_err!(set_weights(), Error::<Test>::MaxSetWeightsPerEpochReached);

        step_block(10_800);

        assert_ok!(set_weights());
        assert_err!(set_weights(), Error::<Test>::MaxSetWeightsPerEpochReached);
    });
}

#[test]
fn set_weights_on_itself_is_invalid() {
    new_test_ext().execute_with(|| {
        zero_min_burn();
        MinimumAllowedStake::<Test>::set(0);

        assert_ok!(register_module(1, 0, 1, false));
        let result = SubspaceMod::set_weights(RuntimeOrigin::signed(0), 1, vec![0], vec![0]);
        assert_err!(result, Error::<Test>::NoSelfWeight);
    });
}

#[test]
fn set_weights_respects_min_and_max_weights() {
    new_test_ext().execute_with(|| {
        zero_min_burn();
        MinimumAllowedStake::<Test>::set(0);

        let account_id = 0;

        assert_ok!(register_module(1, account_id, 1, false));
        update_params!(1 => { min_allowed_weights: 2, max_allowed_weights: 3 });

        for i in 1..5 {
            assert_ok!(register_module(1, i, 1, false));
        }

        let result =
            SubspaceMod::set_weights(RuntimeOrigin::signed(account_id), 1, vec![1], vec![1]);
        assert_err!(result, Error::<Test>::InvalidUidsLength);

        let result = SubspaceMod::set_weights(
            RuntimeOrigin::signed(account_id),
            1,
            vec![1, 2, 3, 4],
            vec![1, 2, 3, 4],
        );
        assert_err!(result, Error::<Test>::InvalidUidsLength);

        let result = SubspaceMod::set_weights(
            RuntimeOrigin::signed(account_id),
            1,
            vec![1, 2, 3],
            vec![1, 2, 3],
        );
        assert_ok!(result);
    });
}

#[test]
fn set_weights_fails_for_stakes_below_minimum() {
    new_test_ext().execute_with(|| {
        let mut global_params = SubspaceMod::global_params();
        global_params.min_weight_stake = to_nano(20);
        assert_ok!(SubspaceMod::set_global_params(global_params));
        zero_min_burn();
        MaxRegistrationsPerBlock::<Test>::set(1000);

        let netuid = 1;
        let module_count = 16u16;
        let voter_key = 0u32;

        // registers the modules
        for i in 0..module_count {
            assert_ok!(register_module(netuid, i as u32, to_nano(10), false));
        }

        let uids: Vec<_> = (0..module_count).filter(|&uid| uid != voter_key as u16).collect();
        let weights = vec![1; uids.len()];

        assert_err!(
            SubspaceMod::set_weights(get_origin(voter_key), netuid, uids.clone(), weights.clone()),
            Error::<Test>::NotEnoughStakePerWeight
        );

        increase_stake(voter_key, to_nano(400));

        assert_ok!(SubspaceMod::set_weights(
            get_origin(voter_key),
            netuid,
            uids,
            weights,
        ));
    });
}

/// Test Setting Rootnet Weights On non-existant subnets
/// 1. Register the rootnet and set the consensus to Root assert_ok!(register_named_subnet(u32::MAX,
///    0, "Rootnet")); Test::set_subnet_consensus_type(0, Some(SubnetConsensus::Root));
/// 2. Register a rootnet validator let _ = assert_ok!(register_root_validator(val1_id,
///    val1_stake));
/// 2. Register the other subnets
/// 3. Set weights on those subnets that exist and exect succes
/// 4. Register another rootnet validator
/// 5. This time set weights on the non-existant subnet and expect an error, use assert_err
#[test]
fn set_weights_on_non_existent_subnets() {
    new_test_ext().execute_with(|| {
        // 1. Register the rootnet and set the consensus to Root
        assert_ok!(register_named_subnet(u32::MAX, 0, "Rootnet"));
        Test::set_subnet_consensus_type(0, Some(SubnetConsensus::Root));
        let universal_stake = to_nano(200);

        // 2. Register a rootnet validator
        let val1_id = 1;
        assert_ok!(register_root_validator(val1_id, universal_stake));

        // 3. Register the other subnets
        assert_ok!(register_named_subnet(1, 1, "Subnet1"));
        assert_ok!(register_named_subnet(2, 2, "Subnet2"));

        // Set weights on existing subnets
        assert_ok!(SubspaceMod::set_weights(
            get_origin(val1_id),
            0,
            vec![1, 2],
            vec![100, 100]
        ));
        // 4. Register another rootnet validator
        let val2_id = 2;
        assert_ok!(register_root_validator(val2_id, universal_stake));

        // 5. Set weights on a non-existent subnet and expect an error
        assert_err!(
            SubspaceMod::set_weights(get_origin(val2_id), 0, vec![1, 2, 3], vec![100, 100, 100]),
            Error::<Test>::InvalidUid
        );
    });
}

#[test]
fn delegate_weight_control() {
    new_test_ext().execute_with(|| {
        assert_ok!(register_named_subnet(u32::MAX, 0, "Rootnet"));
        Test::set_subnet_consensus_type(0, Some(SubnetConsensus::Root));

        assert_ok!(register_named_subnet(u32::MAX, 1, "Test"));

        let val1_id = 1;
        let val2_id = 2;
        let universal_stake = to_nano(200);

        let val1_uid = assert_ok!(register_root_validator(val1_id, universal_stake));
        let val2_uid = assert_ok!(register_root_validator(val2_id, universal_stake));
        assert_ok!(SubspaceMod::set_weights(
            get_origin(val1_id),
            0,
            vec![1],
            vec![u16::MAX]
        ));
        assert_ok!(SubspaceMod::delegate_rootnet_control(
            get_origin(val2_id),
            val1_id
        ));
        step_block(5401);
        assert_eq!(
            Weights::<Test>::get(0, val1_uid),
            Weights::<Test>::get(0, val2_uid)
        )
    });
}

#[test]
fn test_normalize_weights_does_not_mutate_when_sum_not_zero() {
    new_test_ext().execute_with(|| {
        let max_allowed: u16 = 3;

        let weights: Vec<u16> = Vec::from_iter(0..max_allowed);

        let expected = weights.clone();
        let result = SubspaceMod::normalize_weights(&weights);

        assert_eq!(expected.len(), result.len(), "Length of weights changed?!");
    });
}
