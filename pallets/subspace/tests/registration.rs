
use frame_support::traits::Currency;
use ndarray::stack_new_axis;
use pallet_subspace::{Error};
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








