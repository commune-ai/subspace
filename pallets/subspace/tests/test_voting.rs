mod test_mock;
use frame_support::{
	assert_ok,
	dispatch::{DispatchClass, DispatchInfo, GetDispatchInfo, Pays},
};
use frame_system::Config;
use pallet_subspace::Error;
use sp_core::U256;
use test_mock::*;
use sp_std::vec;

/* TO DO SAM: write test for LatuUpdate after it is set */


fn test_add_porposal() {
	let netuid = 0;
	let founder_key = U256::from(0);
	register_module(netuid, founder_key, 1_000_000_000);
	let mut params = SubspaceModule::subnet_params(netuid);
	let result = SubspaceModule::add_subnet_proposal(get_origin(founder_key), netuid, params);
	assert_ok!(result);
	assert_eq!(SubspaceModule::get_subnet_proposals(netuid).len(), 1, "proposal not added");

}