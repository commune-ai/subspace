mod mock;
use mock::*;
use pallet_subspace::{Error};
use frame_support::weights::{GetDispatchInfo, DispatchInfo, DispatchClass, Pays};
use frame_system::Config;
use frame_support::{sp_std::vec};
use frame_support::{assert_ok};
use sp_core::U256;

/*TO DO SAM: write test for LatuUpdate after it is set */

// --- add network tests ----
#[test]
fn test_add_network_dispatch_info_ok() { 
        new_test_ext().execute_with(|| {
        let name: Vec<u8> = 'test'.as_bytes().to_vec();
        let stake = 0;
        let tempo: u16 = 13;
        let call = RuntimeCall::SubspaceModule(subspaceCall::add_network{name, stake});
        assert_eq!(call.get_dispatch_info(), 
                DispatchInfo {
                        weight: frame_support::weights::Weight::from_ref_time(50000000),
                        class: DispatchClass::Operational,
                        pays_fee: Pays::No
                });
});}

#[test]
fn test_add_network() { 
        new_test_ext().execute_with(|| {
        let modality = 0;
        let tempo: u16 = 13;
        add_network(10, tempo, modality);
        assert_eq!(SubspaceModule::get_number_of_subnets(), 1);
        add_network( 20, tempo, modality);
        assert_eq!(SubspaceModule::get_number_of_subnets(), 2); 
});}





#[test]
fn test_network_set_emission_ratios_fail_summation() {
	new_test_ext().execute_with(|| {
        let netuids: Vec<u16> = vec![ 1, 2 ]; 
        let emission: Vec<u64> = vec![ 100000000, 910000000 ]; 
        add_network(1, 0, 0);
        add_network(2, 0, 0);
        assert_eq!( SubspaceModule::sudo_set_emission_values(<<Test as Config>::RuntimeOrigin>::root(), netuids, emission ), Err(Error::<Test>::InvalidEmissionValues.into()) );
});}
