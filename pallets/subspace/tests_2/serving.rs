use pallet_subspace::{Error};
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
		let name : Vec<u8> = 'model1'.as_bytes().to_vec();
		let ip = ipv4(8,8,8,8);
		let port = 8883;
        let call = Call::subspace(subspaceCall::serve_module{name, ip, port});
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
		let name : Vec<u8> = 'model1'.as_bytes().to_vec();
		let ip = ipv4(8,8,8,8);
		let ownership = 50; 
		let port = 1337;
		let key: u64 = 0;

		let result = subspace::serve_module(<<Test as Config>::Origin>::signed(key), name, ip, port );
		assert_eq!( result, Err(Error::<Test>::NotRegistered.into()) );
    });
}

#[test]
fn test_serve_invalid_ip() {
	new_test_ext().execute_with(|| {
		let name = 'dataset'.as_bytes().to_vec();
		let ip = ipv4(127,0,0,1); // Not allowed.
		let port = 1337;
		let key: u64 = 0;
		let key: u64 = 0;

		register_ok_neuron(key, key);
		let result = subspace::serve_module(<<Test as Config>::Origin>::signed(key), name, ip, port );
		assert_eq!(result, Err(Error::<Test>::InvalidIpAddress.into()));
	});
}

#[test]
fn test_serve_invalid_ipv6() {
	new_test_ext().execute_with(|| {
		let ip = ipv6(0,0,0,0,0,0,0,1); // Ipv6 localhost, invalid
        let port = 1337;
		let key: u64 = 0;
		let key: u64 = 0;
		let name = 'bro'.as_bytes().to_vec();

		register_ok_neuron(key, key);
		let result = subspace::serve_module(<<Test as Config>::Origin>::signed(key), name, ip, port );
		assert_eq!(result, Err(Error::<Test>::InvalidIpAddress.into()));
	});
}

#[test]
fn test_serve_invalid_ip_type() {
	new_test_ext().execute_with(|| {
		let ip = ipv4(8,8,8,8); 
		let port = 1337;
		let key: u64 = 0;
		let name = 'bro'.as_bytes().to_vec();

		register_ok_neuron(key, key);
		let result = subspace::serve_module(<<Test as Config>::Origin>::signed(key), name, ip, port );
		assert_eq!(result, Err(Error::<Test>::InvalidIpType.into()));
	});
}

#[test]
fn test_serve_success() {
	new_test_ext().execute_with(|| {
		let name = 'bro'.as_bytes().to_vec();
		let ip = ipv4(8,8,8,8);
		let port = 1337;
		let key: u64 = 0;
		let key: u64 = 0;

		register_ok_neuron(key, key);
		assert_ok!(subspace::serve_module(<<Test as Config>::Origin>::signed(key), name, ip, port ));
        let neuron = subspace::get_neuron_for_key();

		// Check uid setting functionality
		assert_eq!(neuron.uid, 0);

		// Check if metadata is set correctly
		assert_eq!(neuron.ip, ip);
		assert_eq!(neuron.port, port);
		assert_eq!(neuron.key, key);

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
fn test_serve_success_with_update() {
	new_test_ext().execute_with(|| {
		let ip = ipv4(8,8,8,8);
		let port = 1337;
		let name: Vec<u8> = "test".as_bytes().to_vec();
		let key: u64 = 0;
		let key: u64 = 0;


		register_ok_neuron(key, key);
		assert_ok!(subspace::serve_module(<<Test as Config>::Origin>::signed(key), name, ip, port ));
        let neuron = subspace::get_neuron_for_key(&key);

		// Check uid setting functionality
		assert_eq!(neuron.uid, 0);

		// Check if metadata is set correctly
		assert_eq!(neuron.ip, ip);
		assert_eq!(neuron.port, port);
		assert_eq!(neuron.key, key);

		// Check if this function works
		assert_eq!(subspace::is_uid_active(neuron.uid), true);

		// Check neuron count increment functionality
        assert_eq!(subspace::get_neuron_count(), 1);

		// Check if weights are set correctly. Only self weight
		assert_eq!( subspace::get_weights_for_neuron(&neuron), vec![u32::MAX] );

		// Check if the neuron has a key account
		assert_eq!(subspace::has_key_account(&neuron.uid), true);

		// Check if the balance of this key account == 0

		let ip_2 = ipv4(8,8,8,8);
		let ip_type_2 = 4;
		let port_2 = 1337;
		let name_2: Vec<u8> = b"test2".to_vec();
        assert_ok!(subspace::serve_module(<<Test as Config>::Origin>::signed(key), name_2, ip_2, port_2 ));
        let neuron2 = subspace::get_neuron_for_key(&key);

        // Check if metadata is set correctly
		assert_eq!(neuron2.ip, ip_2);
		assert_eq!(neuron2.port, port_2);
		assert_eq!(neuron2.key, key);

        // Check if this function works
		assert_eq!(subspace::is_uid_active(neuron2.uid), true);

		// Check neuron count increment functionality
        assert_eq!(subspace::get_neuron_count(), 1);

		// Check if weights are set correctly. Only self weight
		assert_eq!( subspace::get_weights_for_neuron(&neuron2), vec![u32::MAX] );

		// Check if the neuron has a key account
		assert_eq!(subspace::has_key_account(&neuron2.uid), true);

		// Check if the balance of this key account == 0

	});
}
