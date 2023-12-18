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
		let stake_per_module: u64 = 1_000_000_000;
		register_n_modules(netuid, num_modules, stake_per_module);

		let keys = SubspaceModule::get_keys(netuid);
		let voter_key = keys[0];
		let miner_keys = keys[1..].to_vec();
		let miner_uids: Vec<u16> = miner_keys.iter().map(|k| SubspaceModule::get_uid_for_key(netuid, k)).collect();
		let miner_weights = vec![1; miner_uids.len()];

		let result = SubspaceModule::set_weights(
			get_origin(voter_key),
			netuid,
			miner_uids.clone(),
			miner_weights.clone(),
		);

		assert_ok!(result);

		let delegate_keys: Vec<U256> =
			(0..num_modules).map(|i| U256::from(i + num_modules + 1)).collect();
		for d in delegate_keys.iter() {
			add_balance(*d, stake_per_module + 1);
		}

		let pre_delegate_stake_from_vector = SubspaceModule::get_stake_from_vector(netuid, &voter_key);
		assert_eq!(pre_delegate_stake_from_vector.len(), 1); // +1 for the module itself, +1 for the delegate key on

		for (i, d) in delegate_keys.iter().enumerate() {
			println!("DELEGATE KEY: {}", d);
			assert_ok!(SubspaceModule::add_stake(get_origin(*d), netuid, voter_key, stake_per_module));
			let stake_from_vector = SubspaceModule::get_stake_from_vector(netuid, &voter_key);
			assert_eq!(stake_from_vector.len(), pre_delegate_stake_from_vector.len() + i + 1);
		}
		let ownership_ratios: Vec<(U256, I64F64)> =SubspaceModule::get_ownership_ratios(netuid, &voter_key);
		assert_eq!(ownership_ratios.len(), delegate_keys.len() + 1);

		step_epoch(netuid);

		let dividends = SubspaceModule::get_dividends(netuid);
		let incentives = SubspaceModule::get_incentives(netuid);
		let emissions = SubspaceModule::get_emissions(netuid);

		println!("dividends: {:?}", dividends);
		println!("incentives: {:?}", incentives);
		println!("emissions: {:?}", emissions);

		// assert_eq!(dividends.len(), 0);

		let stake_from_vector = SubspaceModule::get_stake_from_vector(netuid, &voter_key);
		let stake: u64 = SubspaceModule::get_stake(netuid, &voter_key);
		let sumed_stake: u64 = stake_from_vector.iter().fold(0, |acc, (a, x)| acc + x);
		let total_stake: u64 = SubspaceModule::get_total_subnet_stake(netuid);

		

	});
}