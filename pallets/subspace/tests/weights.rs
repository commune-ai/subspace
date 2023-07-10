mod mock;
use mock::*;
use pallet_subspace::{Error};
use frame_system::Config;
use frame_support::dispatch::{GetDispatchInfo, DispatchInfo, DispatchClass, Pays};
use frame_support::{assert_ok};
use sp_runtime::DispatchError;
use substrate_fixed::types::I32F32;
use sp_core::U256;

// /***************************
//   pub fn set_weights() tests
// *****************************/

// // Test the call passes through the subspace module.
// #[test]
// #[cfg(not(tarpaulin))]
// fn test_set_weights_dispatch_info_ok() {
// 	new_test_ext().execute_with(|| {
// 		let dests = vec![1, 1];
// 		let weights = vec![1, 1];
//         let netuid: u16 = 1;
// 		let call = RuntimeCall::SubspaceModule(subspaceCall::set_weights{netuid, dests, weights});
// 		let dispatch_info = call.get_dispatch_info();
		
// 		assert_eq!(dispatch_info.class, DispatchClass::Normal);
// 		assert_eq!(dispatch_info.pays_fee, Pays::No);
// 	});
// }




// // Test ensures that uids -- weights must have the same size.
// #[test]
// fn test_weights_err_weights_vec_not_equal_size() {
// 	new_test_ext().execute_with(|| {
//         let key_account_id = U256::from(55);
// 		add_network(0, key_account_id);
//     	register_module(1, key_account_id);
// 		let neuron_uid: u16 = SubspaceModule::get_uid_for_key( netuid, &key_account_id ).expect("Not registered.");
// 		SubspaceModule::set_validator_permit_for_uid(netuid, neuron_uid, true);
// 		let weights_keys: Vec<u16> = vec![1, 2, 3, 4, 5, 6];
// 		let weight_values: Vec<u16> = vec![1, 2, 3, 4, 5]; // Uneven sizes
// 		let result = SubspaceModule::set_weights(RuntimeOrigin::signed(key_account_id), 1, weights_keys, weight_values, 0);
// 		assert_eq!(result, Err(Error::<Test>::WeightVecNotEqualSize.into()));
// 	});
// }

// // Test ensures that uids can have not duplicates
// #[test]
// fn test_weights_err_has_duplicate_ids() {
// 	new_test_ext().execute_with(|| {
// 		let key_account_id = U256::from(666);
// 		let netuid: u16 = 0;
// 		register_module(netuid, key_account_id);
// 		SubspaceModule::set_max_allowed_uids(netuid, 100); // Allow many registrations per block.
// 		SubspaceModule::set_max_registrations_per_block(netuid, 100); // Allow many registrations per block.
		

// 		// uid 1
// 		register_module( netuid, U256::from(1));
// 		SubspaceModule::get_uid_for_key( netuid, &U256::from(1) ).expect("Not registered.");

// 		// uid 2
// 		register_module( netuid, U256::from(2));
// 		SubspaceModule::get_uid_for_key( netuid, &U256::from(2) ).expect("Not registered.");

// 		// uid 3
// 		register_module( netuid, U256::from(3));
// 		SubspaceModule::get_uid_for_key( netuid, &U256::from(3) ).expect("Not registered.");
		
// 		assert_eq!(SubspaceModule::get_subnet_n(netuid), 4);

// 		let weights_keys: Vec<u16> = vec![1, 1, 1]; // Contains duplicates
// 		let weight_values: Vec<u16> = vec![1, 2, 3];
// 		let result = SubspaceModule::set_weights(RuntimeOrigin::signed(key_account_id), netuid, weights_keys, weight_values);
// 		assert_eq!(result, Err(Error::<Test>::DuplicateUids.into()));
// 	});
// }


// // Tests the call requires a valid origin.
// #[test]
// fn test_no_signature() {
// 	new_test_ext().execute_with(|| {
// 		let uids: Vec<u16> = vec![];
// 		let values: Vec<u16> = vec![];
// 		let result = SubspaceModule::set_weights(RuntimeOrigin::none(), 1, uids, values);
// 		assert_eq!(result, Err(DispatchError::BadOrigin.into()));
// 	});
// }


// // Tests that set weights fails if you pass invalid uids.
// #[test]
// fn test_set_weights_err_invalid_uid() {
// 	new_test_ext().execute_with(|| {
// 		let key_account_id = U256::from(55);
//         let netuid: u16 = 0;
// 		register_module( netuid, key_account_id, 1_000_000_000);
// 		let neuron_uid: u16 = SubspaceModule::get_uid_for_key( netuid, &key_account_id ).expect("Not registered.");
// 		SubspaceModule::set_validator_permit_for_uid(netuid, neuron_uid, true);
// 		let weight_keys : Vec<u16> = vec![9999]; // Does not exist
// 		let weight_values : Vec<u16> = vec![88]; // random value
// 		let result = SubspaceModule::set_weights(RuntimeOrigin::signed(key_account_id), 1, weight_keys, weight_values, 0);
// 		assert_eq!(result, Err(Error::<Test>::InvalidUid.into()));
// 	});
// }

// // Tests that set weights fails if you dont pass enough values.
// #[test]
// fn test_set_weight_not_enough_values() {
// 	new_test_ext().execute_with(|| {
        
// 		let netuid: u16 = 0;
// 		let account_id = U256::from(1);
		
// 		register_module(netuid, account_id,  1_000_000_000);
// 		let neuron_uid: u16 = SubspaceModule::get_uid_for_key( netuid, &U256::from(1) ).expect("Not registered.");

// 		register_module(netuid, U256::from(3),  1_000_000_000);
// 		SubspaceModule::set_min_allowed_weights(netuid, 2);

// 		// Should fail because we are only setting a single value and its not the self weight.
// 		let weight_keys : Vec<u16> = vec![1]; // not weight. 
// 		let weight_values : Vec<u16> = vec![88]; // random value.
// 		let result = SubspaceModule::set_weights(RuntimeOrigin::signed(account_id), netuid, weight_keys, weight_values);
// 		assert_eq!(result, Err(Error::<Test>::NotSettingEnoughWeights.into()));

// 		// Shouldnt fail because we setting a single value but it is the self weight.
// 		let weight_keys : Vec<u16> = vec![0]; // self weight.
// 		let weight_values : Vec<u16> = vec![88]; // random value.
// 		assert_ok!( SubspaceModule::set_weights(RuntimeOrigin::signed(account_id), 1 , weight_keys, weight_values)) ;

// 		// Should pass because we are setting enough values.
// 		let weight_keys : Vec<u16> = vec![0, 1]; // self weight. 
// 		let weight_values : Vec<u16> = vec![10, 10]; // random value.
// 		SubspaceModule::set_min_allowed_weights(1, 1);
// 		assert_ok!( SubspaceModule::set_weights(RuntimeOrigin::signed(account_id), 1,  weight_keys, weight_values)) ;
// 	});
// }


// // Tests that the weights set doesn't panic if you pass weights that sum to larger than u16 max.
// #[test]
// fn test_set_weights_sum_larger_than_u16_max() {
// 	new_test_ext().execute_with(|| {
        
// 		let netuid: u16 = 0;
		
// 		register_module(netuid, U256::from(1), 100);
// 		let neuron_uid: u16 = SubspaceModule::get_uid_for_key( netuid, &U256::from(1) ).expect("Not registered.");

// 		register_module(netuid, U256::from(3), 100);
// 		SubspaceModule::set_min_allowed_weights(1, 2);
	

// 		// Shouldn't fail because we are setting the right number of weights.
// 		let weight_keys : Vec<u16> = vec![0, 1];
// 		let weight_values : Vec<u16> = vec![u16::MAX, u16::MAX];
// 		// sum of weights is larger than u16 max.
// 		assert!( weight_values.iter().map(|x| *x as u64).sum::<u64>() > (u16::MAX as u64) );

// 		let result = SubspaceModule::set_weights(RuntimeOrigin::signed(U256::from(1)), 1, weight_keys, weight_values);
// 		assert_ok!(result);

// 		// Get max-upscaled unnormalized weights.
// 		let all_weights: Vec<Vec<I32F32>> = SubspaceModule::get_weights(netuid);
// 		let weights_set: &Vec<I32F32> = &all_weights[neuron_uid as usize];
// 		assert_eq!( weights_set[0], I32F32::from_num(u16::MAX) );
// 		assert_eq!( weights_set[1], I32F32::from_num(u16::MAX) );
// 	});
// }



// /// Check _falsey_ path for weights outside allowed range
// #[test]
// fn test_check_length_to_few_weights() {
// 	new_test_ext().execute_with(|| {
// 		let netuid: u16 = 1;

// 		let max_allowed: u16 = 3;
// 		let min_allowed_weights = max_allowed + 1;

// 		SubspaceModule::set_min_allowed_weights(netuid, min_allowed_weights);

// 		let uids: Vec<u16> = Vec::from_iter((0..max_allowed).map(|id| { id + 1 }));
// 		let uid: u16 = uids[0].clone();
// 		let weights: Vec<u16> = Vec::from_iter((0..max_allowed).map(|id| { id + 1 }));

// 		let expected = false;
// 		let result = SubspaceModule::check_length(netuid, uid, &uids, &weights);

// 		assert_eq!(
// 			expected,
// 			result,
// 			"Failed get expected result"
// 		);
// 	});
// }

// /// Check do nothing path
// #[test]
// fn test_normalize_weights_does_not_mutate_when_sum_is_zero() {
// 	new_test_ext().execute_with(|| {
// 		let max_allowed: u16 = 3;

// 		let weights: Vec<u16> = Vec::from_iter((0..max_allowed).map(|_| { 0 }));

// 		let expected = weights.clone();
// 		let result = SubspaceModule::normalize_weights(weights);

// 		assert_eq!(
// 			expected,
// 			result,
// 			"Failed get expected result when everything _should_ be fine"
// 		);
// 	});
// }

// /// Check do something path
// #[test]
// fn test_normalize_weights_does_not_mutate_when_sum_not_zero() {
// 	new_test_ext().execute_with(|| {
// 		let max_allowed: u16 = 3;

// 		let weights: Vec<u16> = Vec::from_iter((0..max_allowed).map(|weight| { weight }));

// 		let expected = weights.clone();
// 		let result = SubspaceModule::normalize_weights(weights);

// 		assert_eq!(
// 			expected.len(),
// 			result.len(),
// 			"Length of weights changed?!"
// 		);
// 	});
// }

// /// Check _truthy_ path for weights length
// #[test]
// fn test_max_weight_limited_allow_self_weights_to_exceed_max_weight_limit() {
// 	new_test_ext().execute_with(|| {
// 		let max_allowed: u16 = 1;

// 		let netuid: u16 = 1;
// 		let uids: Vec<u16> = Vec::from_iter((0..max_allowed).map(|id| { id + 1 }));
// 		let uid: u16 = uids[0].clone();
// 		let weights: Vec<u16> = vec![0];

// 		let expected = true;
// 		let result = SubspaceModule::max_weight_limited(netuid, uid, &uids, &weights);

// 		assert_eq!(
// 			expected,
// 			result,
// 			"Failed get expected result when everything _should_ be fine"
// 		);
// 	});
// }

// /// Check _truthy_ path for max weight limit
// #[test]
// fn test_max_weight_limited_when_weight_limit_is_u16_max() {
// 	new_test_ext().execute_with(|| {
// 		let max_allowed: u16 = 3;

// 		let netuid: u16 = 1;
// 		let uids: Vec<u16> = Vec::from_iter((0..max_allowed).map(|id| { id + 1 }));
// 		let uid: u16 = uids[0].clone();
// 		let weights: Vec<u16> = Vec::from_iter((0..max_allowed).map(|_id| { u16::MAX }));

// 		let expected = true;
// 		let result = SubspaceModule::max_weight_limited(netuid, uid, &uids, &weights);

// 		assert_eq!(
// 			expected,
// 			result,
// 			"Failed get expected result when everything _should_ be fine"
// 		);
// 	});
// }

// /// Check _truthy_ path for max weight limit
// #[test]
// fn test_max_weight_limited_when_max_weight_is_within_limit() {
// 	new_test_ext().execute_with(|| {
// 		let max_allowed: u16 = 1;
// 		let max_weight_limit = u16::MAX / 5;

// 		let netuid: u16 = 1;
// 		let uids: Vec<u16> = Vec::from_iter((0..max_allowed).map(|id| { id + 1 }));
// 		let uid: u16 = uids[0].clone();
// 		let weights: Vec<u16> = Vec::from_iter((0..max_allowed).map(|id| { max_weight_limit - id }));

// 		SubspaceModule::set_max_weight_limit(netuid, max_weight_limit);

// 		let expected = true;
// 		let result = SubspaceModule::max_weight_limited(netuid, uid, &uids, &weights);

// 		assert_eq!(
// 			expected,
// 			result,
// 			"Failed get expected result when everything _should_ be fine"
// 		);
// 	});
// }

// /// Check _falsey_ path
// #[test]
// fn test_max_weight_limited_when_guard_checks_are_not_triggered() {
// 	new_test_ext().execute_with(|| {
// 		let max_allowed: u16 = 3;
// 		let max_weight_limit = u16::MAX / 5;

// 		let netuid: u16 = 1;
// 		let uids: Vec<u16> = Vec::from_iter((0..max_allowed).map(|id| { id + 1 }));
// 		let uid: u16 = uids[0].clone();
// 		let weights: Vec<u16> = Vec::from_iter((0..max_allowed).map(|id| { max_weight_limit + id }));

// 		SubspaceModule::set_max_weight_limit(netuid, max_weight_limit);

// 		let expected = false;
// 		let result = SubspaceModule::max_weight_limited(netuid, uid, &uids, &weights);

// 		assert_eq!(
// 			expected,
// 			result,
// 			"Failed get expected result when guard-checks were not triggered"
// 		);
// 	});
// }

// /// Check _falsey_ path for weights length
// #[test]
// fn test_is_self_weight_weights_length_not_one() {
// 	new_test_ext().execute_with(|| {
// 		let max_allowed: u16 = 3;

// 		let uids: Vec<u16> = Vec::from_iter((0..max_allowed).map(|id| { id + 1 }));
// 		let uid: u16 = uids[0].clone();
// 		let weights: Vec<u16> = Vec::from_iter((0..max_allowed).map(|id| { id + 1 }));

// 		let expected = false;
// 		let result = SubspaceModule::is_self_weight(uid, &uids, &weights);

// 		assert_eq!(
// 			expected,
// 			result,
// 			"Failed get expected result when `weights.len() != 1`"
// 		);
// 	});
// }

// /// Check _falsey_ path for uid vs uids[0]
// #[test]
// fn test_is_self_weight_uid_not_in_uids() {
// 	new_test_ext().execute_with(|| {
// 		let max_allowed: u16 = 3;

// 		let uids: Vec<u16> = Vec::from_iter((0..max_allowed).map(|id| { id + 1 }));
// 		let uid: u16 = uids[1].clone();
// 		let weights: Vec<u16> = vec![0];

// 		let expected = false;
// 		let result = SubspaceModule::is_self_weight(uid, &uids, &weights);

// 		assert_eq!(
// 			expected,
// 			result,
// 			"Failed get expected result when `uid != uids[0]`"
// 		);
// 	});
// }

// /// Check _truthy_ path
// /// @TODO: double-check if this really be desired behavior
// #[test]
// fn test_is_self_weight_uid_in_uids() {
// 	new_test_ext().execute_with(|| {
// 		let max_allowed: u16 = 1;

// 		let uids: Vec<u16> = Vec::from_iter((0..max_allowed).map(|id| { id + 1 }));
// 		let uid: u16 = uids[0].clone();
// 		let weights: Vec<u16> = vec![0];

// 		let expected = true;
// 		let result = SubspaceModule::is_self_weight(uid, &uids, &weights);

// 		assert_eq!(
// 			expected,
// 			result,
// 			"Failed get expected result when everything _should_ be fine"
// 		);
// 	});
// }

// /// Check _truthy_ path
// #[test]
// fn test_check_len_uids_within_allowed_within_network_pool() {
// 	new_test_ext().execute_with(|| {
// 		let netuid: u16 = 1;
// 		let tempo: u16 = 13;

// 		let max_registrations_per_block: u16 = 100;

// 		/* @TODO: use a loop maybe */
// 		register_module(netuid, U256::from(1));
// 		register_module(netuid, U256::from(3));
// 		register_module(netuid, U256::from(5));
// 		let max_allowed: u16 = SubspaceModule::get_subnet_n(netuid);

// 		SubspaceModule::set_max_allowed_uids(netuid, max_allowed);
// 		SubspaceModule::set_max_registrations_per_block(netuid, max_registrations_per_block);

// 		let uids: Vec<u16> = Vec::from_iter((0..max_allowed).map(|uid| { uid }));

// 		let expected = true;
// 		let result = SubspaceModule::check_len_uids_within_allowed(netuid, &uids);
// 		assert_eq!(expected, result, "netuid network length and uids length incompatible");
// 	});
// }

// /// Check _falsey_ path
// #[test]
// fn test_check_len_uids_within_allowed_not_within_network_pool() {
// 	new_test_ext().execute_with(|| {
// 		let netuid: u16 = 1;

// 		let tempo: u16 = 13;
// 		let modality: u16 = 0;

// 		let max_registrations_per_block: u16 = 100;

// 		/* @TODO: use a loop maybe */
// 		register_module(netuid, U256::from(1),  1_000_000_000);
// 		register_module(netuid, U256::from(3),  1_000_000_000);
// 		register_module(netuid, U256::from(5),  1_000_000_000);
// 		let max_allowed: u16 = SubspaceModule::get_subnet_n(netuid);

// 		SubspaceModule::set_max_allowed_uids(netuid, max_allowed);
// 		SubspaceModule::set_max_registrations_per_block(netuid, max_registrations_per_block);

// 		let uids: Vec<u16> = Vec::from_iter((0..(max_allowed + 1)).map(|uid| { uid }));

// 		let expected = false;
// 		let result = SubspaceModule::check_len_uids_within_allowed(netuid, &uids);
// 		assert_eq!(expected, result, "Failed to detect incompatible uids for network");
// 	});
// }
