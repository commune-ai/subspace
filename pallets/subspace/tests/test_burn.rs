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
fn test_burn() {
    new_test_ext().execute_with(|| {
        
	let netuid = 0;
	let n = 3;

	let keys : Vec<U256> = (0..n).into_iter().map(|x| U256::from(x)).collect();
	let stakes : Vec<u64> = (0..n).into_iter().map(|x| x as u64).collect();

	for i in 0..n {
		assert_ok!(register_module(netuid, keys[i], stakes[i]));
	}
	let mut params = SubspaceModule::subnet_params(netuid);
	params.burn_rate = 100;
	SubspaceModule::set_subnet_params(netuid, params);
	let epochs = 10;
	for _ in 0..epochs {
		step_epoch(1);
	}
	});
}