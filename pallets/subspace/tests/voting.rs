mod mock;

use std::{array::from_fn, collections::BTreeSet};

use frame_support::assert_ok;
use mock::*;
use pallet_subspace::{
    voting::{ProposalData, ProposalStatus, VoteMode},
    GlobalParams, MinBurn, ProposalCost, ProposalExpiration, Proposals, SubnetParams, Tempo,
    VoteModeSubnet,
};
use sp_core::U256;

#[test]
fn creates_global_params_proposal_correctly_and_expires() {
    new_test_ext().execute_with(|| {
        const COST: u64 = to_nano(10);

        ProposalCost::<Test>::set(COST);
        ProposalExpiration::<Test>::set(100);

        step_block(1);

        let key = U256::from(0);
        add_balance(key, COST + 1);
        assert_ok!(register_module(0, U256::from(1), 1_000_000_000));
        assert_ok!(register_module(0, U256::from(2), 1_000_000_000));

        let original = SubspaceModule::global_params();
        let params = GlobalParams {
            min_burn: 100_000_000,
            ..original.clone()
        };

        let GlobalParams {
            burn_rate,
            max_name_length,
            max_allowed_subnets,
            max_allowed_modules,
            max_registrations_per_block,
            max_allowed_weights,
            min_burn,
            max_burn,
            min_stake,
            floor_delegation_fee,
            min_weight_stake,
            target_registrations_per_interval,
            target_registrations_interval,
            adjustment_alpha,
            unit_emission,
            proposal_cost,
            proposal_expiration,
            proposal_participation_threshold,
        } = params.clone();

        SubspaceModule::add_global_proposal(
            get_origin(key),
            burn_rate,
            max_name_length,
            max_allowed_subnets,
            max_allowed_modules,
            max_registrations_per_block,
            max_allowed_weights,
            min_burn,
            max_burn,
            min_stake,
            floor_delegation_fee,
            min_weight_stake,
            target_registrations_per_interval,
            target_registrations_interval,
            adjustment_alpha,
            unit_emission,
            proposal_cost,
            proposal_expiration,
            proposal_participation_threshold,
        )
        .expect("failed to create proposal");

        assert_eq!(SubspaceModule::get_balance_u64(&key), 1);

        let proposal = Proposals::<Test>::get(0).expect("proposal was not created");
        assert_eq!(proposal.id, 0);
        assert_eq!(proposal.proposer, key);
        assert_eq!(proposal.expiration_block, 200);
        assert_eq!(
            proposal.data,
            ProposalData::<Test>::GlobalParams(params.clone())
        );
        assert_eq!(proposal.proposal_status, ProposalStatus::Pending);
        assert_eq!(proposal.votes_for, Default::default());
        assert_eq!(proposal.votes_against, Default::default());
        assert_eq!(proposal.proposal_cost, COST);
        assert_eq!(proposal.finalization_block, None);

        SubspaceModule::vote_proposal(get_origin(U256::from(1)), 0, true).unwrap();

        step_block(200);

        assert!(Proposals::<Test>::get(0).is_none());
        assert_eq!(SubspaceModule::get_balance_u64(&key), 1);

        assert_eq!(SubspaceModule::global_params(), original);
    });
}

#[test]
fn creates_global_params_proposal_correctly_and_is_approved() {
    new_test_ext().execute_with(|| {
        const COST: u64 = to_nano(10);

        MinBurn::<Test>::set(0);

        let keys: [_; 3] = from_fn(U256::from);
        let stakes = [1_000_000_000, 1_000_000_000, 1_000_000_000];

        for (key, balance) in keys.iter().zip(stakes) {
            assert_ok!(register_module(0, *key, balance));
        }
        add_balance(keys[0], COST);

        ProposalCost::<Test>::set(COST);
        ProposalExpiration::<Test>::set(200);

        let params = GlobalParams {
            min_burn: 100_000_000,
            ..SubspaceModule::global_params()
        };

        let GlobalParams {
            burn_rate,
            max_name_length,
            max_allowed_subnets,
            max_allowed_modules,
            max_registrations_per_block,
            max_allowed_weights,
            min_burn,
            max_burn,
            min_stake,
            floor_delegation_fee,
            min_weight_stake,
            target_registrations_per_interval,
            target_registrations_interval,
            adjustment_alpha,
            unit_emission,
            proposal_cost,
            proposal_expiration,
            proposal_participation_threshold,
        } = params.clone();

        SubspaceModule::add_global_proposal(
            get_origin(keys[0]),
            burn_rate,
            max_name_length,
            max_allowed_subnets,
            max_allowed_modules,
            max_registrations_per_block,
            max_allowed_weights,
            min_burn,
            max_burn,
            min_stake,
            floor_delegation_fee,
            min_weight_stake,
            target_registrations_per_interval,
            target_registrations_interval,
            adjustment_alpha,
            unit_emission,
            proposal_cost,
            proposal_expiration,
            proposal_participation_threshold,
        )
        .expect("failed to create proposal");

        assert_eq!(SubspaceModule::get_balance_u64(&keys[0]), 1);

        SubspaceModule::vote_proposal(get_origin(keys[0]), 0, true).unwrap();
        SubspaceModule::vote_proposal(get_origin(keys[1]), 0, true).unwrap();
        SubspaceModule::vote_proposal(get_origin(keys[2]), 0, false).unwrap();

        ProposalCost::<Test>::set(COST * 2);

        step_block(100);

        let proposal = Proposals::<Test>::get(0).expect("proposal was not created");
        assert_eq!(proposal.proposal_status, ProposalStatus::Accepted);
        assert_eq!(proposal.finalization_block, Some(100));
        assert_eq!(
            SubspaceModule::get_balance_u64(&keys[0]),
            proposal.proposal_cost + 1,
        );

        ProposalCost::<Test>::set(COST);
        assert_eq!(SubspaceModule::global_params(), params);
    });
}

#[test]
fn creates_global_params_proposal_correctly_and_is_refused() {
    new_test_ext().execute_with(|| {
        const COST: u64 = to_nano(10);

        MinBurn::<Test>::set(0);

        let keys: [_; 3] = from_fn(U256::from);
        let stakes = [1_000_000_000, 1_000_000_000, 1_000_000_000];

        for (key, balance) in keys.iter().zip(stakes) {
            assert_ok!(register_module(0, *key, balance));
        }
        add_balance(keys[0], COST);

        ProposalCost::<Test>::set(COST);
        ProposalExpiration::<Test>::set(200);

        let original = SubspaceModule::global_params();
        let GlobalParams {
            burn_rate,
            max_name_length,
            max_allowed_subnets,
            max_allowed_modules,
            max_registrations_per_block,
            max_allowed_weights,
            min_burn,
            max_burn,
            min_stake,
            floor_delegation_fee,
            min_weight_stake,
            target_registrations_per_interval,
            target_registrations_interval,
            adjustment_alpha,
            unit_emission,
            proposal_cost,
            proposal_expiration,
            proposal_participation_threshold,
        } = GlobalParams {
            min_burn: 100_000_000,
            ..original.clone()
        };

        SubspaceModule::add_global_proposal(
            get_origin(keys[0]),
            burn_rate,
            max_name_length,
            max_allowed_subnets,
            max_allowed_modules,
            max_registrations_per_block,
            max_allowed_weights,
            min_burn,
            max_burn,
            min_stake,
            floor_delegation_fee,
            min_weight_stake,
            target_registrations_per_interval,
            target_registrations_interval,
            adjustment_alpha,
            unit_emission,
            proposal_cost,
            proposal_expiration,
            proposal_participation_threshold,
        )
        .expect("failed to create proposal");

        assert_eq!(SubspaceModule::get_balance_u64(&keys[0]), 1);

        SubspaceModule::vote_proposal(get_origin(keys[0]), 0, true).unwrap();
        SubspaceModule::vote_proposal(get_origin(keys[1]), 0, false).unwrap();
        SubspaceModule::vote_proposal(get_origin(keys[2]), 0, false).unwrap();

        ProposalCost::<Test>::set(COST * 2);

        step_block(100);

        let proposal = Proposals::<Test>::get(0).expect("proposal was not created");
        assert_eq!(proposal.proposal_status, ProposalStatus::Refused);
        assert_eq!(proposal.finalization_block, Some(100));
        assert_eq!(SubspaceModule::get_balance_u64(&keys[0]), 01,);

        ProposalCost::<Test>::set(COST);
        assert_eq!(SubspaceModule::global_params(), original);
    });
}

#[test]
fn creates_subnet_params_proposal_correctly_and_is_approved() {
    new_test_ext().execute_with(|| {
        const COST: u64 = to_nano(10);

        MinBurn::<Test>::set(0);

        let keys: [_; 3] = from_fn(U256::from);
        let stakes = [1_000_000_000, 1_000_000_000, 1_000_000_000];

        for (key, balance) in keys.iter().zip(stakes) {
            assert_ok!(register_module(0, *key, balance));
        }
        add_balance(keys[0], COST);

        ProposalCost::<Test>::set(COST);
        ProposalExpiration::<Test>::set(200);
        VoteModeSubnet::<Test>::set(0, VoteMode::Vote);

        let params = SubnetParams {
            tempo: 150,
            ..SubspaceModule::subnet_params(0)
        };

        let SubnetParams {
            founder,
            founder_share,
            immunity_period,
            incentive_ratio,
            max_allowed_uids,
            max_allowed_weights,
            min_allowed_weights,
            max_stake,
            max_weight_age,
            min_stake,
            name,
            tempo,
            trust_ratio,
            vote_mode,
        } = params.clone();

        SubspaceModule::add_subnet_proposal(
            get_origin(keys[0]),
            founder,
            founder_share,
            immunity_period,
            incentive_ratio,
            max_allowed_uids,
            max_allowed_weights,
            min_allowed_weights,
            max_stake,
            max_weight_age,
            min_stake,
            name,
            tempo,
            trust_ratio,
            vote_mode,
            0,
        )
        .expect("failed to create proposal");

        assert_eq!(SubspaceModule::get_balance_u64(&keys[0]), 1);

        SubspaceModule::vote_proposal(get_origin(keys[0]), 0, true).unwrap();
        SubspaceModule::vote_proposal(get_origin(keys[1]), 0, true).unwrap();
        SubspaceModule::vote_proposal(get_origin(keys[2]), 0, false).unwrap();

        ProposalCost::<Test>::set(COST * 2);

        step_block(100);

        let proposal = Proposals::<Test>::get(0).expect("proposal was not created");
        assert_eq!(proposal.proposal_status, ProposalStatus::Accepted);
        assert_eq!(proposal.finalization_block, Some(100));
        assert_eq!(
            SubspaceModule::get_balance_u64(&keys[0]),
            proposal.proposal_cost + 1,
        );

        dbg!(Tempo::<Test>::contains_key(0));

        ProposalCost::<Test>::set(COST);
        assert_eq!(SubspaceModule::subnet_params(0).tempo, 150);
    });
}

#[test]
fn unregister_vote_from_pending_proposal() {
    new_test_ext().execute_with(|| {
        const COST: u64 = to_nano(10);

        let key = U256::from(0);
        assert_ok!(register_module(0, key, 1_000_000_000));
        add_balance(key, COST);

        ProposalCost::<Test>::set(COST);

        SubspaceModule::add_custom_proposal(get_origin(key), vec![])
            .expect("failed to create proposal");

        SubspaceModule::vote_proposal(get_origin(key), 0, true).unwrap();
        let proposal = Proposals::<Test>::get(0).expect("proposal was not created");
        assert_eq!(proposal.votes_for, BTreeSet::from([key]));

        SubspaceModule::unvote_proposal(get_origin(key), 0).unwrap();
        let proposal = Proposals::<Test>::get(0).expect("proposal was not created");
        assert_eq!(proposal.votes_for, BTreeSet::from([]));
    });
}

// /* TO DO SAM: write test for LatuUpdate after it is set */
// fn test_subnet_porposal() {
//     new_test_ext().execute_with(|| {
//         let netuid = 0;
//         let keys = [U256::from(0), U256::from(1), U256::from(2)];
//         let stakes = [1_000_000_000, 1_000_000_000, 1_000_000_000];

//         for (i, key) in keys.iter().enumerate() {
//             assert_ok!(register_module(netuid, *key, stakes[i]));
//         }
//         let mut params = SubspaceModule::subnet_params(netuid);
//         assert_eq!(
//             params.vote_mode,
//             "authority".as_bytes().to_vec(),
//             "vote mode not set"
//         );
//         params.vote_mode = "stake".as_bytes().to_vec();
//         println!("params: {:?}", params);
//         SubspaceModule::set_subnet_params(netuid, params.clone());
//         let mut params = SubspaceModule::subnet_params(netuid);
//         let _initial_tempo = params.tempo;
//         let final_tempo = 1000;
//         params.tempo = final_tempo;

//         assert_eq!(
//             params.vote_mode,
//             "stake".as_bytes().to_vec(),
//             "vote mode not set"
//         );
//         assert_ok!(SubspaceModule::do_add_subnet_proposal(
//             get_origin(keys[0]),
//             netuid,
//             params
//         ));
//         // we have not passed the threshold yet
//         let proposals = SubspaceModule::get_subnet_proposals(netuid);

//         println!("proposals: {:?}", proposals);

//         assert_eq!(proposals.len(), 1, "proposal not added");
//         assert_eq!(proposals[0].votes, stakes[0], "proposal not added");

//         let proposal = SubspaceModule::get_proposal(0);
//         assert_eq!(proposal.netuid, netuid, "proposal not added");
//         assert!(!proposal.accepted, "proposal not added");
//         // now vote for the proposal

//         assert_ok!(SubspaceModule::vote_proposal(get_origin(keys[1]), 0));
//         let proposal = SubspaceModule::get_proposal(0);
//         assert_eq!(proposal.votes, stakes[0] + stakes[1], "proposal not voted");
//         assert!(proposal.accepted, "proposal not voted");

//         println!("proposal: {:?}", proposal);

//         let params = SubspaceModule::subnet_params(netuid);
//         assert_eq!(params.tempo, final_tempo, "proposal not voted");
//     });
// }

// fn test_max_proposals() {
//     new_test_ext().execute_with(|| {
//         let netuid = 0;
//         let n = 100;
//         let keys: Vec<U256> = (0..n).map(U256::from).collect();
//         let mut stakes = vec![1_000_000_000; n];
//         // increase incrementally to avoid overflow
//         let stakes =
//             stakes.iter_mut().enumerate().map(|(i, x)| *x + i as u64).collect::<Vec<u64>>();

//         for (i, key) in keys.iter().enumerate() {
//             assert_ok!(register_module(netuid, *key, stakes[i]));
//         }

//         let mut params = SubspaceModule::global_params();
//         assert_eq!(
//             params.vote_mode,
//             "authority".as_bytes().to_vec(),
//             "vote mode not set"
//         );
//         params.vote_mode = "stake".as_bytes().to_vec();
//         params.max_proposals = (n / 2) as u64;
//         println!("params: {:?}", params);
//         SubspaceModule::set_global_params(params.clone());

//         assert_eq!(
//             params.vote_mode,
//             "stake".as_bytes().to_vec(),
//             "vote mode not set"
//         );
//         let max_proposals = SubspaceModule::get_max_proposals();
//         let _modes = ["authority".as_bytes().to_vec(), "stake".as_bytes().to_vec()];

//         let mut subnet_params = SubspaceModule::subnet_params(netuid);
//         subnet_params.vote_mode = "stake".as_bytes().to_vec();
//         SubspaceModule::set_subnet_params(netuid, subnet_params.clone());
//         subnet_params = SubspaceModule::subnet_params(netuid);
//         assert_eq!(
//             subnet_params.vote_mode,
//             "stake".as_bytes().to_vec(),
//             "vote mode not set"
//         );

//         for (i, &key) in keys.iter().enumerate() {
//             if i % 2 == 0 {
//                 assert_ok!(SubspaceModule::do_add_global_proposal(
//                     get_origin(key),
//                     params.clone()
//                 ));
//             } else {
//                 assert_ok!(SubspaceModule::do_add_subnet_proposal(
//                     get_origin(key),
//                     netuid,
//                     subnet_params.clone()
//                 ));
//             }

//             let num_proposals = SubspaceModule::num_proposals();
//             let proposals = SubspaceModule::get_global_proposals();
//             let has_max_proposals = SubspaceModule::has_max_proposals();
//             println!("max_proposals: {:?}", max_proposals);
//             println!("has_max_proposals: {:?}", has_max_proposals);
//             println!("num_proposals: {:?}", num_proposals);
//             println!("proposals: {:?}", proposals.len());

//             let num_subnet_proposals = SubspaceModule::num_subnet_proposals(netuid);
//             let num_global_proposals = SubspaceModule::num_global_proposals();
//             assert_eq!(
//                 num_subnet_proposals + num_global_proposals,
//                 num_proposals,
//                 "proposal not added"
//             );

//             if num_proposals >= max_proposals {
//                 assert!(SubspaceModule::has_max_proposals(), "proposal not added");
//             } else {
//                 assert!(!SubspaceModule::has_max_proposals(), "proposal not added");
//             }

//             assert!(
//                 proposals.len() as u64 <= max_proposals,
//                 "proposal not added"
//             );
//         }

//         assert!(SubspaceModule::has_max_proposals(), "proposal not added");
//         assert_eq!(
//             SubspaceModule::num_proposals(),
//             max_proposals,
//             "proposal not added"
//         );
//     });
// }

// fn test_global_porposal() {
//     new_test_ext().execute_with(|| {
//         let keys = [U256::from(1), U256::from(2), U256::from(3)];
//         let stakes = [1_000_000_000, 1_000_000_000, 1_000_000_000];

//         // register on seperate subnets
//         for (i, (key, stake)) in keys.iter().zip(stakes).enumerate() {
//             assert_ok!(register_module(i as u16, *key, stake));
//         }

//         let mut params = SubspaceModule::global_params();
//         eprintln!("{}", params.min_burn);
//         let _initial_max_registrations_per_block = params.max_registrations_per_block;
//         let max_registrations_per_block = 1000;

//         params.max_registrations_per_block = max_registrations_per_block;
//         assert_ok!(SubspaceModule::do_add_global_proposal(
//             get_origin(keys[0]),
//             params
//         ));

//         // we have not passed the threshold yet
//         let proposals = SubspaceModule::get_global_proposals();

//         assert_eq!(proposals.len(), 1, "proposal not added");
//         assert_eq!(proposals[0].votes, stakes[0], "proposal not added");

//         let proposal = SubspaceModule::get_proposal(0);
//         assert!(!proposal.accepted, "proposal not added");

//         // now vote for the proposal

//         assert_ok!(SubspaceModule::vote_proposal(get_origin(keys[1]), 0));
//         let proposal = SubspaceModule::get_proposal(0);
//         assert_eq!(proposal.votes, stakes[0] + stakes[1], "proposal not voted");
//         assert!(proposal.accepted, "proposal not voted");

//         println!("proposal: {:?}", proposal);

//         let params = SubspaceModule::global_params();
//         assert_eq!(
//             params.max_registrations_per_block, max_registrations_per_block,
//             "proposal not voted"
//         );
//     });
// }

// fn test_unvote() {
//     new_test_ext().execute_with(|| {
//         let netuid = 0;
//         let keys = [U256::from(0), U256::from(1), U256::from(2)];
//         let stakes = [1_000_000_000, 1_000_000_000, 1_000_000_000];

//         for (i, key) in keys.iter().enumerate() {
//             assert_ok!(register_module(netuid, *key, stakes[i]));
//         }
//         let mut params = SubspaceModule::subnet_params(netuid);
//         assert_eq!(
//             params.vote_mode,
//             "authority".as_bytes().to_vec(),
//             "vote mode not set"
//         );
//         params.vote_mode = "stake".as_bytes().to_vec();
//         println!("params: {:?}", params);
//         SubspaceModule::set_subnet_params(netuid, params.clone());
//         let mut params = SubspaceModule::subnet_params(netuid);
//         let _initial_tempo = params.tempo;
//         let final_tempo = 1000;
//         params.tempo = final_tempo;

//         assert_eq!(
//             params.vote_mode,
//             "stake".as_bytes().to_vec(),
//             "vote mode not set"
//         );
//         assert_ok!(SubspaceModule::do_add_subnet_proposal(
//             get_origin(keys[0]),
//             netuid,
//             params
//         ));
//         assert!(SubspaceModule::proposal_exists(0));
//         assert!(SubspaceModule::is_proposal_owner(&keys[0], 0));
//         assert_ok!(SubspaceModule::unvote_proposal(get_origin(keys[0])));

//         // we have not passed the threshold yet
//         let proposals = SubspaceModule::get_subnet_proposals(netuid);

//         println!("proposals: {:?}", proposals);

//         assert_eq!(proposals.len(), 0, "proposal not added");
//     });
// }
