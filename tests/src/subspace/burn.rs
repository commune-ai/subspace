use std::u32;

use crate::mock::*;
use frame_support::assert_ok;
use global::GeneralBurnConfiguration;
use pallet_subspace::*;

#[test]
fn module_registration_burn_increases() {
    new_test_ext().execute_with(|| {
        let min_burn = to_nano(10);
        let max_burn = to_nano(1000);
        let target_reg_interval = 200;
        let target_reg_per_interval = 25;

        // Adjust max registrations per block to a high number.
        // We will be doing "registration raid"
        let module_burn_config: GeneralBurnConfiguration<Test> = GeneralBurnConfiguration {
            min_burn,
            max_burn,
            adjustment_alpha: 9223372036854775807,
            target_registrations_interval: target_reg_interval,
            target_registrations_per_interval: target_reg_per_interval,
            max_registrations_per_interval: 1_000,
            ..Default::default()
        };
        ModuleBurnConfig::set(0, module_burn_config.clone());

        // register the general subnet
        assert_ok!(register_module(0, 0, to_nano(20), false));

        // register 500 modules on yuma subnet
        let netuid = 1;
        let n = 300u32;
        let initial_stake: u64 = to_nano(500);

        MaxRegistrationsPerBlock::<Test>::set(1000);

        let network = format!("test{netuid}");
        assert_ok!(register_named_subnet(u32::MAX, netuid, network));
        ModuleBurnConfig::set(netuid, module_burn_config);
        // this will perform 300 registrations and step in between
        for module_key in 1..n {
            // this registers five in block
            assert_ok!(register_module(netuid, module_key, initial_stake, false));
            if module_key % 5 == 0 {
                // after that we step 30 blocks
                // meaning that the average registration per block is 0.166..
                step_block(30);
            }
        }

        // We are at block 1,8 k now.
        // We performed 300 registrations
        // this means avg.  0.166.. per block
        // burn has incrased by 90% > up
        let subnet_zero_burn = Burn::<Test>::get(0);
        assert_eq!(subnet_zero_burn, min_burn);
        let subnet_one_burn = Burn::<Test>::get(1);
        assert!(min_burn < subnet_one_burn && subnet_one_burn < max_burn);
    });
}

#[test]
fn subnet_registration_burn_increases() {
    new_test_ext().execute_with(|| {
        let min_burn = to_nano(100);
        let max_burn = to_nano(10000);

        let burn_config = GeneralBurnConfiguration {
            min_burn,
            max_burn,
            adjustment_alpha: 200,
            target_registrations_interval: 5,
            target_registrations_per_interval: 1,
            max_registrations_per_interval: 10,
            ..Default::default()
        };
        SubnetBurnConfig::<Test>::set(burn_config.clone());

        // Set initial subnet burn
        let initial_burn = to_nano(200);
        SubnetBurn::<Test>::set(initial_burn);

        // Register subnets and check burn increase
        for i in 1..=10 {
            // Register a subnet
            // Have enough balance to register
            assert_ok!(register_subnet(i, i as u16));

            step_block(burn_config.target_registrations_interval / 2);

            // Check if burn has increased
            let current_burn = SubnetBurn::<Test>::get();
            assert!(
                current_burn <= max_burn,
                "Burn should not exceed max_burn. Current: {}, Max: {}",
                current_burn,
                max_burn
            );

            if i as u16 % burn_config.target_registrations_interval == 0 {
                assert!(
                    current_burn > initial_burn,
                    "Burn should increase. Current: {}, Initial: {}",
                    current_burn,
                    initial_burn
                );
            }
        }

        // Verify final burn is significantly higher than initial
        let final_burn = SubnetBurn::<Test>::get();
        assert!(
            final_burn > initial_burn * 2,
            "Final burn should be significantly higher. Final: {}, Initial: {}",
            final_burn,
            initial_burn
        );
    });
}
