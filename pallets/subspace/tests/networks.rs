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





