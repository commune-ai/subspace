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
fn test_subnet_porposal() {
    new_test_ext().execute_with(|| {
        
	let netuid = 0;
	let keys = vec![U256::from(1), U256::from(2), U256::from(3)];
	let stakes= vec![1_000_000_000, 1_000_000_000, 1_000_000_000];
	
	for (i, key) in keys.iter().enumerate() {
		register_module(netuid, *key, stakes[i]);
	}

	let mut params = SubspaceModule::subnet_params(netuid);
	assert_eq!(params.vote_mode, "authority".as_bytes().to_vec(), "vote mode not set");
	params.vote_mode = "stake".as_bytes().to_vec();
	SubspaceModule::set_subnet_params(netuid, params.clone());

	let mut params = SubspaceModule::subnet_params(netuid);
	let initial_tempo = params.tempo;
	let final_tempo = 1000;
	params.tempo = final_tempo;
	assert_eq!(params.vote_mode, "stake".as_bytes().to_vec(), "vote mode not set");
	assert_ok!(SubspaceModule::add_subnet_proposal(get_origin(keys[0]), netuid, params.clone()));
	// we have not passed the threshold yet
	let proposals = SubspaceModule::get_subnet_proposals(netuid);
	
	assert_eq!(proposals.len(), 1, "proposal not added");
	assert_eq!(proposals[0].votes, stakes[0], "proposal not added");

	let proposal = SubspaceModule::get_proposal(0);
	assert_eq!(proposal.netuid, netuid, "proposal not added");
	assert_eq!(proposal.accepted, false, "proposal not added");

	// now vote for the proposal

	assert_ok!(SubspaceModule::vote_proposal(get_origin(keys[1]), 0));
	let proposal = SubspaceModule::get_proposal(0);
	assert_eq!(proposal.votes, stakes[0] + stakes[1], "proposal not voted");
	assert_eq!(proposal.accepted, true, "proposal not voted");

	println!("proposal: {:?}", proposal);

	let params = SubspaceModule::subnet_params(netuid);
	assert_eq!(params.tempo, final_tempo, "proposal not voted");


	});

}



#[test]
fn test_global_porposal() {
    new_test_ext().execute_with(|| {
        
	let netuid = 0;
	let keys = vec![U256::from(1), U256::from(2), U256::from(3)];
	let stakes= vec![1_000_000_000, 1_000_000_000, 1_000_000_000];
	
	// register on seperate subnets 
	for (i, key) in keys.iter().enumerate() {
		register_module(netuid + i as u16 , *key, stakes[i]);
	}

	let mut params = SubspaceModule::global_params();
	let initial_max_registrations_per_block = params.max_registrations_per_block;
	let max_registrations_per_block = 1000;

	params.max_registrations_per_block = max_registrations_per_block;
	assert_ok!(SubspaceModule::add_global_proposal(get_origin(keys[0]), params.clone()));
	// we have not passed the threshold yet
	let proposals = SubspaceModule::get_global_proposals();
	
	assert_eq!(proposals.len(), 1, "proposal not added");
	assert_eq!(proposals[0].votes, stakes[0], "proposal not added");

	let proposal = SubspaceModule::get_proposal(0);
	assert_eq!(proposal.accepted, false, "proposal not added");

	// now vote for the proposal

	assert_ok!(SubspaceModule::vote_proposal(get_origin(keys[1]), 0));
	let proposal = SubspaceModule::get_proposal(0);
	assert_eq!(proposal.votes, stakes[0] + stakes[1], "proposal not voted");
	assert_eq!(proposal.accepted, true, "proposal not voted");

	println!("proposal: {:?}", proposal);

	let params = SubspaceModule::global_params();
	assert_eq!(params.max_registrations_per_block, max_registrations_per_block, "proposal not voted");


	});

}