
use frame_support::traits::Currency;
use ndarray::stack_new_axis;
use pallet_subspace::{Error};
use frame_support::{assert_ok};
use frame_system::Config;
use sp_core::U256;
use crate::{test_mock::*};
use frame_support::sp_runtime::DispatchError;
use frame_support::dispatch::{GetDispatchInfo, DispatchInfo};
use frame_support::weights::{DispatchClass, Pays};

mod test_mock;

/********************************************
	subscribing::subscribe() tests
*********************************************/




#[test]
fn test_registration_ok() {
	new_test_ext().execute_with(|| {
		let block_number: u64 = 0;
		let netuid: u16 = 0;
		let tempo: u16 = 13;
		let key: U256 = U256::from(1);

		register_module(netuid, key, 0);
		// Check if neuron has added to the specified network(netuid)
		assert_eq!(SubspaceModule::get_subnet_n(netuid), 1);

		// Check if the neuron has added to the Keys
		let neuron_uid = SubspaceModule::get_uid_for_key(netuid, &key);
		assert_eq!(SubspaceModule::get_uid_for_key(netuid, &key), 0);
		// Check if neuron has added to Uids
		let neuro_uid = SubspaceModule::get_uid_for_key(netuid, &key);
		assert_eq!(neuro_uid, neuron_uid);

		// Check if the balance of this hotkey account for this subnetwork == 0
		assert_eq!(SubspaceModule::get_stake_for_uid(netuid, neuron_uid), 0);
	});
}


#[test]
fn test_many_registrations() {
	new_test_ext().execute_with(|| {
	let netuid = 0;
	let stake = 10;
	let n = 100;
	SubspaceModule::set_max_registrations_per_block(netuid, n);
	for i in 0..n {
		
		register_module(netuid, U256::from(i), stake);
		assert_eq!(SubspaceModule::get_subnet_n(netuid), i+1,"Failed at i={}",i);
	}




});
}


#[test]
fn test_registration_with_stake() {
	new_test_ext().execute_with(|| {
		let netuid = 0;
		let stake_vector: Vec<u64> = [ 100000, 1000000, 10000000].to_vec();
		let n = stake_vector.len() as u16;

		for (i, stake) in stake_vector.iter().enumerate() {
			let uid : u16  = i as u16;
			let stake_value : u64 = *stake;
			
			let key = U256::from(uid);
			println!("key: {:?}", key);
			println!("stake: {:?}", stake_value);
			let stake_before : u64 = SubspaceModule::get_stake(netuid, &key);
			println!("stake_before: {:?}", stake_before);
			register_module(netuid, key, stake_value);
			assert_eq!(SubspaceModule::get_stake_for_uid(netuid, uid), stake_value);
		}
	});
}




fn register_same_key_twice() {
	new_test_ext().execute_with(|| {
		let netuid = 0;
		let stake = 10;
		let key = U256::from(1);
		register_module(netuid, key, stake);
		register_module(netuid, key, stake);
	});




}








