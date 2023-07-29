use frame_support::{assert_ok, traits::Currency};
use frame_system::{Config};
mod test_mock;
use test_mock::*;
use frame_support::sp_runtime::DispatchError;
use pallet_subspace::{Error};
use frame_support::dispatch::{GetDispatchInfo, DispatchInfo, DispatchClass, Pays};
use sp_core::U256;

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
                SubspaceModule::remove_stake(get_origin(*key), netuid, amount_staked);
                assert_eq!(SubspaceModule::get_balance(key), amount_staked);
                assert_eq!(SubspaceModule::get_stake_for_uid(netuid, uid), 0);

                // ADD STAKE AGAIN LOL
                SubspaceModule::add_stake(get_origin(*key), netuid, amount_staked);
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
        let balance : u64 = 10 * token_amount;
        let netuid : u16 = 0;
        let key : U256 = U256::from(0);
        let stake : u64 = balance;

        register_module(netuid, key, stake);





	});
}

