use crate::mock::*;
use pallet_subnet_emission::{
    consensus::util::{
        consensus::EmissionMap,
        params::{AccountKey, ModuleKey},
    },
    types::SubnetDecryptionInfo,
    BannedDecryptionNodes, Weights,
};

use pallet_subspace::{Active, Consensus, Founder, PruningScores, Rank, Trust, ValidatorTrust};
use parity_scale_codec::Encode;
use rand::rngs::OsRng;
use rsa::{traits::PublicKeyParts, RsaPrivateKey};
use sp_runtime::Percent;
use std::collections::BTreeMap;

use frame_support::{assert_ok, traits::Currency};
use log::info;
use pallet_governance::DaoTreasuryAddress;
use pallet_subnet_emission::{
    consensus::{util::params::ConsensusParams, yuma::YumaEpoch},
    PendingEmission, SubnetConsensusType, SubnetEmission, UnitEmission,
};

use pallet_subnet_emission_api::SubnetConsensus;
use pallet_subspace::{
    Dividends, Emission, FloorFounderShare, FounderShare, ImmunityPeriod, Incentive,
    MaxAllowedModules, MaxAllowedSubnets, MaxAllowedValidators, MaxRegistrationsPerBlock,
    MaxWeightAge, MinValidatorStake, MinimumAllowedStake, Pallet as SubspaceMod, RegistrationBlock,
    SubnetImmunityPeriod, SubnetNames, Tempo, ValidatorPermits, N,
};

#[test]
fn test_dividends_same_stake() {
    new_test_ext().execute_with(|| {
        zero_min_validator_stake();
        let netuid: u16 = 1;
        let n: u16 = 10;
        let stake_per_module: u64 = 10_000;

        // Setup Rootnet
        assert_ok!(register_named_subnet(u32::MAX, 0, "Rootnet"));
        SubnetConsensusType::<Test>::insert(0, SubnetConsensus::Root);
        assert_ok!(register_root_validator(u32::MAX, stake_per_module));

        // Disable limitations
        zero_min_burn();
        MaxRegistrationsPerBlock::<Test>::set(1000);

        // SETUP NETWORK
        register_n_modules(netuid, n, stake_per_module, false);
        // Make sure the consensus does not think we are deregistered
        step_block(1);
        SubnetConsensusType::<Test>::insert(netuid, SubnetConsensus::Linear);
        MaxWeightAge::<Test>::insert(netuid, 20_000);
        MinValidatorStake::<Test>::insert(netuid, 0);

        // Set rootnet weight
        set_weights(0, u32::MAX, vec![netuid], vec![1]);

        let keys = get_keys(netuid);

        // do a list of ones for weights
        let weight_uids: Vec<u16> = [2, 3].to_vec();
        // do a list of ones for weights
        let weight_values: Vec<u16> = [2, 1].to_vec();

        set_weights(netuid, keys[0], weight_uids.clone(), weight_values.clone());
        set_weights(netuid, keys[1], weight_uids.clone(), weight_values.clone());

        let stakes_before: Vec<u64> = get_stakes(netuid);
        step_epoch(netuid);
        let incentives: Vec<u16> = Incentive::<Test>::get(netuid);
        let dividends: Vec<u16> = Dividends::<Test>::get(netuid);
        let emissions: Vec<u64> = Emission::<Test>::get(netuid);
        let stakes: Vec<u64> = get_stakes(netuid);

        // evaluate votees
        assert!(incentives[2] > 0);
        assert_eq!(dividends[2], dividends[3]);
        let delta: u64 = 500;
        assert!((incentives[2] as u64) > (weight_values[0] as u64 * incentives[3] as u64) - delta);
        assert!((incentives[2] as u64) < (weight_values[0] as u64 * incentives[3] as u64) + delta);

        assert!(emissions[2] > (weight_values[0] as u64 * emissions[3]) - delta);
        assert!(emissions[2] < (weight_values[0] as u64 * emissions[3]) + delta);

        // evaluate voters
        assert!(
            dividends[0] == dividends[1],
            "dividends[0]: {} != dividends[1]: {}",
            dividends[0],
            dividends[1]
        );
        assert!(
            dividends[0] == dividends[1],
            "dividends[0]: {} != dividends[1]: {}",
            dividends[0],
            dividends[1]
        );

        assert_eq!(incentives[0], incentives[1]);
        assert_eq!(dividends[2], dividends[3]);

        info!("emissions: {emissions:?}");

        for (uid, emission) in emissions.iter().enumerate() {
            if emission == &0 {
                continue;
            }
            let stake: u64 = stakes[uid];
            let stake_before: u64 = stakes_before[uid];
            let stake_difference: u64 = stake - stake_before;
            let expected_stake_difference: u64 = emissions[uid];
            let error_delta: u64 = (emissions[uid] as f64 * 0.001) as u64;

            assert!(
                stake_difference < expected_stake_difference + error_delta
                    && stake_difference > expected_stake_difference - error_delta,
                "stake_difference: {} != expected_stake_difference: {}",
                stake_difference,
                expected_stake_difference
            );
        }
    });
}

#[test]
fn test_dividends_diff_stake() {
    new_test_ext().execute_with(|| {
        zero_min_validator_stake();
        let netuid: u16 = 1;
        let n: u16 = 10;
        let stake_per_module: u64 = 10_000;

        // Setup Rootnet
        assert_ok!(register_named_subnet(u32::MAX, 0, "Rootnet"));
        SubnetConsensusType::<Test>::insert(0, SubnetConsensus::Root);
        assert_ok!(register_root_validator(u32::MAX, stake_per_module));

        // Disable limitations
        zero_min_burn();
        MaxRegistrationsPerBlock::<Test>::set(1000);

        // SETUP NETWORK
        for i in 0..n {
            let mut stake = stake_per_module;
            if i == 0 {
                stake = 2 * stake_per_module
            }
            let key = i as u32;
            assert_ok!(register_module(netuid, key, stake, false));
        }
        SubnetConsensusType::<Test>::insert(netuid, SubnetConsensus::Linear);

        // Set rootnet weight
        set_weights(0, u32::MAX, vec![netuid], vec![1]);

        let keys = get_keys(netuid);

        // Make sure the consensus does not think we are deregistered
        step_block(1);

        // do a list of ones for weights
        let weight_uids: Vec<u16> = [2, 3].to_vec();
        // do a list of ones for weights
        let weight_values: Vec<u16> = [1, 1].to_vec();
        set_weights(netuid, keys[0], weight_uids.clone(), weight_values.clone());
        set_weights(netuid, keys[1], weight_uids.clone(), weight_values.clone());

        let stakes_before: Vec<u64> = get_stakes(netuid);
        step_epoch(netuid);
        let incentives: Vec<u16> = Incentive::<Test>::get(netuid);
        let dividends: Vec<u16> = Dividends::<Test>::get(netuid);
        let emissions: Vec<u64> = Emission::<Test>::get(netuid);
        let stakes: Vec<u64> = get_stakes(netuid);

        // evaluate votees
        assert!(incentives[2] > 0);
        assert_eq!(dividends[2], dividends[3]);
        let delta: u64 = 100;
        assert!((incentives[2] as u64) > (weight_values[0] as u64 * incentives[3] as u64) - delta);
        assert!((incentives[2] as u64) < (weight_values[0] as u64 * incentives[3] as u64) + delta);

        assert!(emissions[2] > (weight_values[0] as u64 * emissions[3]) - delta);
        assert!(emissions[2] < (weight_values[0] as u64 * emissions[3]) + delta);

        // evaluate voters
        let delta: u64 = 100;
        assert!((dividends[0] as u64) > (dividends[1] as u64 * 2) - delta);
        assert!((dividends[0] as u64) < (dividends[1] as u64 * 2) + delta);

        assert_eq!(incentives[0], incentives[1]);
        assert_eq!(dividends[2], dividends[3]);

        info!("emissions: {emissions:?}");

        for (uid, emission) in emissions.iter().enumerate() {
            if emission == &0 {
                continue;
            }
            let stake: u64 = stakes[uid];
            let stake_before: u64 = stakes_before[uid];
            let stake_difference: u64 = stake - stake_before;
            let expected_stake_difference: u64 = emissions[uid];
            let error_delta: u64 = (emissions[uid] as f64 * 0.001) as u64;

            assert!(
                stake_difference < expected_stake_difference + error_delta
                    && stake_difference > expected_stake_difference - error_delta,
                "stake_difference: {} != expected_stake_difference: {}",
                stake_difference,
                expected_stake_difference
            );
        }
    });
}

#[test]
fn test_pruning() {
    new_test_ext().execute_with(|| {
        zero_min_validator_stake();
        // Initialize test environment
        zero_min_burn();
        let netuid: u16 = 1;
        let n: u16 = 100;
        let stake_per_module: u64 = to_nano(10_000);
        let tempo: u16 = 100;

        // Setup Rootnet
        assert_ok!(register_named_subnet(u32::MAX, 0, "Rootnet"));
        SubnetConsensusType::<Test>::insert(0, SubnetConsensus::Root);
        assert_ok!(register_root_validator(u32::MAX, stake_per_module));
        MaxRegistrationsPerBlock::<Test>::set(1000);

        // Setup subnet
        register_n_modules(netuid, n, stake_per_module, false);
        MaxAllowedModules::<Test>::put(n + 2);
        Tempo::<Test>::set(netuid, tempo);
        ImmunityPeriod::<Test>::insert(netuid, 0);

        // Register validator and set consensus type
        let voter_idx = u32::MAX;
        assert_ok!(register_module(netuid, voter_idx, stake_per_module, false));
        SubnetConsensusType::<Test>::insert(netuid, SubnetConsensus::Yuma);

        // Set rootnet weight
        set_weights(0, u32::MAX, vec![netuid], vec![1]);

        // Prepare weights
        let weight_uids: Vec<u16> = (0..n).collect();
        let mut weight_values: Vec<u16> = weight_uids
            .iter()
            .enumerate()
            .map(|(i, _)| (weight_uids.len() - i) as u16)
            .collect();

        // Set prune_uid to the last UID and set its weight to 0
        let prune_uid: u16 = *weight_uids.last().unwrap();
        if let Some(prune_idx) = weight_uids.iter().position(|&uid| uid == prune_uid) {
            weight_values[prune_idx] = 0;
        }

        // Step block so yuma does not think modules are deregistered, and set weights
        step_block(1);
        set_weights(
            netuid,
            voter_idx,
            weight_uids.clone(),
            weight_values.clone(),
        );
        step_block(tempo);

        // Debug emission and lowest priority UID
        let lowest_priority_uid: u16 = SubspaceMod::<Test>::get_lowest_uid(netuid, false).unwrap();
        assert_eq!(lowest_priority_uid, prune_uid);

        // Register new module
        let new_key = n as u32 + 1;
        let prune_key = SubspaceMod::<Test>::get_key_for_uid(netuid, prune_uid).unwrap();
        assert_ok!(register_module(netuid, new_key, stake_per_module, false));

        // Assert new module is registered
        let is_registered: bool = SubspaceMod::<Test>::key_registered(netuid, &new_key);
        assert!(is_registered);

        // Assert total number of modules
        let n_assert = n + 1;
        assert_eq!(
            N::<Test>::get(netuid),
            n_assert,
            "N::<Test>::get(netuid): {} != n: {}",
            N::<Test>::get(netuid),
            n_assert
        );

        // Assert pruned module is no longer registered
        let is_prune_registered: bool = SubspaceMod::<Test>::key_registered(netuid, &prune_key);
        assert!(!is_prune_registered);

        // Now test register a new subnet, with 2 modules, both 0 emission,
        // the oldest will be the lowest_uid
        let new_netuid: u16 = 2;
        // Restart the module limit
        MaxAllowedModules::<Test>::put(1000);
        assert_ok!(register_module(
            new_netuid,
            u32::MAX,
            stake_per_module,
            false
        ));
        assert_ok!(register_module(
            new_netuid,
            u32::MAX - 1,
            stake_per_module,
            false
        ));

        // Temper with the registration blocks
        RegistrationBlock::<Test>::insert(new_netuid, 0, 1);
        RegistrationBlock::<Test>::insert(new_netuid, 1, 0);

        assert_eq!(
            SubspaceMod::<Test>::get_emission_for(new_netuid)
                .get(0)
                .copied()
                .unwrap_or_default(),
            0
        );
        assert_eq!(
            SubspaceMod::<Test>::get_emission_for(new_netuid)
                .get(1)
                .copied()
                .unwrap_or_default(),
            0
        );
        assert_eq!(
            SubspaceMod::<Test>::get_lowest_uid(new_netuid, false),
            Some(1)
        );
    });
}

#[test]
fn test_lowest_priority_mechanism() {
    new_test_ext().execute_with(|| {
        zero_min_validator_stake();
        let netuid: u16 = 1;
        let n: u16 = 100;
        let stake_per_module: u64 = 10_000;
        let tempo: u16 = 100;

        // Setup Rootnet
        assert_ok!(register_named_subnet(u32::MAX, 0, "Rootnet"));
        SubnetConsensusType::<Test>::insert(0, SubnetConsensus::Root);
        assert_ok!(register_root_validator(u32::MAX, stake_per_module));

        // Disable limitations
        zero_min_burn();
        MaxRegistrationsPerBlock::<Test>::set(1000);

        // SETUP NETWORK
        register_n_modules(netuid, n, stake_per_module, false);
        step_block(1);
        SubnetConsensusType::<Test>::insert(netuid, SubnetConsensus::Linear);

        // Set rootnet weight
        set_weights(0, u32::MAX, vec![netuid], vec![1]);

        let keys = get_keys(netuid);
        let voter_idx = 0;

        // Create a list of UIDs excluding the voter_idx
        let weight_uids: Vec<u16> = (0..n).filter(|&x| x != voter_idx).collect();

        // Create a list of ones for weights, excluding the voter_idx
        let mut weight_values: Vec<u16> = weight_uids.iter().map(|_x| 1_u16).collect();

        let prune_uid: u16 = n - 1;

        // Check if the prune_uid is still valid after excluding the voter_idx
        if prune_uid != voter_idx {
            // Find the index of prune_uid in the updated weight_uids vector
            if let Some(prune_idx) = weight_uids.iter().position(|&uid| uid == prune_uid) {
                weight_values[prune_idx] = 0;
            }
        }

        set_weights(
            netuid,
            keys[voter_idx as usize],
            weight_uids.clone(),
            weight_values.clone(),
        );
        step_block(tempo);
        let incentives: Vec<u16> = Incentive::<Test>::get(netuid);
        let dividends: Vec<u16> = Dividends::<Test>::get(netuid);
        let emissions: Vec<u64> = Emission::<Test>::get(netuid);

        assert_eq!(emissions[prune_uid as usize], 0);
        assert_eq!(incentives[prune_uid as usize], 0);
        assert_eq!(dividends[prune_uid as usize], 0);

        let lowest_priority_uid: u16 =
            SubspaceMod::<Test>::get_lowest_uid(netuid, false).unwrap_or(0);
        info!("lowest_priority_uid: {lowest_priority_uid}");
        info!("prune_uid: {prune_uid}");
        info!("emissions: {emissions:?}");
        info!("lowest_priority_uid: {lowest_priority_uid:?}");
        info!("dividends: {dividends:?}");
        info!("incentives: {incentives:?}");
        assert!(lowest_priority_uid == prune_uid);
    });
}

#[test]
fn calculates_blocks_until_epoch() {
    new_test_ext().execute_with(|| {
        let blocks_until_next_epoch = |netuid, tempo, block_number| {
            Tempo::<Test>::set(netuid, tempo);
            SubspaceMod::<Test>::blocks_until_next_epoch(netuid, block_number)
        };

        // Check tempo = 0 block = * netuid = *
        assert_eq!(blocks_until_next_epoch(0, 0, 0), u64::MAX);

        // Check tempo = 1 block = * netuid = *
        assert_eq!(blocks_until_next_epoch(0, 1, 0), 0);
        assert_eq!(blocks_until_next_epoch(1, 1, 0), 0);
        assert_eq!(blocks_until_next_epoch(0, 1, 1), 0);
        assert_eq!(blocks_until_next_epoch(1, 2, 1), 0);
        assert_eq!(blocks_until_next_epoch(0, 4, 3), 3);
        assert_eq!(blocks_until_next_epoch(10, 5, 2), 2);

        // Check general case.
        for netuid in 0..30_u16 {
            for block in 0..30_u64 {
                for tempo in 1..30_u16 {
                    assert_eq!(
                        blocks_until_next_epoch(netuid, tempo, block),
                        (block + netuid as u64) % (tempo as u64)
                    );
                }
            }
        }
    });
}

#[test]
fn test_incentives() {
    new_test_ext().execute_with(|| {
        zero_min_validator_stake();
        let netuid: u16 = 0;
        let n: u16 = 10;
        let stake_per_module: u64 = 10_000;

        // make sure that the results won´t get affected by burn
        zero_min_burn();
        MinimumAllowedStake::<Test>::set(0);

        // SETUP NETWORK
        register_n_modules(netuid, n, stake_per_module, false);
        step_block(1);

        // Test perform under linear consensus network.
        SubnetConsensusType::<Test>::insert(netuid, SubnetConsensus::Linear);
        let mut params = SubspaceMod::<Test>::subnet_params(netuid);
        params.min_allowed_weights = 0;
        params.max_allowed_weights = n;
        params.tempo = 100;

        let keys = get_keys(netuid);
        let weight_uids: Vec<u16> = [1, 2].to_vec();
        let weight_values: Vec<u16> = [1, 1].to_vec();

        set_weights(netuid, keys[0], weight_uids.clone(), weight_values.clone());
        // Make sure network will run the consensus ditribution
        PendingEmission::<Test>::insert(netuid, 90000000000000);
        step_block(params.tempo);

        let incentives: Vec<u16> = Incentive::<Test>::get(netuid);
        let emissions: Vec<u64> = Emission::<Test>::get(netuid);

        // evaluate votes
        assert!(incentives[1] > 0);
        assert!(incentives[1] == incentives[2]);
        assert!(emissions[1] == emissions[2]);

        // do a list of ones for weights
        let weight_values: Vec<u16> = [1, 2].to_vec();

        set_weights(netuid, keys[0], weight_uids.clone(), weight_values.clone());
        set_weights(netuid, keys[9], weight_uids.clone(), weight_values.clone());
        // Make sure network will run the consensus ditribution, again
        PendingEmission::<Test>::insert(netuid, 90000000000000);
        step_block(params.tempo);

        let incentives: Vec<u16> = Incentive::<Test>::get(netuid);
        let emissions: Vec<u64> = Emission::<Test>::get(netuid);

        // evaluate votees
        let delta: u64 = 200 * params.tempo as u64;
        assert!(incentives[1] > 0);

        assert!(
            emissions[2] > 2 * emissions[1] - delta && emissions[2] < 2 * emissions[1] + delta,
            "emissions[1]: {} != emissions[2]: {}",
            emissions[1],
            emissions[2]
        );
    });
}

#[test]
fn test_founder_share() {
    new_test_ext().execute_with(|| {
        zero_min_validator_stake();
        let netuid = 1;
        let n: u16 = 20;
        let initial_stake: u64 = to_nano(1_000);
        let keys: Vec<u32> = (0..n as u32).collect();
        let stakes: Vec<u64> = (0..n).map(|_x| initial_stake).collect();

        // Setup Rootnet
        assert_ok!(register_named_subnet(u32::MAX, 0, "Rootnet"));
        SubnetConsensusType::<Test>::insert(0, SubnetConsensus::Root);
        assert_ok!(register_root_validator(u32::MAX, initial_stake));

        let founder_key = keys[0];
        MaxRegistrationsPerBlock::<Test>::set(1000);
        for i in 0..n {
            assert_ok!(register_module(
                netuid,
                keys[i as usize],
                stakes[i as usize],
                false
            ));
        }
        SubnetConsensusType::<Test>::insert(netuid, SubnetConsensus::Yuma);

        // Set rootnet weight
        set_weights(0, u32::MAX, vec![netuid], vec![1]);

        update_params!(netuid => { founder_share: 12 });
        let founder_share = FounderShare::<Test>::get(netuid);
        let founder_ratio: f64 = founder_share as f64 / 100.0;
        let subnet_params = SubspaceMod::<Test>::subnet_params(netuid);
        let total_emission = UnitEmission::<Test>::get() * subnet_params.tempo as u64;
        let expected_founder_share_precise = total_emission as f64 * founder_ratio;

        <pallet_balances::Pallet<Test> as Currency<_>>::make_free_balance_be(
            &founder_key.into(),
            0u32.into(),
        );

        step_epoch(netuid);

        let founder_balance = SubspaceMod::<Test>::get_balance(&founder_key);

        let tolerance = 3_000_000_000;

        assert!(
            (founder_balance as i64 - expected_founder_share_precise as i64).abs() <= tolerance,
            "Founder balance {} differs from expected {} by more than {}",
            founder_balance,
            expected_founder_share_precise,
            tolerance
        );

        // Repeat with consensus of linear
        SubnetConsensusType::<Test>::insert(netuid, SubnetConsensus::Linear);

        let treasury_address = DaoTreasuryAddress::<Test>::get();
        Founder::<Test>::insert(netuid, treasury_address);

        step_epoch(netuid);

        let treasury_balance = SubspaceMod::<Test>::get_balance(&treasury_address);
        assert!(
            (treasury_balance as i64 - expected_founder_share_precise as i64).abs() <= tolerance,
            "Treasury balance {} differs from expected {} by more than {}",
            founder_balance,
            expected_founder_share_precise,
            tolerance
        );
    });
}

// ------------
// Step Yuma
// ------------

mod utils {
    use crate::mock::Test;
    use pallet_subspace::{Consensus, Dividends, Emission, Incentive, Rank, Trust};

    pub fn get_rank_for_uid(netuid: u16, uid: u16) -> u16 {
        Rank::<Test>::get(netuid).get(uid as usize).copied().unwrap_or_default()
    }

    pub fn get_trust_for_uid(netuid: u16, uid: u16) -> u16 {
        Trust::<Test>::get(netuid).get(uid as usize).copied().unwrap_or_default()
    }

    pub fn get_consensus_for_uid(netuid: u16, uid: u16) -> u16 {
        Consensus::<Test>::get(netuid).get(uid as usize).copied().unwrap_or_default()
    }

    pub fn get_incentive_for(netuid: u16, uid: u16) -> u16 {
        Incentive::<Test>::get(netuid).get(uid as usize).copied().unwrap_or_default()
    }

    pub fn get_dividends_for(netuid: u16, uid: u16) -> u16 {
        Dividends::<Test>::get(netuid).get(uid as usize).copied().unwrap_or_default()
    }

    pub fn get_emission_for(netuid: u16, uid: u16) -> u64 {
        Emission::<Test>::get(netuid).get(uid as usize).copied().unwrap_or_default()
    }
}

const ONE: u64 = to_nano(1);

// We are off by one,
// due to inactive / active calulation on yuma, which is 100% correct.
#[test]
fn test_1_graph() {
    new_test_ext().execute_with(|| {
        zero_min_validator_stake();
        zero_min_burn();
        MinimumAllowedStake::<Test>::set(0);

        UnitEmission::<Test>::put(23148148148);
        FloorFounderShare::<Test>::put(0);

        // Register general subnet
        assert_ok!(register_module(0, 10, 1, false));

        log::info!("test_1_graph:");
        let netuid: u16 = 1;
        let key = 0;
        let uid: u16 = 0;
        let stake_amount: u64 = to_nano(100);

        assert_ok!(register_module(netuid, key, stake_amount, false));
        update_params!(netuid => {
            max_allowed_uids: 2
        });

        assert_ok!(register_module(netuid, key + 1, 1, false));
        assert_eq!(N::<Test>::get(netuid), 2);

        run_to_block(1); // run to next block to ensure weights are set on nodes after their registration block

        assert_ok!(SubnetEmissionMod::set_weights(
            RuntimeOrigin::signed(1),
            netuid,
            vec![uid],
            vec![u16::MAX],
        ));

        let params = ConsensusParams::<Test>::new(netuid, ONE).unwrap();
        let weights = Weights::<Test>::iter_prefix(netuid).collect::<Vec<_>>();
        let emissions = YumaEpoch::<Test>::new(netuid, params).run(weights).unwrap();
        let offset = 1;

        assert_eq!(
            emissions.emission_map,
            [(ModuleKey(key), [(AccountKey(key), ONE - offset)].into())].into()
        );

        emissions.apply();

        let new_stake_amount = stake_amount + ONE;

        assert_eq!(
            SubspaceMod::<Test>::get_delegated_stake(&key),
            new_stake_amount - offset
        );
        assert_eq!(utils::get_rank_for_uid(netuid, uid), 0);
        assert_eq!(utils::get_trust_for_uid(netuid, uid), 0);
        assert_eq!(utils::get_consensus_for_uid(netuid, uid), 0);
        assert_eq!(utils::get_incentive_for(netuid, uid), 0);
        assert_eq!(utils::get_dividends_for(netuid, uid), 0);
        assert_eq!(utils::get_emission_for(netuid, uid), ONE - offset);
    });
}

#[test]
fn test_10_graph() {
    /// Function for adding a nodes to the graph.
    fn add_node(netuid: u16, key: AccountId, uid: u16, stake_amount: u64) {
        log::info!(
            "+Add net:{:?} hotkey:{:?} uid:{:?} stake_amount: {:?} subn: {:?}",
            netuid,
            key,
            uid,
            stake_amount,
            N::<Test>::get(netuid),
        );

        assert_ok!(register_module(netuid, key, stake_amount, false));
        assert_eq!(N::<Test>::get(netuid) - 1, uid);
    }

    new_test_ext().execute_with(|| {
        zero_min_validator_stake();
        UnitEmission::<Test>::put(23148148148);
        zero_min_burn();
        MinimumAllowedStake::<Test>::set(0);
        FloorFounderShare::<Test>::put(0);
        MaxRegistrationsPerBlock::<Test>::set(1000);
        // Register general subnet
        assert_ok!(register_module(0, 10_000, 1, false));

        log::info!("test_10_graph");

        // Build the graph with 10 items
        // each with 1 stake and self weights.
        let n = 10;
        let netuid: u16 = 1;
        let stake_amount_per_node = ONE;

        for i in 0..n {
            add_node(netuid, i, i as u16, stake_amount_per_node)
        }

        update_params!(netuid => {
            max_allowed_uids: n as u16 + 1
        });

        assert_ok!(register_module(netuid, n + 1, 1, false));
        assert_eq!(N::<Test>::get(netuid), 11);

        run_to_block(1); // run to next block to ensure weights are set on nodes after their registration block

        for i in 0..n {
            assert_ok!(SubnetEmissionMod::set_weights(
                get_origin(n + 1),
                netuid,
                vec![i as u16],
                vec![u16::MAX],
            ));
        }

        let params = ConsensusParams::<Test>::new(netuid, ONE).unwrap();
        let weights = Weights::<Test>::iter_prefix(netuid).collect::<Vec<_>>();
        let emissions = YumaEpoch::<Test>::new(netuid, params).run(weights).unwrap();

        let mut expected: EmissionMap<u32> = BTreeMap::new();
        for i in 0..n as u16 {
            expected
                .entry(ModuleKey(i.into()))
                .or_default()
                .insert(AccountKey(i.into()), 99999999);
        }

        assert_eq!(emissions.emission_map, expected);

        emissions.apply();

        // Check return values.
        let emission_per_node = ONE / n as u64;
        for i in 0..n as u16 {
            assert_eq!(
                from_nano(SubspaceMod::<Test>::get_delegated_stake(&(i as u32))),
                from_nano(to_nano(1) + emission_per_node)
            );

            assert_eq!(utils::get_rank_for_uid(netuid, i), 0);
            assert_eq!(utils::get_trust_for_uid(netuid, i), 0);
            assert_eq!(utils::get_consensus_for_uid(netuid, i), 0);
            assert_eq!(utils::get_incentive_for(netuid, i), 0);
            assert_eq!(utils::get_dividends_for(netuid, i), 0);
            assert_eq!(utils::get_emission_for(netuid, i), 99999999);
        }
    });
}

// Testing weight expiration, on subnets running yuma
#[test]
fn yuma_weights_older_than_max_age_are_discarded() {
    new_test_ext().execute_with(|| {
        zero_min_validator_stake();
        const MAX_WEIGHT_AGE: u64 = 300;
        const SUBNET_TEMPO: u16 = 100;

        // first setup the rootnet
        let rootnet_netuid: u16 = 0;
        let rootnet_key = 0;
        let rootnet_stake_amount: u64 = to_nano(1_000);
        assert_ok!(register_module(
            rootnet_netuid,
            rootnet_key,
            rootnet_stake_amount,
            false
        ));
        SubnetConsensusType::<Test>::insert(rootnet_netuid, SubnetConsensus::Root);

        // Register the general subnet.
        let netuid: u16 = 1;
        let key = 1;
        let stake_amount: u64 = to_nano(1_000);

        assert_ok!(register_module(netuid, key, stake_amount, false));

        // Register the yuma subnet.
        let yuma_netuid: u16 = 2;
        let yuma_validator_key = 2;
        let yuma_miner_key = 3;
        let yuma_vali_amount: u64 = to_nano(10_000);
        let yuma_miner_amount = to_nano(1_000);

        // This will act as an validator.
        assert_ok!(register_module(
            yuma_netuid,
            yuma_validator_key,
            yuma_vali_amount,
            false
        ));
        // This will act as an miner.
        assert_ok!(register_module(
            yuma_netuid,
            yuma_miner_key,
            yuma_miner_amount,
            false
        ));

        // Set rootnet weight equally
        let uids = vec![netuid, yuma_netuid];
        let weights = vec![1, 1];
        set_weights(rootnet_netuid, rootnet_key, uids, weights);

        step_block(1);

        // Set the max weight age to 300 blocks
        update_params!(yuma_netuid => {
            tempo: SUBNET_TEMPO,
            max_weight_age: MAX_WEIGHT_AGE
        });

        let miner_uid = SubspaceMod::<Test>::get_uid_for_key(yuma_netuid, &yuma_miner_key).unwrap();
        let validator_uid =
            SubspaceMod::<Test>::get_uid_for_key(yuma_netuid, &yuma_validator_key).unwrap();
        let uid = [miner_uid].to_vec();
        let weight = [1].to_vec();

        // set the weights
        assert_ok!(SubnetEmissionMod::do_set_weights(
            get_origin(yuma_validator_key),
            yuma_netuid,
            uid,
            weight
        ));

        step_block(100);

        // Make sure we have incentive and dividends
        let miner_incentive = SubspaceMod::<Test>::get_incentive_for(yuma_netuid)
            .get(miner_uid as usize)
            .copied()
            .unwrap_or_default();
        let miner_dividends = SubspaceMod::<Test>::get_dividends_for(yuma_netuid)
            .get(miner_uid as usize)
            .copied()
            .unwrap_or_default();
        let validator_incentive = SubspaceMod::<Test>::get_incentive_for(yuma_netuid)
            .get(validator_uid as usize)
            .copied()
            .unwrap_or_default();
        let validator_dividends = SubspaceMod::<Test>::get_dividends_for(yuma_netuid)
            .get(validator_uid as usize)
            .copied()
            .unwrap_or_default();

        assert!(miner_incentive > 0);
        assert_eq!(miner_dividends, 0);
        assert!(validator_dividends > 0);
        assert_eq!(validator_incentive, 0);

        // now go pass the max weight age
        step_block(MAX_WEIGHT_AGE as u16);

        // Make sure we have no incentive and dividends
        let miner_incentive = SubspaceMod::<Test>::get_incentive_for(yuma_netuid)
            .get(miner_uid as usize)
            .copied()
            .unwrap_or_default();
        let miner_dividends = SubspaceMod::<Test>::get_dividends_for(yuma_netuid)
            .get(miner_uid as usize)
            .copied()
            .unwrap_or_default();
        let validator_incentive = SubspaceMod::<Test>::get_incentive_for(yuma_netuid)
            .get(validator_uid as usize)
            .copied()
            .unwrap_or_default();
        let validator_dividends = SubspaceMod::<Test>::get_dividends_for(yuma_netuid)
            .get(validator_uid as usize)
            .copied()
            .unwrap_or_default();

        assert_eq!(miner_incentive, 0);
        assert_eq!(miner_dividends, 0);
        assert_eq!(validator_dividends, 0);
        assert_eq!(validator_incentive, 0);

        // But make sure there are emissions
        let subnet_emission_sum = Emission::<Test>::get(yuma_netuid).iter().sum::<u64>();
        assert!(subnet_emission_sum > 0);
    });
}

// Bad actor will try to move stake quickly from one subnet to another,
// in hopes of increasing their emissions.
// Logic is getting above the subnet_stake threshold with a faster tempo
// (this is not possible due to emissions_to_drain calculated at evry block, making such exploits
// impossible)
#[test]
fn test_emission_exploit() {
    new_test_ext().execute_with(|| {
        const SUBNET_TEMPO: u16 = 25;
        // Register the general subnet.
        let netuid: u16 = 0;
        let key = 0;
        let stake_amount: u64 = to_nano(1_000);

        // Make sure registration cost is not affected
        zero_min_burn();

        assert_ok!(register_module(netuid, key, stake_amount, false));

        // Register the yuma subnet.
        let yuma_netuid: u16 = 1;
        let yuma_badactor_key = 1;
        let yuma_badactor_amount: u64 = to_nano(10_000);

        assert_ok!(register_module(
            yuma_netuid,
            yuma_badactor_key,
            yuma_badactor_amount,
            false
        ));
        update_params!(netuid => { tempo: SUBNET_TEMPO });

        // step first 40 blocks from the registration
        step_block(40);

        let stake_accumulated = SubspaceMod::<Test>::get_delegated_stake(&yuma_badactor_key);
        // User will now unstake and register another subnet.
        assert_ok!(SubspaceMod::<Test>::do_remove_stake(
            get_origin(yuma_badactor_key),
            yuma_badactor_key,
            stake_accumulated - 1
        ));

        // simulate real conditions by stepping  block
        step_block(2); // 42 blocks passed since the registration

        let new_netuid = 2;
        // register the new subnet
        let mut network: Vec<u8> = "test".as_bytes().to_vec();
        network.extend(new_netuid.to_string().as_bytes().to_vec());
        let mut name: Vec<u8> = "module".as_bytes().to_vec();
        name.extend(key.to_string().as_bytes().to_vec());
        let address: Vec<u8> = "0.0.0.0:30333".as_bytes().to_vec();
        let origin = get_origin(yuma_badactor_key);
        let _ = SubspaceMod::<Test>::register_subnet(origin.clone(), network.clone(), None);
        assert_ok!(SubspaceMod::<Test>::register(
            origin.clone(),
            network,
            name,
            address,
            yuma_badactor_key,
            None
        ));
        assert_ok!(SubspaceMod::<Test>::add_stake(
            origin,
            yuma_badactor_key,
            yuma_badactor_amount - 1
        ));

        // set the tempo
        update_params!(netuid => { tempo: SUBNET_TEMPO });

        // now 100 blocks went by since the registration, 1 + 40 + 58 = 100
        step_block(58);

        // remove the stake again
        let stake_accumulated_two = SubspaceMod::<Test>::get_delegated_stake(&yuma_badactor_key);
        assert_ok!(SubspaceMod::<Test>::do_remove_stake(
            get_origin(yuma_badactor_key),
            yuma_badactor_key,
            stake_accumulated_two - 2
        ));

        let badactor_balance_after = SubspaceMod::<Test>::get_balance(&yuma_badactor_key);

        let new_netuid = 3;
        // Now an honest actor will come, the goal is for him to accumulate more
        let honest_actor_key = 3;
        assert_ok!(register_module(
            new_netuid,
            honest_actor_key,
            yuma_badactor_amount,
            false
        ));
        // we will set a slower tempo, standard 100
        update_params!(new_netuid => { tempo: 100 });
        step_block(101);

        // get the stake of honest actor
        let honest_stake = SubspaceMod::<Test>::get_delegated_stake(&honest_actor_key);
        assert!(honest_stake > badactor_balance_after);
    });
}

#[test]
fn test_tempo_compound() {
    new_test_ext().execute_with(|| {
        zero_min_validator_stake();
        zero_min_burn();

        const QUICK_TEMPO: u16 = 25;
        const SLOW_TEMPO: u16 = 1000;
        // Register the root subnet.
        let root_netuid: u16 = 0;
        let root_key = 0;
        let stake_amount: u64 = to_nano(1_000);

        assert_ok!(register_module(root_netuid, root_key, stake_amount, false));
        SubnetConsensusType::<Test>::insert(root_netuid, SubnetConsensus::Root);

        // Register the yuma subnets, the important part of the tests starts here:
        // SLOW
        let s_netuid: u16 = 1;
        let s_key = 1;
        let s_amount: u64 = to_nano(10_000);
        assert_ok!(register_module(s_netuid, s_key, s_amount, false));
        update_params!(s_netuid => { tempo: SLOW_TEMPO });

        // FAST
        let f_netuid = 2;
        // Now an honest actor will come, the goal is for him to accumulate more
        let f_key = 2;
        assert_ok!(register_module(f_netuid, f_key, s_amount, false));
        // we will set a slower tempo
        update_params!(f_netuid => { tempo: QUICK_TEMPO });

        // set the weight on both subnets equally
        let uids: Vec<u16> = vec![s_netuid, f_netuid];
        let weights: Vec<u16> = vec![1, 1];
        set_weights(root_netuid, root_key, uids.clone(), weights.clone());

        // we will now step the blocks
        step_block(SLOW_TEMPO + 24);

        let fast = SubspaceMod::<Test>::get_delegated_stake(&f_key);
        let slow = SubspaceMod::<Test>::get_delegated_stake(&s_key);

        dbg!(fast, slow);

        // faster tempo should have quicker compound rate
        assert!(fast > slow);
    });
}

#[test]
fn test_non_minable_subnet_emisson() {
    new_test_ext().execute_with(|| {
        zero_min_validator_stake();
        // First we register the rootnet
        // Total 3 modules, same stake
        let rootnet_key_zero = 0;
        let rootnet_key_one = 1;
        const ROOTNET_NETUID: u16 = 0;
        assert_ok!(register_named_subnet(u32::MAX, ROOTNET_NETUID, "Rootnet"));
        SubnetConsensusType::<Test>::insert(0, SubnetConsensus::Root);
        assert_ok!(register_root_validator(rootnet_key_zero, to_nano(1000)));
        assert_ok!(register_root_validator(rootnet_key_one, to_nano(1000)));
        // Now we register treasury subnet
        assert_ok!(register_subnet(3, 1));
        SubnetConsensusType::<Test>::insert(1, SubnetConsensus::Treasury);
        UnitEmission::<Test>::put(to_nano(100_000));

        // Set emission only on rootnet and expect recycling
        set_weights(
            ROOTNET_NETUID,
            rootnet_key_zero,
            [ROOTNET_NETUID].to_vec(),
            [1].to_vec(),
        );
        let issuance_before = get_total_issuance();
        // Run the epoch
        step_block(200);
        let issuance_after = get_total_issuance();
        assert_eq!(issuance_before, issuance_after);

        // Start setting weights on treasury as well
        set_weights(ROOTNET_NETUID, rootnet_key_one, [1].to_vec(), [1].to_vec());

        // Run the epoch
        step_block(200);
        let issuance_after_treasury = get_total_issuance();
        assert!(issuance_after_treasury > issuance_after);
        let treasury_balance = get_balance(DaoTreasuryAddress::<Test>::get());
        assert_eq!(treasury_balance, issuance_after_treasury - issuance_after);
    });
}

// Halving
// Tests halving logic of the blockchain
#[test]
fn test_halving() {
    new_test_ext().execute_with(|| {
        // Set the emission configuration
        let decimals = 9;
        let multiplier = 10_u64.pow(decimals as u32);
        set_emission_config(decimals, 250_000_000, 1_000_000_000);

        // Set the initial unit emission to a large value
        let initial_unit_emission = 1_000_000_000_000_000;
        UnitEmission::<Test>::put(initial_unit_emission);

        // Test emission at different total issuance levels
        set_total_issuance(0);
        assert_eq!(
            SubnetEmissionMod::get_total_emission_per_block(),
            initial_unit_emission
        );

        set_total_issuance(250_000_000 * multiplier);
        assert_eq!(
            SubnetEmissionMod::get_total_emission_per_block(),
            initial_unit_emission / 2
        );

        set_total_issuance(500_000_000 * multiplier);
        assert_eq!(
            SubnetEmissionMod::get_total_emission_per_block(),
            initial_unit_emission / 4
        );

        set_total_issuance(750_000_000 * multiplier);
        assert_eq!(
            SubnetEmissionMod::get_total_emission_per_block(),
            initial_unit_emission / 8
        );

        set_total_issuance(1_000_000_000 * multiplier);
        assert_eq!(SubnetEmissionMod::get_total_emission_per_block(), 0);

        // mission beyond the maximum supply
        set_total_issuance(1_250_000_000 * multiplier);
        assert_eq!(SubnetEmissionMod::get_total_emission_per_block(), 0);
    });
}

/// This test is aimed at subnet deregistration based on emission
/// 1. Set MaxAllowedSubnets to 3
/// 2. Register 3 subnets, using the function `register_named_subnet`
/// 3. Set SubnetEmission to 100, 300, 200
/// 4. Register new subnet and expect the first subnet with lowest emission to get deregistered
/// 5. Set subne<t emission to 500 for the new subnet
/// 6. Register new subnet and expect the 3th subnet (the one with 200 emission) to be the one
///    deregistered
#[test]
fn test_subnet_deregistration_based_on_emission() {
    new_test_ext().execute_with(|| {
        zero_min_burn();
        SubnetImmunityPeriod::<Test>::set(0);
        // Set MaxAllowedSubnets to 3
        MaxAllowedSubnets::<Test>::set(3);

        // Register 3 subnets
        assert_ok!(register_named_subnet(0, 0, "subnet1"));
        assert_ok!(register_named_subnet(1, 1, "subnet2"));
        assert_ok!(register_named_subnet(2, 2, "subnet3"));

        // Set SubnetEmission for the three subnets
        SubnetEmission::<Test>::insert(0, 200);
        SubnetEmission::<Test>::insert(1, 100);
        SubnetEmission::<Test>::insert(2, 350);
        SubnetConsensusType::<Test>::insert(0, SubnetConsensus::Yuma);
        SubnetConsensusType::<Test>::insert(1, SubnetConsensus::Yuma);
        SubnetConsensusType::<Test>::insert(2, SubnetConsensus::Yuma);
        N::<Test>::insert(0, 1);
        N::<Test>::insert(1, 1);
        N::<Test>::insert(2, 1);
        // Register a new subnet, expect the first subnet (lowest emission) to be deregistered

        let universal_vec = "subnet4".to_string().as_bytes().to_vec();
        add_balance(3, to_nano(3000));
        let _ = SubspaceMod::<Test>::do_register_subnet(get_origin(3), universal_vec.clone(), None);
        assert_ok!(SubspaceMod::<Test>::do_register(
            get_origin(3),
            universal_vec.clone(),
            universal_vec.clone(),
            universal_vec.clone(),
            2,
            Some(universal_vec.clone()),
        ));
        assert_ok!(SubspaceMod::<Test>::add_stake(
            get_origin(3),
            2,
            to_nano(2000)
        ));

        assert_eq!(SubnetNames::<Test>::get(0), "subnet1".as_bytes().to_vec());
        assert_eq!(SubnetNames::<Test>::get(1), "subnet4".as_bytes().to_vec());
        assert_eq!(SubnetNames::<Test>::get(2), "subnet3".as_bytes().to_vec());

        // Set subnet emission for the new subnet
        SubnetEmission::<Test>::insert(4, 500);
    });
}

#[test]
fn yuma_does_not_fail_if_module_does_not_have_stake() {
    new_test_ext().execute_with(|| {
        zero_min_burn();
        MinimumAllowedStake::<Test>::set(0);

        let netuid: u16 = 1;
        let key = 0;

        let stake: u64 = 1;

        assert_ok!(register_module(netuid, key, stake, false));
        assert_ok!(SubspaceMod::<Test>::do_remove_stake(
            get_origin(key),
            key,
            stake
        ));

        let params = ConsensusParams::<Test>::new(netuid, ONE).unwrap();
        let weights = Weights::<Test>::iter_prefix(netuid).collect::<Vec<_>>();
        assert_ok!(YumaEpoch::<Test>::new(netuid, params).run(weights));
    });
}

#[test]
fn foo() {
    new_test_ext().execute_with(|| {
        register_subnet(0, 0).unwrap();
        // TODO:
        // let last_params = ConsensusParams::<Test>::new(0, to_nano(100)).unwrap();
        // let last_output = YumaEpoch::<Test>::new(0, last_params).run().unwrap();

        // let now_params = ConsensusParams::<Test>::new(0, to_nano(50)).unwrap();
        // let now_output = YumaEpoch::<Test>::new(0, now_params).run().unwrap();

        // let foo = pallet_offworker::ConsensusSimulationResult {
        //     cumulative_copier_divs: I64F64::from_num(0.8),
        //     cumulative_avg_delegate_divs: I64F64::from_num(1.0),
        //     min_underperf_threshold: I64F64::from_num(0.1),
        //     encryption_window_len: 100,
        //     max_encryption_period: 1000,
        //     _phantom: PhantomData,
        // };

        // pallet_offworker::is_copying_irrational::<Test>(last_output, now_output, foo);
    });
}

#[test]
fn yuma_change_permits() {
    new_test_ext().execute_with(|| {
        zero_min_burn();

        let netuid = 6;
        let first_uid = register_module(netuid, 0, to_nano(10_000), false).unwrap();
        let second_uid = register_module(netuid, 1, to_nano(51000), false).unwrap();
        let third_uid = register_module(netuid, 2, to_nano(52000), false).unwrap();

        MaxAllowedValidators::<Test>::set(netuid, Some(2));

        set_weights(netuid, 2, vec![first_uid, second_uid], vec![50, 60]);

        let yuma_params = ConsensusParams::<Test>::new(netuid, ONE).unwrap();

        let weights = Weights::<Test>::iter_prefix(netuid).collect::<Vec<_>>();
        YumaEpoch::<Test>::new(netuid, yuma_params.clone())
            .run(weights)
            .unwrap()
            .apply();

        assert_eq!(
            ValidatorPermits::<Test>::get(netuid)[first_uid as usize],
            false
        );
        assert_eq!(
            ValidatorPermits::<Test>::get(netuid)[second_uid as usize],
            false
        );
        assert_eq!(
            ValidatorPermits::<Test>::get(netuid)[third_uid as usize],
            true
        );

        let fourth_uid = register_module(netuid, 3, to_nano(54000), false).unwrap();

        set_weights(netuid, 1, vec![third_uid, fourth_uid], vec![50, 60]);
        set_weights(netuid, 3, vec![first_uid, second_uid], vec![50, 60]);

        let weights = Weights::<Test>::iter_prefix(netuid).collect::<Vec<_>>();

        let yuma_params = ConsensusParams::<Test>::new(netuid, ONE).unwrap();

        YumaEpoch::<Test>::new(netuid, yuma_params.clone())
            .run(weights)
            .unwrap()
            .apply();

        assert_eq!(
            ValidatorPermits::<Test>::get(netuid)[first_uid as usize],
            false
        );
        assert_eq!(
            ValidatorPermits::<Test>::get(netuid)[second_uid as usize],
            false
        );
        assert_eq!(
            ValidatorPermits::<Test>::get(netuid)[third_uid as usize],
            true
        );
        assert_eq!(
            ValidatorPermits::<Test>::get(netuid)[fourth_uid as usize],
            true
        );
    });
}

#[test]
fn decrypted_weight_run_result_is_applied_and_cleaned_up() {
    new_test_ext().execute_with(|| {
        let netuid = 0;

        // Register founder, validator and miners
        let founder_key = 100;
        let validator_key = 1;
        let first_miner = 2;
        let second_miner = 3;

        // Register validator and miners
        let validator_uid =
            register_module(netuid, validator_key, to_nano(100_000), false).unwrap();
        let first_miner_uid = register_module(netuid, first_miner, to_nano(1), false).unwrap();
        let second_miner_uid = register_module(netuid, second_miner, to_nano(1), false).unwrap();

        SubnetConsensusType::<Test>::insert(netuid, SubnetConsensus::Yuma);

        pallet_subspace::MinFees::<Test>::set(pallet_subspace::MinimumFees {
            stake_delegation_fee: Percent::from_percent(0),
            validator_weight_fee: Percent::from_percent(0),
        });

        // Set validator fees (50% for both stake delegation and validator weight)
        let validator_fees = pallet_subspace::ValidatorFees::new::<Test>(
            Percent::from_percent(0),
            Percent::from_percent(0),
        )
        .unwrap();
        pallet_subspace::ValidatorFeeConfig::<Test>::insert(validator_key, validator_fees);

        // Stake some balance to validator from a delegator
        let delegator_key = 4;
        let stake_amount = to_nano(100_000);
        stake(delegator_key, validator_key, stake_amount);

        let key = RsaPrivateKey::new(&mut OsRng, 2048).unwrap().to_public_key();
        let key = (key.n().to_bytes_be(), key.e().to_bytes_be());

        let subnet_decryption_data = SubnetDecryptionInfo {
            validity_block: Some(0),
            node_id: 1001,
            node_public_key: key.clone(),
            last_keep_alive: pallet_subspace::Tempo::<Test>::get(netuid) as u64,
            rotating_from: None,
        };

        pallet_subnet_emission::DecryptionNodes::<Test>::set(vec![subnet_decryption_data.clone()]);
        pallet_subnet_emission::SubnetDecryptionData::<Test>::set(
            netuid,
            Some(subnet_decryption_data),
        );
        pallet_subspace::UseWeightsEncryption::<Test>::set(netuid, true);

        // Set weights for validator
        let validator_weights = vec![(first_miner_uid, 50u16), (second_miner_uid, 50u16)];

        let emission_amount = to_nano(100);
        PendingEmission::<Test>::set(netuid, emission_amount);

        let miner_uid =
            pallet_subspace::Pallet::<Test>::get_uid_for_key(netuid, &second_miner).unwrap();

        // Deregister second miner
        pallet_subspace::Pallet::<Test>::remove_module(netuid, miner_uid, false).unwrap();

        let weights = vec![(
            pallet_subspace::Tempo::<Test>::get(netuid) as u64,
            vec![(validator_uid, validator_weights.clone(), 1.encode())],
        )];

        // Set founder share to 10%
        FounderShare::<Test>::set(netuid, 10);
        pallet_subspace::Founder::<Test>::set(netuid, founder_key);

        let params = ConsensusParams::<Test>::new(netuid, emission_amount).unwrap();

        pallet_subnet_emission::Pallet::<Test>::handle_decrypted_weights(netuid, weights);

        let res = YumaEpoch::run(
            YumaEpoch::new(netuid, params),
            vec![(validator_uid, validator_weights.clone())],
        )
        .unwrap();

        let initial_founder_balance = pallet_subspace::Pallet::<Test>::get_balance(&founder_key);
        let initial_validator_stake =
            pallet_subspace::Pallet::<Test>::get_owned_stake(&validator_key);
        let initial_delegator_stake = dbg!(pallet_subspace::Pallet::<Test>::get_stake_to_module(
            &delegator_key,
            &validator_key
        ));
        let initial_second_miner_balance =
            pallet_subspace::Pallet::<Test>::get_balance(&second_miner);

        res.clone().apply();

        // Assert founder got 10% of emission
        let founder_emission = emission_amount / 10;
        assert_in_range!(
            pallet_subspace::Pallet::<Test>::get_balance(&founder_key) - initial_founder_balance,
            founder_emission,
            100
        );

        // The weights on the deregistered miner should have been discarded
        assert!(
            pallet_subspace::Pallet::<Test>::get_balance(&second_miner)
                == initial_second_miner_balance
        );

        // Assert validator and delegator stakes increased according to fee split
        let validator_stake_increase = from_nano(
            pallet_subspace::Pallet::<Test>::get_owned_stake(&validator_key)
                - initial_validator_stake,
        );
        let delegator_stake_increase = from_nano(
            pallet_subspace::Pallet::<Test>::get_stake_to_module(&delegator_key, &validator_key)
                - initial_delegator_stake,
        );

        dbg!(validator_stake_increase, delegator_stake_increase);

        // Validator should get their fee percentage
        assert!(validator_stake_increase > 0);

        // Delegator should get the remaining percentage after fees
        assert!(delegator_stake_increase > 0);

        // The ratio between validator and delegator increases should roughly match the fee split
        let total_increase = validator_stake_increase + delegator_stake_increase;
        let validator_percentage =
            ((validator_stake_increase as f64 / total_increase as f64) * 100.0) as u64;

        assert_in_range!(validator_percentage, 50, 2);
        assert_eq!(Active::<Test>::get(netuid), res.active);
        assert_eq!(Consensus::<Test>::get(netuid), res.consensus);
        assert_eq!(Dividends::<Test>::get(netuid), res.dividends);
        assert_eq!(Emission::<Test>::get(netuid), res.combined_emissions);
        assert_eq!(Incentive::<Test>::get(netuid), res.incentives);
        assert_eq!(PruningScores::<Test>::get(netuid), res.pruning_scores);
        assert_eq!(Rank::<Test>::get(netuid), res.ranks);
        assert_eq!(Trust::<Test>::get(netuid), res.trust);
        assert_eq!(ValidatorPermits::<Test>::get(netuid), res.validator_permits);
        assert_eq!(ValidatorTrust::<Test>::get(netuid), res.validator_trust);
    });
}

#[test]
fn rotate_decryption_node() {
    use sp_core::Get;
    new_test_ext().execute_with(|| {
        let netuid = 0;

        let dn_1 = 1001;
        let key_1 = RsaPrivateKey::new(&mut OsRng, 2048).unwrap().to_public_key();
        let key_1 = (key_1.n().to_bytes_be(), key_1.e().to_bytes_be());
        let dn_2 = 1002;
        let key_2 = RsaPrivateKey::new(&mut OsRng, 2048).unwrap().to_public_key();
        let key_2 = (key_2.n().to_bytes_be(), key_2.e().to_bytes_be());

        let decryption_node_interval: u64 =
            <Test as pallet_subnet_emission::Config>::DecryptionNodeRotationInterval::get();

        step_block(decryption_node_interval as u16);

        pallet_subnet_emission::SubnetDecryptionData::<Test>::set(
            netuid,
            Some(SubnetDecryptionInfo {
                validity_block: Some(0),
                node_id: dn_1,
                node_public_key: key_1,
                last_keep_alive: decryption_node_interval,
                rotating_from: None,
            }),
        );

        pallet_subnet_emission::SubnetDecryptionData::<Test>::set(
            netuid,
            Some(SubnetDecryptionInfo {
                validity_block: Some(0),
                node_id: dn_2,
                node_public_key: key_2,
                last_keep_alive: decryption_node_interval,
                rotating_from: None,
            }),
        );

        // one subnet with decryption node set
        pallet_subnet_emission::DecryptionNodeCursor::<Test>::set(1);
        SubnetConsensusType::<Test>::insert(netuid, SubnetConsensus::Yuma);

        pallet_subnet_emission::Pallet::<Test>::handle_decrypted_weights(netuid, vec![]);

        assert_eq!(
            pallet_subnet_emission::SubnetDecryptionData::<Test>::get(netuid)
                .map(|info| info.node_id),
            Some(dn_2)
        );
    });
}

#[test]
fn ban_decryption_node() {
    new_test_ext().execute_with(|| {
        let netuid = 0;

        let dn_1 = 1001;
        let key_1 = RsaPrivateKey::new(&mut OsRng, 2048).unwrap().to_public_key();
        let key_1 = (key_1.n().to_bytes_be(), key_1.e().to_bytes_be());

        let acc_id = 1;
        let _ = register_module(netuid, acc_id, 10000, false).unwrap();

        pallet_subnet_emission::SubnetDecryptionData::<Test>::set(
            netuid,
            Some(SubnetDecryptionInfo {
                validity_block: Some(0),
                node_id: dn_1,
                node_public_key: key_1,
                last_keep_alive: 0,
                rotating_from: None,
            }),
        );

        pallet_subspace::UseWeightsEncryption::<Test>::set(netuid, true);

        let uid = SubspaceMod::<Test>::get_uid_for_key(netuid, &acc_id).unwrap();

        // make sure there are some weights present
        pallet_subnet_emission::WeightEncryptionData::<Test>::set(
            netuid,
            uid,
            Some(pallet_subnet_emission::EncryptionMechanism {
                encrypted: vec![42],
                decrypted_hashes: vec![123],
            }),
        );
        let max_encryption_inteval =
            pallet_subnet_emission::Pallet::<Test>::get_max_encryption_interval(&netuid);
        step_block((max_encryption_inteval + 1) as u16);
        step_epoch(netuid);
        dbg!(&pallet_subnet_emission::DecryptionNodeBanQueue::<Test>::iter().collect::<Vec<_>>());

        // one subnet with decryption node set
        pallet_subnet_emission::DecryptionNodeCursor::<Test>::set(1);

        assert_ne!(BannedDecryptionNodes::<Test>::get(dn_1), 0);
    });
}

#[test]
fn weight_setting_delegation() {
    new_test_ext().execute_with(|| {
        use sp_runtime::Percent;
        const NETUID: u16 = 0;

        let parent_key = 0;
        let child_key = 1;
        let _parent_uid = register_module(NETUID, parent_key, to_nano(500000), false).unwrap();
        let _child_uid = register_module(NETUID, child_key, to_nano(500000), false).unwrap();
        register_module(NETUID, 2, to_nano(0), false).unwrap();

        FounderShare::<Test>::set(NETUID, 0);

        dbg!(pallet_subnet_emission::SubnetConsensusType::<Test>::get(
            NETUID
        ));

        PendingEmission::<Test>::set(NETUID, to_nano(1000));

        pallet_subspace::Pallet::<Test>::update_module(
            get_origin(parent_key),
            NETUID,
            "test".as_bytes().to_vec(),
            "test:2020".as_bytes().to_vec(),
            None,
            Some(Percent::from_percent(5)),
            None,
        )
        .unwrap();

        pallet_subspace::WeightSettingDelegation::<Test>::set(NETUID, child_key, Some(parent_key));

        step_block(1);

        set_weights(NETUID, parent_key, vec![2], vec![2000]);

        step_epoch(NETUID);

        let emissions = Emission::<Test>::get(NETUID);

        assert_in_range!(
            emissions[0],
            ((to_nano(1000) as f32 * 0.25) * 1.05) as u64,
            10000
        );
        assert_in_range!(
            emissions[1],
            ((to_nano(1000) as f32 * 0.25) * 0.95) as u64,
            10000
        );
    });
}

// #[test]
// fn receive_empty_weights() {
//     new_test_ext().execute_with(|| {
//         use sp_runtime::Percent;
//         const NETUID: u16 = 0;

//         let _uid = register_module(NETUID, 0, to_nano(500000), false).unwrap();

//         let result = YumaEpoch::<Test>::new(
//             NETUID,
//             ConsensusParams::<Test>::new(NETUID, to_nano(1000)).unwrap(),
//         )
//         .run(vec![])
//         .unwrap();

//         dbg!(&result);
//         panic!("a");
//     });
// }
