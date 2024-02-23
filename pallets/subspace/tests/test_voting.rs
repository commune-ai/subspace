mod test_mock;
use frame_support::assert_ok;

use sp_core::U256;
use sp_std::vec;
use test_mock::*;

/* TO DO SAM: write test for LatuUpdate after it is set */

#[test]
fn test_subnet_porposal() {
	new_test_ext().execute_with(|| {
		let netuid = 0;
		let keys = [U256::from(0), U256::from(1), U256::from(2)];
		let stakes = [1_000_000_000, 1_000_000_000, 1_000_000_000];

		for (i, key) in keys.iter().enumerate() {
			assert_ok!(register_module(netuid, *key, stakes[i]));
		}
		let mut params = SubspaceModule::subnet_params(netuid);
		assert_eq!(params.vote_mode, "authority".as_bytes().to_vec(), "vote mode not set");
		params.vote_mode = "stake".as_bytes().to_vec();
		println!("params: {:?}", params);
		SubspaceModule::set_subnet_params(netuid, params.clone());
		let mut params = SubspaceModule::subnet_params(netuid);
		let _initial_tempo = params.tempo;
		let final_tempo = 1000;
		params.tempo = final_tempo;

		assert_eq!(params.vote_mode, "stake".as_bytes().to_vec(), "vote mode not set");
		assert_ok!(SubspaceModule::do_add_subnet_proposal(get_origin(keys[0]), netuid, params));
		// we have not passed the threshold yet
		let proposals = SubspaceModule::get_subnet_proposals(netuid);

		println!("proposals: {:?}", proposals);

		assert_eq!(proposals.len(), 1, "proposal not added");
		assert_eq!(proposals[0].votes, stakes[0], "proposal not added");

		let proposal = SubspaceModule::get_proposal(0);
		assert_eq!(proposal.netuid, netuid, "proposal not added");
		assert!(!proposal.accepted, "proposal not added");
		// now vote for the proposal

		assert_ok!(SubspaceModule::vote_proposal(get_origin(keys[1]), 0));
		let proposal = SubspaceModule::get_proposal(0);
		assert_eq!(proposal.votes, stakes[0] + stakes[1], "proposal not voted");
		assert!(proposal.accepted, "proposal not voted");

		println!("proposal: {:?}", proposal);

		let params = SubspaceModule::subnet_params(netuid);
		assert_eq!(params.tempo, final_tempo, "proposal not voted");
	});
}

#[test]
fn test_max_proposals() {
	new_test_ext().execute_with(|| {
		let netuid = 0;
		let n = 100;
		let keys: Vec<U256> = (0..n).map(U256::from).collect();
		let mut stakes = vec![1_000_000_000; n];
		// increase incrementally to avoid overflow
		let stakes =
			stakes.iter_mut().enumerate().map(|(i, x)| *x + i as u64).collect::<Vec<u64>>();

		for (i, key) in keys.iter().enumerate() {
			assert_ok!(register_module(netuid, *key, stakes[i]));
		}

		let mut params = SubspaceModule::global_params();
		assert_eq!(params.vote_mode, "authority".as_bytes().to_vec(), "vote mode not set");
		params.vote_mode = "stake".as_bytes().to_vec();
		params.max_proposals = (n / 2) as u64;
		println!("params: {:?}", params);
		SubspaceModule::set_global_params(params.clone());

		assert_eq!(params.vote_mode, "stake".as_bytes().to_vec(), "vote mode not set");
		let max_proposals = SubspaceModule::get_max_proposals();
		let _modes = ["authority".as_bytes().to_vec(), "stake".as_bytes().to_vec()];

		let mut subnet_params = SubspaceModule::subnet_params(netuid);
		subnet_params.vote_mode = "stake".as_bytes().to_vec();
		SubspaceModule::set_subnet_params(netuid, subnet_params.clone());
		subnet_params = SubspaceModule::subnet_params(netuid);
		assert_eq!(subnet_params.vote_mode, "stake".as_bytes().to_vec(), "vote mode not set");

		for i in 0..n {
			if i % 2 == 0 {
				assert_ok!(SubspaceModule::do_add_global_proposal(
					get_origin(keys[i]),
					params.clone()
				));
			} else {
				assert_ok!(SubspaceModule::do_add_subnet_proposal(
					get_origin(keys[i]),
					netuid,
					subnet_params.clone()
				));
			}
			let num_proposals = SubspaceModule::num_proposals();
			let proposals = SubspaceModule::get_global_proposals();
			let has_max_proposals = SubspaceModule::has_max_proposals();
			// assert_eq!(max_proposals, (n - 1) as u64, "proposal not added");
			println!("max_proposals: {:?}", max_proposals);
			println!("has_max_proposals: {:?}", has_max_proposals);
			println!("num_proposals: {:?}", num_proposals);
			println!("proposals: {:?}", proposals.len());

			let num_subnet_proposals = SubspaceModule::num_subnet_proposals(netuid);
			let num_global_proposals = SubspaceModule::num_global_proposals();
			assert_eq!(
				num_subnet_proposals + num_global_proposals,
				num_proposals,
				"proposal not added"
			);

			if num_proposals >= max_proposals {
				assert!(SubspaceModule::has_max_proposals(), "proposal not added");
			} else {
				assert!(!SubspaceModule::has_max_proposals(), "proposal not added");
			}

			assert!(proposals.len() as u64 <= max_proposals, "proposal not added");
		}

		assert!(SubspaceModule::has_max_proposals(), "proposal not added");
		assert_eq!(SubspaceModule::num_proposals(), max_proposals, "proposal not added");
	});
}

#[test]
fn test_global_porposal() {
	new_test_ext().execute_with(|| {
		let netuid = 0;
		let keys = [U256::from(1), U256::from(2), U256::from(3)];
		let stakes = [1_000_000_000, 1_000_000_000, 1_000_000_000];

		// register on seperate subnets
		for (i, key) in keys.iter().enumerate() {
			register_module(netuid + i as u16, *key, stakes[i]);
		}

		let mut params = SubspaceModule::global_params();
		let _initial_max_registrations_per_block = params.max_registrations_per_block;
		let max_registrations_per_block = 1000;

		params.max_registrations_per_block = max_registrations_per_block;
		assert_ok!(SubspaceModule::do_add_global_proposal(get_origin(keys[0]), params));

		// we have not passed the threshold yet
		let proposals = SubspaceModule::get_global_proposals();

		assert_eq!(proposals.len(), 1, "proposal not added");
		assert_eq!(proposals[0].votes, stakes[0], "proposal not added");

		let proposal = SubspaceModule::get_proposal(0);
		assert!(!proposal.accepted, "proposal not added");

		// now vote for the proposal

		assert_ok!(SubspaceModule::vote_proposal(get_origin(keys[1]), 0));
		let proposal = SubspaceModule::get_proposal(0);
		assert_eq!(proposal.votes, stakes[0] + stakes[1], "proposal not voted");
		assert!(proposal.accepted, "proposal not voted");

		println!("proposal: {:?}", proposal);

		let params = SubspaceModule::global_params();
		assert_eq!(
			params.max_registrations_per_block, max_registrations_per_block,
			"proposal not voted"
		);
	});
}

#[test]
fn test_unvote() {
	new_test_ext().execute_with(|| {
		let netuid = 0;
		let keys = [U256::from(0), U256::from(1), U256::from(2)];
		let stakes = [1_000_000_000, 1_000_000_000, 1_000_000_000];

		for (i, key) in keys.iter().enumerate() {
			assert_ok!(register_module(netuid, *key, stakes[i]));
		}
		let mut params = SubspaceModule::subnet_params(netuid);
		assert_eq!(params.vote_mode, "authority".as_bytes().to_vec(), "vote mode not set");
		params.vote_mode = "stake".as_bytes().to_vec();
		println!("params: {:?}", params);
		SubspaceModule::set_subnet_params(netuid, params.clone());
		let mut params = SubspaceModule::subnet_params(netuid);
		let _initial_tempo = params.tempo;
		let final_tempo = 1000;
		params.tempo = final_tempo;

		assert_eq!(params.vote_mode, "stake".as_bytes().to_vec(), "vote mode not set");
		assert_ok!(SubspaceModule::do_add_subnet_proposal(get_origin(keys[0]), netuid, params));
		assert!(SubspaceModule::proposal_exists(0));
		assert!(SubspaceModule::is_proposal_owner(&keys[0], 0));
		assert_ok!(SubspaceModule::unvote_proposal(get_origin(keys[0])));

		// we have not passed the threshold yet
		let proposals = SubspaceModule::get_subnet_proposals(netuid);

		println!("proposals: {:?}", proposals);

		assert_eq!(proposals.len(), 0, "proposal not added");
	});
}
