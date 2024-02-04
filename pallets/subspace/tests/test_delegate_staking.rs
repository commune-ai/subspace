use frame_support::{assert_ok, traits::Currency};
use frame_system::Config;
mod test_mock;
use frame_support::{
	dispatch::{DispatchClass, DispatchInfo, GetDispatchInfo, Pays},
	sp_runtime::DispatchError,
};
use pallet_subspace::Error;
use sp_core::U256;
use test_mock::*;

use substrate_fixed::types::{I32F32, I64F64};
// /***********************************************************
// 	staking::add_stake() tests
// ************************************************************/
// #[test]
// fn test_stake_overflow() {
// 	new_test_ext().execute_with(|| {

//         let token_amount : u64 = 1_000_000_000;
//         let balance : u64 = 10 * token_amount;
//         let netuid : u16 = 0;

//         for i in [0,1].iter() {
//             let delta : u64 = 1 * token_amount;
//             let stake : u64 = balance + delta*(*i);
//             let key : U256 = U256::from(*i);
//             add_balance(key, balance);
//             let result =register_module(netuid, key, stake);
//             println!("RESULT: {:?}", result);

//             println!("STAKE {}", SubspaceModule::get_stake(netuid, &key));
//             assert_eq!(SubspaceModule::get_stake(netuid, &key), balance);
//             assert_eq!(SubspaceModule::get_balance(&key), 0);
//         }

// 	});
// }

#[test]
fn test_ownership_ratio() {
	new_test_ext().execute_with(|| {
		let netuid: u16 = 0;
		let num_modules: u16 = 10;
		let tempo = 1;
		let stake_per_module: u64 = 1_000_000_000;
		register_n_modules(netuid, num_modules, stake_per_module);
		SubspaceModule::set_tempo(netuid, tempo);

		let keys = SubspaceModule::get_keys(netuid);
		let voter_key = keys[0];
		let miner_keys = keys[1..].to_vec();
		let miner_uids: Vec<u16> =
			miner_keys.iter().map(|k| SubspaceModule::get_uid_for_key(netuid, k)).collect();
		let miner_weights = vec![1; miner_uids.len()];

		let delegate_keys: Vec<U256> =
			(0..num_modules).map(|i| U256::from(i + num_modules + 1)).collect();
		for d in delegate_keys.iter() {
			add_balance(*d, stake_per_module + 1);
		}

		let pre_delegate_stake_from_vector =
			SubspaceModule::get_stake_from_vector(netuid, &voter_key);
		assert_eq!(pre_delegate_stake_from_vector.len(), 1); // +1 for the module itself, +1 for the delegate key on

		for (i, d) in delegate_keys.iter().enumerate() {
			println!("DELEGATE KEY: {}", d);
			assert_ok!(SubspaceModule::add_stake(
				get_origin(*d),
				netuid,
				voter_key,
				stake_per_module
			));
			let stake_from_vector = SubspaceModule::get_stake_from_vector(netuid, &voter_key);
			assert_eq!(stake_from_vector.len(), pre_delegate_stake_from_vector.len() + i + 1);
		}
		let ownership_ratios: Vec<(U256, I64F64)> =
			SubspaceModule::get_ownership_ratios(netuid, &voter_key);
		assert_eq!(ownership_ratios.len(), delegate_keys.len() + 1);

		let founder_tokens_before = SubspaceModule::get_balance(&voter_key) +
			SubspaceModule::get_stake_to_module(netuid, &voter_key, &voter_key);

		let delegate_balances_before = delegate_keys
			.iter()
			.map(|k| SubspaceModule::get_balance(k))
			.collect::<Vec<u64>>();
		let delegate_stakes_before = delegate_keys
			.iter()
			.map(|k| SubspaceModule::get_stake_to_module(netuid, &k, &voter_key))
			.collect::<Vec<u64>>();
		let delegate_total_tokens_before = delegate_balances_before
			.iter()
			.zip(delegate_stakes_before.clone().into_iter())
			.map(|(a, x)| a + x)
			.fold(0, |acc, x| acc + x);

		let total_balance = keys
			.iter()
			.map(|k| SubspaceModule::get_balance(k))
			.collect::<Vec<u64>>()
			.iter()
			.fold(0, |acc, x| acc + x);
		let total_stake = keys
			.iter()
			.map(|k| SubspaceModule::get_stake_to_module(netuid, &k, &k))
			.collect::<Vec<u64>>()
			.iter()
			.fold(0, |acc, x| acc + x);
		let total_delegate_stake = delegate_keys
			.iter()
			.map(|k| SubspaceModule::get_stake_to_module(netuid, &k, &voter_key))
			.collect::<Vec<u64>>()
			.iter()
			.fold(0, |acc, x| acc + x);
		let total_delegate_balance = delegate_keys
			.iter()
			.map(|k| SubspaceModule::get_balance(k))
			.collect::<Vec<u64>>()
			.iter()
			.fold(0, |acc, x| acc + x);
		let total_tokens_before =
			total_balance + total_stake + total_delegate_stake + total_delegate_balance;
		println!("total_tokens_before: {:?}", total_tokens_before);

		println!("delegate_balances before: {:?}", delegate_balances_before);
		println!("delegate_stakes before: {:?}", delegate_stakes_before);
		println!("delegate_total_tokens before: {:?}", delegate_total_tokens_before);

		let result = SubspaceModule::set_weights(
			get_origin(voter_key),
			netuid,
			miner_uids.clone(),
			miner_weights.clone(),
		);

		assert_ok!(result);

		step_epoch(netuid);

		let dividends = SubspaceModule::get_dividends(netuid);
		let incentives = SubspaceModule::get_incentives(netuid);
		let emissions = SubspaceModule::get_emissions(netuid);

		println!("dividends: {:?}", dividends);
		println!("incentives: {:?}", incentives);
		println!("emissions: {:?}", emissions);
		let total_emissions = emissions.iter().fold(0, |acc, x| acc + x);

		println!("total_emissions: {:?}", total_emissions);

		let delegate_balances = delegate_keys
			.iter()
			.map(|k| SubspaceModule::get_balance(k))
			.collect::<Vec<u64>>();
		let delegate_stakes = delegate_keys
			.iter()
			.map(|k| SubspaceModule::get_stake_to_module(netuid, &k, &voter_key))
			.collect::<Vec<u64>>();
		let delegate_total_tokens = delegate_balances
			.iter()
			.zip(delegate_stakes.clone().into_iter())
			.map(|(a, x)| a + x)
			.fold(0, |acc, x| acc + x);
		let founder_tokens = SubspaceModule::get_balance(&voter_key) +
			SubspaceModule::get_stake_to_module(netuid, &voter_key, &voter_key);
		let founder_new_tokens = founder_tokens - founder_tokens_before;
		let delegate_new_tokens: Vec<u64> = delegate_stakes
			.iter()
			.zip(delegate_stakes_before.clone().into_iter())
			.map(|(a, x)| a - x)
			.collect::<Vec<u64>>();

		let total_new_tokens =
			founder_new_tokens + delegate_new_tokens.iter().fold(0, |acc, x| acc + x);

		println!("owner_ratios: {:?}", ownership_ratios);
		println!("total_new_tokens: {:?}", total_new_tokens);
		println!("founder_tokens: {:?}", founder_tokens);
		println!("delegate_balances: {:?}", delegate_balances);
		println!("delegate_stakes: {:?}", delegate_stakes);
		println!("delegate_total_tokens: {:?}", delegate_total_tokens);
		println!("founder_new_tokens: {:?}", founder_new_tokens);
		println!("delegate_new_tokens: {:?}", delegate_new_tokens);

		let total_balance = keys
			.iter()
			.map(|k| SubspaceModule::get_balance(k))
			.collect::<Vec<u64>>()
			.iter()
			.fold(0, |acc, x| acc + x);
		let total_stake = keys
			.iter()
			.map(|k| SubspaceModule::get_stake_to_module(netuid, &k, &k))
			.collect::<Vec<u64>>()
			.iter()
			.fold(0, |acc, x| acc + x);
		let total_delegate_stake = delegate_keys
			.iter()
			.map(|k| SubspaceModule::get_stake_to_module(netuid, &k, &voter_key))
			.collect::<Vec<u64>>()
			.iter()
			.fold(0, |acc, x| acc + x);
		let total_delegate_balance = delegate_keys
			.iter()
			.map(|k| SubspaceModule::get_balance(k))
			.collect::<Vec<u64>>()
			.iter()
			.fold(0, |acc, x| acc + x);
		let total_tokens_after =
			total_balance + total_stake + total_delegate_stake + total_delegate_balance;
		let total_new_tokens = total_tokens_after - total_tokens_before;
		println!("total_tokens_after: {:?}", total_tokens_before);
		println!("total_new_tokens: {:?}", total_new_tokens);
		assert_eq!(total_new_tokens, total_emissions);

		let stake_from_vector = SubspaceModule::get_stake_from_vector(netuid, &voter_key);
		let stake: u64 = SubspaceModule::get_stake(netuid, &voter_key);
		let sumed_stake: u64 = stake_from_vector.iter().fold(0, |acc, (a, x)| acc + x);
		let total_stake: u64 = SubspaceModule::get_total_subnet_stake(netuid);
		println!("stake_from_vector: {:?}", stake_from_vector);
	});
}
