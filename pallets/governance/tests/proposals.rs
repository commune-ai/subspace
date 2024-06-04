use mock::*;
use pallet_governance::{proposal::ProposalStatus, GlobalGovernanceConfig, Proposals};
use pallet_governance_api::GovernanceConfiguration;
use pallet_subspace::{DaoTreasuryAddress, GlobalParams};

mod mock;

fn config(proposal_cost: u64, proposal_expiration: u32) {
    GlobalGovernanceConfig::<Test>::set(GovernanceConfiguration {
        proposal_cost,
        proposal_expiration,
        vote_mode: pallet_governance_api::VoteMode::Vote,
        ..Default::default()
    });
}

fn vote(account: u32, proposal_id: u64, agree: bool) {
    assert_ok!(Governance::do_vote_proposal(
        get_origin(account),
        proposal_id,
        agree
    ));
}

fn register(account: u32, module: u32, stake: u64) {
    if get_balance(account) <= stake {
        add_balance(account, stake + to_nano(1));
    }

    assert_ok!(Subspace::do_register(
        get_origin(account),
        format!("subnet-{account}-{module}").as_bytes().to_vec(),
        format!("module-{account}-{module}").as_bytes().to_vec(),
        format!("address-{account}-{module}").as_bytes().to_vec(),
        stake,
        module,
        None,
    ));
}

#[test]
fn global_governance_config_validates_parameters_correctly() {
    new_test_ext().execute_with(|| {
        Governance::validate(GovernanceConfiguration {
            proposal_cost: 0,
            ..Default::default()
        })
        .expect_err("invalid proposal cost was applied");

        Governance::validate(GovernanceConfiguration {
            proposal_expiration: 0,
            ..Default::default()
        })
        .expect_err("invalid proposal cost was applied");

        Governance::validate(GovernanceConfiguration {
            proposal_cost: 1,
            proposal_expiration: 1,
            ..Default::default()
        })
        .expect("valid config failed to be applied applied");
    });
}

#[test]
fn global_proposal_validates_parameters() {
    new_test_ext().execute_with(|| {
        const KEY: u32 = 0;
        add_balance(KEY, to_nano(100_000));

        let test = |global_params| {
            let GlobalParams {
                max_name_length,
                min_name_length,
                max_allowed_subnets,
                max_allowed_modules,
                max_registrations_per_block,
                max_allowed_weights,
                floor_delegation_fee,
                floor_founder_share,
                min_weight_stake,
                curator,
                general_subnet_application_cost,
                subnet_stake_threshold,
                burn_config,
                governance_config,
            } = global_params;

            Governance::add_global_proposal(
                get_origin(KEY),
                vec![b'0'; 64],
                max_name_length,
                min_name_length,
                max_allowed_subnets,
                max_allowed_modules,
                max_registrations_per_block,
                max_allowed_weights,
                burn_config.max_burn,
                burn_config.min_burn,
                floor_delegation_fee,
                floor_founder_share,
                min_weight_stake,
                curator,
                subnet_stake_threshold,
                governance_config.proposal_cost,
                governance_config.proposal_expiration,
                general_subnet_application_cost,
            )
        };

        test(GlobalParams {
            governance_config: GovernanceConfiguration {
                proposal_cost: 0,
                ..Default::default()
            },
            ..Subspace::global_params()
        })
        .expect_err("created proposal with invalid max name length");

        test(Subspace::global_params()).expect("failed to create proposal with valid parameters");
    });
}

#[test]
fn proposal_is_accepted_correctly() {
    new_test_ext().execute_with(|| {
        const FOR: u32 = 0;
        const AGAINST: u32 = 1;

        zero_min_burn();
        let origin = get_origin(0);

        register(FOR, 0, to_nano(10));
        register(AGAINST, 0, to_nano(5));

        config(1, 300);

        assert_ok!(Governance::do_add_global_custom_proposal(
            origin,
            vec![b'0'; 64]
        ));

        step_block(100);

        vote(FOR, 0, true);
        vote(AGAINST, 0, false);

        step_block(100);

        assert!(matches!(
            Proposals::<Test>::get(0).unwrap().status,
            ProposalStatus::Accepted {
                block: 200,
                stake_for: 10_000_000_000,
                stake_against: 5_000_000_000,
            }
        ));
    });
}

#[test]
fn proposal_is_refused_correctly() {
    new_test_ext().execute_with(|| {
        const FOR: u32 = 0;
        const AGAINST: u32 = 1;

        zero_min_burn();
        let origin = get_origin(0);

        register(FOR, 0, to_nano(5));
        register(AGAINST, 0, to_nano(10));

        config(1, 300);

        assert_ok!(Governance::do_add_global_custom_proposal(
            origin,
            vec![b'0'; 64]
        ));

        step_block(100);

        vote(FOR, 0, true);
        vote(AGAINST, 0, false);

        step_block(100);

        assert!(matches!(
            dbg!(Proposals::<Test>::get(0).unwrap().status),
            ProposalStatus::Refused {
                block: 200,
                stake_for: 5_000_000_000,
                stake_against: 10_000_000_000,
            }
        ));
    });
}

#[test]
fn global_params_proposal_accepted() {
    new_test_ext().execute_with(|| {
        const KEY: u32 = 0;
        zero_min_burn();

        register(KEY, 0, to_nano(10));
        config(1, 200);

        let GlobalParams {
            max_name_length,
            min_name_length,
            max_allowed_subnets,
            max_allowed_modules,
            max_registrations_per_block,
            max_allowed_weights,
            floor_delegation_fee,
            floor_founder_share,
            min_weight_stake,
            curator,
            general_subnet_application_cost,
            subnet_stake_threshold,
            burn_config,
            mut governance_config,
        } = Subspace::global_params();

        governance_config.proposal_cost = 69_420;

        Governance::add_global_proposal(
            get_origin(KEY),
            vec![b'0'; 64],
            max_name_length,
            min_name_length,
            max_allowed_subnets,
            max_allowed_modules,
            max_registrations_per_block,
            max_allowed_weights,
            burn_config.max_burn,
            100_000_000,
            floor_delegation_fee,
            floor_founder_share,
            min_weight_stake,
            curator,
            subnet_stake_threshold,
            governance_config.proposal_cost,
            governance_config.proposal_expiration,
            general_subnet_application_cost,
        )
        .unwrap();

        vote(KEY, 0, true);
        step_block(100);

        assert_eq!(GlobalGovernanceConfig::<Test>::get().proposal_cost, 69_420);
    });
}

// #[test]
// fn creates_global_params_proposal_correctly_and_expires() {
//     new_test_ext().execute_with(|| {
//         const COST: u64 = to_nano(1);
//         const KEY: u32 = 0;

//         add_balance(KEY, COST + 1);

//         let mut config = GlobalGovernanceConfig::<Test>::get();
//         config.proposal_cost = COST;
//         config.proposal_expiration = 50;
//         config.apply_global().unwrap();

//         let original = Subspace::global_params();
//         let GlobalParams {
//             mut max_name_length,
//             min_name_length,
//             max_allowed_subnets,
//             max_allowed_modules,
//             max_registrations_per_block,
//             max_allowed_weights,
//             floor_delegation_fee,
//             floor_founder_share,
//             min_weight_stake,
//             proposal_cost,
//             proposal_expiration,
//             proposal_participation_threshold,
//             curator,
//             general_subnet_application_cost,
//             subnet_stake_threshold,
//             burn_config,
//         } = original.clone();
//         max_name_length /= 2;

//         Governance::add_global_proposal(
//             get_origin(KEY),
//             vec![b'0'; 64],
//             max_name_length,
//             min_name_length,
//             max_allowed_subnets,
//             max_allowed_modules,
//             max_registrations_per_block,
//             max_allowed_weights,
//             burn_config.max_burn,
//             burn_config.min_burn,
//             floor_delegation_fee,
//             floor_founder_share,
//             min_weight_stake,
//             curator,
//             subnet_stake_threshold,
//             proposal_cost,
//             proposal_expiration,
//             proposal_participation_threshold,
//             general_subnet_application_cost,
//         )
//         .expect("failed to create proposal");

//         let proposal = Proposals::<Test>::get(0).expect("proposal was not created");
//         assert_eq!(proposal.id, 0);
//         assert_eq!(proposal.proposer, KEY);
//         assert_eq!(proposal.expiration_block, 50);
//         assert_eq!(
//             proposal.data,
//             ProposalData::<Test>::GlobalParams(original.clone())
//         );
//         assert_eq!(
//             proposal.status,
//             ProposalStatus::Open {
//                 votes_for: Default::default(),
//                 votes_against: Default::default()
//             }
//         );
//         assert_eq!(proposal.proposal_cost, COST);
//         assert_eq!(proposal.creation_block, 0);

//         SubspaceModule::vote_proposal(get_origin(U256::from(1)), 0, true).unwrap();

//         step_block(200);

//         assert_eq!(SubspaceModule::get_balance_u64(&key), 1);

//         assert_eq!(SubspaceModule::global_params(), original);
//     });
// }

// #[test]
// fn creates_global_params_proposal_correctly_and_is_approved() {
//     new_test_ext().execute_with(|| {
//         const COST: u64 = to_nano(10);

//         zero_min_burn();

//         let keys: [_; 3] = from_fn(U256::from);
//         let stakes = [1_000_000_000, 1_000_000_000, 1_000_000_000];

//         for (key, balance) in keys.iter().zip(stakes) {
//             assert_ok!(register_module(0, *key, balance));
//         }
//         add_balance(keys[0], COST);

//         ProposalCost::<Test>::set(COST);
//         ProposalExpiration::<Test>::set(200);

//         let burn_config = BurnConfiguration {
//             min_burn: 100_000_000,
//             ..BurnConfiguration::<Test>::default()
//         };
//         assert_ok!(burn_config.apply());

//         let BurnConfiguration {
//             min_burn, max_burn, ..
//         } = BurnConfig::<Test>::get();

//         let params = SubspaceModule::global_params();

//         let GlobalParams {
//             max_name_length,
//             min_name_length,
//             max_allowed_subnets,
//             max_allowed_modules,
//             max_registrations_per_block,
//             max_allowed_weights,
//             floor_delegation_fee,
//             floor_founder_share,
//             min_weight_stake,
//             curator,
//             subnet_stake_threshold,
//             proposal_cost,
//             proposal_expiration,
//             proposal_participation_threshold,
//             general_subnet_application_cost,
//             ..
//         } = params.clone();

//         SubspaceModule::add_global_proposal(
//             get_origin(keys[0]),
//             max_name_length,
//             min_name_length,
//             max_allowed_subnets,
//             max_allowed_modules,
//             max_registrations_per_block,
//             max_allowed_weights,
//             max_burn,
//             min_burn,
//             floor_delegation_fee,
//             floor_founder_share,
//             min_weight_stake,
//             curator,
//             subnet_stake_threshold,
//             proposal_cost,
//             proposal_expiration,
//             proposal_participation_threshold,
//             general_subnet_application_cost,
//         )
//         .expect("failed to create proposal");

//         assert_eq!(SubspaceModule::get_balance_u64(&keys[0]), 1);

//         SubspaceModule::vote_proposal(get_origin(keys[0]), 0, true).unwrap();
//         SubspaceModule::vote_proposal(get_origin(keys[1]), 0, true).unwrap();
//         SubspaceModule::vote_proposal(get_origin(keys[2]), 0, false).unwrap();

//         ProposalCost::<Test>::set(COST * 2);

//         step_block(100);

//         let proposal = Proposals::<Test>::get(0).expect("proposal was not created");
//         assert_eq!(proposal.status, ProposalStatus::Accepted);
//         assert_eq!(proposal.finalization_block, Some(100));
//         assert_eq!(
//             SubspaceModule::get_balance_u64(&keys[0]),
//             proposal.proposal_cost + 1,
//         );

//         ProposalCost::<Test>::set(COST);
//         assert_eq!(SubspaceModule::global_params(), params);
//     });
// }

// #[test]
// fn creates_global_params_proposal_correctly_and_is_refused() {
//     new_test_ext().execute_with(|| {
//         const COST: u64 = to_nano(10);

//         zero_min_burn();

//         let keys: [_; 3] = from_fn(U256::from);
//         let stakes = [1_000_000_000, 1_000_000_000, 1_000_000_000];

//         for (key, balance) in keys.iter().zip(stakes) {
//             assert_ok!(register_module(0, *key, balance));
//         }
//         add_balance(keys[0], COST);

//         ProposalCost::<Test>::set(COST);
//         ProposalExpiration::<Test>::set(200);

//         let burn_config = BurnConfiguration {
//             min_burn: 100_000_000,
//             ..BurnConfiguration::<Test>::default()
//         };
//         assert_ok!(burn_config.apply());

//         let original = SubspaceModule::global_params();
//         let GlobalParams {
//             floor_founder_share,
//             max_name_length,
//             min_name_length,
//             max_allowed_subnets,
//             max_allowed_modules,
//             max_registrations_per_block,
//             max_allowed_weights,
//             floor_delegation_fee,
//             min_weight_stake,
//             curator,
//             subnet_stake_threshold,
//             proposal_cost,
//             proposal_expiration,
//             proposal_participation_threshold,
//             general_subnet_application_cost,
//             ..
//         } = GlobalParams { ..original.clone() };

//         let BurnConfiguration {
//             min_burn, max_burn, ..
//         } = BurnConfig::<Test>::get();

//         SubspaceModule::add_global_proposal(
//             get_origin(keys[0]),
//             max_name_length,
//             min_name_length,
//             max_allowed_subnets,
//             max_allowed_modules,
//             max_registrations_per_block,
//             max_allowed_weights,
//             max_burn,
//             min_burn,
//             floor_delegation_fee,
//             floor_founder_share,
//             min_weight_stake,
//             curator,
//             subnet_stake_threshold,
//             proposal_cost,
//             proposal_expiration,
//             proposal_participation_threshold,
//             general_subnet_application_cost,
//         )
//         .expect("failed to create proposal");

//         assert_eq!(SubspaceModule::get_balance_u64(&keys[0]), 1);

//         SubspaceModule::vote_proposal(get_origin(keys[0]), 0, true).unwrap();
//         SubspaceModule::vote_proposal(get_origin(keys[1]), 0, false).unwrap();
//         SubspaceModule::vote_proposal(get_origin(keys[2]), 0, false).unwrap();

//         ProposalCost::<Test>::set(COST * 2);

//         step_block(100);

//         let proposal = Proposals::<Test>::get(0).expect("proposal was not created");
//         assert_eq!(proposal.status, ProposalStatus::Refused);
//         assert_eq!(proposal.finalization_block, Some(100));
//         assert_eq!(SubspaceModule::get_balance_u64(&keys[0]), 1,);

//         ProposalCost::<Test>::set(COST);
//         assert_eq!(SubspaceModule::global_params(), original);
//     });
// }

// #[test]
// fn creates_subnet_params_proposal_correctly_and_is_approved() {
//     new_test_ext().execute_with(|| {
//         const COST: u64 = to_nano(10);

//         zero_min_burn();

//         let keys: [_; 3] = from_fn(U256::from);
//         let stakes = [1_000_000_000, 1_000_000_000, 1_000_000_000];

//         for (key, balance) in keys.iter().zip(stakes) {
//             assert_ok!(register_module(0, *key, balance));
//         }
//         add_balance(keys[0], COST);

//         ProposalExpiration::<Test>::set(200);
//         VoteModeSubnet::<Test>::set(0, VoteMode::Vote);

//         let params = SubnetParams {
//             tempo: 150,
//             ..SubspaceModule::subnet_params(0)
//         };

//         let SubnetParams {
//             founder,
//             name,
//             founder_share,
//             immunity_period,
//             incentive_ratio,
//             max_allowed_uids,
//             max_allowed_weights,
//             min_allowed_weights,
//             min_stake,
//             max_weight_age,
//             tempo,
//             trust_ratio,
//             maximum_set_weight_calls_per_epoch,
//             vote_mode,
//             bonds_ma,
//             target_registrations_interval,
//             target_registrations_per_interval,
//             max_registrations_per_interval,
//             adjustment_alpha,
//         } = params.clone();

//         SubspaceModule::add_subnet_proposal(
//             get_origin(keys[0]),
//             0, // netuid
//             founder,
//             name,
//             founder_share,
//             immunity_period,
//             incentive_ratio,
//             max_allowed_uids,
//             max_allowed_weights,
//             min_allowed_weights,
//             min_stake,
//             max_weight_age,
//             tempo,
//             trust_ratio,
//             maximum_set_weight_calls_per_epoch,
//             vote_mode,
//             bonds_ma,
//             target_registrations_interval,
//             target_registrations_per_interval,
//             max_registrations_per_interval,
//             adjustment_alpha,
//         )
//         .expect("failed to create proposal");

//         assert_eq!(SubspaceModule::get_balance_u64(&keys[0]), 1);

//         SubspaceModule::vote_proposal(get_origin(keys[0]), 0, true).unwrap();
//         SubspaceModule::vote_proposal(get_origin(keys[1]), 0, true).unwrap();
//         SubspaceModule::vote_proposal(get_origin(keys[2]), 0, false).unwrap();

//         ProposalCost::<Test>::set(COST * 2);

//         step_block(100);

//         let proposal = Proposals::<Test>::get(0).expect("proposal was not created");
//         assert_eq!(proposal.status, ProposalStatus::Accepted);
//         assert_eq!(proposal.finalization_block, Some(100));
//         assert_eq!(
//             SubspaceModule::get_balance_u64(&keys[0]),
//             proposal.proposal_cost + 1,
//         );

//         dbg!(Tempo::<Test>::contains_key(0));

//         ProposalCost::<Test>::set(COST);
//         assert_eq!(SubspaceModule::subnet_params(0).tempo, 150);
//     });
// }

// #[test]
// fn unregister_vote_from_pending_proposal() {
//     new_test_ext().execute_with(|| {
//         const COST: u64 = to_nano(10);

//         zero_min_burn();

//         let key = U256::from(0);
//         assert_ok!(register_module(0, key, 1_000_000_000));
//         add_balance(key, COST);

//         ProposalCost::<Test>::set(COST);

//         SubspaceModule::add_custom_proposal(get_origin(key), b"test".to_vec())
//             .expect("failed to create proposal");

//         SubspaceModule::vote_proposal(get_origin(key), 0, true).unwrap();
//         let proposal = Proposals::<Test>::get(0).expect("proposal was not created");
//         assert_eq!(proposal.votes_for, BTreeSet::from([key]));

//         SubspaceModule::unvote_proposal(get_origin(key), 0).unwrap();
//         let proposal = Proposals::<Test>::get(0).expect("proposal was not created");
//         assert_eq!(proposal.votes_for, BTreeSet::from([]));
//     });
// }

#[test]
fn creates_treasury_transfer_proposal_and_transfers() {
    new_test_ext().execute_with(|| {
        zero_min_burn();

        let origin = get_origin(0);
        Governance::add_transfer_dao_treasury_proposal(
            origin.clone(),
            vec![b'0'; 64],
            to_nano(5),
            0,
        )
        .expect_err("proposal should not be created when treasury does not have enough money");

        add_balance(DaoTreasuryAddress::<Test>::get(), to_nano(10));
        add_balance(0, to_nano(3));
        register(0, 0, to_nano(1));
        config(to_nano(1), 200);

        Governance::add_transfer_dao_treasury_proposal(origin, vec![b'0'; 64], to_nano(5), 0)
            .expect("proposal should be created");
        vote(0, 0, true);

        step_block(100);

        assert_eq!(get_balance(DaoTreasuryAddress::<Test>::get()), to_nano(5));
        assert_eq!(get_balance(0), to_nano(7));
    });
}
