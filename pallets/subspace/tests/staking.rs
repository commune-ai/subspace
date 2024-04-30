mod mock;

use frame_support::{assert_noop, assert_ok};
use log::info;
use mock::*;
use pallet_subspace::Error;
use sp_core::U256;
use substrate_fixed::types::I64F64;

// /***********************************************************
// 	staking::add_stake() tests
// ************************************************************/
#[test]
fn test_stake() {
    new_test_ext().execute_with(|| {
        let max_uids: u16 = 10;
        let netuids: [u16; 4] = core::array::from_fn(|i| i as u16);
        let amount_staked_vector: Vec<u64> = netuids.iter().map(|_| to_nano(10)).collect();
        let mut total_stake: u64 = 0;
        let mut subnet_stake: u64 = 0;
        // make sure that the results won´t get affected by burn
        SubspaceModule::set_min_burn(0);
        SubspaceModule::set_max_registrations_per_block(1000);

        for netuid in netuids {
            info!("NETUID: {}", netuid);
            let amount_staked = amount_staked_vector[netuid as usize];
            let key_vector: Vec<U256> =
                (0..max_uids).map(|i| U256::from(i + max_uids * netuid)).collect();

            for key in key_vector.iter() {
                info!(
                    " KEY {} KEY STAKE {} STAKING AMOUNT {} ",
                    key,
                    SubspaceModule::get_stake(netuid, key),
                    amount_staked
                );

                assert_ok!(register_module(netuid, *key, amount_staked));
                info!(
                    " KEY STAKE {} STAKING AMOUNT {} ",
                    SubspaceModule::get_stake(netuid, key),
                    amount_staked
                );

                // SubspaceModule::add_stake(get_origin(*key), netuid, amount_staked);
                assert_eq!(SubspaceModule::get_stake(netuid, key), amount_staked);
                assert_eq!(SubspaceModule::get_balance(key), 1);

                // REMOVE STAKE
                assert_ok!(SubspaceModule::remove_stake(
                    get_origin(*key),
                    netuid,
                    *key,
                    amount_staked
                ));
                assert_eq!(SubspaceModule::get_balance(key), amount_staked + 1);
                assert_eq!(SubspaceModule::get_stake(netuid, key), 0);

                // ADD STAKE AGAIN LOL
                assert_ok!(SubspaceModule::add_stake(
                    get_origin(*key),
                    netuid,
                    *key,
                    amount_staked
                ));
                assert_eq!(SubspaceModule::get_stake(netuid, key), amount_staked);
                assert_eq!(SubspaceModule::get_balance(key), 1);

                // AT THE END WE SHOULD HAVE THE SAME TOTAL STAKE
                subnet_stake += SubspaceModule::get_stake(netuid, key);
            }
            assert_eq!(SubspaceModule::get_total_subnet_stake(netuid), subnet_stake);
            total_stake += subnet_stake;
            assert_eq!(SubspaceModule::total_stake(), total_stake);
            subnet_stake = 0;
            info!("TOTAL STAKE: {}", total_stake);
            info!(
                "TOTAL SUBNET STAKE: {}",
                SubspaceModule::get_total_subnet_stake(netuid)
            );
        }
    });
}

#[test]
fn test_multiple_stake() {
    new_test_ext().execute_with(|| {
        let n: u16 = 10;
        let stake_amount: u64 = 10_000_000_000;
        let _total_stake: u64 = 0;
        let netuid: u16 = 0;
        let _subnet_stake: u64 = 0;
        let _uid: u16 = 0;
        let num_staked_modules: u16 = 10;
        let total_stake: u64 = stake_amount * num_staked_modules as u64;
        // make sure that the results won´t get affected by burn
        SubspaceModule::set_min_burn(0);

        register_n_modules(netuid, n, 10);
        let controler_key = U256::from(n + 1);
        let og_staker_balance: u64 = total_stake + 1;
        add_balance(controler_key, og_staker_balance);

        let keys: Vec<U256> = SubspaceModule::get_keys(netuid);

        // stake to all modules

        let stake_amounts: Vec<u64> = vec![stake_amount; num_staked_modules as usize];

        info!("STAKE AMOUNTS: {stake_amounts:?}");
        let total_actual_stake: u64 =
            keys.clone().into_iter().map(|k| SubspaceModule::get_stake(netuid, &k)).sum();
        let staker_balance = SubspaceModule::get_balance(&controler_key);
        info!("TOTAL ACTUAL STAKE: {total_actual_stake}");
        info!("TOTAL STAKE: {total_stake}");
        info!("STAKER BALANCE: {staker_balance}");
        assert_ok!(SubspaceModule::add_stake_multiple(
            get_origin(controler_key),
            netuid,
            keys.clone(),
            stake_amounts.clone(),
        ));

        let total_actual_stake: u64 =
            keys.clone().into_iter().map(|k| SubspaceModule::get_stake(netuid, &k)).sum();
        let staker_balance = SubspaceModule::get_balance(&controler_key);

        assert_eq!(
            total_actual_stake,
            total_stake + (n as u64 * 10),
            "total stake should be equal to the sum of all stakes"
        );
        assert_eq!(
            staker_balance,
            og_staker_balance - total_stake,
            "staker balance should be 0"
        );

        // unstake from all modules
        assert_ok!(SubspaceModule::remove_stake_multiple(
            get_origin(controler_key),
            netuid,
            keys.clone(),
            stake_amounts.clone(),
        ));

        let total_actual_stake: u64 =
            keys.clone().into_iter().map(|k| SubspaceModule::get_stake(netuid, &k)).sum();
        let staker_balance = SubspaceModule::get_balance(&controler_key);
        assert_eq!(
            total_actual_stake,
            n as u64 * 10,
            "total stake should be equal to the sum of all stakes"
        );
        assert_eq!(
            staker_balance, og_staker_balance,
            "staker balance should be 0"
        );
    });
}

#[test]
fn test_transfer_stake() {
    new_test_ext().execute_with(|| {
        let n: u16 = 10;
        let stake_amount: u64 = 10_000_000_000;
        let netuid: u16 = 0;
        SubspaceModule::set_min_burn(0);

        register_n_modules(netuid, n, stake_amount);

        let keys: Vec<U256> = SubspaceModule::get_keys(netuid);

        assert_ok!(SubspaceModule::transfer_stake(
            get_origin(keys[0]),
            netuid,
            keys[0],
            keys[1],
            stake_amount
        ));

        let key0_stake = SubspaceModule::get_stake(netuid, &keys[0]);
        let key1_stake = SubspaceModule::get_stake(netuid, &keys[1]);
        assert_eq!(key0_stake, 0);
        assert_eq!(key1_stake, stake_amount * 2);

        assert_ok!(SubspaceModule::transfer_stake(
            get_origin(keys[0]),
            netuid,
            keys[1],
            keys[0],
            stake_amount
        ));

        let key0_stake = SubspaceModule::get_stake(netuid, &keys[0]);
        let key1_stake = SubspaceModule::get_stake(netuid, &keys[1]);
        assert_eq!(key0_stake, stake_amount);
        assert_eq!(key1_stake, stake_amount);
    });
}

#[test]
fn test_delegate_stake() {
    new_test_ext().execute_with(|| {
        let max_uids: u16 = 10;
        let netuids: Vec<u16> = [0, 1, 2, 3].to_vec();
        let amount_staked_vector: Vec<u64> = netuids.iter().map(|_i| to_nano(10)).collect();
        let mut total_stake: u64 = 0;
        let mut subnet_stake: u64 = 0;
        // make sure that the results won´t get affected by burn
        SubspaceModule::set_min_burn(0);
        SubspaceModule::set_max_registrations_per_block(1000);

        for i in netuids.iter() {
            let netuid = *i;
            info!("NETUID: {}", netuid);
            let amount_staked = amount_staked_vector[netuid as usize];
            let key_vector: Vec<U256> =
                (0..max_uids).map(|i| U256::from(i + max_uids * netuid)).collect();
            let delegate_key_vector: Vec<U256> = key_vector.iter().map(|i| (*i + 1)).collect();

            for (i, key) in key_vector.iter().enumerate() {
                info!(
                    " KEY {} KEY STAKE {} STAKING AMOUNT {} ",
                    key,
                    SubspaceModule::get_stake(netuid, key),
                    amount_staked
                );

                let delegate_key: U256 = delegate_key_vector[i];
                add_balance(delegate_key, amount_staked + 1);

                assert_ok!(register_module(netuid, *key, 10));
                info!(
                    " DELEGATE KEY STAKE {} STAKING AMOUNT {} ",
                    SubspaceModule::get_stake(netuid, &delegate_key),
                    amount_staked
                );

                assert_ok!(SubspaceModule::add_stake(
                    get_origin(delegate_key),
                    netuid,
                    *key,
                    amount_staked
                ));
                let uid = SubspaceModule::get_uid_for_key(netuid, key);
                // SubspaceModule::add_stake(get_origin(*key), netuid, amount_staked);
                assert_eq!(
                    SubspaceModule::get_stake_for_uid(netuid, uid),
                    amount_staked + 10
                );
                assert_eq!(SubspaceModule::get_balance(&delegate_key), 1);
                assert_eq!(
                    SubspaceModule::get_stake_to_vector(netuid, &delegate_key).len(),
                    1
                );
                // REMOVE STAKE
                assert_ok!(SubspaceModule::remove_stake(
                    get_origin(delegate_key),
                    netuid,
                    *key,
                    amount_staked
                ));
                assert_eq!(
                    SubspaceModule::get_balance(&delegate_key),
                    amount_staked + 1
                );
                assert_eq!(SubspaceModule::get_stake_for_uid(netuid, uid), 10);
                assert_eq!(
                    SubspaceModule::get_stake_to_vector(netuid, &delegate_key).len(),
                    0
                );

                // ADD STAKE AGAIN
                assert_ok!(SubspaceModule::add_stake(
                    get_origin(delegate_key),
                    netuid,
                    *key,
                    amount_staked
                ));
                assert_eq!(
                    SubspaceModule::get_stake_for_uid(netuid, uid),
                    amount_staked + 10
                );
                assert_eq!(SubspaceModule::get_balance(&delegate_key), 1);
                assert_eq!(
                    SubspaceModule::get_stake_to_vector(netuid, &delegate_key).len(),
                    1
                );

                // AT THE END WE SHOULD HAVE THE SAME TOTAL STAKE
                subnet_stake += SubspaceModule::get_stake_for_uid(netuid, uid);
            }
            assert_eq!(SubspaceModule::get_total_subnet_stake(netuid), subnet_stake);
            total_stake += subnet_stake;
            assert_eq!(SubspaceModule::total_stake(), total_stake);
            subnet_stake = 0;
            info!("TOTAL STAKE: {}", total_stake);
            info!(
                "TOTAL SUBNET STAKE: {}",
                SubspaceModule::get_total_subnet_stake(netuid)
            );
        }
    });
}

#[test]
fn test_ownership_ratio() {
    new_test_ext().execute_with(|| {
        let netuid: u16 = 0;
        let num_modules: u16 = 10;
        let stake_per_module: u64 = 1_000_000_000;
        // make sure that the results won´t get affected by burn
        SubspaceModule::set_min_burn(0);

        register_n_modules(netuid, num_modules, 10);

        let keys = SubspaceModule::get_keys(netuid);

        for k in &keys {
            let delegate_keys: Vec<U256> =
                (0..num_modules).map(|i| U256::from(i + num_modules + 1)).collect();
            for d in delegate_keys.iter() {
                add_balance(*d, stake_per_module + 1);
            }

            let pre_delegate_stake_from_vector = SubspaceModule::get_stake_from_vector(netuid, k);
            assert_eq!(pre_delegate_stake_from_vector.len(), 1); // +1 for the module itself, +1 for the delegate key on

            info!("KEY: {}", k);
            for (i, d) in delegate_keys.iter().enumerate() {
                info!("DELEGATE KEY: {d}");
                assert_ok!(SubspaceModule::add_stake(
                    get_origin(*d),
                    netuid,
                    *k,
                    stake_per_module
                ));
                let stake_from_vector = SubspaceModule::get_stake_from_vector(netuid, k);
                assert_eq!(
                    stake_from_vector.len(),
                    pre_delegate_stake_from_vector.len() + i + 1
                );
            }
            let ownership_ratios: Vec<(U256, I64F64)> =
                SubspaceModule::get_ownership_ratios(netuid, k);

            assert_eq!(ownership_ratios.len(), delegate_keys.len() + 1);
            info!("OWNERSHIP RATIOS: {ownership_ratios:?}");

            step_epoch(netuid);

            let stake_from_vector = SubspaceModule::get_stake_from_vector(netuid, k);
            let stake: u64 = SubspaceModule::get_stake(netuid, k);
            let sumed_stake: u64 = stake_from_vector.iter().fold(0, |acc, (_a, x)| acc + x);
            let total_stake: u64 = SubspaceModule::get_total_subnet_stake(netuid);

            info!("STAKE: {}", stake);
            info!("SUMED STAKE: {sumed_stake}");
            info!("TOTAL STAKE: {total_stake}");

            assert_eq!(stake, sumed_stake);

            // for (d_a, o) in ownership_ratios.iter() {
            //     info!("OWNERSHIP RATIO: {}", o);

            // }
        }
    });
}

#[test]
fn test_min_stake() {
    new_test_ext().execute_with(|| {
        let netuid: u16 = 0;
        let num_modules: u16 = 10;
        let min_stake: u64 = 10_000_000_000;
        // make sure that the results won´t get affected by burn
        SubspaceModule::set_min_burn(0);

        register_n_modules(netuid, num_modules, min_stake);
        let keys = SubspaceModule::get_keys(netuid);

        update_params!(netuid => { min_stake: min_stake - 100 });

        assert_ok!(SubspaceModule::remove_stake(
            get_origin(keys[0]),
            netuid,
            keys[0],
            10_000_000_000
        ));
    });
}

#[test]
fn test_stake_zero() {
    new_test_ext().execute_with(|| {
        // Register the general subnet.
        let netuid: u16 = 0;
        let key = U256::from(0);
        let stake_amount: u64 = to_nano(1_000);

        // Make sure registration cost is not affected
        SubspaceModule::set_min_burn(0);

        assert_ok!(register_module(netuid, key, stake_amount));

        // try to stake zero
        let key_two = U256::from(1);

        assert_noop!(
            SubspaceModule::do_add_stake(get_origin(key_two), netuid, key, 0),
            Error::<Test>::NotEnoughBalanceToStake
        );
    });
}

#[test]
fn test_unstake_zero() {
    new_test_ext().execute_with(|| {
        // Register the general subnet.
        let netuid: u16 = 0;
        let key = U256::from(0);
        let stake_amount: u64 = to_nano(1_000);

        // Make sure registration cost is not affected
        SubspaceModule::set_min_burn(0);

        assert_ok!(register_module(netuid, key, stake_amount));

        // try to unstake zero
        let key_two = U256::from(1);

        assert_noop!(
            SubspaceModule::do_remove_stake(get_origin(key_two), netuid, key, 0),
            Error::<Test>::NotEnoughStakeToWithdraw
        );
    });
}
