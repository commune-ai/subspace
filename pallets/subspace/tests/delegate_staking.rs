mod mock;

use frame_support::assert_ok;
use log::info;
use mock::*;
use pallet_subspace::{Emission, Tempo};
use sp_core::U256;

// /***********************************************************
// 	staking::add_stake() tests
// ************************************************************/
#[test]
fn test_ownership_ratio() {
    new_test_ext().execute_with(|| {
        let netuid: u16 = 0;
        let num_modules: u16 = 10;
        let tempo = 1;
        let stake_per_module: u64 = 1_000_000_000;
        // make sure that the results won´t get affected by burn
        zero_min_burn();

        register_n_modules(netuid, num_modules, stake_per_module);
        Tempo::<Test>::insert(netuid, tempo);

        let keys = SubspaceModule::get_keys(netuid);
        let voter_key = keys[0];
        let miner_keys = keys[1..].to_vec();
        let miner_uids: Vec<u16> =
            miner_keys.iter().map(|k| SubspaceModule::get_uid_for_key(netuid, k)).collect();
        let miner_weights = vec![1; miner_uids.len()];

        let delegate_keys: Vec<U256> =
            (0..num_modules).map(|i| U256::from(i + num_modules + 1)).collect();
        for d in delegate_keys.iter() {
            add_balance(*d, stake_per_module + 1);
        }

        let pre_delegate_stake_from_vector =
            SubspaceModule::get_stake_from_vector(netuid, &voter_key);
        assert_eq!(pre_delegate_stake_from_vector.len(), 1); // +1 for the module itself, +1 for the delegate key on

        for (i, d) in delegate_keys.iter().enumerate() {
            assert_ok!(SubspaceModule::add_stake(
                get_origin(*d),
                netuid,
                voter_key,
                stake_per_module,
            ));
            let stake_from_vector = SubspaceModule::get_stake_from_vector(netuid, &voter_key);
            assert_eq!(
                stake_from_vector.len(),
                pre_delegate_stake_from_vector.len() + i + 1
            );
        }
        let ownership_ratios = SubspaceModule::get_ownership_ratios(netuid, &voter_key);
        assert_eq!(ownership_ratios.len(), delegate_keys.len() + 1);

        let total_balance = keys
            .iter()
            .map(SubspaceModule::get_balance)
            .collect::<Vec<u64>>()
            .iter()
            .sum::<u64>();
        let total_stake = keys
            .iter()
            .map(|k| SubspaceModule::get_stake_to_module(netuid, k, k))
            .collect::<Vec<u64>>()
            .iter()
            .sum::<u64>();
        let total_delegate_stake = delegate_keys
            .iter()
            .map(|k| SubspaceModule::get_stake_to_module(netuid, k, &voter_key))
            .collect::<Vec<u64>>()
            .iter()
            .sum::<u64>();
        let total_delegate_balance = delegate_keys
            .iter()
            .map(SubspaceModule::get_balance)
            .collect::<Vec<u64>>()
            .iter()
            .sum::<u64>();
        let total_tokens_before =
            total_balance + total_stake + total_delegate_stake + total_delegate_balance;

        let result = SubspaceModule::set_weights(
            get_origin(voter_key),
            netuid,
            miner_uids.clone(),
            miner_weights.clone(),
        );

        assert_ok!(result);

        step_epoch(netuid);

        let emissions = Emission::<Test>::get(netuid);

        let total_emissions = emissions.iter().sum::<u64>();

        let total_balance = keys
            .iter()
            .map(SubspaceModule::get_balance)
            .collect::<Vec<u64>>()
            .iter()
            .sum::<u64>();
        let total_stake = keys
            .iter()
            .map(|k| SubspaceModule::get_stake_to_module(netuid, k, k))
            .collect::<Vec<u64>>()
            .iter()
            .sum::<u64>();
        let total_delegate_stake = delegate_keys
            .iter()
            .map(|k| SubspaceModule::get_stake_to_module(netuid, k, &voter_key))
            .collect::<Vec<u64>>()
            .iter()
            .sum::<u64>();
        let total_delegate_balance = delegate_keys
            .iter()
            .map(SubspaceModule::get_balance)
            .collect::<Vec<u64>>()
            .iter()
            .sum::<u64>();
        let total_tokens_after =
            total_balance + total_stake + total_delegate_stake + total_delegate_balance;
        let total_new_tokens = total_tokens_after - total_tokens_before;

        assert_eq!(total_new_tokens, total_emissions);

        let stake_from_vector = SubspaceModule::get_stake_from_vector(netuid, &voter_key);
        let _stake: u64 = SubspaceModule::get_stake(netuid, &voter_key);
        let _sumed_stake: u64 = stake_from_vector.iter().fold(0, |acc, (_a, x)| acc + x);
        let _total_stake: u64 = SubspaceModule::get_total_subnet_stake(netuid);
        info!("stake_from_vector: {stake_from_vector:?}");
    });
}
