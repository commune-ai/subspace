mod test_mock;
use sp_core::U256;
use test_mock::*;

use substrate_fixed::types::I64F64;

#[test]
fn test_stake() {
	new_test_ext().execute_with(|| {
		let max_uids: u16 = 10;
		let token_amount: u64 = 1_000_000_000;
		let netuids: Vec<u16> = [0, 1, 2, 3].to_vec();
		let amount_staked_vector: Vec<u64> = netuids.iter().map(|i| 10 * token_amount).collect();
		let mut total_stake: u64 = 0;
		let mut netuid: u16 = 0;
		let mut subnet_stake: u64 = 0;
		let mut uid: u16 = 0;

		for i in netuids.iter() {
			netuid = *i;

			println!("NETUID: {}", netuid);

			let amount_staked = amount_staked_vector[netuid as usize];
			let key_vector: Vec<U256> =
				(0..max_uids).map(|i| U256::from(i + max_uids * netuid)).collect();

			for key in key_vector.iter() {
				println!(
					" KEY {} KEY STAKE {} STAKING AMOUNT {} ",
					key,
					SubspaceModule::get_stake(netuid, key),
					amount_staked
				);

				register_module(netuid, *key, amount_staked).ok();
				
				// add_stake_and_balance(netuid, *key, amount_staked);
				println!(
					" KEY STAKE {} STAKING AMOUNT {} ",
					SubspaceModule::get_stake(netuid, key),
					amount_staked
				);

				uid = SubspaceModule::get_uid_for_key(netuid, &key);

				// SubspaceModule::add_stake(get_origin(*key), netuid, amount_staked);
				assert_eq!(SubspaceModule::get_stake(netuid, key), amount_staked);
				assert_eq!(SubspaceModule::get_balance(key), 1);

				// REMOVE STAKE
				SubspaceModule::remove_stake(get_origin(*key), netuid, *key, amount_staked).ok();
				assert_eq!(SubspaceModule::get_balance(key), amount_staked + 1);
				assert_eq!(SubspaceModule::get_stake(netuid, key), 0);

				// ADD STAKE AGAIN LOL
				SubspaceModule::add_stake(get_origin(*key), netuid, *key, amount_staked).ok();
				assert_eq!(SubspaceModule::get_stake(netuid, key), amount_staked);
				assert_eq!(SubspaceModule::get_balance(key), 1);

				// AT THE END WE SHOULD HAVE THE SAME TOTAL STAKE
				subnet_stake += SubspaceModule::get_stake(netuid, key).clone();
			}

			assert_eq!(SubspaceModule::get_total_subnet_stake(netuid), subnet_stake);

			total_stake += subnet_stake.clone();

			assert_eq!(SubspaceModule::total_stake(), total_stake);

			subnet_stake = 0;

			println!("TOTAL STAKE: {}", total_stake);
			println!("TOTAL SUBNET STAKE: {}", SubspaceModule::get_total_subnet_stake(netuid));
		}
	});
}

#[test]
fn test_multiple_stake() {
	new_test_ext().execute_with(|| {
		let n: u16 = 10;
		let stake_amount: u64 = 10_000_000_000;
		let mut netuid: u16 = 0;
		let num_staked_modules: u16 = 10;
		let total_stake: u64 = stake_amount * num_staked_modules as u64;

		register_n_modules(netuid, n, 0);

		let controler_key = U256::from(n + 1);
		let og_staker_balance: u64 = total_stake + 1;

		add_balance(controler_key.clone(), og_staker_balance);

		let keys: Vec<U256> = SubspaceModule::get_keys(netuid);

		// stake to all modules

		let stake_amounts: Vec<u64> = vec![stake_amount; num_staked_modules as usize];

		println!("STAKE AMOUNTS: {:?}", stake_amounts);
		let total_actual_stake: u64 =
			keys.clone().into_iter().map(|k| SubspaceModule::get_stake(netuid, &k)).sum();
		let staker_balance = SubspaceModule::get_balance(&controler_key);
		println!("TOTAL ACTUAL STAKE: {}", total_actual_stake);
		println!("TOTAL STAKE: {}", total_stake);
		println!("STAKER BALANCE: {}", staker_balance);

		SubspaceModule::add_stake_multiple(
			get_origin(controler_key),
			netuid,
			keys.clone(),
			stake_amounts.clone(),
		)
		.ok();

		let total_actual_stake: u64 =
			keys.clone().into_iter().map(|k| SubspaceModule::get_stake(netuid, &k)).sum();
		let staker_balance = SubspaceModule::get_balance(&controler_key);

		assert_eq!(
			total_actual_stake, total_stake,
			"total stake should be equal to the sum of all stakes"
		);
		assert_eq!(staker_balance, og_staker_balance - total_stake, "staker balance should be 0");

		// unstake from all modules
		SubspaceModule::remove_stake_multiple(
			get_origin(controler_key),
			netuid,
			keys.clone(),
			stake_amounts.clone(),
		)
		.ok();

		let total_actual_stake: u64 =
			keys.clone().into_iter().map(|k| SubspaceModule::get_stake(netuid, &k)).sum();
		let staker_balance = SubspaceModule::get_balance(&controler_key);
		assert_eq!(total_actual_stake, 0, "total stake should be equal to the sum of all stakes");
		assert_eq!(staker_balance, og_staker_balance, "staker balance should be 0");
	});
}

#[test]
fn test_transfer_stake() {
	new_test_ext().execute_with(|| {
		let n: u16 = 10;
		let stake_amount: u64 = 10_000_000_000;
		let mut netuid: u16 = 0;
		let num_staked_modules: u16 = 10;

		register_n_modules(netuid, n, stake_amount);

		let keys: Vec<U256> = SubspaceModule::get_keys(netuid);

		SubspaceModule::transfer_stake(get_origin(keys[0]), netuid, keys[0], keys[1], stake_amount)
			.ok();

		let key0_stake = SubspaceModule::get_stake(netuid, &keys[0]);
		let key1_stake = SubspaceModule::get_stake(netuid, &keys[1]);
		assert_eq!(key0_stake, 0);
		assert_eq!(key1_stake, stake_amount * 2);

		SubspaceModule::transfer_stake(get_origin(keys[0]), netuid, keys[1], keys[0], stake_amount)
			.ok();

		let key0_stake = SubspaceModule::get_stake(netuid, &keys[0]);
		let key1_stake = SubspaceModule::get_stake(netuid, &keys[1]);
		assert_eq!(key0_stake, stake_amount);
		assert_eq!(key1_stake, stake_amount);
	});
}

#[test]
fn test_delegate_stake() {
	new_test_ext().execute_with(|| {
		let max_uids: u16 = 10;
		let token_amount: u64 = 1_000_000_000;
		let netuids: Vec<u16> = [0, 1, 2, 3].to_vec();
		let amount_staked_vector: Vec<u64> = netuids.iter().map(|i| 10 * token_amount).collect();
		let mut total_stake: u64 = 0;
		let mut netuid: u16 = 0;
		let mut subnet_stake: u64 = 0;
		let mut uid: u16 = 0;

		for i in netuids.iter() {
			netuid = *i;
			println!("NETUID: {}", netuid);
			let amount_staked = amount_staked_vector[netuid as usize];
			let key_vector: Vec<U256> =
				(0..max_uids).map(|i| U256::from(i + max_uids * netuid)).collect();
			let delegate_key_vector: Vec<U256> =
				key_vector.iter().map(|i| U256::from(i.clone() + 1)).collect();

			for (i, key) in key_vector.iter().enumerate() {
				println!(
					" KEY {} KEY STAKE {} STAKING AMOUNT {} ",
					key,
					SubspaceModule::get_stake(netuid, key),
					amount_staked
				);

				let delegate_key: U256 = delegate_key_vector[i];
				add_balance(delegate_key, amount_staked + 1);

				register_module(netuid, *key, 0).ok();
				// add_stake_and_balance(netuid, *key, amount_staked);
				println!(
					" DELEGATE KEY STAKE {} STAKING AMOUNT {} ",
					SubspaceModule::get_stake(netuid, &delegate_key),
					amount_staked
				);

				SubspaceModule::add_stake(get_origin(delegate_key), netuid, *key, amount_staked)
					.ok();
				uid = SubspaceModule::get_uid_for_key(netuid, &key);
				// SubspaceModule::add_stake(get_origin(*key), netuid, amount_staked);
				assert_eq!(SubspaceModule::get_stake_for_uid(netuid, uid), amount_staked);
				assert_eq!(SubspaceModule::get_balance(&delegate_key), 1);
				assert_eq!(SubspaceModule::get_stake_to(netuid, &delegate_key).len(), 1);
				// REMOVE STAKE
				SubspaceModule::remove_stake(get_origin(delegate_key), netuid, *key, amount_staked)
					.ok();
				assert_eq!(SubspaceModule::get_balance(&delegate_key), amount_staked + 1);
				assert_eq!(SubspaceModule::get_stake_for_uid(netuid, uid), 0);
				assert_eq!(SubspaceModule::get_stake_to(netuid, &delegate_key).len(), 0);

				// ADD STAKE AGAIN LOL
				SubspaceModule::add_stake(get_origin(delegate_key), netuid, *key, amount_staked)
					.ok();
				assert_eq!(SubspaceModule::get_stake_for_uid(netuid, uid), amount_staked);
				assert_eq!(SubspaceModule::get_balance(&delegate_key), 1);
				assert_eq!(SubspaceModule::get_stake_to(netuid, &delegate_key).len(), 1);

				// AT THE END WE SHOULD HAVE THE SAME TOTAL STAKE
				subnet_stake += SubspaceModule::get_stake_for_uid(netuid, uid).clone();
			}

			assert_eq!(SubspaceModule::get_total_subnet_stake(netuid), subnet_stake);

			total_stake += subnet_stake.clone();

			assert_eq!(SubspaceModule::total_stake(), total_stake);

			subnet_stake = 0;

			println!("TOTAL STAKE: {}", total_stake);
			println!("TOTAL SUBNET STAKE: {}", SubspaceModule::get_total_subnet_stake(netuid));
		}
	});
}

#[test]
fn test_ownership_ratio() {
	new_test_ext().execute_with(|| {
		let netuid: u16 = 0;
		let num_modules: u16 = 10;
		let stake_per_module: u64 = 1_000_000_000;
		register_n_modules(netuid, num_modules, 0);

		let keys = SubspaceModule::get_keys(netuid);

		for (_k_i, k) in keys.iter().enumerate() {
			let uid = SubspaceModule::get_uid_for_key(netuid, k);

			let delegate_keys: Vec<U256> =
				(0..num_modules).map(|i| U256::from(i + num_modules + 1)).collect();
			for d in delegate_keys.iter() {
				add_balance(*d, stake_per_module + 1);
			}

			let pre_delegate_stake_from_vector = SubspaceModule::get_stake_from(netuid, uid);
			assert_eq!(pre_delegate_stake_from_vector.len(), 1); // +1 for the module itself, +1 for the delegate key on

			println!("KEY: {}", k);

			for (i, d) in delegate_keys.iter().enumerate() {
				println!("DELEGATE KEY: {}", d);

				SubspaceModule::add_stake(get_origin(*d), netuid, *k, stake_per_module).ok();
				let stake_from_vector = SubspaceModule::get_stake_from(netuid, uid);

				assert_eq!(stake_from_vector.len(), pre_delegate_stake_from_vector.len() + i + 1);
			}
			
			let ownership_ratios: Vec<(U256, I64F64)> =
				SubspaceModule::get_ownership_ratios(netuid, k);

			assert_eq!(ownership_ratios.len(), delegate_keys.len() + 1);
			println!("OWNERSHIP RATIOS: {:?}", ownership_ratios);
			// step_block();

			step_epoch(netuid);

			let stake_from_vector = SubspaceModule::get_stake_from(netuid, uid);
			let stake: u64 = SubspaceModule::get_stake(netuid, k);
			let sumed_stake: u64 = stake_from_vector.iter().fold(0, |acc, (a, x)| acc + x);
			let total_stake: u64 = SubspaceModule::get_total_subnet_stake(netuid);

			println!("STAKE: {}", stake);
			println!("SUMED STAKE: {}", sumed_stake);
			println!("TOTAL STAKE: {}", total_stake);

			assert_eq!(stake, sumed_stake);
		}
	});
}

#[test]
fn test_min_stake() {
	new_test_ext().execute_with(|| {
		let netuid: u16 = 0;
		let num_modules: u16 = 10;
		let min_stake: u64 = 10_000_000_000;
		register_n_modules(netuid, num_modules, min_stake);

		let keys = SubspaceModule::get_keys(netuid);
		register_n_modules(netuid, num_modules, min_stake);

		SubspaceModule::set_min_stake(netuid, min_stake - 100);

		// SubspaceModule::set_min_stake( netuid, min_stake - 100);

		SubspaceModule::remove_stake(get_origin(keys[0]), netuid, keys[0], 10_000_000_000).ok();
		let pending_deregistration_uids = SubspaceModule::get_pending_deregister_uids(netuid);
		println!("PENDING DEREGISTRATION UIDS: {:?}", pending_deregistration_uids);

		let stakes = SubspaceModule::get_stakes(netuid);
		println!("STAKES: {:?}", stakes);
		step_epoch(netuid);
		let stakes = SubspaceModule::get_stakes(netuid);
		println!("STAKES: {:?}", stakes);
		let pending_deregistration_uids = SubspaceModule::get_pending_deregister_uids(netuid);
		println!("PENDING DEREGISTRATION UIDS: {:?}", pending_deregistration_uids);
		assert_eq!(pending_deregistration_uids.len(), 1);
		assert_eq!(pending_deregistration_uids[0], 0);
		let is_registered = SubspaceModule::is_registered(netuid, &keys[0]);
		assert_eq!(is_registered, true);
		step_block(1);
		let is_registered = SubspaceModule::is_registered(netuid, &keys[0]);
		assert_eq!(is_registered, false);
	});
}
