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
        let token_amount : u64 = 1_000_000_000;
        let netuids : Vec<u16> = (0..10).collect();
        let amount_staked_vector: Vec<u64> = [1, 10, 100, 1000, 100000, 1_000_000, 10_000_000, 42_000_000].iter().map(|x| x * token_amount).collect();
        let mut total_stake : u64 = 0;
        let key_vector: Vec<U256> = (0..max_uids).map(|i| U256::from(i)).collect();
        let mut balance: u64 = 0; 
        let mut netuid: u16 = 0;
        let mut subnet_stake: u64 = 0;
        for i in netuids.iter() {
            netuid = *i;
            println!("NETUID: {}", netuid);

            for (key, amount_staked) in key_vector.iter().zip(amount_staked_vector.iter()) {
                println!(" KEY STAKE {} STAKING AMOUNT {} ",SubspaceModule::get_stake(netuid, key), *amount_staked);

                register_module(netuid, *key, *amount_staked);
                let uid = SubspaceModule::get_uid_for_key(netuid, &key);
                // ADD STAKE
                // SubspaceModule::add_stake(get_origin(*key), netuid, *amount_staked);
                assert_eq!(SubspaceModule::get_stake_for_uid(netuid, uid), *amount_staked);
                assert_eq!(SubspaceModule::get_balance(key), 0);
    
                // ADD STAKE

                println!("BEFORE REMOVE 1 stake for uid ({}): {}", uid, SubspaceModule::get_stake_for_uid(netuid, uid));
                println!("BEFORE REMOVE 1 total stake: {}", SubspaceModule::get_total_stake());
                // REMOVE STAKE
                SubspaceModule::remove_stake(get_origin(*key), netuid, *amount_staked);
                assert_eq!(SubspaceModule::get_balance(key), *amount_staked);
                assert_eq!(SubspaceModule::get_stake_for_uid(netuid, uid), 0);
    
                // ADD STAKE
                println!("AFTER REMOVE stake for uid ({}): {}", uid, SubspaceModule::get_stake_for_uid(netuid, uid));
                println!("AFTER REMOVE total stake: {}", SubspaceModule::get_total_stake());
                SubspaceModule::add_stake(get_origin(*key), netuid, *amount_staked);
                assert_eq!(SubspaceModule::get_stake_for_uid(netuid, uid), *amount_staked);
                assert_eq!(SubspaceModule::get_balance(key), 0);
                subnet_stake += SubspaceModule::get_stake_for_uid(netuid, uid);
                 // ADD STAKE
                 println!("AFTER ADD stake for uid ({}): {}", uid, SubspaceModule::get_stake_for_uid(netuid, uid));
                 println!("AFTER ADD total stake: {}", SubspaceModule::get_total_stake());
    
            }
            assert_eq!(SubspaceModule::get_total_subnet_stake(netuid), subnet_stake);
            total_stake += subnet_stake.clone();
            assert_eq!(SubspaceModule::get_total_stake(), total_stake);
            subnet_stake = 0;

        }




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


