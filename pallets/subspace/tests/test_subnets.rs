mod test_mock;
use frame_support::{
	assert_ok,
	dispatch::{DispatchClass, DispatchInfo, GetDispatchInfo, Pays},
};
use frame_system::Config;
use pallet_subspace::Error;
use sp_core::U256;
use test_mock::*;
use sp_std::vec;

/* TO DO SAM: write test for LatuUpdate after it is set */

#[test]
fn test_add_subnets() {
	new_test_ext().execute_with(|| {
		let tempo: u16 = 13;
		let num_subnets: u16 = 1000;
		let mut stake_per_module: u64 = 1_000_000_000;
		let mut max_allowed_subnets: u16 = SubspaceModule::get_max_allowed_subnets();
		let mut expected_subnets = 0;

		for i in 0..num_subnets{
			register_module(i, U256::from(0), stake_per_module);
			expected_subnets += 1;
			if expected_subnets > max_allowed_subnets {
				expected_subnets = max_allowed_subnets;
			} else {
				assert_eq!(SubspaceModule::get_subnet_n(i), 1);
			}
			assert_eq!(
				SubspaceModule::get_number_of_subnets(),
				expected_subnets,
				"number of subnets is not equal to expected subnets"
			);

		}

		for netuid in 0..num_subnets {
			SubspaceModule::remove_network_for_netuid(netuid);
			expected_subnets = expected_subnets.saturating_sub(1);
			assert_eq!(
				SubspaceModule::get_number_of_subnets(),
				expected_subnets,
				"number of subnets is not equal to expected subnets"
			);

			
		}

	});
}


fn test_set_single_temple(tempo: u16) {
	new_test_ext().execute_with(|| {
		// creates a subnet when you register a module
		let netuid: u16 = 0;
		let stake: u64 = 0;
		let key = U256::from(0);
		let tempos: Vec<u16> = vec![2, 4];
		register_module(netuid, key, stake);
		let mut params = SubspaceModule::subnet_params(netuid).clone();
		params.tempo = tempo;

		let total_blocks = 100;
		let emission_per_block: u64 = SubspaceModule::get_subnet_emission(netuid);
		let mut total_stake: u64 = 0;
		let tempo = 5;
		let min_stake = 1_000_000_000;

		let result = SubspaceModule::set_subnet_params(
			netuid,
			params.clone()
		);

		let subnet_params = SubspaceModule::subnet_params(netuid);
		
		assert_eq!(subnet_params.tempo, tempo);
		assert_eq!(subnet_params.min_stake, min_stake);
		assert_eq!(subnet_params.max_allowed_uids, params.max_allowed_uids);
		assert_eq!(subnet_params.min_allowed_weights, params.min_allowed_weights);
		assert_eq!(subnet_params.max_allowed_weights, params.max_allowed_weights);
		assert_eq!(subnet_params.immunity_period, params.immunity_period);
		assert_eq!(subnet_params.name, params.name);

		let previous_total_stake: u64 = block_number() * emission_per_block;

		let n_epochs = 3;
		let n_steps = n_epochs * tempo;
		for i in 0..n_steps {
			println!(
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

			println!("emission {:?}", emissions);
			println!("incentives {:?}", incentives);
			println!("dividends {:?}", dividends);
			// println!("EMMISSIONS {:?}", SubspaceModule::get_ownership_ratios_emission(netuid,
			// &U256::from(0),emissions[0] ));

			let stake: u64 = SubspaceModule::get_stake_for_uid(netuid, 0);
			println!("stake {:?}", stake);
			total_stake = SubspaceModule::get_total_subnet_stake(netuid);
			println!("total stake {}", total_stake);
		}

		assert_eq!(
			total_stake,
			(tempo as u64) * emission_per_block * (n_epochs as u64) + previous_total_stake
		);
	});
}

// TODO:
// #[test]
// fn test_set_tempo() {
// 	for tempo in [1, 2, 4, 8, 16, 32, 64, 128] {
// 		test_set_single_temple(tempo);
// 	}
// }

#[test]
fn test_emission_ratio() {
	new_test_ext().execute_with(|| {
		let netuids: Vec<u16> = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9].to_vec();
		let stake_per_module: u64 = 1_000_000_000;
		let mut emissions_per_subnet: Vec<u64> = Vec::new();
		let max_delta: f64 = 1.0;
		let n: u16 = 10;

		for i in 0..netuids.len() {
			let key = U256::from(netuids[i]);
			let netuid = netuids[i];
			register_n_modules(netuid, 1, stake_per_module);
			let subnet_emission: u64 = SubspaceModule::get_subnet_emission(netuid);
			emissions_per_subnet.push(subnet_emission);
			let expected_emission_factor: f64 = 1.0 / (netuids.len() as f64);
			let emission_per_block = SubspaceModule::get_total_emission_per_block();
			let expected_emission: u64 = emission_per_block / (i as u64 + 1);

			let block = block_number();
			// magnitude of difference between expected and actual emission
			let mut delta: f64 = 0.0;
			if subnet_emission > expected_emission {
				delta = subnet_emission as f64 - expected_emission as f64;
			} else {
				delta = expected_emission as f64 - subnet_emission as f64;
			}

			assert!(
				delta <= max_delta,
				"emission {} is too far from expected emission {} ",
				subnet_emission,
				expected_emission
			);
			assert!(block == 0, "block {} is not 0", block);
			println!("block {} subnet_emission {} ", block, subnet_emission);
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
		register_module(netuid, U256::from(0), stake);
		SubspaceModule::set_max_registrations_per_block(max_uids + extra_uids * rounds);
		for i in 1..max_uids {
			assert_ok!(register_module(netuid, U256::from(i), stake));
			assert_eq!(SubspaceModule::get_subnet_n(netuid), i + 1);
		}
		let mut n: u16 = SubspaceModule::get_subnet_n(netuid);
		let mut old_n: u16 = n.clone();
		let mut uids: Vec<u16>;
		assert_eq!(SubspaceModule::get_subnet_n(netuid), max_uids);
		let mut new_n: u16 = SubspaceModule::get_subnet_n(netuid);
		for r in 1..rounds {
			// set max allowed uids to max_uids + extra_uids
			SubspaceModule::set_max_allowed_uids(netuid, max_uids + extra_uids * (r - 1));
			max_uids = SubspaceModule::get_max_allowed_uids(netuid);
			new_n = old_n + extra_uids * (r - 1);
			// print the pruned uids
			for uid in old_n + extra_uids * (r - 1)..old_n + extra_uids * r {
				register_module(netuid, U256::from(uid), stake);
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
		let max_uids: u16 = 2000;
		let extra_uids: u16 = 20;

		let mut n = SubspaceModule::get_subnet_n(netuid);
		println!("registering module {}", n);
		register_module(netuid, U256::from(0), stake);
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

		println!("total stake {}", total_stake);
		println!("expected stake {}", expected_stake);
		assert_eq!(total_stake, expected_stake);

		let og_keys = SubspaceModule::get_keys(netuid);
		let mut old_total_subnet_balance: u64 = 0;
		for key in og_keys.clone() {
			old_total_subnet_balance =
				old_total_subnet_balance + SubspaceModule::get_balance_u64(&key);
		}

		let subnet = SubspaceModule::subnet_info(netuid);

		let mut params = SubspaceModule::subnet_params(netuid).clone();
		params.max_allowed_uids = max_uids;
		let result = SubspaceModule::do_update_subnet(get_origin(keys[0]), netuid, params);
		assert_ok!(result);
		let params = SubspaceModule::subnet_params(netuid);
		let mut n = SubspaceModule::get_subnet_n(netuid);
		assert_eq!(params.max_allowed_uids, max_uids, "max allowed uids is not equal to expected max allowed uids");
		assert_eq!(params.max_allowed_uids, n, "min allowed weights is not equal to expected min allowed weights");

		let mut new_total_subnet_balance: u64 = 0;
		for key in og_keys.clone() {
			new_total_subnet_balance =
				new_total_subnet_balance + SubspaceModule::get_balance_u64(&key);
		}
		// let expected_total_subnet_balance: u64 =
		// 	(extra_uids as u64) * (stake + 1) + max_uids as u64; // this is weitd, but we needed to add 1 to make sure that the stake is not 0
		// assert!(
		// 	new_total_subnet_balance == expected_total_subnet_balance,
		// 	"new total subnet balance {} is not equal to expected total subnet balance {}",
		// 	new_total_subnet_balance,
		// 	expected_total_subnet_balance
		// );


		n = SubspaceModule::get_subnet_n(netuid);
		let stake_vector: Vec<u64> = SubspaceModule::get_stakes(netuid);
		let calc_stake: u64 = stake_vector.iter().sum();

		println!("calculated  stake {}", calc_stake);

		expected_stake = (max_uids) as u64 * stake;
		let subnet_stake = SubspaceModule::get_total_subnet_stake(netuid);
		total_stake = SubspaceModule::total_stake();

		assert_eq!(total_stake, expected_stake);
	});
}

#[test]
fn test_set_max_allowed_modules() {
	new_test_ext().execute_with(|| {
		let netuid: u16 = 0;
		let stake: u64 = 1_000_000_000;
		let max_uids: u16 = 2000;
		let extra_uids: u16 = 20;
		let max_allowed_modules: u16 = 100;
		let mut n = SubspaceModule::get_subnet_n(netuid);

		SubspaceModule::set_max_allowed_modules(max_allowed_modules);
		// set max_total modules

		for i in 1..(2*max_allowed_modules ) {
			assert_ok!(register_module(netuid, U256::from(i), stake));
			n = SubspaceModule::get_subnet_n(netuid);
			assert!(n <= max_allowed_modules,"subnet_n {:?} is not less than max_allowed_modules {:?}", n, max_allowed_modules);
		}
	})
	}