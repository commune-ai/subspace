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
        let mut max_uids : u16 = 1000;
        let extra_uids : u16 = 10;
        let rounds = 10;
        register_module(netuid, U256::from(0), stake);
        SubspaceModule::set_max_registrations_per_block(netuid, max_uids + extra_uids*rounds );
        for i in 1..max_uids {
            register_module(netuid, U256::from(i), stake);
            assert_eq!(SubspaceModule::get_subnet_n(netuid), i+1);
        }
        let mut n : u16 = SubspaceModule::get_subnet_n(netuid);
        let mut old_n : u16 = n.clone();
        let mut uids : Vec<u16>; 
        assert_eq!(SubspaceModule::get_subnet_n(netuid), max_uids);
        let mut new_n: u16 = SubspaceModule::get_subnet_n(netuid);
        for r in 1..rounds {
            // set max allowed uids to max_uids + extra_uids

            SubspaceModule::set_max_allowed_uids(netuid, max_uids + extra_uids*(r-1));
            max_uids = SubspaceModule::get_max_allowed_uids(netuid);
            new_n = old_n + extra_uids*(r-1);

            // print the pruned uids
            for uid in old_n+extra_uids*(r-1)..old_n+extra_uids*r {
                register_module(netuid, U256::from(uid), stake);

            }
            
            // set max allowed uids to max_uids
            
            n = SubspaceModule::get_subnet_n(netuid);
            assert_eq!(n, new_n);

            uids = SubspaceModule::get_uids(netuid) ; 
            assert_eq!(uids.len() as u16,  n);


        }
});
}





