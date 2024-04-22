use crate::mock::*;
use frame_support::assert_ok;
use pallet_subspace::yuma::{AccountKey, EmissionMap, ModuleKey, YumaCalc};
use sp_core::U256;
use std::collections::BTreeMap;
mod mock;

mod utils {
    use pallet_subspace::{Consensus, Dividends, Emission, Incentive, Rank, Trust};

    use crate::Test;

    pub fn get_rank_for_uid(netuid: u16, uid: u16) -> u16 {
        Rank::<Test>::get(netuid).get(uid as usize).copied().unwrap_or_default()
    }

    pub fn get_trust_for_uid(netuid: u16, uid: u16) -> u16 {
        Trust::<Test>::get(netuid).get(uid as usize).copied().unwrap_or_default()
    }

    pub fn get_consensus_for_uid(netuid: u16, uid: u16) -> u16 {
        Consensus::<Test>::get(netuid).get(uid as usize).copied().unwrap_or_default()
    }

    pub fn get_incentive_for_uid(netuid: u16, uid: u16) -> u16 {
        Incentive::<Test>::get(netuid).get(uid as usize).copied().unwrap_or_default()
    }

    pub fn get_dividends_for_uid(netuid: u16, uid: u16) -> u16 {
        Dividends::<Test>::get(netuid).get(uid as usize).copied().unwrap_or_default()
    }

    pub fn get_emission_for_uid(netuid: u16, uid: u16) -> u64 {
        Emission::<Test>::get(netuid).get(uid as usize).copied().unwrap_or_default()
    }
}

const ONE: u64 = to_nano(1);

// We are off by one,
// due to inactive / active calulation on yuma, which is 100% correct.
#[test]
fn test_1_graph() {
    new_test_ext().execute_with(|| {
        SubspaceModule::set_unit_emission(23148148148);
        SubspaceModule::set_min_burn(0);

        // Register general subnet
        assert_ok!(register_module(0, 10.into(), 1));

        log::info!("test_1_graph:");
        let netuid: u16 = 1;
        let key = U256::from(0);
        let uid: u16 = 0;
        let stake_amount: u64 = to_nano(100);

        assert_ok!(register_module(netuid, key, stake_amount));
        update_params!(netuid => {
            max_allowed_uids: 2
        });

        assert_ok!(register_module(netuid, key + 1, 1));
        assert_eq!(SubspaceModule::get_subnet_n(netuid), 2);

        run_to_block(1); // run to next block to ensure weights are set on nodes after their registration block

        assert_ok!(SubspaceModule::set_weights(
            RuntimeOrigin::signed(U256::from(1)),
            netuid,
            vec![uid],
            vec![u16::MAX],
        ));

        let emissions = YumaCalc::<Test>::new(netuid, ONE).run();
        let offset = 1;

        assert_eq!(
            emissions.unwrap(),
            [(ModuleKey(key), [(AccountKey(key), ONE - offset)].into())].into()
        );

        let new_stake_amount = stake_amount + ONE;

        assert_eq!(
            SubspaceModule::get_total_stake_to(netuid, &key),
            new_stake_amount - offset
        );
        assert_eq!(utils::get_rank_for_uid(netuid, uid), 0);
        assert_eq!(utils::get_trust_for_uid(netuid, uid), 0);
        assert_eq!(utils::get_consensus_for_uid(netuid, uid), 0);
        assert_eq!(utils::get_incentive_for_uid(netuid, uid), 0);
        assert_eq!(utils::get_dividends_for_uid(netuid, uid), 0);
        assert_eq!(utils::get_emission_for_uid(netuid, uid), ONE - offset);
    });
}

#[test]
fn test_10_graph() {
    /// Function for adding a nodes to the graph.
    fn add_node(netuid: u16, key: U256, uid: u16, stake_amount: u64) {
        log::info!(
            "+Add net:{:?} hotkey:{:?} uid:{:?} stake_amount: {:?} subn: {:?}",
            netuid,
            key,
            uid,
            stake_amount,
            SubspaceModule::get_subnet_n(netuid),
        );

        assert_ok!(register_module(netuid, key, stake_amount));
        assert_eq!(SubspaceModule::get_subnet_n(netuid) - 1, uid);
    }

    new_test_ext().execute_with(|| {
        SubspaceModule::set_unit_emission(23148148148);
        SubspaceModule::set_min_burn(0);
        SubspaceModule::set_max_registrations_per_block(1000);
        // Register general subnet
        assert_ok!(register_module(0, 10_000.into(), 1));

        log::info!("test_10_graph");

        // Build the graph with 10 items
        // each with 1 stake and self weights.
        let n: usize = 10;
        let netuid: u16 = 1;
        let stake_amount_per_node = ONE;

        for i in 0..n {
            add_node(netuid, U256::from(i), i as u16, stake_amount_per_node)
        }

        update_params!(netuid => {
            max_allowed_uids: n as u16 + 1
        });

        assert_ok!(register_module(netuid, U256::from(n + 1), 1));
        assert_eq!(SubspaceModule::get_subnet_n(netuid), 11);

        run_to_block(1); // run to next block to ensure weights are set on nodes after their registration block

        for i in 0..n {
            assert_ok!(SubspaceModule::set_weights(
                RuntimeOrigin::signed(U256::from(n + 1)),
                netuid,
                vec![i as u16],
                vec![u16::MAX],
            ));
        }

        let emissions = YumaCalc::<Test>::new(netuid, ONE).run();
        let mut expected: EmissionMap<Test> = BTreeMap::new();

        // Check return values.
        let emission_per_node = ONE / n as u64;
        for i in 0..n as u16 {
            assert_eq!(
                from_nano(SubspaceModule::get_total_stake_to(netuid, &(U256::from(i)))),
                from_nano(to_nano(1) + emission_per_node)
            );

            assert_eq!(utils::get_rank_for_uid(netuid, i), 0);
            assert_eq!(utils::get_trust_for_uid(netuid, i), 0);
            assert_eq!(utils::get_consensus_for_uid(netuid, i), 0);
            assert_eq!(utils::get_incentive_for_uid(netuid, i), 0);
            assert_eq!(utils::get_dividends_for_uid(netuid, i), 0);
            assert_eq!(utils::get_emission_for_uid(netuid, i), 99999999);

            expected
                .entry(ModuleKey(i.into()))
                .or_default()
                .insert(AccountKey(i.into()), 99999999);
        }

        assert_eq!(emissions.unwrap(), expected);
    });
}

// Testing weight expiration, on subnets running yuma
#[test]
fn yuma_weights_older_than_max_age_are_discarded() {
    new_test_ext().execute_with(|| {
        const MAX_WEIGHT_AGE: u64 = 300;
        const SUBNET_TEMPO: u16 = 100;
        // Register the general subnet.
        let netuid: u16 = 0;
        let key = U256::from(0);
        let stake_amount: u64 = to_nano(1_000);

        assert_ok!(register_module(netuid, key, stake_amount));

        // Register the yuma subnet.
        let yuma_netuid: u16 = 1;
        let yuma_validator_key = U256::from(1);
        let yuma_miner_key = U256::from(2);
        let yuma_stake_amount: u64 = to_nano(10_000);
        let yuma_miner_amount = to_nano(1_000);

        // This will act as an validator.
        assert_ok!(register_module(
            yuma_netuid,
            yuma_validator_key,
            yuma_stake_amount
        ));
        // This will act as an miner.
        assert_ok!(register_module(
            yuma_netuid,
            yuma_miner_key,
            yuma_miner_amount
        ));

        step_block(1);

        // Set the max weight age to 300 blocks
        update_params!(yuma_netuid => {
            tempo: SUBNET_TEMPO,
            max_weight_age: MAX_WEIGHT_AGE
        });

        let miner_uid = SubspaceModule::get_uid_for_key(yuma_netuid, &yuma_miner_key);
        let validator_uid = SubspaceModule::get_uid_for_key(yuma_netuid, &yuma_validator_key);
        let uid = [miner_uid].to_vec();
        let weight = [1].to_vec();

        // set the weights
        assert_ok!(SubspaceModule::do_set_weights(
            get_origin(yuma_validator_key),
            yuma_netuid,
            uid,
            weight
        ));

        step_block(100);

        // Make sure we have incentive and dividends
        let miner_incentive = SubspaceModule::get_incentive_for_uid(yuma_netuid, miner_uid);
        let miner_dividends = SubspaceModule::get_dividends_for_uid(yuma_netuid, miner_uid);
        let validator_incentive = SubspaceModule::get_incentive_for_uid(yuma_netuid, validator_uid);
        let validator_dividends = SubspaceModule::get_dividends_for_uid(yuma_netuid, validator_uid);

        assert!(miner_incentive > 0);
        assert_eq!(miner_dividends, 0);
        assert!(validator_dividends > 0);
        assert_eq!(validator_incentive, 0);

        // now go pass the max weight age
        step_block(MAX_WEIGHT_AGE as u16);

        // Make sure we have no incentive and dividends
        let miner_incentive = SubspaceModule::get_incentive_for_uid(yuma_netuid, miner_uid);
        let miner_dividends = SubspaceModule::get_dividends_for_uid(yuma_netuid, miner_uid);
        let validator_incentive = SubspaceModule::get_incentive_for_uid(yuma_netuid, validator_uid);
        let validator_dividends = SubspaceModule::get_dividends_for_uid(yuma_netuid, validator_uid);

        assert_eq!(miner_incentive, 0);
        assert_eq!(miner_dividends, 0);
        assert_eq!(validator_dividends, 0);
        assert_eq!(validator_incentive, 0);

        // But make sure there are emissions

        let subnet_emission_sum = SubspaceModule::get_emissions(yuma_netuid).iter().sum::<u64>();
        assert!(subnet_emission_sum > 0);
    });
}
