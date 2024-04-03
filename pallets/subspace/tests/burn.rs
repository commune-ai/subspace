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
        let mut subnet_params = SubspaceModule::subnet_params(netuid);
        subnet_params.tempo = tempo;

        SubspaceModule::set_subnet_params(netuid, subnet_params);

        for (key, stake) in keys.iter().zip(stakes.iter()) {
            assert_ok!(register_module(netuid, *key, *stake));
        }

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
