mod mock;
use mock::*;
use pallet_subspace::{Error};
use frame_support::weights::{GetDispatchInfo, DispatchInfo, DispatchClass, Pays};
use frame_system::Config;
use frame_support::{sp_std::vec};
use frame_support::{assert_ok};
use sp_core::U256;

/*TO DO SAM: write test for LatuUpdate after it is set */


#[test]
fn test_add_network() { 
        new_test_ext().execute_with(|| {
        let modality = 0;
        let tempo: u16 = 13;
        add_network(0, U256::from(0));
        assert_eq!(SubspaceModule::get_number_of_subnets(), 1);
        add_network( 1, U256::from(0));
        assert_eq!(SubspaceModule::get_number_of_subnets(), 2); 
});}


#[test]
fn test_add_many_subnets() { 
        new_test_ext().execute_with(|| {
        for i in 0..100 {
            add_network(i, U256::from(0));
            assert_eq!(SubspaceModule::get_number_of_subnets(), i+1);
        }
});}



#[test]
fn test_set_max_allowed_uids() { 
        new_test_ext().execute_with(|| {
        let netuid : u16 = 0;
        let stake : u64 = 1_000_000_000;
        let max_uids : u16 = 1000;
        let extra_uids : u16 = 10;
        register_module(netuid, U256::from(0), stake);
        SubspaceModule::set_max_allowed_uids(netuid, max_uids);
        SubspaceModule::set_max_registrations_per_block(netuid, max_uids + extra_uids*2 );
        for i in 1..max_uids {
            register_module(netuid, U256::from(i), stake);
            assert_eq!(SubspaceModule::get_subnet_n(netuid), i+1);
        }
        let mut n : u16 = SubspaceModule::get_subnet_n(netuid);
        assert_eq!(SubspaceModule::get_subnet_n(netuid), max_uids);
        let mut pruned_uids : Vec<u16> = vec![];
        let mut pruned_keys : Vec<U256> = vec![];
        for i in max_uids..max_uids+10 {
            let pruned_uid = SubspaceModule::get_uid_to_replace(netuid);
            pruned_uids.push(pruned_uid);
            pruned_keys.push(SubspaceModule::get_key_for_uid(netuid, pruned_uid));

            register_module(netuid, U256::from(i), stake);
            assert_eq!(SubspaceModule::get_subnet_n(netuid), max_uids);
        }
        assert!(SubspaceModule::get_uids(netuid).len() as u16 == max_uids);
        assert_eq!(SubspaceModule::get_subnet_n(netuid), max_uids);

        SubspaceModule::set_max_allowed_uids(netuid, max_uids + extra_uids);
        // print the pruned uids
        println!("pruned keys: {:?}", pruned_keys);
        for (i, pruned_key) in pruned_keys.iter().enumerate() {
            register_module(netuid, *pruned_key, stake);
            assert_eq!(SubspaceModule::get_subnet_n(netuid),max_uids + i as u16 + 1);
        }
        println!("new length of uids: {:?}", SubspaceModule::get_uids(netuid).len());



        assert_eq!(SubspaceModule::get_subnet_n(netuid), max_uids+extra_uids);
        assert!(SubspaceModule::get_uids(netuid).len() as u16 == max_uids+extra_uids);
});}





