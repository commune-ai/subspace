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
        assert_ok!(register_module(0, U256::from(2), 1_000_000_100));

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
            min_name_length: _,
            nominator: _,
            subnet_stake_threshold: _,
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
        assert_eq!(proposal.status, ProposalStatus::Pending);
        assert_eq!(proposal.votes_for, Default::default());
        assert_eq!(proposal.votes_against, Default::default());
        assert_eq!(proposal.proposal_cost, COST);
        assert_eq!(proposal.finalization_block, None);

        SubspaceModule::vote_proposal(get_origin(U256::from(1)), 0, true).unwrap();

        step_block(200);

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
            min_name_length: _,
            nominator: _,
            subnet_stake_threshold: _,
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
        assert_eq!(proposal.status, ProposalStatus::Accepted);
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
            min_name_length: _,
            nominator: _,
            subnet_stake_threshold: _,
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
        assert_eq!(proposal.status, ProposalStatus::Refused);
        assert_eq!(proposal.finalization_block, Some(100));
        assert_eq!(SubspaceModule::get_balance_u64(&keys[0]), 1,);

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
        assert_eq!(proposal.status, ProposalStatus::Accepted);
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

        SubspaceModule::add_custom_proposal(get_origin(key), b"test".to_vec())
            .expect("failed to create proposal");

        SubspaceModule::vote_proposal(get_origin(key), 0, true).unwrap();
        let proposal = Proposals::<Test>::get(0).expect("proposal was not created");
        assert_eq!(proposal.votes_for, BTreeSet::from([key]));

        SubspaceModule::unvote_proposal(get_origin(key), 0).unwrap();
        let proposal = Proposals::<Test>::get(0).expect("proposal was not created");
        assert_eq!(proposal.votes_for, BTreeSet::from([]));
    });
}
