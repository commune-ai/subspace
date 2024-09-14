// ---------
// Proposal
// ---------
use crate::mock::*;
pub use frame_support::{assert_err, assert_noop, assert_ok};
use pallet_governance::{
    dao::ApplicationStatus, proposal::get_reward_allocation, Curator, CuratorApplications,
    DaoTreasuryAddress, Error, GeneralSubnetApplicationCost, GlobalGovernanceConfig, GovernanceApi,
    ProposalStatus, Proposals, SubnetGovernanceConfig, VoteMode,
};
use pallet_governance_api::GovernanceConfiguration;
use pallet_subspace::{subnet::SubnetChangeset, GlobalParams, SubnetParams};
use substrate_fixed::{types::extra::U32, FixedI128};

fn register(account: AccountId, subnet_id: u16, module: AccountId, stake: u64) {
    if get_balance(account) <= to_nano(1) {
        add_balance(account, to_nano(1));
    }

    let _ = SubspaceMod::do_register_subnet(
        get_origin(account),
        format!("subnet-{subnet_id}").as_bytes().to_vec(),
        None,
    );

    assert_ok!(SubspaceMod::do_register(
        get_origin(account),
        format!("subnet-{subnet_id}").as_bytes().to_vec(),
        format!("module-{module}").as_bytes().to_vec(),
        format!("address-{account}-{module}").as_bytes().to_vec(),
        module,
        None,
    ));
    SubspaceMod::increase_stake(&account, &module, stake);
}

#[test]
fn global_governance_config_validates_parameters_correctly() {
    new_test_ext().execute_with(|| {
        GovernanceMod::validate(GovernanceConfiguration {
            proposal_cost: 0,
            ..Default::default()
        })
        .expect_err("invalid proposal cost was applied");

        GovernanceMod::validate(GovernanceConfiguration {
            proposal_expiration: 0,
            ..Default::default()
        })
        .expect_err("invalid proposal cost was applied");

        GovernanceMod::validate(GovernanceConfiguration {
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
                governance_config,
                kappa,
                rho,
                subnet_immunity_period,
            } = global_params;

            GovernanceMod::add_global_params_proposal(
                get_origin(KEY),
                vec![b'0'; 64],
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
                governance_config.proposal_cost,
                governance_config.proposal_expiration,
                general_subnet_application_cost,
                kappa,
                rho,
                subnet_immunity_period,
            )
        };

        test(GlobalParams {
            governance_config: GovernanceConfiguration {
                proposal_cost: 0,
                ..Default::default()
            },
            ..SubspaceMod::global_params()
        })
        .expect_err("created proposal with invalid max name length");

        test(SubspaceMod::global_params())
            .expect("failed to create proposal with valid parameters");
    });
}

#[test]
fn global_custom_proposal_is_accepted_correctly() {
    new_test_ext().execute_with(|| {
        zero_min_burn();

        const FOR: u32 = 0;
        const AGAINST: u32 = 1;

        let key = 0;
        let origin = get_origin(key);

        register(FOR, 0, 0, to_nano(10));
        register(AGAINST, 0, 1, to_nano(5));

        config(1, 100);

        assert_ok!(GovernanceMod::do_add_global_custom_proposal(
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
        zero_min_burn();

        const FOR: u32 = 0;
        const AGAINST: u32 = 1;

        let origin = get_origin(0);

        register(FOR, 0, 0, to_nano(10));
        register(AGAINST, 0, 1, to_nano(5));
        register(AGAINST, 1, 0, to_nano(10));

        config(1, 100);

        assert_ok!(GovernanceMod::do_add_subnet_custom_proposal(
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
                stake_for: 20_000_000_000,
                stake_against: 5_000_000_000,
            }
        );
    });
}

#[test]
fn global_proposal_is_refused_correctly() {
    new_test_ext().execute_with(|| {
        zero_min_burn();

        const FOR: u32 = 0;
        const AGAINST: u32 = 1;

        let origin = get_origin(0);

        register(FOR, 0, 0, to_nano(5));
        register(AGAINST, 0, 1, to_nano(10));

        config(1, 100);

        assert_ok!(GovernanceMod::do_add_global_custom_proposal(
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
        zero_min_burn();

        const KEY: u32 = 0;

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
            mut governance_config,
            rho,
            kappa,
            subnet_immunity_period,
        } = SubspaceMod::global_params();

        governance_config.proposal_cost = 69_420;

        GovernanceMod::add_global_params_proposal(
            get_origin(KEY),
            vec![b'0'; 64],
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
            governance_config.proposal_cost,
            governance_config.proposal_expiration,
            general_subnet_application_cost,
            kappa,
            rho,
            subnet_immunity_period,
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
        zero_min_burn();

        const KEY: u32 = 0;

        register(KEY, 0, 0, to_nano(10));
        config(1, 100);

        SubnetChangeset::update(
            0,
            SubnetParams {
                governance_config: Default::default(),
                ..SubspaceMod::subnet_params(0)
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
            name,
            metadata,
            tempo,
            maximum_set_weight_calls_per_epoch,
            bonds_ma,
            module_burn_config,
            min_validator_stake,
            max_allowed_validators,
            mut governance_config,
            ..
        } = SubspaceMod::subnet_params(0);

        governance_config.vote_mode = VoteMode::Authority;

        GovernanceMod::add_subnet_params_proposal(
            get_origin(KEY),
            0,
            vec![b'0'; 64],
            founder,
            founder_share,
            name,
            metadata,
            immunity_period,
            incentive_ratio,
            max_allowed_uids,
            max_allowed_weights,
            min_allowed_weights,
            max_weight_age,
            tempo,
            maximum_set_weight_calls_per_epoch,
            governance_config.vote_mode,
            bonds_ma,
            module_burn_config,
            min_validator_stake,
            max_allowed_validators,
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
        zero_min_burn();

        const FOR: u32 = 0;
        const AGAINST: u32 = 1;
        const FOR_DELEGATED: u32 = 2;
        const AGAINST_DELEGATED: u32 = 3;

        let origin = get_origin(0);

        register(FOR, 0, 0, to_nano(5));
        delegate(FOR);
        register(AGAINST, 0, 1, to_nano(10));

        stake(FOR_DELEGATED, 0, to_nano(10));
        delegate(FOR_DELEGATED);
        stake(AGAINST_DELEGATED, 1, to_nano(3));
        delegate(AGAINST_DELEGATED);

        config(1, 100);

        assert_ok!(GovernanceMod::do_add_global_custom_proposal(
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
        zero_min_burn();

        const FOR: u32 = 0;
        const AGAINST: u32 = 1;
        const FOR_DELEGATED: u32 = 2;
        const AGAINST_DELEGATED: u32 = 3;
        const FOR_DELEGATED_WRONG: u32 = 4;
        const AGAINST_DELEGATED_WRONG: u32 = 5;

        let origin = get_origin(0);

        register(FOR, 0, 0, to_nano(5));
        register(FOR, 1, 0, to_nano(5));
        register(AGAINST, 0, 1, to_nano(10));
        register(AGAINST, 1, 1, to_nano(10));

        stake(FOR_DELEGATED, 0, to_nano(10));
        delegate(FOR_DELEGATED);
        stake(AGAINST_DELEGATED, 1, to_nano(3));
        delegate(AGAINST_DELEGATED);

        stake(FOR_DELEGATED_WRONG, 0, to_nano(10));
        delegate(FOR_DELEGATED_WRONG);
        stake(AGAINST_DELEGATED_WRONG, 1, to_nano(3));
        delegate(AGAINST_DELEGATED_WRONG);

        config(1, 100);

        assert_ok!(GovernanceMod::do_add_subnet_custom_proposal(
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
                stake_for: 30_000_000_000,
                stake_against: 26_000_000_000,
            }
        );
    });
}

#[test]
fn creates_treasury_transfer_proposal_and_transfers() {
    new_test_ext().execute_with(|| {
        zero_min_burn();

        let origin = get_origin(0);
        GovernanceMod::add_transfer_dao_treasury_proposal(
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

        GovernanceMod::add_transfer_dao_treasury_proposal(origin, vec![b'0'; 64], to_nano(5), 0)
            .expect("proposal should be created");
        vote(0, 0, true);

        step_block(100);

        assert_eq!(get_balance(DaoTreasuryAddress::<Test>::get()), to_nano(5));
        assert_eq!(get_balance(0), to_nano(8));
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
fn whitelist_executes_application_correctly() {
    new_test_ext().execute_with(|| {
        let key = 0;
        let adding_key = 1;
        let mut params = SubspaceMod::global_params();
        params.curator = key;
        assert_ok!(SubspaceMod::set_global_params(params));

        let proposal_cost = GeneralSubnetApplicationCost::<Test>::get();
        let data = "test".as_bytes().to_vec();

        add_balance(key, proposal_cost + 1);
        // first submit an application
        let balance_before = SubspaceMod::get_balance_u64(&key);

        assert_ok!(GovernanceMod::add_dao_application(
            get_origin(key),
            adding_key,
            data.clone(),
        ));

        let balance_after = SubspaceMod::get_balance_u64(&key);
        assert_eq!(balance_after, balance_before - proposal_cost);

        // Assert that the proposal is initially in the Pending status
        for (_, value) in CuratorApplications::<Test>::iter() {
            assert_eq!(value.status, ApplicationStatus::Pending);
            assert_eq!(value.user_id, adding_key);
            assert_eq!(value.data, data);
        }

        // add key to whitelist
        assert_ok!(GovernanceMod::add_to_whitelist(get_origin(key), adding_key,));

        let balance_after_accept = SubspaceMod::get_balance_u64(&key);

        assert_eq!(balance_after_accept, balance_before);

        // Assert that the proposal is now in the Accepted status
        for (_, value) in CuratorApplications::<Test>::iter() {
            assert_eq!(value.status, ApplicationStatus::Accepted);
            assert_eq!(value.user_id, adding_key);
            assert_eq!(value.data, data);
        }

        assert!(GovernanceMod::is_in_legit_whitelist(&adding_key));
    });
}

// ----------------
// Registration
// ----------------

#[test]
fn user_is_removed_from_whitelist() {
    new_test_ext().execute_with(|| {
        let whitelist_key = 0;
        let module_key = 1;
        Curator::<Test>::put(whitelist_key);

        let proposal_cost = Test::get_global_governance_configuration().proposal_cost;
        let data = "test".as_bytes().to_vec();

        // apply
        add_balance(whitelist_key, proposal_cost + 1);
        // first submit an application
        assert_ok!(GovernanceMod::add_dao_application(
            get_origin(whitelist_key),
            module_key,
            data.clone(),
        ));

        // Add the module_key to the whitelist
        assert_ok!(GovernanceMod::add_to_whitelist(
            get_origin(whitelist_key),
            module_key,
        ));
        assert!(GovernanceMod::is_in_legit_whitelist(&module_key));

        // Remove the module_key from the whitelist
        assert_ok!(GovernanceMod::remove_from_whitelist(
            get_origin(whitelist_key),
            module_key
        ));
        assert!(!GovernanceMod::is_in_legit_whitelist(&module_key));
    });
}

#[test]
fn whitelist_curator_must_be_a_valid_key() {
    new_test_ext().execute_with(|| {
        let whitelist_key = 0;
        let invalid_key = 1;
        let module_key = 2;
        Curator::<Test>::put(whitelist_key);

        // Try to add to whitelist with an invalid curator key
        assert_noop!(
            GovernanceMod::add_to_whitelist(get_origin(invalid_key), module_key),
            Error::<Test>::NotCurator
        );
        assert!(!GovernanceMod::is_in_legit_whitelist(&module_key));
    });
}
