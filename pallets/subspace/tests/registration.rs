
use frame_support::traits::Currency;
use ndarray::stack_new_axis;
use pallet_subspace::{Error, AxonInfoOf};
use frame_support::{assert_ok};
use frame_system::Config;
use sp_core::U256;
use crate::{mock::*};
use frame_support::sp_runtime::DispatchError;
use frame_support::dispatch::{GetDispatchInfo, DispatchInfo};
use frame_support::weights::{DispatchClass, Pays};

mod mock;

/********************************************
	subscribing::subscribe() tests
*********************************************/




#[test]
fn test_registration_ok() {
	new_test_ext().execute_with(|| {
		let block_number: u64 = 0;
		let netuid: u16 = 1;
		let tempo: u16 = 13;
		let hotkey_account_id: U256 = U256::from(1);
		let coldkey_account_id = U256::from(667); // Neighbour of the beast, har har
		let (nonce, work): (u64, Vec<u8>) = SubspaceModule::create_work_for_block_number( netuid, block_number, 129123813, &hotkey_account_id);

		//add network
		add_network(netuid, tempo, 0);
		
		// Subscribe and check extrinsic output
		assert_ok!(SubspaceModule::register(<<Test as Config>::RuntimeOrigin>::signed(hotkey_account_id), netuid, block_number, nonce, work, hotkey_account_id, coldkey_account_id));

		// Check if neuron has added to the specified network(netuid)
		assert_eq!(SubspaceModule::get_subnet_n(netuid), 1);

		//check if hotkey is added to the Hotkeys
		assert_eq!(SubspaceModule::get_owning_coldkey_for_hotkey(&hotkey_account_id), coldkey_account_id);

		// Check if the neuron has added to the Keys
		let neuron_uid = SubspaceModule::get_uid_for_net_and_hotkey(netuid, &hotkey_account_id).unwrap();
		
		assert!(SubspaceModule::get_uid_for_net_and_hotkey(netuid, &hotkey_account_id).is_ok());
		// Check if neuron has added to Uids
		let neuro_uid = SubspaceModule::get_uid_for_net_and_hotkey(netuid, &hotkey_account_id).unwrap();
		assert_eq!(neuro_uid, neuron_uid);

		// Check if the balance of this hotkey account for this subnetwork == 0
		assert_eq!(SubspaceModule::get_stake_for_uid_and_subnetwork(netuid, neuron_uid), 0);
	});
}



#[test]
#[cfg(not(tarpaulin))]
fn test_registration_too_many_registrations_per_block() {
	new_test_ext().execute_with(|| {
		
		let netuid: u16 = 1;
		let tempo: u16 = 13;
		SubspaceModule::set_max_registrations_per_block( netuid, 10 );
		SubspaceModule::set_target_registrations_per_interval( netuid, 10 );
		assert_eq!( SubspaceModule::get_max_registrations_per_block(netuid), 10 );

		let block_number: u64 = 0;
		let (nonce0, work0): (u64, Vec<u8>) = SubspaceModule::create_work_for_block_number( netuid, block_number, 3942084, &U256::from(0));
		let (nonce1, work1): (u64, Vec<u8>) = SubspaceModule::create_work_for_block_number( netuid, block_number, 11231312312, &U256::from(1));
		let (nonce2, work2): (u64, Vec<u8>) = SubspaceModule::create_work_for_block_number( netuid, block_number, 212312414, &U256::from(2));
		let (nonce3, work3): (u64, Vec<u8>) = SubspaceModule::create_work_for_block_number( netuid, block_number, 21813123, &U256::from(3));
		let (nonce4, work4): (u64, Vec<u8>) = SubspaceModule::create_work_for_block_number( netuid, block_number, 148141209, &U256::from(4));
		let (nonce5, work5): (u64, Vec<u8>) = SubspaceModule::create_work_for_block_number( netuid, block_number, 1245235534, &U256::from(5));
		let (nonce6, work6): (u64, Vec<u8>) = SubspaceModule::create_work_for_block_number( netuid, block_number, 256234, &U256::from(6));
		let (nonce7, work7): (u64, Vec<u8>) = SubspaceModule::create_work_for_block_number( netuid, block_number, 6923424, &U256::from(7));
		let (nonce8, work8): (u64, Vec<u8>) = SubspaceModule::create_work_for_block_number( netuid, block_number, 124242, &U256::from(8));
		let (nonce9, work9): (u64, Vec<u8>) = SubspaceModule::create_work_for_block_number( netuid, block_number, 153453, &U256::from(9));
		let (nonce10, work10): (u64, Vec<u8>) = SubspaceModule::create_work_for_block_number( netuid, block_number, 345923888, &U256::from(10));
		assert_eq!( SubspaceModule::get_difficulty_as_u64(netuid), 10000 );

		//add network
		add_network(netuid, tempo, 0);
		
		// Subscribe and check extrinsic output
		assert_ok!(SubspaceModule::register(<<Test as Config>::RuntimeOrigin>::signed(U256::from(0)), netuid, block_number, nonce0, work0, U256::from(0), U256::from(0)));
		assert_eq!( SubspaceModule::get_registrations_this_block(netuid), 1 );
		assert_ok!(SubspaceModule::register(<<Test as Config>::RuntimeOrigin>::signed(U256::from(1)), netuid, block_number, nonce1, work1, U256::from(1), U256::from(1)));
		assert_eq!( SubspaceModule::get_registrations_this_block(netuid), 2 );
		assert_ok!(SubspaceModule::register(<<Test as Config>::RuntimeOrigin>::signed(U256::from(2)), netuid, block_number, nonce2, work2, U256::from(2), U256::from(2)));
		assert_eq!( SubspaceModule::get_registrations_this_block(netuid), 3 );
		assert_ok!(SubspaceModule::register(<<Test as Config>::RuntimeOrigin>::signed(U256::from(3)), netuid, block_number, nonce3, work3, U256::from(3), U256::from(3)));
		assert_eq!( SubspaceModule::get_registrations_this_block(netuid), 4 );
		assert_ok!(SubspaceModule::register(<<Test as Config>::RuntimeOrigin>::signed(U256::from(4)), netuid, block_number, nonce4, work4, U256::from(4), U256::from(4)));
		assert_eq!( SubspaceModule::get_registrations_this_block(netuid), 5 );
		assert_ok!(SubspaceModule::register(<<Test as Config>::RuntimeOrigin>::signed(U256::from(5)), netuid, block_number, nonce5, work5, U256::from(5), U256::from(5)));
		assert_eq!( SubspaceModule::get_registrations_this_block(netuid), 6 );
		assert_ok!(SubspaceModule::register(<<Test as Config>::RuntimeOrigin>::signed(U256::from(6)), netuid, block_number, nonce6, work6, U256::from(6), U256::from(6)));
		assert_eq!( SubspaceModule::get_registrations_this_block(netuid), 7 );
		assert_ok!(SubspaceModule::register(<<Test as Config>::RuntimeOrigin>::signed(U256::from(7)), netuid, block_number, nonce7, work7, U256::from(7), U256::from(7)));
		assert_eq!( SubspaceModule::get_registrations_this_block(netuid), 8 );
		assert_ok!(SubspaceModule::register(<<Test as Config>::RuntimeOrigin>::signed(U256::from(8)), netuid, block_number, nonce8, work8, U256::from(8), U256::from(8)));
		assert_eq!( SubspaceModule::get_registrations_this_block(netuid), 9 );
		assert_ok!(SubspaceModule::register(<<Test as Config>::RuntimeOrigin>::signed(U256::from(9)), netuid, block_number, nonce9, work9, U256::from(9), U256::from(9)));
		assert_eq!( SubspaceModule::get_registrations_this_block(netuid), 10 );
		let result = SubspaceModule::register(<<Test as Config>::RuntimeOrigin>::signed(U256::from(10)), netuid, block_number, nonce10, work10, U256::from(10), U256::from(10));
		assert_eq!( result, Err(Error::<Test>::TooManyRegistrationsThisBlock.into()) );
	});
}






