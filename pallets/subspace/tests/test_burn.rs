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

#[test]
fn test_burn() {
    new_test_ext().execute_with(|| {
        
	let netuid = 0;
	let n = 10;

	let initial_stake: u64 = 1000;

	let keys : Vec<U256> = (0..n).into_iter().map(|x| U256::from(x)).collect();
	let stakes : Vec<u64> = (0..n).into_iter().map(|x| initial_stake * 1_000_000_000).collect();

	for i in 0..n {
		assert_ok!(register_module(netuid, keys[i], stakes[i]));
	}

	let voter_key = keys[1];


	// vote to avoid key[0] as we want to see the key[0] burn
	let mut votes : Vec<u16> = vec![];
	let mut uids : Vec<u16> = vec![];
	for i in 0..n {
		if i != 0 {
			votes.push(1);
			uids.push(i as u16);
		}
	}
	assert_ok!(SubspaceModule::set_weights(get_origin(voter_key),netuid, uids, votes));
	let mut params = SubspaceModule::subnet_params(netuid);
	params.burn_rate = 100;
	SubspaceModule::set_subnet_params(netuid, params);
	params = SubspaceModule::subnet_params(netuid);
	println!("params: {:?}", params);

	println!("burn : {:?}", SubspaceModule::get_burn_emission_per_epoch(netuid));
	let epochs = 10;
	let mut previous_key_stake = SubspaceModule::get_total_stake_to(netuid,&keys[0]);

	let dividends = SubspaceModule::get_dividends(netuid);
	let incentives = SubspaceModule::get_incentives(netuid);
	let emissions = SubspaceModule::get_emissions(netuid);

	println!("dividends: {:?} incentives: {:?} emissions: {:?}", 
				dividends, 
				incentives, 
				emissions);

	step_epoch(1);


	let dividends = SubspaceModule::get_dividends(netuid);
	let incentives = SubspaceModule::get_incentives(netuid);
	let emissions = SubspaceModule::get_emissions(netuid);

	println!("dividends: {:?} incentives: {:?} emissions: {:?}", 
				dividends, 
				incentives, 
				emissions);
	let key_stake = SubspaceModule::get_total_stake_to(netuid,&keys[0]);
	println!("key_stake: {:?} prev_key_stake {:?}", key_stake, previous_key_stake);
	assert!( previous_key_stake > key_stake , "key_stake: {:?} prev_key_stake {:?}", key_stake, previous_key_stake);
	


	});
}


#[test]
fn test_min_burn() {
    new_test_ext().execute_with(|| {
        
	let netuid = 0;
	let n = 10;

	let initial_stake: u64 = 1000;

	let keys : Vec<U256> = (0..n).into_iter().map(|x| U256::from(x)).collect();
	let stakes : Vec<u64> = (0..n).into_iter().map(|x| initial_stake * 1_000_000_000).collect();

	// founder register_module(netuid, keys[i]);
	let founder_initial_stake = stakes[0];
	assert_ok!(register_module(netuid, keys[0], stakes[0]));
	let founder_current_stake = SubspaceModule::get_total_stake_to(netuid,&keys[0]);
	assert_eq!(founder_initial_stake, founder_current_stake, 
				"founder_initial_stake: {:?} founder_current_stake: {:?}", 
					founder_initial_stake, founder_current_stake);

	// set the burn min to 1000000000
	// register_module(netuid, keys[i]);
	let mut params = SubspaceModule::subnet_params(netuid);
	params.min_burn = 100;
	SubspaceModule::set_subnet_params(netuid, params.clone());
	params = SubspaceModule::subnet_params(netuid);
	println!("params: {:?}", params);
	for i in 1..n {
		assert_ok!(register_module(netuid, keys[i], stakes[i]));
		let key_stake_after = SubspaceModule::get_total_stake_to(netuid,&keys[i]);
		assert_eq!(key_stake_after, stakes[i] - params.min_burn, 
					"key_stake_after: {:?} stakes[i]: {:?}", 
						key_stake_after, stakes[i]);
	}


	let voter_key = keys[1];


	});
}