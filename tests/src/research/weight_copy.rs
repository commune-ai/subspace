// use crate::mock::*;
// use csv;
// use frame_support::{
//     storage::{self},
//     traits::PalletInfoAccess,
// };

// use pallet_offworker::ConsensusSimulationResult;
// use pallet_subnet_emission::{
//     subnet_consensus::yuma::{YumaEpoch, YumaParams},
//     SubnetConsensusType,
// };
// use pallet_subnet_emission_api::SubnetConsensus;
// use pallet_subspace::{
//     BondsMovingAverage, CopierMargin, FounderShare, LastUpdate, MaxAllowedUids, MaxAllowedWeights,
//     MaxEncryptionPeriod, MaxRegistrationsPerBlock, MaxWeightAge, RegistrationBlock, Tempo,
//     UseWeightsEncrytyption, ValidatorPermits, Weights, N,
// };
// use serde_json::Value;
// use sp_runtime::Percent;
// use std::{fs::File, io::Read, path::PathBuf};
// use substrate_fixed::types::I64F64;

// const JSON_NETUID: &str = "1";
// // todo: look at the passing of copier stake
// // dbg copier and avg delta
// #[test]
// fn test_backtest_simulation_with_copier_margins() {
//     new_test_ext().execute_with(|| {
//         // TEST SETTINGS
//         const TEST_NETUID: u16 = 0;
//         const TEMPO: u64 = 100;
//         const UNIVERSAL_PENDING_EMISSION: u64 = to_nano(1000);
//         const DELEGATION_FEE: Percent = Percent::from_percent(0);
//         // BACKTEST SETTINGS
//         const MAX_EPOCHS: usize = 50;
//         let file_name = format!("simulation_results_{}.csv", JSON_NETUID,);

//         // Create a CSV writer
//         let mut wtr = csv::Writer::from_path(file_name).expect("Failed to create CSV writer");

//         // Write CSV header
//         wtr.write_record(&[
//             "Copier Margin",
//             "Block Number",
//             "Cumulative Copier Dividends",
//             "Cumulative Average Delegate Dividends",
//             "Gini Coefficient",
//             "Encryption Window",
//             "Black Box Age",
//         ])
//         .expect("Failed to write CSV header");

//         let json = load_json_data();
//         let first_block =
//             json["weights"].as_object().unwrap().keys().next().unwrap().parse().unwrap();
//         // Loop through different copier margin values
//         for margin in 0..=0 {
//             let pallet_name = <SubspaceMod as PalletInfoAccess>::name().as_bytes();

//             let pallet_prefix = sp_io::hashing::twox_128(pallet_name);
//             let _ = storage::unhashed::clear_prefix(&pallet_prefix, None, None);
//             dbg!(N::<Test>::get(TEST_NETUID));

//             // REGISTER AND SETUP SUBNET
//             setup_subnet(TEST_NETUID, TEMPO);

//             // REGISTER MODULES, AND OVERWRITE THEM TO MAINNET STATE
//             // Register at genesis
//             register_modules_from_json(&json, TEST_NETUID);

//             let copier_uid: u16 = N::<Test>::get(TEST_NETUID);

//             dbg!(&margin);
//             // Set block number the simulation will start from
//             System::set_block_number(first_block);
//             // Overwrite last update and registration blocks
//             make_parameter_consensus_overwrites(TEST_NETUID, first_block, &json, None);

//             let copier_margin = I64F64::from_num(margin as f64 / 100.0);
//             CopierMargin::<Test>::set(TEST_NETUID, copier_margin);

//             // copier will set perfectly in consensus weight
//             let copier_uid = setup_copier(
//                 TEST_NETUID,
//                 &json,
//                 first_block,
//                 JSON_NETUID,
//                 UNIVERSAL_PENDING_EMISSION,
//                 copier_uid,
//             );

//             let (simulation_result, iteration_counter) = run_simulation(
//                 TEST_NETUID,
//                 copier_uid,
//                 &json,
//                 first_block,
//                 JSON_NETUID,
//                 TEMPO,
//                 DELEGATION_FEE,
//                 MAX_EPOCHS,
//                 UNIVERSAL_PENDING_EMISSION,
//                 &mut wtr,
//                 copier_margin,
//             );

//             assert!(
//                 pallet_offworker::is_copying_irrational(simulation_result.clone())
//                     || iteration_counter >= MAX_EPOCHS,
//                 "Copying should have become irrational or max iterations should have been reached"
//             );
//         }

//         wtr.flush().expect("Failed to flush CSV writer");
//     });
// }

// fn _clear_out_data() {
//     // System::reset_events();
//     let _ = Weights::<Test>::clear(u32::MAX, None);
//     // let _ = ValidatorPermits::<Test>::clear(u32::MAX, None);
//     let _ = LastUpdate::<Test>::clear(u32::MAX, None);
//     // let _ = RegistrationBlock::<Test>::clear(u32::MAX, None);
// }

// fn setup_subnet(netuid: u16, tempo: u64) {
//     register_subnet(u32::MAX, 0).unwrap();
//     zero_min_burn();
//     SubnetConsensusType::<Test>::set(netuid, Some(SubnetConsensus::Yuma));
//     Tempo::<Test>::insert(netuid, tempo as u16);
//     BondsMovingAverage::<Test>::insert(netuid, 0);
//     UseWeightsEncrytyption::<Test>::set(netuid, true);

//     // Things that should never expire / exceed
//     MaxWeightAge::<Test>::set(netuid, 300_000);
//     MaxEncryptionPeriod::<Test>::set(netuid, u64::MAX);
//     MaxRegistrationsPerBlock::<Test>::set(u16::MAX);
//     MaxAllowedUids::<Test>::set(netuid, u16::MAX);
//     MaxAllowedWeights::<Test>::set(netuid, u16::MAX);
//     FounderShare::<Test>::set(netuid, 0);
// }

// fn setup_copier(
//     netuid: u16,
//     json: &Value,
//     first_block: u64,
//     json_netuid: &str,
//     universal_pending_emission: u64,
//     copier_uid: u16,
// ) -> u16 {
//     let v_permits = get_validator_permits(first_block, json);

//     insert_weights_for_block(
//         &json["weights"][&first_block.to_string()][json_netuid],
//         netuid,
//         v_permits,
//     );

//     let last_params = YumaParams::<Test>::new(netuid, universal_pending_emission).unwrap();
//     let last_output = YumaEpoch::<Test>::new(netuid, last_params).run().unwrap();
//     last_output.clone().apply();

//     // Copier set perfectly in consensus weight
//     let consensus = last_output.consensus;
//     add_weight_copier(
//         netuid,
//         copier_uid as u32,
//         (0..consensus.len() as u16).collect(),
//         consensus,
//     );

//     copier_uid
// }
// fn run_simulation(
//     netuid: u16,
//     copier_uid: u16,
//     json: &Value,
//     first_block: u64,
//     json_netuid: &str,
//     tempo: u64,
//     delegation_fee: Percent,
//     _max_epochs: usize,
//     universal_pending_emission: u64,
//     wtr: &mut csv::Writer<std::fs::File>,
//     copier_margin: I64F64,
// ) -> (ConsensusSimulationResult<Test>, usize) {
//     let mut simulation_result = ConsensusSimulationResult::default();
//     let mut iter_counter = 0;

//     let mut copier_last_update = SubspaceMod::get_last_update_for_uid(netuid, copier_uid);

//     let mut current_window_data: Vec<f64> = Vec::new();
//     let mut encryption_window = 0;

//     for (block_number, block_weights) in json["weights"].as_object().unwrap() {
//         let block_number: u64 = block_number.parse().unwrap();
//         if block_number == first_block {
//             continue;
//         }

//         System::set_block_number(block_number);
//         make_parameter_consensus_overwrites(netuid, block_number, &json, Some(copier_last_update));

//         let weights: &Value = &block_weights[json_netuid];

//         let mut v_permits = get_validator_permits(block_number, json);
//         v_permits.push(true);

//         insert_weights_for_block(weights, netuid, v_permits);

//         let last_params = YumaParams::<Test>::new(netuid, universal_pending_emission).unwrap();
//         let last_output = YumaEpoch::<Test>::new(netuid, last_params).run().unwrap();
//         last_output.clone().apply();
//         simulation_result.update(last_output.clone(), tempo, copier_uid, delegation_fee);

//         // Calculate the percent difference between Dc(e) and Dd(e) for this epoch
//         let dc = simulation_result.cumulative_copier_divs.to_num::<f64>();
//         let dd = simulation_result.cumulative_avg_delegate_divs.to_num::<f64>();
//         let diff = (dc - dd).abs();

//         current_window_data.push(diff);

//         if pallet_offworker::is_copying_irrational(simulation_result.clone()) {
//             println!(
//                 "Copying became irrational at block {} with copier margin {:?}",
//                 block_number, copier_margin
//             );
//             encryption_window += 1;

//             let black_box_age = simulation_result.black_box_age / tempo;
//             let gini = calculate_gini(&current_window_data);

//             dbg!(
//                 block_number,
//                 copier_margin,
//                 dc,
//                 dd,
//                 gini,
//                 encryption_window,
//                 black_box_age
//             );
//             // Write to CSV
//             wtr.write_record(&[
//                 format!("{:?}", copier_margin),
//                 block_number.to_string(),
//                 format!("{:?}", dc),
//                 format!("{:?}", dd),
//                 format!("{:.4}", gini),
//                 encryption_window.to_string(),
//                 black_box_age.to_string(),
//             ])
//             .expect("Failed to write CSV row");

//             current_window_data.clear();

//             // Setup copier for next window
//             let mut values = last_output.consensus;
//             // filter last weight
//             values.pop();

//             // setup copier
//             let copier_acc = SubspaceMod::get_key_for_uid(netuid, copier_uid).unwrap();
//             let copier_stake = get_copier_stake(netuid);
//             make_keys_all_stake_be(copier_acc, copier_stake);

//             let uids: Vec<u16> = (0..values.len() as u16).collect();
//             let zipped_weights: Vec<(u16, u16)> = uids.iter().copied().zip(values).collect();

//             let copy_l_u = block_number - 100;
//             // update copiers last update
//             let latest_last_update = LastUpdate::<Test>::get(netuid);
//             let mut last_update_vec = latest_last_update.clone();
//             last_update_vec.push(copy_l_u);
//             LastUpdate::<Test>::set(netuid, last_update_vec);
//             copier_last_update = copy_l_u;

//             Weights::<Test>::insert(netuid, copier_uid, zipped_weights);

//             // Reset the simulation result
//             simulation_result = ConsensusSimulationResult::default();
//             dbg!(iter_counter);
//         }
//         iter_counter += 1;
//     }

//     (simulation_result, iter_counter)
// }

// fn calculate_gini(values: &[f64]) -> f64 {
//     if values.is_empty() {
//         return 0.0;
//     }

//     let n = values.len() as f64;
//     let mean = values.iter().sum::<f64>() / n;

//     if mean == 0.0 {
//         return 0.0;
//     }

//     let mut sum_of_abs_differences = 0.0;

//     for &value in values {
//         for &other_value in values {
//             sum_of_abs_differences += (value - other_value).abs();
//         }
//     }

//     sum_of_abs_differences / (2.0 * n * n * mean)
// }
// fn load_json_data() -> Value {
//     let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
//     let file_name = format!(
//         "/root/subnet_snapshots/sn{}_weights_stake.json",
//         JSON_NETUID
//     );
//     path.push(file_name);
//     let mut file = File::open(path).expect("Failed to open weights_stake.json");
//     let mut contents = String::new();
//     file.read_to_string(&mut contents).expect("Failed to read file");
//     serde_json::from_str(&contents).expect("Failed to parse JSON")
// }

// // see if sorting might not be problmeatic
// fn register_modules_from_json(json: &Value, netuid: u16) {
//     if let Some(stake_map) = json["stake"].as_object() {
//         let mut sorted_uids: Vec<u16> =
//             stake_map.keys().filter_map(|uid_str| uid_str.parse::<u16>().ok()).collect();
//         sorted_uids.sort_unstable();

//         for uid in sorted_uids {
//             if let Some(stake_value) = stake_map.get(&uid.to_string()) {
//                 let stake: u64 = stake_value.as_u64().expect("Failed to parse stake value");
//                 register_module(netuid, uid as u32, stake, false).unwrap();
//             }
//         }
//     }
// }

// fn insert_weights_for_block(block_weights: &Value, netuid: u16, permits: Vec<bool>) {
//     let uids: Vec<u16> = block_weights
//         .as_object()
//         .unwrap()
//         .keys()
//         .filter_map(|uid_str| uid_str.parse::<u16>().ok())
//         .collect();

//     for uid in uids {
//         if let Some(weights) = block_weights[&uid.to_string()].as_array() {
//             let weight_vec: Vec<(u16, u16)> = weights
//                 .iter()
//                 .filter_map(|w| {
//                     let pair = w.as_array()?;
//                     Some((pair[0].as_u64()? as u16, pair[1].as_u64()? as u16))
//                 })
//                 .collect();

//             let (uids, values): (Vec<u16>, Vec<u16>) = weight_vec.into_iter().unzip();
//             let zipped_weights: Vec<(u16, u16)> = uids.iter().copied().zip(values).collect();

//             if !zipped_weights.is_empty() && zipped_weights.iter().any(|(_, value)| *value != 0) {
//                 ValidatorPermits::<Test>::insert(netuid, permits.clone());
//                 Weights::<Test>::insert(netuid, uid, zipped_weights);
//             }
//         }
//     }
// }

// fn get_value_for_block(module: &str, block_number: u64, json: &Value) -> Vec<u64> {
//     let stuff = json[module].as_object().unwrap();
//     let stuff_vec: Vec<u64> = stuff[&block_number.to_string()]
//         .as_object()
//         .unwrap()
//         .values()
//         .filter_map(|v| v.as_u64())
//         .collect();
//     stuff_vec
// }

// fn get_validator_permits(block_number: u64, json: &Value) -> Vec<bool> {
//     let stuff = json["validator_permits"].as_object().unwrap();
//     let block_data = stuff[&block_number.to_string()].as_object().unwrap();

//     let mut stuff_vec: Vec<bool> = Vec::new();
//     for i in 0..block_data.len() {
//         if let Some(value) = block_data.get(&i.to_string()) {
//             stuff_vec.push(value.as_bool().unwrap_or(false));
//         }
//     }

//     stuff_vec
// }

// fn make_parameter_consensus_overwrites(
//     netuid: u16,
//     block: u64,
//     json: &Value,
//     copier_last_update: Option<u64>,
// ) {
//     // Overwrite the last update completelly to the mainnet state
//     let mut last_update_vec = get_value_for_block("last_update", block, &json);
//     if let Some(copier_last_update) = copier_last_update {
//         last_update_vec.push(copier_last_update);
//     }

//     LastUpdate::<Test>::set(netuid, last_update_vec);

//     // Overwrite the registration block
//     let registration_blocks_vec = get_value_for_block("registration_blocks", block, &json);
//     for (i, block) in registration_blocks_vec.iter().enumerate() {
//         RegistrationBlock::<Test>::set(netuid, i as u16, *block);
//     }
// }
