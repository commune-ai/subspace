use frame_support::{assert_ok, traits::Currency};
use frame_system::{Config};
mod mock;
use mock::*;
use frame_support::sp_runtime::DispatchError;
use pallet_subspace::{Error};
use frame_support::dispatch::{GetDispatchInfo, DispatchInfo, DispatchClass, Pays};
use sp_core::U256;

// /***********************************************************
// 	staking::add_stake() tests
// ************************************************************/





#[test]
fn test_stake() {
	new_test_ext().execute_with(|| {
        let max_uids: u16 = 10;
        let netuid: u16 = 0;
        let token_amount : u64 = 1_000_000_000;
        let amount_staked_vector: Vec<u64> = [1, 10, 100, 1000, 100000, 1_000_000, 1_000_000_000].iter().map(|x| x * token_amount).collect();
        let total_stake: u64 = amount_staked_vector.iter().sum();
        let key_vector: Vec<U256> = (0..max_uids).map(|i| U256::from(i)).collect();

        for (key, amount_staked) in key_vector.iter().zip(amount_staked_vector.iter()) {
            register_module(netuid, *key, *amount_staked);
            let uid = SubspaceModule::get_uid_for_key(netuid, &key);
            assert_eq!(SubspaceModule::get_stake_for_uid(netuid, uid), *amount_staked);

        }

        assert_eq!(SubspaceModule::get_total_subnet_stake(netuid), total_stake);
        assert_eq!(SubspaceModule::get_total_stake(), total_stake);



	});
}

#[test]
fn test_unstake() {
	new_test_ext().execute_with(|| {
        let max_uids: u16 = 10;
        let netuid: u16 = 0;
        let token_amount : u64 = 1_000_000_000;
        let amount_staked_vector: Vec<u64> = [1, 10, 100, 1000, 100000, 1_000_000, 1_000_000_000].iter().map(|x| x * token_amount).collect();
        let mut total_stake: u64 = amount_staked_vector.iter().sum();
        let key_vector: Vec<U256> = (0..max_uids).map(|i| U256::from(i)).collect();

        for (key, amount_staked) in key_vector.iter().zip(amount_staked_vector.iter()) {
            register_module(netuid, *key, *amount_staked);
            // remove_stake(netuid, *key, *amount_staked);
            // add_stake(netuid, *key, *amount_staked);
            let uid = SubspaceModule::get_uid_for_key(netuid, &key);
            assert_eq!(SubspaceModule::get_stake_for_uid(netuid, uid), *amount_staked);

        }


        for (key, amount_staked) in key_vector.iter().zip(amount_staked_vector.iter()) {
            remove_stake(netuid, *key, *amount_staked);
            assert_eq!(SubspaceModule::get_stake_for_key(netuid, key), 0);

        }
        total_stake = key_vector.iter().map(|x| SubspaceModule::get_stake_for_key(netuid, x)).sum();

        assert_eq!(SubspaceModule::get_total_subnet_stake(netuid), total_stake);


        
	});
}


