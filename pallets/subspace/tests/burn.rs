mod mock;
use frame_support::assert_ok;

use mock::*;
use pallet_subspace::{TargetRegistrationsInterval, TargetRegistrationsPerInterval};
use sp_core::U256;

// test subnet specific burn
#[test]
fn test_local_subnet_burn() {
    new_test_ext().execute_with(|| {
        let min_burn = to_nano(10);
        let target_reg_interval = 200;
        let target_reg_per_interval = 25;
        // set the min_burn to 10 $COMAI
        SubspaceModule::set_min_burn(min_burn);

        let max_burn = to_nano(1000);
        // Adjust max burn to allow for the burn to move
        SubspaceModule::set_max_burn(max_burn);
        SubspaceModule::set_max_registrations_per_block(5);

        // register the general subnet
        assert_ok!(register_module(0, U256::from(0), to_nano(20)));
        // Adjust max registrations per block to a high number.
        // We will be doing "registration raid"
        TargetRegistrationsInterval::<Test>::insert(0, target_reg_interval); // for the netuid 0
        TargetRegistrationsPerInterval::<Test>::insert(0, target_reg_per_interval); // for the netuid 0

        // register 500 modules on yuma subnet
        let netuid = 1;
        let n = 300;
        let initial_stake: u64 = to_nano(500);

        SubspaceModule::set_max_registrations_per_block(1000);
        // this will perform 300 registrations and step in between
        for i in 1..n {
            // this registers five in block
            assert_ok!(register_module(netuid, U256::from(i), initial_stake));
            if i % 5 == 0 {
                // after that we step 30 blocks
                // meaning that the average registration per block is 0.166..
                TargetRegistrationsInterval::<Test>::insert(netuid, target_reg_interval); // for the netuid 0
                TargetRegistrationsPerInterval::<Test>::insert(netuid, target_reg_per_interval); // fo
                step_block(30);
            }
        }

        // We are at block 1,8 k now.
        // We performed 300 registrations
        // this means avg.  0.166.. per block
        // burn has incrased by 90% > up

        let subnet_zero_burn = SubspaceModule::get_burn(0);
        assert_eq!(subnet_zero_burn, min_burn);
        let subnet_one_burn = SubspaceModule::get_burn(1);
        assert!(min_burn < subnet_one_burn && subnet_one_burn < max_burn);
    });
}
