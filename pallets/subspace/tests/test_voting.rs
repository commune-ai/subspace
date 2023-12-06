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

#[test]
fn test_add_porposal() {
    new_test_ext().execute_with(|| {
        
	let netuid = 0;
	let founder_key = U256::from(0);
	let initial_stake = 1_000_000_000;
	register_module(netuid, founder_key, initial_stake);
	let mut params = SubspaceModule::subnet_params(netuid);
	params.vote_mode = "stake".as_bytes().to_vec();
	SubspaceModule::set_subnet_params(netuid, params.clone());
	let params = SubspaceModule::subnet_params(netuid);
	assert_eq!(params.vote_mode, "stake".as_bytes().to_vec(), "vote mode not set");
	SubspaceModule::add_subnet_proposal(get_origin(founder_key), netuid, params.clone());

	// test for 2 proposals
	assert_eq!(SubspaceModule::get_subnet_proposals(netuid).len(), 1, "proposal not added");
	assert_eq!(SubspaceModule::get_subnet_proposals(netuid)[0].votes, initial_stake, "proposal not added");
	assert_ok!(SubspaceModule::add_subnet_proposal(get_origin(founder_key), netuid, params));
	assert_eq!(SubspaceModule::get_subnet_proposals(netuid).len(), 2, "proposal not added");
	let proposal = SubspaceModule::get_proposal(0);

	// test whether proposal is added to the initial_stake
	assert_eq!(proposal.votes, initial_stake, "proposal not added");

	});

}