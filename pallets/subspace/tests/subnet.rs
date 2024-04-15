mod mock;

use frame_support::assert_ok;
use log::info;
use mock::*;
use sp_core::U256;
use sp_runtime::Percent;
use sp_std::vec;

#[test]
fn test_add_subnets() {
    new_test_ext().execute_with(|| {
        let _tempo: u16 = 13;
        let stake_per_module: u64 = 1_000_000_000;
        let max_allowed_subnets: u16 = SubspaceModule::get_global_max_allowed_subnets();
        let mut expected_subnets = 0;
        let n = 20;
        let num_subnets: u16 = n;

        // make sure that the results won´t get affected by burn
        SubspaceModule::set_min_burn(0);

        for i in 0..num_subnets {
            assert_ok!(register_module(i, U256::from(i), stake_per_module));
            for j in 0..n {
                if j != i {
                    let n = SubspaceModule::get_subnet_n(i);
                    info!("registering module i:{} j:{} n:{}", i, j, n);
                    assert_ok!(register_module(i, U256::from(j), stake_per_module));
                }
            }
            expected_subnets += 1;
            if expected_subnets > max_allowed_subnets {
                expected_subnets = max_allowed_subnets;
            } else {
                assert_eq!(SubspaceModule::get_subnet_n(i), n);
            }
            assert_eq!(
                SubspaceModule::num_subnets(),
                expected_subnets,
                "number of subnets is not equal to expected subnets"
            );
        }

        for netuid in 0..num_subnets {
            let total_stake = SubspaceModule::get_total_subnet_stake(netuid);
            let total_balance = SubspaceModule::get_total_subnet_balance(netuid);
            let total_tokens_before = total_stake + total_balance;

            let keys = SubspaceModule::get_keys(netuid);

            info!("total stake {total_stake}");
            info!("total balance {total_balance}");
            info!("total tokens before {total_tokens_before}");

            assert_eq!(keys.len() as u16, n);
            assert!(SubspaceModule::check_subnet_storage(netuid));
            SubspaceModule::remove_subnet(netuid);
            assert_eq!(SubspaceModule::get_subnet_n(netuid), 0);
            assert!(SubspaceModule::check_subnet_storage(netuid));

            let total_tokens_after: u64 = keys.iter().map(SubspaceModule::get_balance_u64).sum();
            info!("total tokens after {}", total_tokens_after);

            assert_eq!(total_tokens_after, total_tokens_before);
            expected_subnets = expected_subnets.saturating_sub(1);
            assert_eq!(
                SubspaceModule::num_subnets(),
                expected_subnets,
                "number of subnets is not equal to expected subnets"
            );
        }
    });
}

#[allow(dead_code)]
fn test_set_single_temple(tempo: u16) {
    new_test_ext().execute_with(|| {
        // creates a subnet when you register a module
        let netuid: u16 = 0;
        let stake: u64 = 0;
        let key = U256::from(0);
        let _tempos: Vec<u16> = vec![2, 4];
        assert_ok!(register_module(netuid, key, stake));
        let mut params = SubspaceModule::subnet_params(netuid).clone();
        params.tempo = tempo;

        let _total_blocks = 100;
        let threshold = SubspaceModule::get_subnet_stake_threshold();
        let emission_per_block: u64 = SubspaceModule::calculate_network_emission(netuid, threshold);
        let mut total_stake: u64 = 0;
        let tempo = 5;
        let min_stake = 1_000_000_000;

        SubspaceModule::set_subnet_params(netuid, params.clone());

        let subnet_params = SubspaceModule::subnet_params(netuid);

        assert_eq!(subnet_params.tempo, tempo);
        assert_eq!(subnet_params.min_stake, min_stake);
        assert_eq!(subnet_params.max_allowed_uids, params.max_allowed_uids);
        assert_eq!(
            subnet_params.min_allowed_weights,
            params.min_allowed_weights
        );
        assert_eq!(
            subnet_params.max_allowed_weights,
            params.max_allowed_weights
        );
        assert_eq!(subnet_params.immunity_period, params.immunity_period);
        assert_eq!(subnet_params.name, params.name);

        let previous_total_stake: u64 = block_number() * emission_per_block;

        let n_epochs = 3;
        let n_steps = n_epochs * tempo;
        for _i in 0..n_steps {
            info!(
                "tempo {} block number: {} stake {} pending_emissiion {}",
                tempo,
                block_number(),
                SubspaceModule::get_total_subnet_stake(netuid),
                SubspaceModule::get_pending_emission(netuid)
            );
            step_block(1);
            // get_block_number() is a function in mock.rs
            let incentives: Vec<u16> = SubspaceModule::get_incentives(netuid);
            let dividends: Vec<u16> = SubspaceModule::get_dividends(netuid);
            let emissions: Vec<u64> = SubspaceModule::get_emissions(netuid);

            info!("emission {emissions:?}");
            info!("incentives {incentives:?}");
            info!("dividends {dividends:?}");

            let stake: u64 = SubspaceModule::get_stake_for_uid(netuid, 0);
            info!("stake {:?}", stake);
            total_stake = SubspaceModule::get_total_subnet_stake(netuid);
            info!("total stake {}", total_stake);
        }

        assert_eq!(
            total_stake,
            (tempo as u64) * emission_per_block * (n_epochs as u64) + previous_total_stake
        );
    });
}

#[test]
fn test_emission_ratio() {
    new_test_ext().execute_with(|| {
        let netuids: Vec<u16> = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9].to_vec();
        let stake_per_module: u64 = 1_000_000_000;
        let mut emissions_per_subnet: Vec<u64> = Vec::new();
        let max_delta: f64 = 1.0;
        let _n: u16 = 10;

        // make sure that the results won´t get affected by burn
        SubspaceModule::set_min_burn(0);

        for i in 0..netuids.len() {
            let _key = U256::from(netuids[i]);
            let netuid = netuids[i];
            register_n_modules(netuid, 1, stake_per_module);
            let threshold = SubspaceModule::get_subnet_stake_threshold();
            let subnet_emission: u64 =
                SubspaceModule::calculate_network_emission(netuid, threshold);
            emissions_per_subnet.push(subnet_emission);
            let _expected_emission_factor: f64 = 1.0 / (netuids.len() as f64);
            let emission_per_block = SubspaceModule::get_total_emission_per_block();
            let expected_emission: u64 = emission_per_block / (i as u64 + 1);

            let block = block_number();
            // magnitude of difference between expected and actual emission
            let delta = if subnet_emission > expected_emission {
                subnet_emission - expected_emission
            } else {
                expected_emission - subnet_emission
            } as f64;

            assert!(
                delta <= max_delta,
                "emission {} is too far from expected emission {} ",
                subnet_emission,
                expected_emission
            );
            assert!(block == 0, "block {} is not 0", block);
            info!("block {} subnet_emission {} ", block, subnet_emission);
        }
    });
}

#[test]
fn test_set_max_allowed_uids_growing() {
    new_test_ext().execute_with(|| {
        let netuid: u16 = 0;
        let stake: u64 = 1_000_000_000;
        let mut max_uids: u16 = 100;
        let extra_uids: u16 = 10;
        let rounds = 10;

        // make sure that the results won´t get affected by burn
        SubspaceModule::set_min_burn(0);

        assert_ok!(register_module(netuid, U256::from(0), stake));
        SubspaceModule::set_max_registrations_per_block(max_uids + extra_uids * rounds);
        for i in 1..max_uids {
            assert_ok!(register_module(netuid, U256::from(i), stake));
            assert_eq!(SubspaceModule::get_subnet_n(netuid), i + 1);
        }
        let mut n: u16 = SubspaceModule::get_subnet_n(netuid);
        let old_n: u16 = n;
        let mut _uids: Vec<u16>;
        assert_eq!(SubspaceModule::get_subnet_n(netuid), max_uids);
        for r in 1..rounds {
            // set max allowed uids to max_uids + extra_uids
            SubspaceModule::set_max_allowed_uids(netuid, max_uids + extra_uids * (r - 1));
            max_uids = SubspaceModule::get_max_allowed_uids(netuid);
            let new_n = old_n + extra_uids * (r - 1);
            // print the pruned uids
            for uid in old_n + extra_uids * (r - 1)..old_n + extra_uids * r {
                assert_ok!(register_module(netuid, U256::from(uid), stake));
            }

            // set max allowed uids to max_uids

            n = SubspaceModule::get_subnet_n(netuid);
            assert_eq!(n, new_n);

            let uids = SubspaceModule::get_uids(netuid);
            assert_eq!(uids.len() as u16, n);

            let keys = SubspaceModule::get_keys(netuid);
            assert_eq!(keys.len() as u16, n);

            let names = SubspaceModule::get_names(netuid);
            assert_eq!(names.len() as u16, n);

            let addresses = SubspaceModule::get_addresses(netuid);
            assert_eq!(addresses.len() as u16, n);
        }
    });
}

#[test]
fn test_set_max_allowed_uids_shrinking() {
    new_test_ext().execute_with(|| {
        let netuid: u16 = 0;
        let stake: u64 = 1_000_000_000;
        let max_uids: u16 = 100;
        let extra_uids: u16 = 20;

        // make sure that the results won´t get affected by burn
        SubspaceModule::set_min_burn(0);

        let mut n = SubspaceModule::get_subnet_n(netuid);
        info!("registering module {}", n);
        assert_ok!(register_module(netuid, U256::from(0), stake));
        SubspaceModule::set_max_allowed_uids(netuid, max_uids + extra_uids);
        SubspaceModule::set_max_registrations_per_block(max_uids + extra_uids);

        for i in 1..(max_uids + extra_uids) {
            let result = register_module(netuid, U256::from(i), stake);
            result.unwrap();
            n = SubspaceModule::get_subnet_n(netuid);
        }

        assert_eq!(n, max_uids + extra_uids);

        let keys = SubspaceModule::get_keys(netuid);

        let mut total_stake: u64 = SubspaceModule::get_total_subnet_stake(netuid);
        let mut expected_stake: u64 = n as u64 * stake;

        info!("total stake {total_stake}");
        info!("expected stake {expected_stake}");
        assert_eq!(total_stake, expected_stake);

        let _subnet = SubspaceModule::subnet_info(netuid);

        let mut params = SubspaceModule::subnet_params(netuid).clone();
        params.max_allowed_uids = max_uids;
        params.name = "test2".as_bytes().to_vec();
        let result = SubspaceModule::update_subnet(
            get_origin(keys[0]),
            netuid,
            params.founder,
            params.founder_share,
            params.immunity_period,
            params.incentive_ratio,
            params.max_allowed_uids,
            params.max_allowed_weights,
            params.max_stake,
            params.min_allowed_weights,
            params.max_weight_age,
            params.min_stake,
            params.name.clone(),
            params.tempo,
            params.trust_ratio,
            params.vote_mode,
        );
        let global_params = SubspaceModule::global_params();
        info!("global params {:?}", global_params);
        info!("subnet params {:?}", SubspaceModule::subnet_params(netuid));
        assert_ok!(result);
        let params = SubspaceModule::subnet_params(netuid);
        let n = SubspaceModule::get_subnet_n(netuid);
        assert_eq!(
            params.max_allowed_uids, max_uids,
            "max allowed uids is not equal to expected max allowed uids"
        );
        assert_eq!(
            params.max_allowed_uids, n,
            "min allowed weights is not equal to expected min allowed weights"
        );

        let stake_vector: Vec<u64> = SubspaceModule::get_stakes(netuid);
        let calc_stake: u64 = stake_vector.iter().sum();

        info!("calculated  stake {}", calc_stake);

        expected_stake = (max_uids) as u64 * stake;
        let _subnet_stake = SubspaceModule::get_total_subnet_stake(netuid);
        total_stake = SubspaceModule::total_stake();

        assert_eq!(total_stake, expected_stake);
    });
}

#[test]
fn test_set_max_allowed_modules() {
    new_test_ext().execute_with(|| {
        let netuid: u16 = 0;
        let stake: u64 = 1_000_000_000;
        let _max_uids: u16 = 2000;
        let _extra_uids: u16 = 20;
        let max_allowed_modules: u16 = 100;

        // make sure that the results won´t get affected by burn
        SubspaceModule::set_min_burn(0);

        SubspaceModule::set_max_allowed_modules(max_allowed_modules);
        // set max_total modules

        for i in 1..(2 * max_allowed_modules) {
            assert_ok!(register_module(netuid, U256::from(i), stake));
            let n = SubspaceModule::get_subnet_n(netuid);
            assert!(
                n <= max_allowed_modules,
                "subnet_n {:?} is not less than max_allowed_modules {:?}",
                n,
                max_allowed_modules
            );
        }
    })
}

#[test]
fn test_deregister_subnet_when_overflows_max_allowed_subnets() {
    new_test_ext().execute_with(|| {
        let extra = 1;
        let mut params = SubspaceModule::global_params();
        params.max_allowed_subnets = 3;
        SubspaceModule::set_global_params(params.clone());
        // make sure that the results won´t get affected by burn
        SubspaceModule::set_min_burn(0);

        assert_eq!(params.max_allowed_subnets, 3);

        let stakes: Vec<u64> = vec![
            2_000_000_000,
            6_000_000_000,
            3_000_000_000,
            4_000_000_000,
            9_000_000_000,
        ];

        for netuid in 0..params.max_allowed_subnets + extra {
            let stake: u64 = stakes[netuid as usize];
            assert_ok!(register_module(netuid, U256::from(netuid), stake));
        }

        assert_eq!(SubspaceModule::get_total_subnet_stake(1), stakes[1]);
        assert_eq!(SubspaceModule::get_total_subnet_stake(2), stakes[2]);
        assert_eq!(SubspaceModule::get_total_subnet_stake(0), stakes[3]);
        assert_eq!(SubspaceModule::num_subnets(), 3);
    });
}

#[test]
fn test_emission_distribution_novote() {
    // test if subnet emissions are distributed correctly, even without voting
    new_test_ext().execute_with(|| {
        let netuid_general: u16 = 0; // hold 50% of the networks stake
        let stake_general: u64 = to_nano(500_000);

        let netuid_yuma: u16 = 1; // holds 45% of the networks stake
        let stake_yuma: u64 = to_nano(450_000);

        let netuid_below_threshold: u16 = 2; // holds 5% of the networks stake
        let stake_below_threshold: u64 = to_nano(50_000);

        // making sure the unit emission are set correctly
        SubspaceModule::set_unit_emission(23148148148);
        SubspaceModule::set_min_burn(0);
        SubspaceModule::set_subnet_stake_threshold(Percent::from_percent(10));
        let blocks_in_day: u16 = 10_800;
        // this is aprox. the stake we expect at the end of the day with the above unit emission
        let expected_stake_change = to_nano(250_000);
        let expected_stake_change_general = (stake_general as f64
            / ((stake_general + stake_yuma) as f64)
            * expected_stake_change as f64) as u64;
        let expected_stake_change_yuma = (stake_yuma as f64 / ((stake_general + stake_yuma) as f64)
            * expected_stake_change as f64) as u64;
        let expected_stake_change_below = 0;
        let change_tolerance = to_nano(22) as i64; // we tolerate 22 token difference (due to rounding)

        dbg!(expected_stake_change_general);
        dbg!(expected_stake_change_yuma);
        dbg!(expected_stake_change_below);
        // first register the general subnet
        assert_ok!(register_module(
            netuid_general,
            U256::from(0),
            stake_general
        ));

        // then register the yuma subnet
        assert_ok!(register_module(netuid_yuma, U256::from(1), stake_yuma));

        // then register the below threshold subnet
        assert_ok!(register_module(
            netuid_below_threshold,
            U256::from(2),
            stake_below_threshold
        ));

        step_block(blocks_in_day);

        let general_netuid_stake =
            from_nano(SubspaceModule::get_total_subnet_stake(netuid_general));
        let yuma_netuid_stake = from_nano(SubspaceModule::get_total_subnet_stake(netuid_yuma));
        let below_threshold_netuid_stake = from_nano(SubspaceModule::get_total_subnet_stake(
            netuid_below_threshold,
        ));

        let general_netuid_stake = (general_netuid_stake as f64 / 100.0).round() * 100.0;
        let yuma_netuid_stake = (yuma_netuid_stake as f64 / 100.0).round() * 100.0;
        let below_threshold_netuid_stake =
            (below_threshold_netuid_stake as f64 / 100.0).round() * 100.0;

        dbg!(general_netuid_stake);
        dbg!(yuma_netuid_stake);
        dbg!(below_threshold_netuid_stake);

        let start_stake = stake_general + stake_yuma + stake_below_threshold;
        let end_day_stake = to_nano(
            (general_netuid_stake + yuma_netuid_stake + below_threshold_netuid_stake) as u64,
        );
        let stake_change = end_day_stake - start_stake;
        assert_eq!(stake_change, expected_stake_change);

        // Check the expected difference for the general subnet
        let general_stake_change = to_nano(general_netuid_stake as u64) - stake_general;
        dbg!(general_stake_change);
        dbg!(expected_stake_change_general);
        assert!(
            (general_stake_change as i64 - expected_stake_change_general as i64).abs()
                <= change_tolerance
        );

        // Check the expected difference for the yuma subnet
        let yuma_stake_change = to_nano(yuma_netuid_stake as u64) - stake_yuma;
        assert!(
            (yuma_stake_change as i64 - expected_stake_change_yuma as i64).abs()
                <= change_tolerance
        );

        // Check the expected difference for the below threshold subnet
        let below_stake_change =
            to_nano(below_threshold_netuid_stake as u64) - stake_below_threshold;
        assert_eq!(below_stake_change, expected_stake_change_below);
    });
}

#[test]
fn test_yuma_self_vote() {
    new_test_ext().execute_with(|| {
        let netuid_general: u16 = 0;
        let netuid_yuma: u16 = 1;
        let netuid_below_threshold: u16 = 2;
        // this much stake is on the general subnet 0
        let stake_general: u64 = to_nano(500_000);
        // this is how much the first voter on yuma consensus has
        let stake_yuma_voter: u64 = to_nano(440_000);
        // miner
        let stake_yuma_miner: u64 = to_nano(10_000);
        // this is how much the self voter on yuma consensus has
        let stake_yuma_voter_self: u64 = to_nano(400_000);
        let stake_yuma_miner_self: u64 = to_nano(2_000);
        // below threshold subnet, emission distribution should not even start
        let stake_below_threshold: u64 = to_nano(50_000);
        let blocks_in_day: u16 = 10_800;
        let validator_key = U256::from(1);
        let miner_key = U256::from(2);
        let validator_self_key = U256::from(3);
        let miner_self_key = U256::from(4);

        // making sure the unit emission are set correctly
        SubspaceModule::set_unit_emission(23148148148);
        SubspaceModule::set_min_burn(0);

        assert_ok!(register_module(
            netuid_general,
            U256::from(0),
            stake_general
        ));
        assert_ok!(register_module(
            netuid_yuma,
            validator_key,
            stake_yuma_voter
        ));
        SubspaceModule::set_max_weight_age(netuid_yuma, (blocks_in_day + 1) as u64);
        assert_ok!(register_module(netuid_yuma, miner_key, stake_yuma_miner));
        assert_ok!(register_module(
            netuid_yuma,
            validator_self_key,
            stake_yuma_voter_self
        ));
        assert_ok!(register_module(
            netuid_yuma,
            miner_self_key,
            stake_yuma_miner_self
        ));
        step_block(1);
        set_weights(
            netuid_yuma,
            validator_key,
            [SubspaceModule::get_uid_for_key(netuid_yuma, &miner_key)].to_vec(),
            [1].to_vec(),
        );
        set_weights(
            netuid_yuma,
            validator_self_key,
            [SubspaceModule::get_uid_for_key(
                netuid_yuma,
                &miner_self_key,
            )]
            .to_vec(),
            [1].to_vec(),
        );
        assert_ok!(register_module(
            netuid_below_threshold,
            U256::from(2),
            stake_below_threshold
        ));

        // Calculate the expected daily change in total stake
        let expected_stake_change = to_nano(250_000);

        step_block(blocks_in_day);

        let stake_validator = SubspaceModule::get_stake(netuid_yuma, &validator_key);
        let stake_miner = SubspaceModule::get_stake(netuid_yuma, &miner_key);
        let stake_validator_self_vote = SubspaceModule::get_stake(netuid_yuma, &validator_self_key);
        let stake_miner_self_vote = SubspaceModule::get_stake(netuid_yuma, &miner_self_key);

        assert!(stake_yuma_voter < stake_validator);
        assert!(stake_yuma_miner < stake_miner);
        assert_eq!(stake_yuma_miner_self, stake_miner_self_vote);
        assert_eq!(stake_yuma_voter_self, stake_validator_self_vote);

        let general_netuid_stake = SubspaceModule::get_total_subnet_stake(netuid_general);
        let yuma_netuid_stake = SubspaceModule::get_total_subnet_stake(netuid_yuma);
        let below_threshold_netuid_stake =
            SubspaceModule::get_total_subnet_stake(netuid_below_threshold);

        assert!(stake_general < general_netuid_stake);
        assert!(stake_yuma_voter < yuma_netuid_stake);
        assert_eq!(stake_below_threshold, below_threshold_netuid_stake);
        // Check the actual daily change in total stake
        let start_stake = stake_below_threshold
            + stake_general
            + stake_yuma_voter
            + stake_yuma_voter_self
            + stake_yuma_miner
            + stake_yuma_miner_self;
        let end_day_stake = general_netuid_stake + yuma_netuid_stake + below_threshold_netuid_stake;
        let actual_stake_change = round_first_five(end_day_stake - start_stake);

        assert_eq!(actual_stake_change, expected_stake_change);
    });
}

#[test]
fn test_emission_activation() {
    new_test_ext().execute_with(|| {
        // Define the subnet stakes
        let subnet_stakes = [
            ("Subnet A", to_nano(10), true),
            ("Subnet B", to_nano(4), false), // This one should not activate
            ("Subnet C", to_nano(86), true),
        ];

        // Set the stake threshold and minimum burn
        SubspaceModule::set_subnet_stake_threshold(Percent::from_percent(5));
        SubspaceModule::set_min_burn(0);

        // Register the subnets
        for (i, (name, stake, _)) in subnet_stakes.iter().enumerate() {
            assert_ok!(register_module(i as u16, U256::from(i as u64), *stake));
            info!("Registered {name} with stake: {stake}");
        }

        step_block(1_000);

        // Check if subnet rewards have increased, but Subnet B should not have activated
        for (i, (name, initial_stake, should_activate)) in subnet_stakes.iter().enumerate() {
            let current_stake = SubspaceModule::get_total_subnet_stake(i as u16);
            if *should_activate {
                assert!(
                    current_stake > *initial_stake,
                    "{name} should have activated and increased its stake"
                );
            } else {
                assert_eq!(
                    current_stake, *initial_stake,
                    "{name} should not have activated"
                );
            }
            info!("{name} current stake: {current_stake}");
        }
    });
}
