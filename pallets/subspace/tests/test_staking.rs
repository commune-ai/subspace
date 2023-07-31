use frame_support::{assert_ok, traits::Currency};
use frame_system::{Config};
mod test_mock;
use test_mock::*;
use frame_support::sp_runtime::DispatchError;
use pallet_subspace::{Error};
use frame_support::dispatch::{GetDispatchInfo, DispatchInfo, DispatchClass, Pays};
use sp_core::U256;

use substrate_fixed::types::{I64F64, I32F32};
// /***********************************************************
// 	staking::add_stake() tests
// ************************************************************/


#[test]
fn test_stake_overflow() {
	new_test_ext().execute_with(|| {

        let token_amount : u64 = 1_000_000_000;
        let balance : u64 = 10 * token_amount;
        let netuid : u16 = 0;


        for i in [0,1].iter() {
            let delta : u64 = 1 * token_amount;
            let stake : u64 = balance + delta*(*i);
            let key : U256 = U256::from(*i);
            add_balance(key, balance);
            register(netuid, key, stake);
            println!("STAKE {}", SubspaceModule::get_stake(netuid, &key));
            assert_eq!(SubspaceModule::get_stake(netuid, &key), balance);
            assert_eq!(SubspaceModule::get_balance(&key), 0);
        }


	});
}


#[test]
fn test_stake() {
	new_test_ext().execute_with(|| {
        let max_uids: u16 = 10;
        let token_amount : u64 = 1_000_000_000;
        let netuids : Vec<u16> = [0,1,2,3].to_vec();
        let amount_staked_vector: Vec<u64> = netuids.iter().map(|i| 10 * token_amount).collect();
        let mut total_stake : u64 = 0;
        let mut netuid: u16 = 0;
        let mut subnet_stake: u64 = 0;
        let mut uid : u16 = 0;

        for i in netuids.iter() {
            netuid = *i;
            println!("NETUID: {}", netuid);
            let amount_staked = amount_staked_vector[netuid as usize];
            let key_vector: Vec<U256> = (0..max_uids).map(|i| U256::from(i + max_uids*netuid)).collect();

            
            for key in key_vector.iter() {
                println!(" KEY {} KEY STAKE {} STAKING AMOUNT {} ",key, SubspaceModule::get_stake(netuid, key), amount_staked);


                register_module(netuid, *key, amount_staked);
                // add_stake_and_balance(netuid, *key, amount_staked);
                println!(" KEY STAKE {} STAKING AMOUNT {} ",SubspaceModule::get_stake(netuid, key), amount_staked);

                uid = SubspaceModule::get_uid_for_key(netuid, &key);
                // SubspaceModule::add_stake(get_origin(*key), netuid, amount_staked);
                assert_eq!(SubspaceModule::get_stake_for_uid(netuid, uid), amount_staked);
                assert_eq!(SubspaceModule::get_balance(key), 0);

                // REMOVE STAKE
                SubspaceModule::remove_stake(get_origin(*key), netuid, *key, amount_staked);
                assert_eq!(SubspaceModule::get_balance(key), amount_staked);
                assert_eq!(SubspaceModule::get_stake_for_uid(netuid, uid), 0);

                // ADD STAKE AGAIN LOL
                SubspaceModule::add_stake(get_origin(*key), netuid, *key, amount_staked);
                assert_eq!(SubspaceModule::get_stake_for_uid(netuid, uid), amount_staked);
                assert_eq!(SubspaceModule::get_balance(key), 0);


                // AT THE END WE SHOULD HAVE THE SAME TOTAL STAKE
                subnet_stake += SubspaceModule::get_stake_for_uid(netuid, uid).clone();




            }
            assert_eq!(SubspaceModule::get_total_subnet_stake(netuid), subnet_stake);
            total_stake += subnet_stake.clone();
            assert_eq!(SubspaceModule::get_total_stake(), total_stake);
            subnet_stake = 0;
            println!("TOTAL STAKE: {}", total_stake);
            println!("TOTAL SUBNET STAKE: {}", SubspaceModule::get_total_subnet_stake(netuid));

        }




	});
}



#[test]
fn test_delegate_stake() {
	new_test_ext().execute_with(|| {
        let max_uids: u16 = 10;
        let token_amount : u64 = 1_000_000_000;
        let netuids : Vec<u16> = [0,1,2,3].to_vec();
        let amount_staked_vector: Vec<u64> = netuids.iter().map(|i| 10 * token_amount).collect();
        let mut total_stake : u64 = 0;
        let mut netuid: u16 = 0;
        let mut subnet_stake: u64 = 0;
        let mut uid : u16 = 0;

        for i in netuids.iter() {
            netuid = *i;
            println!("NETUID: {}", netuid);
            let amount_staked = amount_staked_vector[netuid as usize];
            let key_vector: Vec<U256> = (0..max_uids).map(|i| U256::from(i + max_uids*netuid)).collect();
            let delegate_key_vector : Vec<U256> = key_vector.iter().map(|i| U256::from(i.clone() + 1)).collect();

            
            for (i, key) in key_vector.iter().enumerate() {
                println!(" KEY {} KEY STAKE {} STAKING AMOUNT {} ",key, SubspaceModule::get_stake(netuid, key), amount_staked);
                
                let delegate_key: U256 = delegate_key_vector[i];
                add_balance(delegate_key, amount_staked);


                register_module(netuid, *key, 0);
                // add_stake_and_balance(netuid, *key, amount_staked);
                println!(" DELEGATE KEY STAKE {} STAKING AMOUNT {} ",SubspaceModule::get_stake(netuid, &delegate_key), amount_staked);


                SubspaceModule::add_stake(get_origin(delegate_key), netuid, *key, amount_staked);
                uid = SubspaceModule::get_uid_for_key(netuid, &key);
                // SubspaceModule::add_stake(get_origin(*key), netuid, amount_staked);
                assert_eq!(SubspaceModule::get_stake_for_uid(netuid, uid), amount_staked);
                assert_eq!(SubspaceModule::get_balance(&delegate_key), 0);
                assert_eq!(SubspaceModule::get_stake_to_vector(netuid, &delegate_key).len(), 1);
                // REMOVE STAKE
                SubspaceModule::remove_stake(get_origin(delegate_key), netuid, *key, amount_staked);
                assert_eq!(SubspaceModule::get_balance(&delegate_key), amount_staked);
                assert_eq!(SubspaceModule::get_stake_for_uid(netuid, uid), 0);
                assert_eq!(SubspaceModule::get_stake_to_vector(netuid, &delegate_key).len(), 0);


                // ADD STAKE AGAIN LOL
                SubspaceModule::add_stake(get_origin(delegate_key), netuid, *key, amount_staked);
                assert_eq!(SubspaceModule::get_stake_for_uid(netuid, uid), amount_staked);
                assert_eq!(SubspaceModule::get_balance(&delegate_key), 0);
                assert_eq!(SubspaceModule::get_stake_to_vector(netuid, &delegate_key).len(), 1);


                // AT THE END WE SHOULD HAVE THE SAME TOTAL STAKE
                subnet_stake += SubspaceModule::get_stake_for_uid(netuid, uid).clone();




            }
            assert_eq!(SubspaceModule::get_total_subnet_stake(netuid), subnet_stake);
            total_stake += subnet_stake.clone();
            assert_eq!(SubspaceModule::get_total_stake(), total_stake);
            subnet_stake = 0;
            println!("TOTAL STAKE: {}", total_stake);
            println!("TOTAL SUBNET STAKE: {}", SubspaceModule::get_total_subnet_stake(netuid));

        }




	});
}



#[test]
fn test_ownership_ratio() {
	new_test_ext().execute_with(|| {
        let netuid: u16 = 0;
        let num_modules: u16 = 10;
        let stake_per_module : u64 = 1_000_000_000;
        register_n_modules(netuid, num_modules, 0);
        
        let keys  = SubspaceModule::get_keys(netuid);

        for (k_i, k) in keys.iter().enumerate() {
            let delegate_keys : Vec<U256> = (0..num_modules).map(|i| U256::from(i + num_modules)).collect();
            for d in delegate_keys.iter() {
                add_balance(*d, stake_per_module);
            }

            let pre_delegate_stake_from_vector = SubspaceModule::get_stake_from_vector(netuid, k);
            assert_eq!(pre_delegate_stake_from_vector.len(), 1); // +1 for the module itself, +1 for the delegate key on 

            println!("KEY: {}", k);
            for (i, d) in delegate_keys.iter().enumerate() {
                println!("DELEGATE KEY: {}", d);
                SubspaceModule::add_stake(get_origin(*d), netuid, *k, stake_per_module);
                let stake_from_vector = SubspaceModule::get_stake_from_vector(netuid, k);
                assert_eq!(stake_from_vector.len(), pre_delegate_stake_from_vector.len()+i+1); // +1 for the module itself, +1 for the delegate key on 
            }
            let ownership_ratios : Vec<(U256,I64F64)> = SubspaceModule::get_ownership_ratios(netuid, k);

            assert_eq!(ownership_ratios.len(), delegate_keys.len()+1);
            println!("OWNERSHIP RATIOS: {:?}", ownership_ratios);
            // step_block();

            step_epoch(netuid);

            let stake_from_vector = SubspaceModule::get_stake_from_vector(netuid, k);
            let stake: u64 = SubspaceModule::get_stake(netuid, k);
            let sumed_stake : u64 = stake_from_vector.iter().fold(0, |acc, (a, x)| acc + x);
            let total_stake : u64 = SubspaceModule::get_total_subnet_stake(netuid);

            println!("STAKE: {}", stake);
            println!("SUMED STAKE: {}", sumed_stake);
            println!("TOTAL STAKE: {}", total_stake);

            assert_eq!(stake, sumed_stake);

            // for (d_a, o) in ownership_ratios.iter() {
            //     println!("OWNERSHIP RATIO: {}", o);
                
            // }




        }


        

	});
}

