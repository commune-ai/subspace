mod mock;
use frame_support::assert_ok;

use mock::*;
use sp_core::U256;

#[test]
fn test_burn() {
    new_test_ext().execute_with(|| {
        let netuid = 0;
        let n = 20;
        let initial_stake: u64 = 1000;
        let tempo = 100;
        let keys: Vec<U256> = (0..n).map(U256::from).collect();
        let stakes: Vec<u64> = std::iter::repeat(initial_stake * 1_000_000_000).take(n).collect();
        let voter_key_index = 1;
        let voter_key = keys[voter_key_index];

        SubspaceModule::set_max_registrations_per_block(1000);
        for (key, stake) in keys.iter().zip(stakes.iter()) {
            assert_ok!(register_module(netuid, *key, *stake));
        }

        update_params!(netuid => { tempo: tempo });

        for burn_rate in (0..9).map(|n| n * 10) {
            let mut params = SubspaceModule::global_params();
            params.burn_rate = burn_rate;

            SubspaceModule::set_global_params(params);

            let votes: Vec<u16> = (0..n)
                .filter(|&x| x != voter_key_index)
                .map(|_| 1)
                .collect();

            let uids: Vec<u16> = (0..n)
                .filter(|&x| x != voter_key_index)
                .map(|x| x as u16)
                .collect();

            assert_ok!(SubspaceModule::set_weights(
                get_origin(voter_key),
                netuid,
                uids,
                votes
            ));

            let total_stake_before = SubspaceModule::get_total_subnet_stake(netuid);
            step_epoch(netuid);

            let total_stake_after = SubspaceModule::get_total_subnet_stake(netuid);
            let subnet_params = SubspaceModule::subnet_params(netuid);

            let threshold = SubspaceModule::get_subnet_stake_threshold();
            let expected_subnet_emission: u64 =
                ((SubspaceModule::calculate_network_emission(netuid, threshold) as f64
                    * (subnet_params.tempo as f64))
                    * (((100 - burn_rate) as f64) / 100.0)) as u64;

            let delta_ratio = 0.01;
            let delta = (total_stake_before as f64 * delta_ratio) as u64;

            let expected_subnet_emission =
                total_stake_before.saturating_add(expected_subnet_emission);

            assert!(
                total_stake_after.saturating_sub(delta) < expected_subnet_emission
                    && total_stake_after + delta > expected_subnet_emission,
                "total_stake_after: {total_stake_after:?} expected_subnet_emission: {expected_subnet_emission:?}",
            );
        }
    });
}

// test subnet specific burn
#[test]
fn test_local_subnet_burn() {
    new_test_ext().execute_with(|| {
        let min_burn = to_nano(10);
        // set the min_burn to 10 $COMAI
        SubspaceModule::set_min_burn(min_burn);

        let max_burn = to_nano(1000);
        // Adjust max burn to allow for the burn to move
        SubspaceModule::set_max_burn(max_burn);

        // Adjust max registrations per block to a high number.
        // We will be doing "registration raid"
        SubspaceModule::set_target_registrations_interval(200);
        SubspaceModule::set_target_registrations_per_interval(25);

        SubspaceModule::set_max_registrations_per_block(5);

        // register the general subnet
        assert_ok!(register_module(0, U256::from(0), to_nano(20)));

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
