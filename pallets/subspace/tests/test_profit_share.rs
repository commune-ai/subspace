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
fn test_add_profit_share() {
    new_test_ext().execute_with(|| {
        let netuid = 0;
        let founder_key = U256::from(0);
        register_module(netuid, founder_key, 1_000_000_000u64);
        let profit_sharers = vec![U256::from(1), U256::from(2), U256::from(3)];
        let shares = vec![3,3,3];

        let result = SubspaceModule::add_profit_shares(get_origin(founder_key), profit_sharers, shares);

        assert_ok!(result);
        assert_eq!(SubspaceModule::get_profit_shares(founder_key).len(), 3, "profit shares not added");
    });
}