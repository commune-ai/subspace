// -----------
// Step linear
// -----------

use std::collections::BTreeMap;

use crate::mock::*;

use frame_support::assert_ok;
use log::info;
use pallet_subnet_emission::{
    subnet_consensus::yuma::{AccountKey, EmissionMap, ModuleKey, YumaEpoch},
    UnitEmission,
};
use pallet_subspace::*;

// fn update_params(netuid: u16, tempo: u16, max_weights: u16, min_weights: u16) {
//     Tempo::<Test>::insert(netuid, tempo);
//     MaxAllowedWeights::<Test>::insert(netuid, max_weights);
//     MinAllowedWeights::<Test>::insert(netuid, min_weights);
// }

// TODO:
// get back to life
// fn check_network_stats(netuid: u16) {
//     let emission_buffer: u64 = 1_000; // the numbers arent perfect but we want to make sure they
// fall within a range (10_000 / 2**64)     let threshold = SubnetStakeThreshold::<Test>::get();
//     let subnet_emission: u64 = SubspaceMod::calculate_network_emission(netuid, threshold);
//     let incentives: Vec<u16> = Incentive::<Test>::get(netuid);
//     let dividends: Vec<u16> = Dividends::<Test>::get(netuid);
//     let emissions: Vec<u64> = Emission::<Test>::get(netuid);
//     let total_incentives: u16 = incentives.iter().sum();
//     let total_dividends: u16 = dividends.iter().sum();
//     let total_emissions: u64 = emissions.iter().sum();

//     info!("total_emissions: {total_emissions}");
//     info!("total_incentives: {total_incentives}");
//     info!("total_dividends: {total_dividends}");

//     info!("emission: {emissions:?}");
//     info!("incentives: {incentives:?}");
//     info!("dividends: {dividends:?}");

//     assert!(
//         total_emissions >= subnet_emission - emission_buffer
//             || total_emissions <= subnet_emission + emission_buffer
//     );
// }

#[test]
fn test_no_weights() {
    new_test_ext().execute_with(|| {
        let netuid: u16 = 0;

        // make sure that the results won´t get affected by burn
        zero_min_burn();

        register_n_modules(0, 10, 1000);
        Tempo::<Test>::insert(netuid, 1);
        let _keys = SubspaceMod::get_keys(netuid);
        let _uids = SubspaceMod::get_uids(netuid);

        let incentives: Vec<u16> = Incentive::<Test>::get(netuid);
        let dividends: Vec<u16> = Dividends::<Test>::get(netuid);
        let emissions: Vec<u64> = Emission::<Test>::get(netuid);
        let _total_incentives: u16 = incentives.iter().sum();
        let _total_dividends: u16 = dividends.iter().sum();
        let _total_emissions: u64 = emissions.iter().sum();
    });
}

// TODO:
// get back to life
// #[test]
// fn test_dividends_same_stake() {
//     new_test_ext().execute_with(|| {
//         // CONSSTANTS
//         let netuid: u16 = 0;
//         let n: u16 = 10;
//         let stake_per_module: u64 = 10_000;

//         // make sure that the results won´t get affected by burn
//         zero_min_burn();

//         // SETUP NETWORK
//         register_n_modules(netuid, n, stake_per_module);
//         update_params(netuid, 1, n, 0);

//         let keys = SubspaceMod::get_keys(netuid);
//         let _uids = SubspaceMod::get_uids(netuid);

//         // do a list of ones for weights
//         let weight_uids: Vec<u16> = [2, 3].to_vec();
//         // do a list of ones for weights
//         let weight_values: Vec<u16> = [2, 1].to_vec();
//         set_weights(netuid, keys[0], weight_uids.clone(), weight_values.clone());
//         set_weights(netuid, keys[1], weight_uids.clone(), weight_values.clone());

//         let stakes_before: Vec<u64> = get_stakes(netuid);
//         step_epoch(netuid);
//         let incentives: Vec<u16> = Incentive::<Test>::get(netuid);
//         let dividends: Vec<u16> = Dividends::<Test>::get(netuid);
//         let emissions: Vec<u64> = Emission::<Test>::get(netuid);
//         let stakes: Vec<u64> = get_stakes(netuid);

//         // evaluate votees
//         assert!(incentives[2] > 0);
//         assert_eq!(dividends[2], dividends[3]);
//         let delta: u64 = 100;
//         assert!((incentives[2] as u64) > (weight_values[0] as u64 * incentives[3] as u64) -
// delta);         assert!((incentives[2] as u64) < (weight_values[0] as u64 * incentives[3] as u64)
// + delta);

//         assert!(emissions[2] > (weight_values[0] as u64 * emissions[3]) - delta);
//         assert!(emissions[2] < (weight_values[0] as u64 * emissions[3]) + delta);

//         // evaluate voters
//         assert!(
//             dividends[0] == dividends[1],
//             "dividends[0]: {} != dividends[1]: {}",
//             dividends[0],
//             dividends[1]
//         );
//         assert!(
//             dividends[0] == dividends[1],
//             "dividends[0]: {} != dividends[1]: {}",
//             dividends[0],
//             dividends[1]
//         );

//         assert_eq!(incentives[0], incentives[1]);
//         assert_eq!(dividends[2], dividends[3]);

//         info!("emissions: {emissions:?}");

//         for (uid, emission) in emissions.iter().enumerate() {
//             if emission == &0 {
//                 continue;
//             }
//             let stake: u64 = stakes[uid];
//             let stake_before: u64 = stakes_before[uid];
//             let stake_difference: u64 = stake - stake_before;
//             let expected_stake_difference: u64 = emissions[uid];
//             let error_delta: u64 = (emissions[uid] as f64 * 0.001) as u64;

//             assert!(
//                 stake_difference < expected_stake_difference + error_delta
//                     && stake_difference > expected_stake_difference - error_delta,
//                 "stake_difference: {} != expected_stake_difference: {}",
//                 stake_difference,
//                 expected_stake_difference
//             );
//         }

//         check_network_stats(netuid);
//     });
// }

// TODO:
// get back to life
// #[test]
// fn test_dividends_diff_stake() {
//     new_test_ext().execute_with(|| {
//         // CONSSTANTS
//         let netuid: u16 = 0;
//         let n: u16 = 10;
//         let _n_list: Vec<u16> = vec![10, 50, 100, 1000];
//         let _blocks_per_epoch_list: u64 = 1;
//         let stake_per_module: u64 = 10_000;
//         let tempo: u16 = 100;

//         // make sure that the results won´t get affected by burn
//         zero_min_burn();

//         // SETUP NETWORK
//         for i in 0..n {
//             let mut stake = stake_per_module;
//             if i == 0 {
//                 stake = 2 * stake_per_module
//             }
//             let key: U256 = i;
//             assert_ok!(register_module(netuid, key, stake));
//         }
//         update_params(netuid, tempo, n, 0);

//         let keys = SubspaceMod::get_keys(netuid);
//         let _uids = SubspaceMod::get_uids(netuid);

//         // do a list of ones for weights
//         let weight_uids: Vec<u16> = [2, 3].to_vec();
//         // do a list of ones for weights
//         let weight_values: Vec<u16> = [1, 1].to_vec();
//         set_weights(netuid, keys[0], weight_uids.clone(), weight_values.clone());
//         set_weights(netuid, keys[1], weight_uids.clone(), weight_values.clone());

//         let stakes_before: Vec<u64> = get_stakes(netuid);
//         step_epoch(netuid);
//         let incentives: Vec<u16> = Incentive::<Test>::get(netuid);
//         let dividends: Vec<u16> = Dividends::<Test>::get(netuid);
//         let emissions: Vec<u64> = Emission::<Test>::get(netuid);
//         let stakes: Vec<u64> = get_stakes(netuid);

//         // evaluate votees
//         assert!(incentives[2] > 0);
//         assert_eq!(dividends[2], dividends[3]);
//         let delta: u64 = 100;
//         assert!((incentives[2] as u64) > (weight_values[0] as u64 * incentives[3] as u64) -
// delta);         assert!((incentives[2] as u64) < (weight_values[0] as u64 * incentives[3] as u64)
// + delta);

//         assert!(emissions[2] > (weight_values[0] as u64 * emissions[3]) - delta);
//         assert!(emissions[2] < (weight_values[0] as u64 * emissions[3]) + delta);

//         // evaluate voters
//         let delta: u64 = 100;
//         assert!((dividends[0] as u64) > (dividends[1] as u64 * 2) - delta);
//         assert!((dividends[0] as u64) < (dividends[1] as u64 * 2) + delta);

//         assert_eq!(incentives[0], incentives[1]);
//         assert_eq!(dividends[2], dividends[3]);

//         info!("emissions: {emissions:?}");

//         for (uid, emission) in emissions.iter().enumerate() {
//             if emission == &0 {
//                 continue;
//             }
//             let stake: u64 = stakes[uid];
//             let stake_before: u64 = stakes_before[uid];
//             let stake_difference: u64 = stake - stake_before;
//             let expected_stake_difference: u64 = emissions[uid];
//             let error_delta: u64 = (emissions[uid] as f64 * 0.001) as u64;

//             assert!(
//                 stake_difference < expected_stake_difference + error_delta
//                     && stake_difference > expected_stake_difference - error_delta,
//                 "stake_difference: {} != expected_stake_difference: {}",
//                 stake_difference,
//                 expected_stake_difference
//             );
//         }
//         check_network_stats(netuid);
//     });
// }

// TODO:
// get back to life
// #[test]
// fn test_pruning() {
//     new_test_ext().execute_with(|| {
//         // CONSTANTS
//         let netuid: u16 = 0;
//         let n: u16 = 100;
//         let stake_per_module: u64 = 10_000;
//         let tempo: u16 = 100;

//         // make sure that the results won´t get affected by burn
//         zero_min_burn();
//         MaxRegistrationsPerBlock::<Test>::set(1000);

//         // SETUP NETWORK
//         register_n_modules(netuid, n, stake_per_module);
//         MaxAllowedModules::<Test>::put(n);
//         update_params(netuid, 1, n, 0);

//         let voter_idx = 0;
//         let keys = SubspaceMod::get_keys(netuid);
//         let _uids = SubspaceMod::get_uids(netuid);

//         // Create a list of UIDs excluding the voter_idx
//         let weight_uids: Vec<u16> = (0..n).filter(|&x| x != voter_idx as u16).collect();

//         // Create a list of ones for weights, excluding the voter_idx
//         let mut weight_values: Vec<u16> = weight_uids.iter().map(|_x| 1_u16).collect();

//         let prune_uid: u16 = weight_uids.last().cloned().unwrap_or(0);

//         if let Some(prune_idx) = weight_uids.iter().position(|&uid| uid == prune_uid) {
//             weight_values[prune_idx] = 0;
//         }

//         set_weights(
//             netuid,
//             keys[voter_idx as usize],
//             weight_uids.clone(),
//             weight_values.clone(),
//         );

//         step_block(tempo);

//         let lowest_priority_uid: u16 = SubspaceMod::get_lowest_uid(netuid, false).unwrap_or(0);
//         assert!(lowest_priority_uid == prune_uid);

//         let new_key: U256 = U256::from(n + 1);

//         assert_ok!(register_module(netuid, new_key, stake_per_module));

//         let is_registered: bool = SubspaceMod::key_registered(netuid, &new_key);
//         assert!(is_registered);

//         assert!(
//             N::<Test>::get(netuid) == n,
//             "N::<Test>::get(netuid): {} != n: {}",
//             N::<Test>::get(netuid),
//             n
//         );

//         let is_prune_registered: bool =
//             SubspaceMod::key_registered(netuid, &keys[prune_uid as usize]);
//         assert!(!is_prune_registered);

//         check_network_stats(netuid);
//     });
// }

// TODO:
// get back to life
// #[test]
// fn test_lowest_priority_mechanism() {
//     new_test_ext().execute_with(|| {
//         // CONSSTANTS
//         let netuid: u16 = 0;
//         let n: u16 = 100;
//         let stake_per_module: u64 = 10_000;
//         let tempo: u16 = 100;

//         // make sure that the results won´t get affected by burn
//         zero_min_burn();
//         MaxRegistrationsPerBlock::<Test>::set(1000);

//         // SETUP NETWORK
//         register_n_modules(netuid, n, stake_per_module);

//         update_params(netuid, tempo, n, 0);

//         let keys = SubspaceMod::get_keys(netuid);
//         let voter_idx = 0;

//         // Create a list of UIDs excluding the voter_idx
//         let weight_uids: Vec<u16> = (0..n).filter(|&x| x != voter_idx).collect();

//         // Create a list of ones for weights, excluding the voter_idx
//         let mut weight_values: Vec<u16> = weight_uids.iter().map(|_x| 1_u16).collect();

//         let prune_uid: u16 = n - 1;

//         // Check if the prune_uid is still valid after excluding the voter_idx
//         if prune_uid != voter_idx {
//             // Find the index of prune_uid in the updated weight_uids vector
//             if let Some(prune_idx) = weight_uids.iter().position(|&uid| uid == prune_uid) {
//                 weight_values[prune_idx] = 0;
//             }
//         }

//         set_weights(
//             netuid,
//             keys[voter_idx as usize],
//             weight_uids.clone(),
//             weight_values.clone(),
//         );
//         step_block(tempo);
//         let incentives: Vec<u16> = Incentive::<Test>::get(netuid);
//         let dividends: Vec<u16> = Dividends::<Test>::get(netuid);
//         let emissions: Vec<u64> = Emission::<Test>::get(netuid);
//         let _stakes: Vec<u64> = get_stakes(netuid);

//         assert!(emissions[prune_uid as usize] == 0);
//         assert!(incentives[prune_uid as usize] == 0);
//         assert!(dividends[prune_uid as usize] == 0);

//         let lowest_priority_uid: u16 = SubspaceMod::get_lowest_uid(netuid, false).unwrap_or(0);
//         info!("lowest_priority_uid: {lowest_priority_uid}");
//         info!("prune_uid: {prune_uid}");
//         info!("emissions: {emissions:?}");
//         info!("lowest_priority_uid: {lowest_priority_uid:?}");
//         info!("dividends: {dividends:?}");
//         info!("incentives: {incentives:?}");
//         assert!(lowest_priority_uid == prune_uid);
//         check_network_stats(netuid);
//     });
// }

// #[test]
// fn test_deregister_zero_emission_uids() {
// 	new_test_ext().execute_with(|| {
//     // CONSSTANTS
//     let netuid: u16 = 0;
//     let n : u16 = 100;
//     let num_zero_uids : u16 = 10;
//     let blocks_per_epoch_list : u64 = 1;
//     let stake_per_module : u64 = 10_000;

//     // SETUP NETWORK
//     let tempo: u16 = 1;
//     register_n_modules( netuid, n, stake_per_module );
//     SubspaceMod::set_tempo( netuid, tempo );
//     SubspaceMod::set_max_allowed_weights(netuid, n );
//     SubspaceMod::set_min_allowed_weights(netuid, 0 );
//     SubspaceMod::set_immunity_period(netuid, tempo );

//     let keys = SubspaceMod::get_keys( netuid );
//     let uids = SubspaceMod::get_uids( netuid );
//     // do a list of ones for weights
//     let weight_uids : Vec<u16> = (0..n).collect();
//     // do a list of ones for weights
//     let mut weight_values : Vec<u16> = weight_uids.iter().map(|x| 1 as u16 ).collect();

//     let mut shuffled_uids: Vec<u16> = weight_uids.clone().to_vec();
//     shuffled_uids.shuffle(&mut thread_rng());

//     let mut zero_uids : Vec<u16> = shuffled_uids[0..num_zero_uids as usize].to_vec();

//     for uid in zero_uids.iter() {
//         weight_values[*uid as usize] = 0;

//     }
//     let old_n  : u16 = N::<Test>::get( netuid );
//     set_weights(netuid, keys[0], weight_uids.clone() , weight_values.clone() );
//     step_block( tempo );
//     let n: u16 = N::<Test>::get( netuid );
//     assert !( old_n - num_zero_uids == n );

//     });

// }

// TODO:
// #[test]
// fn test_with_weights() {
// 	new_test_ext().execute_with(|| {
// 		let n_list: Vec<u16> = vec![10, 50, 100, 1000];
// 		let blocks_per_epoch_list: u64 = 1;
// 		let stake_per_module: u64 = 10_000;

// 		for (netuid, n) in n_list.iter().enumerate() {
// 			info!("netuid: {}", netuid);
// 			let netuid: u16 = netuid as u16;
// 			let n: u16 = *n;

// 			for i in 0..n {
// 				info!("i: {}", i);
// 				info!("keys: {:?}", SubspaceMod::get_keys(netuid));
// 				info!("uids: {:?}", SubspaceMod::get_uids(netuid));
// 				let key: U256 = i;
// 				info!(
// 					"Before Registered: {:?} -> {:?}",
// 					key,
// 					SubspaceMod::key_registered(netuid, &key)
// 				);
// 				register_module(netuid, key, stake_per_module);
// 				info!(
// 					"After Registered: {:?} -> {:?}",
// 					key,
// 					SubspaceMod::key_registered(netuid, &key)
// 				);
// 			}
// 			SubspaceMod::set_tempo(netuid, 1);
// 			SubspaceMod::set_max_allowed_weights(netuid, n);
// 			let keys = SubspaceMod::get_keys(netuid);
// 			let uids = SubspaceMod::get_uids(netuid);

// 			let weight_values: Vec<u16> = (0..n).collect();
// 			let weight_uids: Vec<u16> = (0..n).collect();

// 			for i in 0..n {
// 				SubspaceMod::set_weights(
// 					get_origin(keys[i as usize]),
// 					netuid,
// 					weight_values.clone(),
// 					weight_uids.clone(),
// 				)
// 				.unwrap();
// 			}
// 			step_block(1);
// 			check_network_stats(netuid);
// 		}
// 	});
// }

#[test]
fn calculates_blocks_until_epoch() {
    new_test_ext().execute_with(|| {
        use pallet_subnet_emission::blocks_until_next_epoch;

        // Check tempo = 0 block = * netuid = *
        assert_eq!(blocks_until_next_epoch(0, 0, 0), 1000);

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
        // CONSSTANTS
        let netuid: u16 = 0;
        let n: u16 = 10;
        let stake_per_module: u64 = 10_000;

        // make sure that the results won´t get affected by burn
        zero_min_burn();

        // SETUP NETWORK
        register_n_modules(netuid, n, stake_per_module);
        let mut params = SubspaceMod::subnet_params(netuid);
        params.min_allowed_weights = 0;
        params.max_allowed_weights = n;
        params.tempo = 100;

        let keys = SubspaceMod::get_keys(netuid);
        let _uids = SubspaceMod::get_uids(netuid);

        // do a list of ones for weights
        let weight_uids: Vec<u16> = [1, 2].to_vec();
        // do a list of ones for weights
        let weight_values: Vec<u16> = [1, 1].to_vec();

        set_weights(netuid, keys[0], weight_uids.clone(), weight_values.clone());
        step_block(params.tempo);

        let incentives: Vec<u16> = Incentive::<Test>::get(netuid);
        let emissions: Vec<u64> = Emission::<Test>::get(netuid);

        // evaluate votees
        assert!(incentives[1] > 0);
        assert!(incentives[1] == incentives[2]);
        assert!(emissions[1] == emissions[2]);

        // do a list of ones for weights
        let weight_values: Vec<u16> = [1, 2].to_vec();

        set_weights(netuid, keys[0], weight_uids.clone(), weight_values.clone());
        set_weights(netuid, keys[9], weight_uids.clone(), weight_values.clone());

        step_block(params.tempo);

        let incentives: Vec<u16> = Incentive::<Test>::get(netuid);
        let emissions: Vec<u64> = Emission::<Test>::get(netuid);

        // evaluate votees
        let delta: u64 = 100 * params.tempo as u64;
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
fn test_trust() {
    new_test_ext().execute_with(|| {
        // CONSSTANTS
        let netuid: u16 = 0;
        let n: u16 = 10;
        let _n_list: Vec<u16> = vec![10, 50, 100, 1000];
        let _blocks_per_epoch_list: u64 = 1;
        let stake_per_module: u64 = 10_000;
        // make sure that the results won´t get affected by burn
        zero_min_burn();

        // SETUP NETWORK
        register_n_modules(netuid, n, stake_per_module);
        let mut params = SubspaceMod::subnet_params(netuid);
        params.min_allowed_weights = 1;
        params.max_allowed_weights = n;
        params.tempo = 100;
        params.trust_ratio = 100;

        update_params!(netuid => params.clone());

        let keys = SubspaceMod::get_keys(netuid);
        let _uids = SubspaceMod::get_uids(netuid);

        // do a list of ones for weights
        let weight_uids: Vec<u16> = [2].to_vec();
        let weight_values: Vec<u16> = [1].to_vec();

        set_weights(netuid, keys[8], weight_uids.clone(), weight_values.clone());
        // do a list of ones for weights
        let weight_uids: Vec<u16> = [1, 2].to_vec();
        let weight_values: Vec<u16> = [1, 1].to_vec();
        set_weights(netuid, keys[9], weight_uids.clone(), weight_values.clone());
        step_block(params.tempo);

        let trust: Vec<u16> = Trust::<Test>::get(netuid);
        let emission: Vec<u64> = Emission::<Test>::get(netuid);

        // evaluate votees
        info!("trust: {:?}", trust);
        assert!(trust[1] as u32 > 0);
        assert!(trust[2] as u32 > 2 * (trust[1] as u32) - 10);
        // evaluate votees
        info!("trust: {emission:?}");
        assert!(emission[1] > 0);
        assert!(emission[2] > 2 * (emission[1]) - 1000);

        // assert!(trust[2] as u32 < 2*(trust[1] as u32)   );
    });
}

// TODO:
// get back to life
// #[test]
// fn test_founder_share() {
//     new_test_ext().execute_with(|| {
//         let netuid = 0;
//         let n = 20;
//         let initial_stake: u64 = 1000;
//         let keys: Vec<U256> = (0..n).map(U256::from).collect();
//         let stakes: Vec<u64> = (0..n).map(|_x| initial_stake * 1_000_000_000).collect();

//         let founder_key = keys[0];
//         MaxRegistrationsPerBlock::<Test>::set(1000);
//         for i in 0..n {
//             assert_ok!(register_module(netuid, keys[i], stakes[i]));
//             let stake_from_vector = SubspaceMod::get_stake_to_vector(netuid, &keys[i]);
//             info!("{:?}", stake_from_vector);
//         }
//         update_params!(netuid => { founder_share: 12 });
//         let founder_share = FounderShare::<Test>::get(netuid);
//         let founder_ratio: f64 = founder_share as f64 / 100.0;

//         let subnet_params = SubspaceMod::subnet_params(netuid);

//         let founder_stake_before = Stake::<Test>::get(netuid, founder_key);
//         info!("founder_stake_before: {founder_stake_before:?}");
//         // vote to avoid key[0] as we want to see the key[0] burn
//         step_epoch(netuid);
//         let threshold = SubnetStakeThreshold::<Test>::get();
//         let total_emission =
//             SubspaceMod::calculate_network_emission(netuid, threshold) * subnet_params.tempo as
// u64;         let expected_founder_share = (total_emission as f64 * founder_ratio) as u64;
//         let expected_emission = total_emission - expected_founder_share;
//         let emissions = Emission::<Test>::get(netuid);
//         let dividends = Dividends::<Test>::get(netuid);
//         let incentives = Incentive::<Test>::get(netuid);
//         let total_dividends: u64 = dividends.iter().sum::<u16>() as u64;
//         let total_incentives: u64 = incentives.iter().sum::<u16>() as u64;

//         let founder_dividend_emission = ((dividends[0] as f64 / total_dividends as f64)
//             * (expected_emission / 2) as f64) as u64;
//         let founder_incentive_emission = ((incentives[0] as f64 / total_incentives as f64)
//             * (expected_emission / 2) as f64) as u64;
//         let founder_emission = founder_incentive_emission + founder_dividend_emission;

//         let calcualted_total_emission = emissions.iter().sum::<u64>();

//         let key_stake = Stake::<Test>::get(netuid, founder_key);
//         let founder_total_stake = founder_stake_before + founder_emission;
//         assert_eq!(
//             key_stake - (key_stake % 1000),
//             founder_total_stake - (founder_total_stake % 1000)
//         );
//         assert_eq!(
//             SubspaceMod::get_balance(&Test::get_dao_treasury_address()),
//             expected_founder_share - 1 /* Account for rounding errors */
//         );

//         assert_eq!(
//             expected_emission - (expected_emission % 100000),
//             calcualted_total_emission - (calcualted_total_emission % 100000)
//         );
//     });
// }

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
        zero_min_burn();
        max_subnet_registrations_per_interval(2);

        UnitEmission::<Test>::put(23148148148);
        FloorFounderShare::<Test>::put(0);

        // Register general subnet
        assert_ok!(register_module(0, 10, 1));

        log::info!("test_1_graph:");
        let netuid: u16 = 1;
        let key = 0;
        let uid: u16 = 0;
        let stake_amount: u64 = to_nano(100);

        assert_ok!(register_module(netuid, key, stake_amount));
        update_params!(netuid => {
            max_allowed_uids: 2
        });

        assert_ok!(register_module(netuid, key + 1, 1));
        assert_eq!(N::<Test>::get(netuid), 2);

        run_to_block(1); // run to next block to ensure weights are set on nodes after their registration block

        assert_ok!(SubspaceMod::set_weights(
            RuntimeOrigin::signed(1),
            netuid,
            vec![uid],
            vec![u16::MAX],
        ));

        let emissions = YumaEpoch::<Test>::new(netuid, ONE).run();
        let offset = 1;

        assert_eq!(
            emissions.unwrap(),
            [(ModuleKey(key), [(AccountKey(key), ONE - offset)].into())].into()
        );

        let new_stake_amount = stake_amount + ONE;

        assert_eq!(Stake::<Test>::get(&key), new_stake_amount - offset);
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
    fn add_node(netuid: u16, key: AccountId, uid: u16, stake_amount: u64) {
        log::info!(
            "+Add net:{:?} hotkey:{:?} uid:{:?} stake_amount: {:?} subn: {:?}",
            netuid,
            key,
            uid,
            stake_amount,
            N::<Test>::get(netuid),
        );

        assert_ok!(register_module(netuid, key, stake_amount));
        assert_eq!(N::<Test>::get(netuid) - 1, uid);
    }

    new_test_ext().execute_with(|| {
        max_subnet_registrations_per_interval(10);

        UnitEmission::<Test>::put(23148148148);
        zero_min_burn();
        FloorFounderShare::<Test>::put(0);
        MaxRegistrationsPerBlock::<Test>::set(1000);
        // Register general subnet
        assert_ok!(register_module(0, 10_000, 1));

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

        assert_ok!(register_module(netuid, n + 1, 1));
        assert_eq!(N::<Test>::get(netuid), 11);

        run_to_block(1); // run to next block to ensure weights are set on nodes after their registration block

        for i in 0..n {
            assert_ok!(SubspaceMod::set_weights(
                get_origin(n + 1),
                netuid,
                vec![i as u16],
                vec![u16::MAX],
            ));
        }

        let emissions = YumaEpoch::<Test>::new(netuid, ONE).run();
        let mut expected: EmissionMap<Test> = BTreeMap::new();

        // Check return values.
        let emission_per_node = ONE / n as u64;
        for i in 0..n as u16 {
            assert_eq!(
                from_nano(Stake::<Test>::get(i as u32)),
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
        max_subnet_registrations_per_interval(2);

        const MAX_WEIGHT_AGE: u64 = 300;
        const SUBNET_TEMPO: u16 = 100;
        // Register the general subnet.
        let netuid: u16 = 0;
        let key = 0;
        let stake_amount: u64 = to_nano(1_000);

        assert_ok!(register_module(netuid, key, stake_amount));

        // Register the yuma subnet.
        let yuma_netuid: u16 = 1;
        let yuma_validator_key = 1;
        let yuma_miner_key = 2;
        let yuma_vali_amount: u64 = to_nano(10_000);
        let yuma_miner_amount = to_nano(1_000);

        // This will act as an validator.
        assert_ok!(register_module(
            yuma_netuid,
            yuma_validator_key,
            yuma_vali_amount
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

        let miner_uid = SubspaceMod::get_uid_for_key(yuma_netuid, &yuma_miner_key);
        let validator_uid = SubspaceMod::get_uid_for_key(yuma_netuid, &yuma_validator_key);
        let uid = [miner_uid].to_vec();
        let weight = [1].to_vec();

        // set the weights
        assert_ok!(SubspaceMod::do_set_weights(
            get_origin(yuma_validator_key),
            yuma_netuid,
            uid,
            weight
        ));

        step_block(100);

        // Make sure we have incentive and dividends
        let miner_incentive = SubspaceMod::get_incentive_for_uid(yuma_netuid, miner_uid);
        let miner_dividends = SubspaceMod::get_dividends_for_uid(yuma_netuid, miner_uid);
        let validator_incentive = SubspaceMod::get_incentive_for_uid(yuma_netuid, validator_uid);
        let validator_dividends = SubspaceMod::get_dividends_for_uid(yuma_netuid, validator_uid);

        assert!(miner_incentive > 0);
        assert_eq!(miner_dividends, 0);
        assert!(validator_dividends > 0);
        assert_eq!(validator_incentive, 0);

        // now go pass the max weight age
        step_block(MAX_WEIGHT_AGE as u16);

        // Make sure we have no incentive and dividends
        let miner_incentive = SubspaceMod::get_incentive_for_uid(yuma_netuid, miner_uid);
        let miner_dividends = SubspaceMod::get_dividends_for_uid(yuma_netuid, miner_uid);
        let validator_incentive = SubspaceMod::get_incentive_for_uid(yuma_netuid, validator_uid);
        let validator_dividends = SubspaceMod::get_dividends_for_uid(yuma_netuid, validator_uid);

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
        max_subnet_registrations_per_interval(10);

        const SUBNET_TEMPO: u16 = 25;
        // Register the general subnet.
        let netuid: u16 = 0;
        let key = 0;
        let stake_amount: u64 = to_nano(1_000);

        // Make sure registration cost is not affected
        zero_min_burn();

        assert_ok!(register_module(netuid, key, stake_amount));

        // Register the yuma subnet.
        let yuma_netuid: u16 = 1;
        let yuma_badactor_key = 1;
        let yuma_badactor_amount: u64 = to_nano(10_000);

        assert_ok!(register_module(
            yuma_netuid,
            yuma_badactor_key,
            yuma_badactor_amount
        ));
        update_params!(netuid => { tempo: SUBNET_TEMPO });

        // step first 40 blocks from the registration
        step_block(40);

        let stake_accumulated = Stake::<Test>::get(yuma_badactor_key as u32);
        // User will now unstake and register another subnet.
        assert_ok!(SubspaceMod::do_remove_stake(
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
        assert_ok!(SubspaceMod::register(
            origin,
            network,
            name,
            address,
            yuma_badactor_amount - 1,
            yuma_badactor_key,
            None
        ));

        // set the tempo
        update_params!(netuid => { tempo: SUBNET_TEMPO });

        // now 100 blocks went by since the registration, 1 + 40 + 58 = 100
        step_block(58);

        // remove the stake again
        let stake_accumulated_two = Stake::<Test>::get(yuma_badactor_key);
        assert_ok!(SubspaceMod::do_remove_stake(
            get_origin(yuma_badactor_key),
            yuma_badactor_key,
            stake_accumulated_two - 2
        ));

        let badactor_balance_after = SubspaceMod::get_balance(&yuma_badactor_key);

        let new_netuid = 3;
        // Now an honest actor will come, the goal is for him to accumulate more
        let honest_actor_key = 3;
        assert_ok!(register_module(
            new_netuid,
            honest_actor_key,
            yuma_badactor_amount
        ));
        // we will set a slower tempo, standard 100
        update_params!(new_netuid => { tempo: 100 });
        step_block(101);

        // get the stake of honest actor
        let hones_stake = Stake::<Test>::get(honest_actor_key);
        assert!(hones_stake > badactor_balance_after);
    });
}

#[test]
fn test_tempo_compound() {
    new_test_ext().execute_with(|| {
        zero_min_burn();
        max_subnet_registrations_per_interval(3);

        const QUICK_TEMPO: u16 = 25;
        const SLOW_TEMPO: u16 = 1000;
        // Register the general subnet.
        let netuid: u16 = 0;
        let key = 0;
        let stake_amount: u64 = to_nano(1_000);

        assert_ok!(register_module(netuid, key, stake_amount));

        // Register the yuma subnets, the important part of the tests starts here:
        // FAST
        let s_netuid: u16 = 1;
        let s_key = 1;
        let s_amount: u64 = to_nano(10_000);

        assert_ok!(register_module(s_netuid, s_key, s_amount));
        update_params!(s_netuid => { tempo: SLOW_TEMPO });

        // SLOW
        let f_netuid = 2;
        // Now an honest actor will come, the goal is for him to accumulate more
        let f_key = 3;
        assert_ok!(register_module(f_netuid, f_key, s_amount));
        // we will set a slower tempo
        update_params!(f_netuid => { tempo: QUICK_TEMPO });

        // we will now step, SLOW_TEMPO -> 1000 blocks
        step_block(SLOW_TEMPO);

        let fast = Stake::<Test>::get(f_key);
        let slow = Stake::<Test>::get(s_key);

        // faster tempo should have quicker compound rate
        assert!(fast > slow);
    });
}
