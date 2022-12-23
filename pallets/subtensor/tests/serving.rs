use pallet_subtensor::{Error};
use frame_support::{assert_ok};
use frame_system::Config;
mod mock;
use mock::*;
use frame_support::dispatch::{GetDispatchInfo, DispatchInfo};
use frame_support::weights::{DispatchClass, Pays};

/********************************************
	subscribing::serving() tests
*********************************************/
#[test]
fn test_serve_ok_dispatch_info_ok() {
	new_test_ext().execute_with(|| {
		let version = 0;
		let ip = ipv4(8,8,8,8);
		let port = 8883;
		let ip_type = 4;
        let modality = 0;
        let call = Call::Subtensor(SubtensorCall::serve_axon{version, ip, port, ip_type, modality});
		assert_eq!(call.get_dispatch_info(), DispatchInfo {
			weight: 0,
			class: DispatchClass::Normal,
			pays_fee: Pays::No
		});
	});
}

#[test]
fn test_serve_not_registered() {
	new_test_ext().execute_with(|| {
		let version = 0;
		let ip = ipv4(8,8,8,8);
		let ip_type = 4;
		let port = 1337;
		let modality = 0;
		let hotkey: u64 = 0;

		let result = Subtensor::serve_axon(<<Test as Config>::Origin>::signed(hotkey), version, ip, port, ip_type, modality );
		assert_eq!( result, Err(Error::<Test>::NotRegistered.into()) );
    });
}

#[test]
fn test_serve_invalid_modality() {
	new_test_ext().execute_with(|| {
		let version = 0;
		let ip = ipv4(8,8,8,8);
		let ip_type = 4;
		let port = 1337;
		let modality = 1; // Not Allowed.
		let hotkey: u64 = 0;
		let coldkey: u64 = 0;

		register_ok_neuron(hotkey, coldkey);
		let result = Subtensor::serve_axon(<<Test as Config>::Origin>::signed(hotkey), version, ip, port, ip_type, modality );
		assert_eq!(result, Err(Error::<Test>::InvalidModality.into()));
    });
}

#[test]
fn test_serve_invalid_ip() {
	new_test_ext().execute_with(|| {
		let version = 0;
		let ip = ipv4(127,0,0,1); // Not allowed.
		let ip_type = 4;
		let port = 1337;
		let modality = 0;
		let hotkey: u64 = 0;
		let coldkey: u64 = 0;

		register_ok_neuron(hotkey, coldkey);
		let result = Subtensor::serve_axon(<<Test as Config>::Origin>::signed(hotkey), version, ip, port, ip_type, modality );
		assert_eq!(result, Err(Error::<Test>::InvalidIpAddress.into()));
	});
}

#[test]
fn test_serve_invalid_ipv6() {
	new_test_ext().execute_with(|| {
		let version = 0;
		let ip = ipv6(0,0,0,0,0,0,0,1); // Ipv6 localhost, invalid
		let ip_type = 6;
        let port = 1337;
		let modality = 0;
		let hotkey: u64 = 0;
		let coldkey: u64 = 0;

		register_ok_neuron(hotkey, coldkey);
		let result = Subtensor::serve_axon(<<Test as Config>::Origin>::signed(hotkey), version, ip, port, ip_type, modality );
		assert_eq!(result, Err(Error::<Test>::InvalidIpAddress.into()));
	});
}

#[test]
fn test_serve_invalid_ip_type() {
	new_test_ext().execute_with(|| {
		let version = 0;
		let ip = ipv4(8,8,8,8); 
		let ip_type = 10; // must be 4 or 6
		let port = 1337;
		let modality = 0;
		let hotkey: u64 = 0;
		let coldkey: u64 = 0;

		register_ok_neuron(hotkey, coldkey);
		let result = Subtensor::serve_axon(<<Test as Config>::Origin>::signed(hotkey), version, ip, port, ip_type, modality );
		assert_eq!(result, Err(Error::<Test>::InvalidIpType.into()));
	});
}

#[test]
fn test_serve_success() {
	new_test_ext().execute_with(|| {
		let version = 0;
		let ip = ipv4(8,8,8,8);
		let ip_type = 4;
		let port = 1337;
		let modality = 0;
		let hotkey: u64 = 0;
		let coldkey: u64 = 0;

		register_ok_neuron(hotkey, coldkey);
		assert_ok!(Subtensor::serve_axon(<<Test as Config>::Origin>::signed(hotkey), version, ip, port, ip_type, modality ));
        let neuron = Subtensor::get_neuron_for_hotkey(&hotkey);

		// Check uid setting functionality
		assert_eq!(neuron.uid, 0);

		// Check if metadata is set correctly
		assert_eq!(neuron.ip, ip);
		assert_eq!(neuron.ip_type, ip_type);
		assert_eq!(neuron.port, port);
		assert_eq!(neuron.coldkey, coldkey);

		// Check if this function works
		assert_eq!(Subtensor::is_uid_active(neuron.uid), true);

		// Check neuron count increment functionality
        assert_eq!(Subtensor::get_neuron_count(), 1);

		// Check if weights are set correctly. Only self weight
		assert_eq!( Subtensor::get_weights_for_neuron(&neuron), vec![u32::MAX] );

		// Check if the neuron has a hotkey account
		assert_eq!(Subtensor::has_hotkey_account(&neuron.uid), true);

		// Check if the balance of this hotkey account == 0
		assert_eq!(Subtensor::get_stake_of_neuron_hotkey_account_by_uid(neuron.uid), 0);
	});
}

#[test]
fn test_serve_success_with_update() {
	new_test_ext().execute_with(|| {
		let version = 0;
		let ip = ipv4(8,8,8,8);
		let ip_type = 4;
		let port = 1337;
		let modality = 0;
		let hotkey: u64 = 0;
		let coldkey: u64 = 0;

		register_ok_neuron(hotkey, coldkey);
		assert_ok!(Subtensor::serve_axon(<<Test as Config>::Origin>::signed(hotkey), version, ip, port, ip_type, modality ));
        let neuron = Subtensor::get_neuron_for_hotkey(&hotkey);

		// Check uid setting functionality
		assert_eq!(neuron.uid, 0);

		// Check if metadata is set correctly
		assert_eq!(neuron.ip, ip);
		assert_eq!(neuron.ip_type, ip_type);
		assert_eq!(neuron.port, port);
		assert_eq!(neuron.coldkey, coldkey);

		// Check if this function works
		assert_eq!(Subtensor::is_uid_active(neuron.uid), true);

		// Check neuron count increment functionality
        assert_eq!(Subtensor::get_neuron_count(), 1);

		// Check if weights are set correctly. Only self weight
		assert_eq!( Subtensor::get_weights_for_neuron(&neuron), vec![u32::MAX] );

		// Check if the neuron has a hotkey account
		assert_eq!(Subtensor::has_hotkey_account(&neuron.uid), true);

		// Check if the balance of this hotkey account == 0
		assert_eq!(Subtensor::get_stake_of_neuron_hotkey_account_by_uid(neuron.uid), 0);

        let version_2 = 0;
		let ip_2 = ipv4(8,8,8,8);
		let ip_type_2 = 4;
		let port_2 = 1337;
		let modality_2 = 0;
        assert_ok!(Subtensor::serve_axon(<<Test as Config>::Origin>::signed(hotkey), version_2, ip_2, port_2, ip_type_2, modality_2 ));
        let neuron2 = Subtensor::get_neuron_for_hotkey(&hotkey);

        // Check if metadata is set correctly
		assert_eq!(neuron2.ip, ip_2);
		assert_eq!(neuron2.ip_type, ip_type_2);
		assert_eq!(neuron2.port, port_2);
		assert_eq!(neuron2.version, version_2);
		assert_eq!(neuron2.coldkey, coldkey);

        // Check if this function works
		assert_eq!(Subtensor::is_uid_active(neuron2.uid), true);

		// Check neuron count increment functionality
        assert_eq!(Subtensor::get_neuron_count(), 1);

		// Check if weights are set correctly. Only self weight
		assert_eq!( Subtensor::get_weights_for_neuron(&neuron2), vec![u32::MAX] );

		// Check if the neuron has a hotkey account
		assert_eq!(Subtensor::has_hotkey_account(&neuron2.uid), true);

		// Check if the balance of this hotkey account == 0
		assert_eq!(Subtensor::get_stake_of_neuron_hotkey_account_by_uid(neuron2.uid), 0);

	});
}
