use crate::mock::*;
use frame_support::assert_err;
use pallet_subspace::*;
use sp_runtime::DispatchError;

#[test]
fn set_weights_call_must_fail_with_keys_and_values_are_not_the_same_length() {
    new_test_ext().execute_with(|| {
        zero_min_burn();

        assert_ok!(register_module(0, 0, 1));

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

        let netuid = 0;

        MaxRegistrationsPerBlock::<Test>::set(100);

        assert_ok!(register_module(netuid, 0, 10));
        update_params!(netuid => { max_allowed_uids: 100 });

        assert_ok!(register_module(netuid, 1, 100));
        assert_ok!(register_module(netuid, 2, 10000));
        assert_ok!(register_module(netuid, 3, 10000000));

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

        assert_ok!(register_module(0, 0, 1));

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
