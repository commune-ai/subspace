mod test_mock;
use frame_support::assert_ok;

use sp_core::U256;
use sp_std::vec;
use test_mock::*;

#[test]
fn test_burn() {
	new_test_ext().execute_with(|| {
		let netuid = 0;
		let n = 20;
		let initial_stake: u64 = 1000;
		let tempo = 100;

		let keys: Vec<U256> = (0..n).map(U256::from).collect();
		let stakes: Vec<u64> = (0..n).map(|_x| initial_stake * 1_000_000_000).collect();

		let mut subnet_params = SubspaceModule::subnet_params(netuid);
		subnet_params.tempo = tempo;
		SubspaceModule::set_subnet_params(netuid, subnet_params);
		subnet_params = SubspaceModule::subnet_params(netuid);

		for i in 0..n {
			assert_ok!(register_module(netuid, keys[i], stakes[i]));
			let stake_from_vector = SubspaceModule::get_stake_to_vector(netuid, &keys[i]);
			println!("{:?}", stake_from_vector);
		}

		for burn_rate in [0, 10, 20, 30, 40, 50, 60, 70, 80, 90].iter() {
			let mut params = SubspaceModule::global_params();
			params.burn_rate = *burn_rate;
			SubspaceModule::set_global_params(params);
			params = SubspaceModule::global_params();

			let voter_key = keys[1];

			// vote to avoid key[0] as we want to see the key[0] burn
			let mut votes: Vec<u16> = vec![];
			let mut uids: Vec<u16> = vec![];
			for i in 0..n {
				votes.push(1);
				uids.push(i as u16);
			}
			println!("{:?}", SubspaceModule::get_stake_for_key(netuid, &voter_key));
			assert_ok!(SubspaceModule::set_weights(get_origin(voter_key), netuid, uids, votes));
			println!("burn : {:?}", SubspaceModule::get_burn_emission_per_epoch(netuid));

			let _total_stake = SubspaceModule::get_total_subnet_stake(netuid);

			let dividends = SubspaceModule::get_dividends(netuid);
			let incentives = SubspaceModule::get_incentives(netuid);
			let emissions = SubspaceModule::get_emissions(netuid);

			println!(
				"dividends: {:?} incentives: {:?} emissions: {:?}",
				dividends, incentives, emissions
			);

			let burn_per_epoch = SubspaceModule::get_burn_per_epoch(netuid);

			println!("burn_per_epoch: {:?}", burn_per_epoch);

			let stake_vector_before = SubspaceModule::get_stakes(netuid);
			let total_stake_before = SubspaceModule::get_total_subnet_stake(netuid);
			step_epoch(netuid);
			let stake_vector_after = SubspaceModule::get_stakes(netuid);
			let total_stake_after = SubspaceModule::get_total_subnet_stake(netuid);

			println!(
				"stake_vector_before: {:?} stake_vector_after: {:?}",
				stake_vector_before, stake_vector_after
			);
			println!(
				"total_stake: {:?} total_stake_after: {:?}",
				total_stake_before, total_stake_after
			);
			let subnet_params = SubspaceModule::subnet_params(netuid);

			let burn_per_epoch = SubspaceModule::get_burn_emission_per_epoch(netuid);
			let dividends = SubspaceModule::get_dividends(netuid);
			let incentives = SubspaceModule::get_incentives(netuid);
			let emissions = SubspaceModule::get_emissions(netuid);

			println!("burn_per_epoch: {:?}", burn_per_epoch);

			println!(
				"dividends: {:?} incentives: {:?} emissions: {:?}",
				dividends, incentives, emissions
			);

			let _calculated_subnet_emission = emissions.iter().sum::<u64>();
			let expected_subnet_emission: u64 = ((SubspaceModule::get_subnet_emission(netuid)
				as f64 * (subnet_params.tempo as f64)) *
				(((100 - burn_rate) as f64) / 100.0)) as u64;

			let delta_ratio = 0.01;
			let delta = (total_stake_before as f64 * delta_ratio) as u64;

			let expected_subnet_emission =
				total_stake_before.saturating_add(expected_subnet_emission);
			assert!(
				total_stake_after.saturating_sub(delta) < expected_subnet_emission &&
					total_stake_after + delta > expected_subnet_emission,
				"total_stake_after: {:?} expected_subnet_emission: {:?}",
				total_stake_after,
				expected_subnet_emission
			);
		}
	});
}

#[test]
fn test_min_burn() {
	new_test_ext().execute_with(|| {
		let netuid = 0;
		let n = 10;

		let initial_stake: u64 = 1000;

		let keys: Vec<U256> = (0..n).map(U256::from).collect();
		let stakes: Vec<u64> = (0..n).map(|_x| initial_stake * 1_000_000_000).collect();

		// founder register_module(netuid, keys[i]);
		let founder_initial_stake = stakes[0];
		assert_ok!(register_module(netuid, keys[0], stakes[0]));
		let founder_current_stake = SubspaceModule::get_total_stake_to(netuid, &keys[0]);
		assert_eq!(
			founder_initial_stake, founder_current_stake,
			"founder_initial_stake: {:?} founder_current_stake: {:?}",
			founder_initial_stake, founder_current_stake
		);

		// set the burn min to 1000000000
		// register_module(netuid, keys[i]);
		let mut params = SubspaceModule::global_params();
		params.min_burn = 100;
		SubspaceModule::set_global_params(params.clone());
		params = SubspaceModule::global_params();
		for i in 1..n {
			assert_ok!(register_module(netuid, keys[i], stakes[i]));
			println!("params: {:?}", params);

			let key_stake_after = SubspaceModule::get_total_stake_to(netuid, &keys[i]);
			assert_eq!(
				key_stake_after,
				stakes[i] - params.min_burn,
				"key_stake_after: {:?} stakes[i]: {:?}",
				key_stake_after,
				stakes[i]
			);
		}

		let _voter_key = keys[1];
	});
}
