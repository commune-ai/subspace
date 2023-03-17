use pallet_subspace::{Error};
use frame_support::{assert_ok};
use frame_system::Config;
mod mock;
use mock::*;
use frame_support::sp_runtime::DispatchError;
use frame_support::dispatch::{GetDispatchInfo, DispatchInfo};
use frame_support::weights::{DispatchClass, Pays};

/********************************************
	subscribing::subscribe() tests
*********************************************/
#[test]
fn test_subscribe_ok_dispatch_info_ok() {
	new_test_ext().execute_with(|| {
		let block_number: u64 = 0;
		let nonce: u64 = 0;
		let work: Vec<u8> = vec![0;32];
		let key: u64 = 0;
		let key: u64 = 0;
        let call = Call::subspace(subspaceCall::register{block_number, nonce, work, key, key });
		assert_eq!(call.get_dispatch_info(), DispatchInfo {
			weight: 0,
			class: DispatchClass::Normal,
			pays_fee: Pays::No
		});
	});
}

#[test]
fn test_difficulty() {
	new_test_ext().execute_with(|| {
		assert_eq!( subspace::get_difficulty().as_u64(), 10000 );
	});

}

#[test]
fn test_registration_repeat_work() {
	new_test_ext().execute_with(|| {
		let block_number: u64 = 0;
		let key_account_id_1 = 1;
		let key_account_id_2 = 2;
		let key_account_id = 667; // Neighbour of the beast, har har
		let (nonce, work): (u64, Vec<u8>) = subspace::create_work_for_block_number( block_number, 0);
		assert_ok!(subspace::register(<<Test as Config>::Origin>::signed(key_account_id_1)));
		let result = subspace::register(<<Test as Config>::Origin>::signed(key_account_id_2));
		assert_eq!( result, Err(Error::<Test>::WorkRepeated.into()) );
	});
}

#[test]
fn test_registration_ok() {
	new_test_ext().execute_with(|| {
		let block_number: u64 = 0;
		let (nonce, work): (u64, Vec<u8>) = subspace::create_work_for_block_number( block_number, 129123813);
		let key_account_id = 1;
		let key_account_id = 667; // Neighbour of the beast, har har

		// Subscribe and check extrinsic output
		assert_ok!(subspace::register(<<Test as Config>::Origin>::signed(key_account_id), block_number, nonce, work, key_account_id, key_account_id));
		let neuron = subspace::get_neuron_for_key(&key_account_id);

		// Check uid setting functionality
		assert_eq!(neuron.uid, 0);

		// Check if metadata is set correctly
		assert_eq!(neuron.ip, 0);
		assert_eq!(neuron.port, 0);
		assert_eq!(neuron.key, key_account_id);

		// Check if this function works
		assert_eq!(subspace::is_uid_active(neuron.uid), true);

		// Check neuron count increment functionality
        assert_eq!(subspace::get_neuron_count(), 1);

		// Check if weights are set correctly. Only self weight
		assert_eq!( subspace::get_weights_for_neuron(&neuron), vec![u32::MAX] );

		// Check if the neuron has a key account
		assert_eq!(subspace::has_key_account(&neuron.uid), true);

		// Check if the balance of this key account == 0
	});
}

#[test]
fn test_too_many_registrations_per_block() {
	new_test_ext().execute_with(|| {
		
		subspace::set_max_registratations_per_block( 10 );

		// Subscribe and check extrinsic output
		assert_ok!(subspace::register(<<Test as Config>::Origin>::signed(0)));
		assert_ok!(subspace::register(<<Test as Config>::Origin>::signed(1)));
		assert_ok!(subspace::register(<<Test as Config>::Origin>::signed(2)));
		assert_ok!(subspace::register(<<Test as Config>::Origin>::signed(3)));
		assert_ok!(subspace::register(<<Test as Config>::Origin>::signed(4)));
		assert_ok!(subspace::register(<<Test as Config>::Origin>::signed(5)));
		assert_ok!(subspace::register(<<Test as Config>::Origin>::signed(6)));
		assert_ok!(subspace::register(<<Test as Config>::Origin>::signed(7)));
		assert_ok!(subspace::register(<<Test as Config>::Origin>::signed(8)));
		assert_ok!(subspace::register(<<Test as Config>::Origin>::signed(9)));
		let result = subspace::register(<<Test as Config>::Origin>::signed(10));
		assert_eq!( result, Err(Error::<Test>::ToManyRegistrationsThisBlock.into()) );
	});
}

#[test]
fn test_defaults() {
	new_test_ext().execute_with(|| {
		assert_eq!( subspace::get_target_registrations_per_interval(), 2 );
		assert_eq!( subspace::get_adjustment_interval(), 100 );
		assert_eq!( subspace::get_max_registratations_per_block(), 2 );
		step_block ( 1 );
		assert_eq!( subspace::get_target_registrations_per_interval(), 2 );
		assert_eq!( subspace::get_adjustment_interval(), 100 );
		assert_eq!( subspace::get_max_registratations_per_block(), 2 );
		subspace::set_adjustment_interval( 2 );
		subspace::set_target_registrations_per_interval( 2 );
		subspace::set_max_registratations_per_block( 2 );
		assert_eq!( subspace::get_target_registrations_per_interval(), 2 );
		assert_eq!( subspace::get_adjustment_interval(), 2 );
		assert_eq!( subspace::get_max_registratations_per_block(), 2 );
	});
}

#[test]
fn test_difficulty_adjustment() {
	new_test_ext().execute_with(|| {
		subspace::set_adjustment_interval( 1 );
		subspace::set_target_registrations_per_interval( 1 );
		subspace::set_difficulty_from_u64( 1 );
		assert_eq!( subspace::get_target_registrations_per_interval(), 1 );
		assert_eq!( subspace::get_adjustment_interval(), 1 );
		assert_eq!( subspace::get_max_registratations_per_block(), 2 );

		assert_ok!(subspace::register(<<Test as Config>::Origin>::signed(0), ));
		assert_ok!(subspace::register(<<Test as Config>::Origin>::signed(1)));
		assert_eq!( subspace::get_registrations_this_interval(), 2 );
		assert_eq!( subspace::get_registrations_this_block(), 2 );

		step_block ( 1 );
		assert_ok!(subspace::register(<<Test as Config>::Origin>::signed(2)));
		assert_ok!(subspace::register(<<Test as Config>::Origin>::signed(3)));
		step_block ( 1 );
		assert_ok!(subspace::register(<<Test as Config>::Origin>::signed(4)));
		assert_ok!(subspace::register(<<Test as Config>::Origin>::signed(5)));

	});
}

#[test]
fn test_immunity_period() {
	new_test_ext().execute_with(|| {
		subspace::set_max_allowed_uids ( 2 );
		subspace::set_immunity_period ( 2 );
		assert_eq!( subspace::get_max_allowed_uids(), 2 );
		assert_eq!( subspace::get_immunity_period(), 2 );

		// Register two neurons into the first two slots.
		let neuron0 = register_ok_neuron_with_nonce( 0, 0, 38282211);
		assert_eq!( neuron0.uid, 0 );
		let neuron1 = register_ok_neuron_with_nonce( 1, 1, 18912831231);
		assert_eq!( neuron1.uid, 1 );
		assert!( !subspace::will_be_prunned(0) );
		assert!( !subspace::will_be_prunned(1) );

		// Step to the next block.
		step_block ( 1 );

		// Register the next neuron, this causes the overflow over top of the max allowed.
		// Because both previous are immune, we will take the first uid to be prunned.
		let neuron2 = register_ok_neuron_with_nonce( 2, 2, 1979183123);
		assert_eq!( neuron2.uid, 0 );

		// Register the next neuron, this causes the overflow over top of the max allowed.
		// Because uid0 is owned by a uid with a larger registration block number the uid to
		// prune is now 0. All uids are immune at this stage.
		let neuron3 = register_ok_neuron_with_nonce( 3, 3, 8129123823582 );
		assert_eq!( neuron3.uid, 1 );
		assert!( subspace::will_be_prunned(0) );
		assert!( subspace::will_be_prunned(1) );

		// Step to the next block.
		// Add stake to subspace::::get_stake_pruning_min()
		subspace::set_stake_from_vector( vec![ subspace::get_stake_pruning_min(), 0 ] );
		assert_eq!( subspace::get_stake(), vec![ subspace::get_stake_pruning_min(), 0 ] );
		step_block ( 1 );

		// Register the next neuron, the previous neurons have immunity however the first has stake.
		let neuron4 = register_ok_neuron_with_nonce( 4, 4, 23525321);
		assert_eq!( neuron4.uid, 1 );

		// Register the next neuron, the first neuron still has stake but he was registed a block earlier. 
		// than neuron4, we go into slot 0
		let neuron5 = register_ok_neuron_with_nonce( 5, 5, 1235325532);
		assert_eq!( neuron5.uid, 0 );
		assert!( subspace::will_be_prunned(0) );
		assert!( subspace::will_be_prunned(1) );

		subspace::set_stake_from_vector( vec![ subspace::get_stake_pruning_min(), 0 ] );
		step_block ( 1 );
		step_block ( 1 );
		step_block ( 1 );

		// Register the next neuron, the first slot has stake go into slot 1
		let neuron6 = register_ok_neuron_with_nonce( 6, 6,21352352 );
		assert_eq!( neuron6.uid, 1 );
		assert!( !subspace::will_be_prunned(0) );
		assert!( subspace::will_be_prunned(1) );

		step_block ( 1 );
		// Prunned set is dropped.
		assert!( !subspace::will_be_prunned(0) );
		assert!( !subspace::will_be_prunned(1) );
		step_block ( 1 );
		step_block ( 1 );

		// Register the next neuron, the first slot has stake and both are no longer immune
		// so this goes into slot 1 again.
		let neuron7 = register_ok_neuron_with_nonce( 7, 7,12352352532 );
		assert_eq!( neuron7.uid, 1 );
		assert!( !subspace::will_be_prunned(0) );
		assert!( subspace::will_be_prunned(1) );

		step_block ( 1 );

		// Set stake of neuron7 to 2.
		subspace::set_stake_from_vector( vec![ subspace::get_stake_pruning_min(), subspace::get_stake_pruning_min() * 2 ] );

		// Register another this time going into slot 0.
		let neuron8 = register_ok_neuron_with_nonce( 8, 8 , 123213124234);
		assert_eq!( neuron8.uid, 0 );
		assert!( subspace::will_be_prunned(0) );
		assert!( !subspace::will_be_prunned(1) );

		// Check that the stake in slot 0 has decremented.
		// Note that the stake has been decremented.
		assert_eq!( subspace::get_stake(), vec![0, subspace::get_stake_pruning_min() * 2 ] );
		assert_eq!( subspace::get_total_stake(), subspace::get_stake_pruning_min() * 2 ); // Total stake has been decremented.
		assert_eq!(subspace::get_key_balance( &5 ) as u64, subspace::get_stake_pruning_min()); // The unstaked funds have been added to the neuron 5 key account.

		// Step blocks, nobody is immune anymore.
		step_block ( 1 );
		step_block ( 1 );
		step_block ( 1 );
		step_block ( 1 );

		// Set weight matrix so that slot 1 has an incentive.
		subspace::set_stake_from_vector( vec![ subspace::get_stake_pruning_min() * 2, subspace::get_stake_pruning_min() * 1 ] );
		let weights_matrix: Vec<Vec<u32>> = vec! [
            vec! [0, u32::max_value()],
            vec! [0, u32::max_value()]
        ];
        subspace::set_weights_from_matrix( weights_matrix.clone() );
		step_block ( 1 ); // Run epoch step to populate incentives.

		// Check that incentive match expected.
		let u64m: u64 = 18446744073709551615;
		assert_eq!( subspace::get_incentive(), vec![0, u64m] );

		// Register another, this time we are comparing stake proportion to incentive proportion.
		// Slot 1 has incentive proportion 1, slot0 has stake proportion 2/3. So this goes into slot 1.
		let neuron9 = register_ok_neuron_with_nonce( 9, 9 , 18203182312);
		assert_eq!( neuron9.uid, 1 );
		assert!( subspace::will_be_prunned(1) );
	});
}

#[test]
fn test_already_active_key() {
	new_test_ext().execute_with(|| {

		let block_number: u64 = 0;
		let (nonce, work): (u64, Vec<u8>) = subspace::create_work_for_block_number( block_number, 0);
		let key_account_id = 1;
		let key_account_id = 667;

		assert_ok!(subspace::register(<<Test as Config>::Origin>::signed(key_account_id), block_number, nonce, work, key_account_id, key_account_id));

		let block_number: u64 = 0;
		let (nonce, work): (u64, Vec<u8>) = subspace::create_work_for_block_number( block_number, 0);
		let key_account_id = 1;
		let key_account_id = 667;
		let result = subspace::register(<<Test as Config>::Origin>::signed(key_account_id), block_number, nonce, work, key_account_id, key_account_id);
		assert_eq!( result, Err(Error::<Test>::AlreadyRegistered.into()) );
	});
}


#[test]
fn test_invalid_seal() {
	new_test_ext().execute_with(|| {
		let block_number: u64 = 0;
		let (nonce, work): (u64, Vec<u8>) = subspace::create_work_for_block_number( 1, 0);
		let key_account_id = 1;
		let key_account_id = 667;
		let result = subspace::register(<<Test as Config>::Origin>::signed(key_account_id), block_number, nonce, work, key_account_id, key_account_id);
		assert_eq!( result, Err(Error::<Test>::InvalidSeal.into()) );
	});
}

#[test]
fn test_invalid_block_number() {
	new_test_ext().execute_with(|| {
		let block_number: u64 = 1;
		let (nonce, work): (u64, Vec<u8>) = subspace::create_work_for_block_number( block_number, 0);
		let key_account_id = 1;
		let key_account_id = 667;
		let result = subspace::register(<<Test as Config>::Origin>::signed(key_account_id), block_number, nonce, work, key_account_id, key_account_id);
		assert_eq!( result, Err(Error::<Test>::InvalidWorkBlock.into()) );
	});
}

#[test]
fn test_invalid_difficulty() {
	new_test_ext().execute_with(|| {
		let block_number: u64 = 0;
		let (nonce, work): (u64, Vec<u8>) = subspace::create_work_for_block_number( block_number, 0);
		let key_account_id = 1;
		let key_account_id = 667;
		subspace::set_difficulty_from_u64( 18_446_744_073_709_551_615u64 );
		let result = subspace::register(<<Test as Config>::Origin>::signed(key_account_id), block_number, nonce, work, key_account_id, key_account_id);
		assert_eq!( result, Err(Error::<Test>::InvalidDifficulty.into()) );
	});
}

#[test]
fn test_register_failed_no_signature() {
	new_test_ext().execute_with(|| {

		let block_number: u64 = 1;
		let (nonce, work): (u64, Vec<u8>) = subspace::create_work_for_block_number( block_number, 0);
		let key_account_id = 1;
		let key_account_id = 667; // Neighbour of the beast, har har

		// Subscribe and check extrinsic output
		let result = subspace::register(<<Test as Config>::Origin>::none(), block_number, nonce, work, key_account_id, key_account_id);
		assert_eq!(result, Err(DispatchError::BadOrigin.into()));
	});
}

/********************************************
	subscribing::get_next_uid() tests
*********************************************/
#[test]
fn test_get_next_uid() {
	new_test_ext().execute_with(|| {
        assert_eq!(subspace::get_next_uid(), 0); // We start with id 0
		assert_eq!(subspace::get_next_uid(), 1); // One up
		assert_eq!(subspace::get_next_uid(), 2) // One more
	});
}

