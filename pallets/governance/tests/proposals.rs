use dao::ApplicationStatus;
use mock::*;
use pallet_subspace::{subnet::SubnetChangeset, GlobalParams, SubnetParams};
use proposal::get_reward_allocation;
use substrate_fixed::{types::extra::U32, FixedI128};

mod mock;

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

            Governance::add_global_params_proposal(
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
fn global_custom_proposal_is_accepted_correctly() {
    new_test_ext().execute_with(|| {
        const FOR: u32 = 0;
        const AGAINST: u32 = 1;

        zero_min_burn();
        let origin = get_origin(0);

        register(FOR, 0, 0, to_nano(10));
        register(AGAINST, 0, 1, to_nano(5));

        config(1, 100);

        assert_ok!(Governance::do_add_global_custom_proposal(
            origin,
            vec![b'0'; 64]
        ));

        vote(FOR, 0, true);
        vote(AGAINST, 0, false);

        step_block(100);

        assert_eq!(
            Proposals::<Test>::get(0).unwrap().status,
            ProposalStatus::Accepted {
                block: 100,
                stake_for: 10_000_000_000,
                stake_against: 5_000_000_000,
            }
        );
    });
}

#[test]
fn subnet_custom_proposal_is_accepted_correctly() {
    new_test_ext().execute_with(|| {
        const FOR: u32 = 0;
        const AGAINST: u32 = 1;

        zero_min_burn();
        let origin = get_origin(0);

        register(FOR, 0, 0, to_nano(10));
        register(AGAINST, 0, 1, to_nano(5));
        register(AGAINST, 1, 0, to_nano(10));

        config(1, 100);

        assert_ok!(Governance::do_add_subnet_custom_proposal(
            origin,
            0,
            vec![b'0'; 64]
        ));

        vote(FOR, 0, true);
        vote(AGAINST, 0, false);

        step_block(100);

        assert_eq!(
            Proposals::<Test>::get(0).unwrap().status,
            ProposalStatus::Accepted {
                block: 100,
                stake_for: 10_000_000_000,
                stake_against: 5_000_000_000,
            }
        );
    });
}

#[test]
fn global_proposal_is_refused_correctly() {
    new_test_ext().execute_with(|| {
        const FOR: u32 = 0;
        const AGAINST: u32 = 1;

        zero_min_burn();
        let origin = get_origin(0);

        register(FOR, 0, 0, to_nano(5));
        register(AGAINST, 0, 1, to_nano(10));

        config(1, 100);

        assert_ok!(Governance::do_add_global_custom_proposal(
            origin,
            vec![b'0'; 64]
        ));

        vote(FOR, 0, true);
        vote(AGAINST, 0, false);

        step_block(100);

        assert_eq!(
            Proposals::<Test>::get(0).unwrap().status,
            ProposalStatus::Refused {
                block: 100,
                stake_for: 5_000_000_000,
                stake_against: 10_000_000_000,
            }
        );
    });
}

#[test]
fn global_params_proposal_accepted() {
    new_test_ext().execute_with(|| {
        const KEY: u32 = 0;
        zero_min_burn();

        register(KEY, 0, 0, to_nano(10));
        config(1, 100);

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

        Governance::add_global_params_proposal(
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

#[test]
fn subnet_params_proposal_accepted() {
    new_test_ext().execute_with(|| {
        const KEY: u32 = 0;
        zero_min_burn();

        register(KEY, 0, 0, to_nano(10));
        config(1, 100);

        SubnetChangeset::update(
            0,
            SubnetParams {
                governance_config: Default::default(),
                ..Subspace::subnet_params(0)
            },
        )
        .unwrap()
        .apply(0)
        .unwrap();

        let SubnetParams {
            founder,
            founder_share,
            immunity_period,
            incentive_ratio,
            max_allowed_uids,
            max_allowed_weights,
            min_allowed_weights,
            max_weight_age,
            min_stake,
            name,
            tempo,
            trust_ratio,
            maximum_set_weight_calls_per_epoch,
            bonds_ma,
            target_registrations_interval,
            target_registrations_per_interval,
            max_registrations_per_interval,
            adjustment_alpha,
            mut governance_config,
        } = Subspace::subnet_params(0);

        governance_config.vote_mode = VoteMode::Authority;

        Governance::add_subnet_params_proposal(
            get_origin(KEY),
            0,
            vec![b'0'; 64],
            founder,
            name,
            founder_share,
            immunity_period,
            incentive_ratio,
            max_allowed_uids,
            max_allowed_weights,
            min_allowed_weights,
            min_stake,
            max_weight_age,
            tempo,
            trust_ratio,
            maximum_set_weight_calls_per_epoch,
            governance_config.vote_mode,
            bonds_ma,
            target_registrations_interval,
            target_registrations_per_interval,
            max_registrations_per_interval,
            adjustment_alpha,
        )
        .unwrap();

        vote(KEY, 0, true);
        step_block(100);

        assert_eq!(
            SubnetGovernanceConfig::<Test>::get(0).vote_mode,
            VoteMode::Authority
        );
    });
}

#[test]
fn global_proposals_counts_delegated_stake() {
    new_test_ext().execute_with(|| {
        const FOR: u32 = 0;
        const AGAINST: u32 = 1;
        const FOR_DELEGATED: u32 = 2;
        const AGAINST_DELEGATED: u32 = 3;

        zero_min_burn();
        let origin = get_origin(0);

        register(FOR, 0, 0, to_nano(5));
        delegate(FOR);
        register(AGAINST, 0, 1, to_nano(10));

        stake(FOR_DELEGATED, 0, 0, to_nano(10));
        delegate(FOR_DELEGATED);
        stake(AGAINST_DELEGATED, 0, 1, to_nano(3));
        delegate(AGAINST_DELEGATED);

        config(1, 100);

        assert_ok!(Governance::do_add_global_custom_proposal(
            origin,
            vec![b'0'; 64]
        ));

        vote(FOR, 0, true);
        vote(AGAINST, 0, false);

        step_block(100);

        assert_eq!(
            Proposals::<Test>::get(0).unwrap().status,
            ProposalStatus::Accepted {
                block: 100,
                stake_for: 15_000_000_000,
                stake_against: 13_000_000_000,
            }
        );
    });
}

#[test]
fn subnet_proposals_counts_delegated_stake() {
    new_test_ext().execute_with(|| {
        const FOR: u32 = 0;
        const AGAINST: u32 = 1;
        const FOR_DELEGATED: u32 = 2;
        const AGAINST_DELEGATED: u32 = 3;
        const FOR_DELEGATED_WRONG: u32 = 4;
        const AGAINST_DELEGATED_WRONG: u32 = 5;

        zero_min_burn();
        let origin = get_origin(0);

        register(FOR, 0, 0, to_nano(5));
        register(FOR, 1, 0, to_nano(5));
        register(AGAINST, 0, 1, to_nano(10));
        register(AGAINST, 1, 1, to_nano(10));

        stake(FOR_DELEGATED, 0, 0, to_nano(10));
        delegate(FOR_DELEGATED);
        stake(AGAINST_DELEGATED, 0, 1, to_nano(3));
        delegate(AGAINST_DELEGATED);

        stake(FOR_DELEGATED_WRONG, 1, 0, to_nano(10));
        delegate(FOR_DELEGATED_WRONG);
        stake(AGAINST_DELEGATED_WRONG, 1, 1, to_nano(3));
        delegate(AGAINST_DELEGATED_WRONG);

        config(1, 100);

        assert_ok!(Governance::do_add_subnet_custom_proposal(
            origin,
            0,
            vec![b'0'; 64]
        ));

        vote(FOR, 0, true);
        vote(AGAINST, 0, false);

        step_block(100);

        assert_eq!(
            Proposals::<Test>::get(0).unwrap().status,
            ProposalStatus::Accepted {
                block: 100,
                stake_for: 15_000_000_000,
                stake_against: 13_000_000_000,
            }
        );
    });
}

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
        register(0, 0, 0, to_nano(1));
        config(to_nano(1), 100);

        Governance::add_transfer_dao_treasury_proposal(origin, vec![b'0'; 64], to_nano(5), 0)
            .expect("proposal should be created");
        vote(0, 0, true);

        step_block(100);

        assert_eq!(get_balance(DaoTreasuryAddress::<Test>::get()), to_nano(5));
        assert_eq!(get_balance(0), to_nano(7));
    });
}

/// This test, observes the distribution of governance reward logic over time.
#[test]
fn rewards_wont_exceed_treasury() {
    new_test_ext().execute_with(|| {
        zero_min_burn();
        // Fill the governance address with 1 mil so we are not limited by the max allocation
        let amount = to_nano(1_000_000_000);
        let key = DaoTreasuryAddress::<Test>::get();
        add_balance(key, amount);

        let governance_config: GovernanceConfiguration = GlobalGovernanceConfig::<Test>::get();
        let n = 0;
        let allocation = get_reward_allocation::<Test>(&governance_config, n).unwrap();
        assert_eq!(
            FixedI128::<U32>::saturating_from_num(allocation),
            governance_config.max_proposal_reward_treasury_allocation
        );
    });
}

#[test]
fn test_whitelist() {
    new_test_ext().execute_with(|| {
        let key = 0;
        let adding_key = 1;
        let mut params = Subspace::global_params();
        params.curator = key;
        assert_ok!(Subspace::set_global_params(params));

        let proposal_cost = GeneralSubnetApplicationCost::<Test>::get();
        let data = "test".as_bytes().to_vec();

        add_balance(key, proposal_cost + 1);
        // first submit an application
        let balance_before = Subspace::get_balance_u64(&key);

        assert_ok!(Governance::add_dao_application(
            get_origin(key),
            adding_key,
            data.clone(),
        ));

        let balance_after = Subspace::get_balance_u64(&key);
        assert_eq!(balance_after, balance_before - proposal_cost);

        // Assert that the proposal is initially in the Pending status
        for (_, value) in CuratorApplications::<Test>::iter() {
            assert_eq!(value.status, ApplicationStatus::Pending);
            assert_eq!(value.user_id, adding_key);
            assert_eq!(value.data, data);
        }

        // add key to whitelist
        assert_ok!(Governance::add_to_whitelist(get_origin(key), adding_key, 1,));

        let balance_after_accept = Subspace::get_balance_u64(&key);

        assert_eq!(balance_after_accept, balance_before);

        // Assert that the proposal is now in the Accepted status
        for (_, value) in CuratorApplications::<Test>::iter() {
            assert_eq!(value.status, ApplicationStatus::Accepted);
            assert_eq!(value.user_id, adding_key);
            assert_eq!(value.data, data);
        }

        assert!(Governance::is_in_legit_whitelist(&adding_key));
    });
}
