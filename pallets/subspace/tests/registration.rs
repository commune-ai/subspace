mod mock;

use frame_support::{assert_err, assert_ok};
use mock::*;
use sp_core::U256;

use log::info;
use pallet_subspace::Error;

/********************************************
    subscribing::subscribe() tests
*********************************************/

#[test]
fn test_min_stake() {
    new_test_ext().execute_with(|| {
        let netuid: u16 = 0;
        let min_stake = 100_000_000;
        let max_registrations_per_block = 10;
        let reg_this_block: u16 = 100;

        register_module(netuid, U256::from(0), 0).expect("register module failed");
        SubspaceModule::set_min_stake(netuid, min_stake);
        SubspaceModule::set_max_registrations_per_block(max_registrations_per_block);
        step_block(1);
        assert_eq!(SubspaceModule::get_registrations_this_block(), 0);

        let n = U256::from(reg_this_block); // Example: if you want a list of numbers from 1 to 9
        let keys_list: Vec<U256> = (1..n.as_u64()) // Assuming n fits into a u64 for simplicity
            .map(U256::from)
            .collect();

        let min_stake_to_register = SubspaceModule::get_min_stake(netuid);

        for key in keys_list {
            let _ = register_module(netuid, key, min_stake_to_register);
            info!("registered module with key: {key:?} and min_stake_to_register: {min_stake_to_register:?}");
        }
        let registrations_this_block = SubspaceModule::get_registrations_this_block();
        info!("registrations_this_block: {registrations_this_block:?}");
        assert_eq!(registrations_this_block, max_registrations_per_block);

        step_block(1);
        assert_eq!(SubspaceModule::get_registrations_this_block(), 0);
    });
}

#[test]
fn test_max_registration() {
    new_test_ext().execute_with(|| {
        let netuid: u16 = 0;
        let min_stake = 100_000_000;
        let rounds = 3;
        let max_registrations_per_block = 100;

        SubspaceModule::set_min_stake(netuid, min_stake);
        SubspaceModule::set_max_registrations_per_block(max_registrations_per_block);

        assert_eq!(SubspaceModule::get_registrations_this_block(), 0);

        for i in 1..(max_registrations_per_block * rounds) {
            let key = U256::from(i);
            let min_stake_to_register = SubspaceModule::get_min_stake(netuid);
            let factor: u64 = min_stake_to_register / min_stake;
            info!("min_stake_to_register: {min_stake_to_register:?} min_stake: {min_stake:?} factor {factor:?}");
            register_module(netuid, key, factor * min_stake).expect("register module failed");

            let registrations_this_block = SubspaceModule::get_registrations_this_block();
            assert_eq!(registrations_this_block, i);
            assert!(SubspaceModule::is_registered(netuid, &key));
        }
        step_block(1);
        assert_eq!(SubspaceModule::get_registrations_this_block(), 0);
    });
}

#[test]
fn test_delegate_register() {
    new_test_ext().execute_with(|| {
        let netuid: u16 = 0;
        let n: u16 = 10;
        let key: U256 = U256::from(n + 1);
        let module_keys: Vec<U256> = (0..n).map(U256::from).collect();
        let stake_amount: u64 = 10_000_000_000;
        SubspaceModule::add_balance_to_account(&key, stake_amount * n as u64);
        for module_key in module_keys {
            delegate_register_module(netuid, key, module_key, stake_amount)
                .expect("delegate register module failed");
            let key_balance = SubspaceModule::get_balance_u64(&key);
            let stake_to_module = SubspaceModule::get_stake_to_module(netuid, &key, &module_key);
            info!("key_balance: {key_balance:?}");
            let stake_to_vector = SubspaceModule::get_stake_to_vector(netuid, &key);
            info!("stake_to_vector: {stake_to_vector:?}");
            assert_eq!(stake_to_module, stake_amount);
        }
    });
}

#[test]
fn test_registration_ok() {
    new_test_ext().execute_with(|| {
        let netuid: u16 = 0;
        let key: U256 = U256::from(1);

        register_module(netuid, key, 0)
            .unwrap_or_else(|_| panic!("register module failed for key {key:?}"));

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
        SubspaceModule::set_max_registrations_per_block(n);
        for i in 0..n {
            register_module(netuid, U256::from(i), stake).unwrap_or_else(|_| {
                panic!("Failed to register module with key: {i:?} and stake: {stake:?}",)
            });
            assert_eq!(
                SubspaceModule::get_subnet_n(netuid),
                i + 1,
                "Failed at i={i}",
            );
        }
    });
}

#[test]
fn test_registration_with_stake() {
    new_test_ext().execute_with(|| {
        let netuid = 0;
        let stake_vector: Vec<u64> = [100000, 1000000, 10000000].to_vec();

        for (i, stake) in stake_vector.iter().enumerate() {
            let uid: u16 = i as u16;
            let stake_value: u64 = *stake;

            let key = U256::from(uid);
            info!("key: {key:?}");
            info!("stake: {stake_value:?}");
            let stake_before: u64 = SubspaceModule::get_stake(netuid, &key);
            info!("stake_before: {stake_before:?}");
            register_module(netuid, key, stake_value).unwrap_or_else(|_| {
                panic!("Failed to register module with key: {key:?} and stake: {stake_value:?}",)
            });
            info!("balance: {:?}", SubspaceModule::get_balance_u64(&key));
            assert_eq!(SubspaceModule::get_stake_for_uid(netuid, uid), stake_value);
        }
    });
}

#[test]
fn register_same_key_twice() {
    new_test_ext().execute_with(|| {
        let netuid = 0;
        let stake = 10;
        let key = U256::from(1);
        assert_ok!(register_module(netuid, key, stake));
        assert_err!(
            register_module(netuid, key, stake),
            Error::<Test>::KeyAlreadyRegistered
        );
    });
}
