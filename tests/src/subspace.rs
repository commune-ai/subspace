use crate::mock::*;
use frame_support::{assert_err, assert_noop, assert_ok};
use log::info;
use pallet_subspace::{
    global::{BurnConfiguration, SubnetBurnConfiguration},
    *,
};
use sp_core::U256;
use sp_runtime::{DispatchError, DispatchResult, Percent};
use sp_std::vec;
use std::collections::BTreeSet;
use substrate_fixed::types::I64F64;

// ------------------
// Delegate Staking
// ------------------

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

        let keys = SubspaceMod::get_keys(netuid);
        let voter_key = keys[0];
        let miner_keys = keys[1..].to_vec();
        let miner_uids: Vec<u16> =
            miner_keys.iter().map(|k| SubspaceMod::get_uid_for_key(netuid, k)).collect();
        let miner_weights = vec![1; miner_uids.len()];

        let delegate_keys: Vec<U256> =
            (0..num_modules).map(|i| U256::from(i + num_modules + 1)).collect();
        for d in delegate_keys.iter() {
            add_balance(*d, stake_per_module + 1);
        }

        let pre_delegate_stake_from_vector = SubspaceMod::get_stake_from_vector(&voter_key);
        assert_eq!(pre_delegate_stake_from_vector.len(), 1); // +1 for the module itself, +1 for the delegate key on

        for (i, d) in delegate_keys.iter().enumerate() {
            assert_ok!(SubspaceMod::add_stake(
                get_origin(*d),
                voter_key,
                stake_per_module,
            ));
            let stake_from_vector = SubspaceMod::get_stake_from_vector(&voter_key);
            assert_eq!(
                stake_from_vector.len(),
                pre_delegate_stake_from_vector.len() + i + 1
            );
        }
        let ownership_ratios = SubspaceMod::get_ownership_ratios(netuid, &voter_key);
        assert_eq!(ownership_ratios.len(), delegate_keys.len() + 1);

        let total_balance = keys
            .iter()
            .map(SubspaceMod::get_balance)
            .collect::<Vec<u64>>()
            .iter()
            .sum::<u64>();
        let total_stake = keys
            .iter()
            .map(|k| SubspaceMod::get_stake_to_module(k, k))
            .collect::<Vec<u64>>()
            .iter()
            .sum::<u64>();
        let total_delegate_stake = delegate_keys
            .iter()
            .map(|k| SubspaceMod::get_stake_to_module(k, &voter_key))
            .collect::<Vec<u64>>()
            .iter()
            .sum::<u64>();
        let total_delegate_balance = delegate_keys
            .iter()
            .map(SubspaceMod::get_balance)
            .collect::<Vec<u64>>()
            .iter()
            .sum::<u64>();
        let total_tokens_before =
            total_balance + total_stake + total_delegate_stake + total_delegate_balance;

        let result = SubspaceMod::set_weights(
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
            .map(SubspaceMod::get_balance)
            .collect::<Vec<u64>>()
            .iter()
            .sum::<u64>();
        let total_stake = keys
            .iter()
            .map(|k| SubspaceMod::get_stake_to_module(k, k))
            .collect::<Vec<u64>>()
            .iter()
            .sum::<u64>();
        let total_delegate_stake = delegate_keys
            .iter()
            .map(|k| SubspaceMod::get_stake_to_module(k, &voter_key))
            .collect::<Vec<u64>>()
            .iter()
            .sum::<u64>();
        let total_delegate_balance = delegate_keys
            .iter()
            .map(SubspaceMod::get_balance)
            .collect::<Vec<u64>>()
            .iter()
            .sum::<u64>();
        let total_tokens_after =
            total_balance + total_stake + total_delegate_stake + total_delegate_balance;
        let total_new_tokens = total_tokens_after - total_tokens_before;

        assert_eq!(total_new_tokens, total_emissions);

        let stake_from_vector = SubspaceMod::get_stake_from_vector(&voter_key);
        let _stake: u64 = SubspaceMod::get_stake(netuid, &voter_key);
        let _sumed_stake: u64 = stake_from_vector.iter().fold(0, |acc, (_a, x)| acc + x);
        let _total_stake: u64 = SubspaceMod::get_total_subnet_stake(netuid);
        info!("stake_from_vector: {stake_from_vector:?}");
    });
}

// ------------------
// Registrations
// ------------------

#[test]
fn test_delegate_register() {
    new_test_ext().execute_with(|| {
        let netuid: u16 = 0;
        let n: u16 = 10;
        let key: U256 = U256::from(n + 1);
        let module_keys: Vec<U256> = (0..n).map(U256::from).collect();
        let stake_amount: u64 = 10_000_000_000;
        // make sure that the results won´t get affected by burn
        zero_min_burn();

        SubspaceMod::add_balance_to_account(&key, stake_amount * n as u64);
        for module_key in module_keys {
            delegate_register_module(netuid, key, module_key, stake_amount)
                .expect("delegate register module failed");
            let key_balance = SubspaceMod::get_balance_u64(&key);
            let stake_to_module = SubspaceMod::get_stake_to_module(&key, &module_key);
            info!("key_balance: {key_balance:?}");
            let stake_to_vector = SubspaceMod::get_stake_to_vector(&key);
            info!("stake_to_vector: {stake_to_vector:?}");
            assert_eq!(stake_to_module, stake_amount);
        }
    });
}

#[test]
fn test_registration_with_stake() {
    new_test_ext().execute_with(|| {
        let netuid = 0;
        let stake_vector: Vec<u64> = [100000, 1000000, 10000000].to_vec();

        // make sure that the results won´t get affected by burn
        zero_min_burn();

        for (i, stake) in stake_vector.iter().enumerate() {
            let uid: u16 = i as u16;
            let stake_value: u64 = *stake;

            let key = U256::from(uid);
            info!("key: {key:?}");
            info!("stake: {stake_value:?}");
            let stake_before: u64 = SubspaceMod::get_stake(netuid, &key);
            info!("stake_before: {stake_before:?}");
            register_module(netuid, key, stake_value).unwrap_or_else(|_| {
                panic!("Failed to register module with key: {key:?} and stake: {stake_value:?}",)
            });
            info!("balance: {:?}", SubspaceMod::get_balance_u64(&key));
            assert_eq!(get_stake_for_uid(netuid, uid), stake_value);
        }
    });
}

#[test]
fn deregister_within_subnet_when_limit_is_reached() {
    new_test_ext().execute_with(|| {
        MaxAllowedModules::<Test>::set(3);
        assert_ok!(register_module(0, 0.into(), to_nano(10_000)));
        assert_ok!(register_module(1, 1.into(), to_nano(5_000)));

        assert_eq!(Stake::<Test>::get(0, U256::from(0)), to_nano(9_996));
        assert_eq!(Stake::<Test>::get(1, U256::from(1)), to_nano(4_996));

        MaxAllowedUids::<Test>::set(0, 1);
        MaxAllowedUids::<Test>::set(1, 1);

        assert_ok!(register_module(0, 2.into(), to_nano(15_000)));

        assert_eq!(Stake::<Test>::get(0, U256::from(2)), to_nano(14_996));
        assert_eq!(Stake::<Test>::get(1, U256::from(1)), to_nano(4_996));

        assert_eq!(Emission::<Test>::get(0).len(), 1);
        assert_eq!(Emission::<Test>::get(1).len(), 1);
    });
}

#[test]
fn deregister_globally_when_global_limit_is_reached() {
    new_test_ext().execute_with(|| {
        MaxAllowedModules::<Test>::set(2);

        assert_ok!(register_module(0, 0.into(), to_nano(10_000)));
        assert_ok!(register_module(1, 1.into(), to_nano(5_000)));

        assert_eq!(Stake::<Test>::get(0, U256::from(0)), to_nano(9_996));
        assert_eq!(Stake::<Test>::get(1, U256::from(1)), to_nano(4_996));

        MaxAllowedUids::<Test>::set(0, 2);
        MaxAllowedUids::<Test>::set(1, 1);

        assert_ok!(register_module(0, 2.into(), to_nano(15_000)));

        assert_eq!(Stake::<Test>::get(0, U256::from(0)), to_nano(9_996));
        assert_eq!(Stake::<Test>::get(0, U256::from(2)), to_nano(14_996));
        assert_eq!(Stake::<Test>::get(1, U256::from(1)), 0);

        assert_eq!(Emission::<Test>::get(0).len(), 2);
        assert_eq!(Emission::<Test>::get(1).len(), 0);
    });
}

#[test]
fn test_register_invalid_subnet_name() {
    new_test_ext().execute_with(|| {
        let address = b"0x1234567890".to_vec();
        let stake = to_nano(1);
        let module_name = b"test".to_vec();

        // Make registrations free
        zero_min_burn();

        // Set min name length
        MinNameLength::<Test>::put(2);

        // Get the minimum and maximum name lengths from the configuration
        let min_name_length = MinNameLength::<Test>::get();
        let max_name_length = MaxNameLength::<Test>::get();

        let register_one = U256::from(0);
        let empty_name = Vec::new();
        add_balance(register_one, stake + 1);
        assert_noop!(
            SubspaceMod::register(
                get_origin(register_one),
                empty_name,
                module_name.clone(),
                address.clone(),
                stake,
                register_one,
                None,
            ),
            Error::<Test>::InvalidSubnetName
        );

        // Try registering with a name that is too short (invalid)
        let register_two = U256::from(1);
        let short_name = b"a".to_vec();
        add_balance(register_two, stake + 1);
        assert_noop!(
            SubspaceMod::register(
                get_origin(register_two),
                short_name,
                module_name.clone(),
                address.clone(),
                stake,
                register_two,
                None,
            ),
            Error::<Test>::SubnetNameTooShort
        );

        // Try registering with a name that is exactly the minimum length (valid)
        let register_three = U256::from(2);
        let min_length_name = vec![b'a'; min_name_length as usize];
        add_balance(register_three, stake + 1);
        assert_ok!(SubspaceMod::register(
            get_origin(register_three),
            min_length_name,
            module_name.clone(),
            address.clone(),
            stake,
            register_three,
            None,
        ));

        // Try registering with a name that is exactly the maximum length (valid)
        let max_length_name = vec![b'a'; max_name_length as usize];
        let register_four = U256::from(3);
        add_balance(register_four, stake + 1);
        assert_ok!(SubspaceMod::register(
            get_origin(register_four),
            max_length_name,
            module_name.clone(),
            address.clone(),
            stake,
            register_four,
            None,
        ));

        // Try registering with a name that is too long (invalid)
        let long_name = vec![b'a'; (max_name_length + 1) as usize];
        let register_five = U256::from(4);
        add_balance(register_five, stake + 1);
        assert_noop!(
            SubspaceMod::register(
                get_origin(register_five),
                long_name,
                module_name.clone(),
                address.clone(),
                stake,
                register_five,
                None,
            ),
            Error::<Test>::SubnetNameTooLong
        );

        // Try registering with an invalid UTF-8 name (invalid)
        let invalid_utf8_name = vec![0xFF, 0xFF];
        let register_six = U256::from(5);
        add_balance(register_six, stake + 1);
        assert_noop!(
            SubspaceMod::register(
                get_origin(register_six),
                invalid_utf8_name,
                module_name.clone(),
                address,
                stake,
                register_six,
                None,
            ),
            Error::<Test>::InvalidSubnetName
        );
    });
}

// Subnet 0 Whitelist

#[test]
fn new_subnet_reutilized_removed_netuid_if_total_is_bigger_than_removed() {
    new_test_ext().execute_with(|| {
        zero_min_burn();

        TotalSubnets::<Test>::set(10);
        SubnetGaps::<Test>::set(BTreeSet::from([5]));
        assert_ok!(register_module(0, 0.into(), to_nano(1)));

        let subnets: Vec<_> = N::<Test>::iter().collect();
        assert_eq!(subnets, vec![(5, 1)]);
        assert_eq!(SubnetGaps::<Test>::get(), BTreeSet::from([]));
    });
}

#[test]
fn new_subnet_does_not_reute_removed_netuid_if_total_is_smaller_than_removed() {
    new_test_ext().execute_with(|| {
        zero_min_burn();

        TotalSubnets::<Test>::set(3);
        SubnetGaps::<Test>::set(BTreeSet::from([7]));
        assert_ok!(register_module(0, 0.into(), to_nano(1)));

        let subnets: Vec<_> = N::<Test>::iter().collect();
        assert_eq!(subnets, vec![(7, 1)]);
        assert_eq!(SubnetGaps::<Test>::get(), BTreeSet::from([]));
    });
}

#[test]
fn new_subnets_on_removed_uids_register_modules_to_the_correct_netuids() {
    fn assert_subnets(v: &[(u16, &str)]) {
        let v: Vec<_> = v.iter().map(|(u, n)| (*u, n.as_bytes().to_vec())).collect();
        let names: Vec<_> = SubnetNames::<Test>::iter().collect();
        assert_eq!(names, v);
    }

    new_test_ext().execute_with(|| {
        zero_min_burn();
        MaxAllowedSubnets::<Test>::put(3);

        assert_ok!(register_module(0, 0.into(), to_nano(10)));
        assert_ok!(register_module(1, 1.into(), to_nano(5)));
        assert_ok!(register_module(2, 2.into(), to_nano(1)));
        assert_subnets(&[(0, "test0"), (1, "test1"), (2, "test2")]);

        assert_ok!(register_module(3, 3.into(), to_nano(15)));
        assert_subnets(&[(0, "test0"), (1, "test1"), (2, "test3")]);

        assert_ok!(register_module(4, 4.into(), to_nano(20)));
        assert_subnets(&[(0, "test0"), (1, "test4"), (2, "test3")]);

        add_balance(0.into(), to_nano(50));
        add_stake(0, 0.into(), to_nano(10));

        assert_ok!(register_module(5, 5.into(), to_nano(17)));
        assert_subnets(&[(0, "test0"), (1, "test4"), (2, "test5")]);

        assert_eq!(Stake::<Test>::iter_key_prefix(0).count(), 1);
        assert_eq!(Stake::<Test>::iter_key_prefix(1).count(), 1);
        assert_eq!(Stake::<Test>::iter_key_prefix(2).count(), 1);

        assert_eq!(N::<Test>::get(0), 1);
        assert_eq!(N::<Test>::get(1), 1);
        assert_eq!(N::<Test>::get(2), 1);
    });
}

// ------------------
// Staking
// ------------------

#[test]
fn test_stake() {
    new_test_ext().execute_with(|| {
        let max_uids: u16 = 10;
        let netuids: [u16; 4] = core::array::from_fn(|i| i as u16);
        let amount_staked_vector: Vec<u64> = netuids.iter().map(|_| to_nano(10)).collect();
        let mut total_stake: u64 = 0;
        let mut subnet_stake: u64 = 0;
        // make sure that the results won´t get affected by burn
        zero_min_burn();
        MaxRegistrationsPerBlock::<Test>::set(1000);

        for netuid in netuids {
            info!("NETUID: {}", netuid);
            let amount_staked = amount_staked_vector[netuid as usize];
            let key_vector: Vec<U256> =
                (0..max_uids).map(|i| U256::from(i + max_uids * netuid)).collect();

            for key in key_vector.iter() {
                info!(
                    " KEY {} KEY STAKE {} STAKING AMOUNT {} ",
                    key,
                    SubspaceMod::get_stake(netuid, key),
                    amount_staked
                );

                assert_ok!(register_module(netuid, *key, amount_staked));
                info!(
                    " KEY STAKE {} STAKING AMOUNT {} ",
                    SubspaceMod::get_stake(netuid, key),
                    amount_staked
                );

                // SubspaceMod::add_stake(get_origin(*key), netuid, amount_staked);
                assert_eq!(SubspaceMod::get_stake(netuid, key), amount_staked);
                assert_eq!(SubspaceMod::get_balance(key), 1);

                // REMOVE STAKE
                assert_ok!(SubspaceMod::remove_stake(
                    get_origin(*key),
                    netuid,
                    *key,
                    amount_staked
                ));
                assert_eq!(SubspaceMod::get_balance(key), amount_staked + 1);
                assert_eq!(SubspaceMod::get_stake(netuid, key), 0);

                // ADD STAKE AGAIN LOL
                assert_ok!(SubspaceMod::add_stake(
                    get_origin(*key),
                    netuid,
                    *key,
                    amount_staked,
                ));
                assert_eq!(SubspaceMod::get_stake(netuid, key), amount_staked);
                assert_eq!(SubspaceMod::get_balance(key), 1);

                // AT THE END WE SHOULD HAVE THE SAME TOTAL STAKE
                subnet_stake += SubspaceMod::get_stake(netuid, key);
            }
            assert_eq!(SubspaceMod::get_total_subnet_stake(netuid), subnet_stake);
            total_stake += subnet_stake;
            assert_eq!(SubspaceMod::total_stake(), total_stake);
            subnet_stake = 0;
            info!("TOTAL STAKE: {}", total_stake);
            info!(
                "TOTAL SUBNET STAKE: {}",
                SubspaceMod::get_total_subnet_stake(netuid)
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
        zero_min_burn();

        register_n_modules(netuid, n, 10);
        let controler_key = U256::from(n + 1);
        let og_staker_balance: u64 = total_stake + 1;
        add_balance(controler_key, og_staker_balance);

        let keys: Vec<U256> = SubspaceMod::get_keys(netuid);

        // stake to all modules

        let stake_amounts: Vec<u64> = vec![stake_amount; num_staked_modules as usize];

        info!("STAKE AMOUNTS: {stake_amounts:?}");
        let total_actual_stake: u64 =
            keys.clone().into_iter().map(|k| SubspaceMod::get_stake(netuid, &k)).sum();
        let staker_balance = SubspaceMod::get_balance(&controler_key);
        info!("TOTAL ACTUAL STAKE: {total_actual_stake}");
        info!("TOTAL STAKE: {total_stake}");
        info!("STAKER BALANCE: {staker_balance}");
        assert_ok!(SubspaceMod::add_stake_multiple(
            get_origin(controler_key),
            netuid,
            keys.clone(),
            stake_amounts.clone(),
        ));

        let total_actual_stake: u64 =
            keys.clone().into_iter().map(|k| SubspaceMod::get_stake(netuid, &k)).sum();
        let staker_balance = SubspaceMod::get_balance(&controler_key);

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
        assert_ok!(SubspaceMod::remove_stake_multiple(
            get_origin(controler_key),
            netuid,
            keys.clone(),
            stake_amounts.clone(),
        ));

        let total_actual_stake: u64 =
            keys.clone().into_iter().map(|k| SubspaceMod::get_stake(netuid, &k)).sum();
        let staker_balance = SubspaceMod::get_balance(&controler_key);
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
        zero_min_burn();

        register_n_modules(netuid, n, stake_amount);

        let keys: Vec<U256> = SubspaceMod::get_keys(netuid);

        assert_ok!(SubspaceMod::transfer_stake(
            get_origin(keys[0]),
            netuid,
            keys[0],
            keys[1],
            stake_amount,
        ));

        let key0_stake = SubspaceMod::get_stake(netuid, &keys[0]);
        let key1_stake = SubspaceMod::get_stake(netuid, &keys[1]);
        assert_eq!(key0_stake, 0);
        assert_eq!(key1_stake, stake_amount * 2);

        assert_ok!(SubspaceMod::transfer_stake(
            get_origin(keys[0]),
            netuid,
            keys[1],
            keys[0],
            stake_amount,
        ));

        let key0_stake = SubspaceMod::get_stake(netuid, &keys[0]);
        let key1_stake = SubspaceMod::get_stake(netuid, &keys[1]);
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
        zero_min_burn();
        MaxRegistrationsPerBlock::<Test>::set(1000);

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
                    SubspaceMod::get_stake(netuid, key),
                    amount_staked
                );

                let delegate_key: U256 = delegate_key_vector[i];
                add_balance(delegate_key, amount_staked + 1);

                assert_ok!(register_module(netuid, *key, 10));
                info!(
                    " DELEGATE KEY STAKE {} STAKING AMOUNT {} ",
                    SubspaceMod::get_stake(netuid, &delegate_key),
                    amount_staked
                );

                assert_ok!(SubspaceMod::add_stake(
                    get_origin(delegate_key),
                    netuid,
                    *key,
                    amount_staked,
                ));
                let uid = SubspaceMod::get_uid_for_key(netuid, key);
                // SubspaceMod::add_stake(get_origin(*key), netuid, amount_staked);
                assert_eq!(get_stake_for_uid(netuid, uid), amount_staked + 10);
                assert_eq!(SubspaceMod::get_balance(&delegate_key), 1);
                assert_eq!(
                    SubspaceMod::get_stake_to_vector(netuid, &delegate_key).len(),
                    1
                );
                // REMOVE STAKE
                assert_ok!(SubspaceMod::remove_stake(
                    get_origin(delegate_key),
                    netuid,
                    *key,
                    amount_staked
                ));
                assert_eq!(SubspaceMod::get_balance(&delegate_key), amount_staked + 1);
                assert_eq!(get_stake_for_uid(netuid, uid), 10);
                assert_eq!(
                    SubspaceMod::get_stake_to_vector(netuid, &delegate_key).len(),
                    0
                );

                // ADD STAKE AGAIN
                assert_ok!(SubspaceMod::add_stake(
                    get_origin(delegate_key),
                    netuid,
                    *key,
                    amount_staked,
                ));
                assert_eq!(get_stake_for_uid(netuid, uid), amount_staked + 10);
                assert_eq!(SubspaceMod::get_balance(&delegate_key), 1);
                assert_eq!(
                    SubspaceMod::get_stake_to_vector(netuid, &delegate_key).len(),
                    1
                );

                // AT THE END WE SHOULD HAVE THE SAME TOTAL STAKE
                subnet_stake += get_stake_for_uid(netuid, uid);
            }
            assert_eq!(SubspaceMod::get_total_subnet_stake(netuid), subnet_stake);
            total_stake += subnet_stake;
            assert_eq!(SubspaceMod::total_stake(), total_stake);
            subnet_stake = 0;
            info!("TOTAL STAKE: {}", total_stake);
            info!(
                "TOTAL SUBNET STAKE: {}",
                SubspaceMod::get_total_subnet_stake(netuid)
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
        zero_min_burn();

        register_n_modules(netuid, num_modules, 10);

        let keys = SubspaceMod::get_keys(netuid);

        for k in &keys {
            let delegate_keys: Vec<U256> =
                (0..num_modules).map(|i| U256::from(i + num_modules + 1)).collect();
            for d in delegate_keys.iter() {
                add_balance(*d, stake_per_module + 1);
            }

            let pre_delegate_stake_from_vector = SubspaceMod::get_stake_from_vector(netuid, k);
            assert_eq!(pre_delegate_stake_from_vector.len(), 1); // +1 for the module itself, +1 for the delegate key on

            info!("KEY: {}", k);
            for (i, d) in delegate_keys.iter().enumerate() {
                info!("DELEGATE KEY: {d}");
                assert_ok!(SubspaceMod::add_stake(
                    get_origin(*d),
                    netuid,
                    *k,
                    stake_per_module,
                ));
                let stake_from_vector = SubspaceMod::get_stake_from_vector(netuid, k);
                assert_eq!(
                    stake_from_vector.len(),
                    pre_delegate_stake_from_vector.len() + i + 1
                );
            }
            let ownership_ratios: Vec<(U256, I64F64)> =
                SubspaceMod::get_ownership_ratios(netuid, k);

            assert_eq!(ownership_ratios.len(), delegate_keys.len() + 1);
            info!("OWNERSHIP RATIOS: {ownership_ratios:?}");

            step_epoch(netuid);

            let stake_from_vector = SubspaceMod::get_stake_from_vector(netuid, k);
            let stake: u64 = SubspaceMod::get_stake(netuid, k);
            let sumed_stake: u64 = stake_from_vector.iter().fold(0, |acc, (_a, x)| acc + x);
            let total_stake: u64 = SubspaceMod::get_total_subnet_stake(netuid);

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
        zero_min_burn();

        register_n_modules(netuid, num_modules, min_stake);
        let keys = SubspaceMod::get_keys(netuid);

        update_params!(netuid => { min_stake: min_stake - 100 });

        assert_ok!(SubspaceMod::remove_stake(
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
        zero_min_burn();

        assert_ok!(register_module(netuid, key, stake_amount));

        // try to stake zero
        let key_two = U256::from(1);

        assert_noop!(
            SubspaceMod::do_add_stake(get_origin(key_two), netuid, key, 0,),
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
        zero_min_burn();

        assert_ok!(register_module(netuid, key, stake_amount));

        // try to unstake zero
        let key_two = U256::from(1);

        assert_noop!(
            SubspaceMod::do_remove_stake(get_origin(key_two), netuid, key, 0),
            Error::<Test>::NotEnoughStakeToWithdraw
        );
    });
}

// ------------------
// Subnet
// ------------------

#[test]
fn test_remove_subnet_integrity() {
    new_test_ext().execute_with(|| {
        let n = 1;

        // make sure that the results won´t get affected by burn
        zero_min_burn();
        MaxRegistrationsPerBlock::<Test>::set(1000);

        assert_ok!(register_module(0, U256::from(0), 1_000_000_000));

        SubspaceMod::remove_subnet(0);

        assert_eq!(SubnetNames::<Test>::get(n), Vec::<u8>::new());
        assert_eq!(Name::<Test>::iter_prefix(n).collect::<Vec<_>>(), vec![]);
        assert_eq!(Address::<Test>::iter_prefix(n).collect::<Vec<_>>(), vec![]);
        assert_eq!(Uids::<Test>::iter_prefix(n).collect::<Vec<_>>(), vec![]);
        assert_eq!(Keys::<Test>::iter_prefix(n).collect::<Vec<_>>(), vec![]);
        assert_eq!(
            DelegationFee::<Test>::iter_prefix(n).collect::<Vec<_>>(),
            vec![]
        );
        assert_eq!(Weights::<Test>::iter_prefix(n).collect::<Vec<_>>(), vec![]);
        assert_eq!(Active::<Test>::get(n), Vec::<bool>::new());
        assert_eq!(Consensus::<Test>::get(n), Vec::<u16>::new());
        assert_eq!(Emission::<Test>::get(n), Vec::<u64>::new());
        assert_eq!(Incentive::<Test>::get(n), Vec::<u16>::new());
        assert_eq!(LastUpdate::<Test>::get(n), Vec::<u64>::new());
        assert_eq!(PruningScores::<Test>::get(n), Vec::<u16>::new());
        assert_eq!(Rank::<Test>::get(n), Vec::<u16>::new());
        assert_eq!(Trust::<Test>::get(n), Vec::<u16>::new());
        assert_eq!(ValidatorPermits::<Test>::get(n), Vec::<bool>::new());
        assert_eq!(ValidatorTrust::<Test>::get(n), Vec::<u16>::new());
        assert_eq!(
            RegistrationBlock::<Test>::iter_prefix(n).collect::<Vec<_>>(),
            vec![]
        );
        assert_eq!(SubnetEmission::<Test>::get(n), 0);
        assert_eq!(Founder::<Test>::get(n), U256::from(0));
        assert_eq!(FounderShare::<Test>::get(n), 8);
        assert_eq!(Tempo::<Test>::get(n), 100);
        assert_eq!(ImmunityPeriod::<Test>::get(n), 0);
        assert_eq!(MaxWeightAge::<Test>::get(n), 3600);
        assert_eq!(MaxAllowedWeights::<Test>::get(n), 420);
        assert_eq!(MinStake::<Test>::get(n), 0);
        assert_eq!(TrustRatio::<Test>::get(n), 0);
        assert_eq!(MaximumSetWeightCallsPerEpoch::<Test>::get(n), 0);
        assert_eq!(BondsMovingAverage::<Test>::get(n), 900000);
        assert_eq!(TargetRegistrationsInterval::<Test>::get(n), 200);
        assert_eq!(TargetRegistrationsPerInterval::<Test>::get(n), 100);
        assert_eq!(MaxRegistrationsPerInterval::<Test>::get(n), 42);
        assert_eq!(AdjustmentAlpha::<Test>::get(n), 9223372036854775807);
        assert_eq!(N::<Test>::get(n), 0);
        assert_eq!(TotalSubnets::<Test>::get(), 0);
        assert!(SubnetGaps::<Test>::get().contains(&0));
    });
}

#[test]
fn test_integrity_apply() {
    new_test_ext().execute_with(|| {
        let netuid: u16 = 0;
        let stake: u64 = 1_000_000_000;
        let extra_uids: u16 = 10;
        let rounds = 10;

        // make sure that the results won´t get affected by burn
        zero_min_burn();

        assert_ok!(register_module(netuid, U256::from(0), stake));
        MaxRegistrationsPerBlock::<Test>::set(100 + extra_uids * rounds);

        update_params!(netuid => {
            max_allowed_weights: 512,
            max_allowed_uids: 820,
            tempo: 25,
            max_weight_age: 100,
            trust_ratio: 99,
            founder: U256::from(0),
            founder_share: 8,
            immunity_period: 3,
            incentive_ratio: 4,
            min_stake: 6,
            name: bounded_vec![1, 2, 3],
            maximum_set_weight_calls_per_epoch: 10,
            bonds_ma: 11,
            target_registrations_interval: 12,
            target_registrations_per_interval: 13,
            max_registrations_per_interval: 14,
            adjustment_alpha: 15,
            governance_config: GovernanceConfiguration::default()
        });
        assert_eq!(Founder::<Test>::get(0), U256::from(0));
        assert_eq!(FounderShare::<Test>::get(0), 8);
        assert_eq!(ImmunityPeriod::<Test>::get(0), 3);
        assert_eq!(IncentiveRatio::<Test>::get(0), 4);
        assert_eq!(MinStake::<Test>::get(0), 6);
        assert_eq!(Tempo::<Test>::get(0), 25);
        assert_eq!(TrustRatio::<Test>::get(0), 99);
        assert_eq!(MaxAllowedWeights::<Test>::get(0), 512);
        assert_eq!(MaxAllowedUids::<Test>::get(0), 820);
        assert_eq!(MaxWeightAge::<Test>::get(0), 100);
        assert_eq!(MaximumSetWeightCallsPerEpoch::<Test>::get(0), 10);
        assert_eq!(BondsMovingAverage::<Test>::get(0), 11);
        assert_eq!(TargetRegistrationsInterval::<Test>::get(0), 12);
        assert_eq!(TargetRegistrationsPerInterval::<Test>::get(0), 13);
        assert_eq!(MaxRegistrationsPerInterval::<Test>::get(0), 14);
        assert_eq!(AdjustmentAlpha::<Test>::get(0), 15);
        assert_eq!(N::<Test>::get(0), 1);
        assert_eq!(TotalSubnets::<Test>::get(), 1);
        assert_eq!(SubnetNames::<Test>::get(0), vec![1, 2, 3]);
    });
}

#[test]
fn test_add_subnets() {
    new_test_ext().execute_with(|| {
        let _tempo: u16 = 13;
        let stake_per_module: u64 = 1_000_000_000;
        let max_allowed_subnets: u16 = MaxAllowedSubnets::<Test>::get();
        let mut expected_subnets = 0;
        let n = 20;
        let num_subnets: u16 = n;

        // make sure that the results won´t get affected by burn
        zero_min_burn();
        MaxRegistrationsPerBlock::<Test>::set(1000);

        for i in 0..num_subnets {
            assert_ok!(register_module(i, U256::from(i), stake_per_module));
            for j in 0..n {
                if j != i {
                    let n = N::<Test>::get(i);
                    info!("registering module i:{} j:{} n:{}", i, j, n);
                    assert_ok!(register_module(i, U256::from(j), stake_per_module));
                }
            }
            expected_subnets += 1;
            if expected_subnets > max_allowed_subnets {
                expected_subnets = max_allowed_subnets;
            } else {
                assert_eq!(N::<Test>::get(i), n);
            }
            assert_eq!(
                TotalSubnets::<Test>::get(),
                expected_subnets,
                "number of subnets is not equal to expected subnets"
            );
        }

        for netuid in 0..num_subnets {
            let total_stake = SubspaceMod::get_total_subnet_stake(netuid);
            let total_balance = get_total_subnet_balance(netuid);
            let total_tokens_before = total_stake + total_balance;

            let keys = SubspaceMod::get_keys(netuid);

            info!("total stake {total_stake}");
            info!("total balance {total_balance}");
            info!("total tokens before {total_tokens_before}");

            assert_eq!(keys.len() as u16, n);
            assert!(check_subnet_storage(netuid));
            SubspaceMod::remove_subnet(netuid);
            assert_eq!(N::<Test>::get(netuid), 0);
            assert!(check_subnet_storage(netuid));

            let total_tokens_after: u64 = keys.iter().map(SubspaceMod::get_balance_u64).sum();
            info!("total tokens after {}", total_tokens_after);

            assert_eq!(total_tokens_after, total_tokens_before);
            expected_subnets = expected_subnets.saturating_sub(1);
            assert_eq!(
                TotalSubnets::<Test>::get(),
                expected_subnets,
                "number of subnets is not equal to expected subnets"
            );
        }
    });
}

#[test]
fn test_emission_ratio() {
    new_test_ext().execute_with(|| {
        let netuids: Vec<u16> = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9].to_vec();
        let stake_per_module: u64 = 1_000_000_000;
        let mut emissions_per_subnet: Vec<u64> = Vec::new();
        let max_delta: f64 = 1.0;
        let _n: u16 = 10;

        // make sure that the results won´t get affected by burn
        zero_min_burn();

        for i in 0..netuids.len() {
            let _key = U256::from(netuids[i]);
            let netuid = netuids[i];
            register_n_modules(netuid, 1, stake_per_module);
            let threshold = SubnetStakeThreshold::<Test>::get();
            let subnet_emission: u64 = SubspaceMod::calculate_network_emission(netuid, threshold);
            emissions_per_subnet.push(subnet_emission);
            let _expected_emission_factor: f64 = 1.0 / (netuids.len() as f64);
            let emission_per_block = SubspaceMod::get_total_emission_per_block();
            let expected_emission: u64 = emission_per_block / (i as u64 + 1);

            let block = block_number();
            // magnitude of difference between expected and actual emission
            let delta = if subnet_emission > expected_emission {
                subnet_emission - expected_emission
            } else {
                expected_emission - subnet_emission
            } as f64;

            assert!(
                delta <= max_delta,
                "emission {} is too far from expected emission {} ",
                subnet_emission,
                expected_emission
            );
            assert!(block == 0, "block {} is not 0", block);
            info!("block {} subnet_emission {} ", block, subnet_emission);
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

        // make sure that the results won´t get affected by burn
        zero_min_burn();

        assert_ok!(register_module(netuid, U256::from(0), stake));
        MaxRegistrationsPerBlock::<Test>::set(max_uids + extra_uids * rounds);
        for i in 1..max_uids {
            assert_ok!(register_module(netuid, U256::from(i), stake));
            assert_eq!(N::<Test>::get(netuid), i + 1);
        }
        let mut n: u16 = N::<Test>::get(netuid);
        let old_n: u16 = n;
        let mut _uids: Vec<u16>;
        assert_eq!(N::<Test>::get(netuid), max_uids);
        for r in 1..rounds {
            // set max allowed uids to max_uids + extra_uids
            update_params!(netuid => {
                max_allowed_uids: max_uids + extra_uids * (r - 1)
            });
            max_uids = MaxAllowedUids::<Test>::get(netuid);
            let new_n = old_n + extra_uids * (r - 1);
            // print the pruned uids
            for uid in old_n + extra_uids * (r - 1)..old_n + extra_uids * r {
                assert_ok!(register_module(netuid, U256::from(uid), stake));
            }

            // set max allowed uids to max_uids

            n = N::<Test>::get(netuid);
            assert_eq!(n, new_n);

            let uids = SubspaceMod::get_uids(netuid);
            assert_eq!(uids.len() as u16, n);

            let keys = SubspaceMod::get_keys(netuid);
            assert_eq!(keys.len() as u16, n);

            let names = SubspaceMod::get_names(netuid);
            assert_eq!(names.len() as u16, n);

            let addresses = SubspaceMod::get_addresses(netuid);
            assert_eq!(addresses.len() as u16, n);
        }
    });
}

#[test]
fn test_set_max_allowed_uids_shrinking() {
    new_test_ext().execute_with(|| {
        let netuid: u16 = 0;
        let stake: u64 = 1_000_000_000;
        let max_uids: u16 = 100;
        let extra_uids: u16 = 20;

        // make sure that the results won´t get affected by burn
        zero_min_burn();

        let mut n = N::<Test>::get(netuid);
        info!("registering module {}", n);
        assert_ok!(register_module(netuid, U256::from(0), stake));
        update_params!(netuid => {
            max_allowed_uids: max_uids + extra_uids
        });
        MaxRegistrationsPerBlock::<Test>::set(max_uids + extra_uids);

        for i in 1..(max_uids + extra_uids) {
            let result = register_module(netuid, U256::from(i), stake);
            result.unwrap();
            n = N::<Test>::get(netuid);
        }

        assert_eq!(n, max_uids + extra_uids);

        let keys = SubspaceMod::get_keys(netuid);

        let mut total_stake: u64 = SubspaceMod::get_total_subnet_stake(netuid);
        let mut expected_stake: u64 = n as u64 * stake;

        info!("total stake {total_stake}");
        info!("expected stake {expected_stake}");
        assert_eq!(total_stake, expected_stake);

        let mut params = SubspaceMod::subnet_params(netuid).clone();
        params.max_allowed_uids = max_uids;
        params.name = "test2".as_bytes().to_vec().try_into().unwrap();
        let result = SubspaceMod::update_subnet(
            get_origin(keys[0]),
            netuid,
            params.founder,
            params.founder_share,
            params.immunity_period,
            params.incentive_ratio,
            params.max_allowed_uids,
            params.max_allowed_weights,
            params.min_allowed_weights,
            params.max_weight_age,
            params.min_stake,
            params.name.clone(),
            params.tempo,
            params.trust_ratio,
            params.maximum_set_weight_calls_per_epoch,
            params.governance_config.vote_mode,
            params.bonds_ma,
            params.target_registrations_interval,
            params.target_registrations_per_interval,
            params.max_registrations_per_interval,
            params.adjustment_alpha,
        );
        let global_params = SubspaceMod::global_params();
        info!("global params {:?}", global_params);
        info!("subnet params {:?}", SubspaceMod::subnet_params(netuid));
        assert_ok!(result);
        let params = SubspaceMod::subnet_params(netuid);
        let n = N::<Test>::get(netuid);
        assert_eq!(
            params.max_allowed_uids, max_uids,
            "max allowed uids is not equal to expected max allowed uids"
        );
        assert_eq!(
            params.max_allowed_uids, n,
            "min allowed weights is not equal to expected min allowed weights"
        );

        let stake_vector: Vec<u64> = get_stakes(netuid);
        let calc_stake: u64 = stake_vector.iter().sum();

        info!("calculated  stake {}", calc_stake);

        expected_stake = (max_uids) as u64 * stake;
        let _subnet_stake = SubspaceMod::get_total_subnet_stake(netuid);
        total_stake = SubspaceMod::total_stake();

        assert_eq!(total_stake, expected_stake);
    });
}

#[test]
fn test_set_max_allowed_modules() {
    new_test_ext().execute_with(|| {
        let netuid: u16 = 0;
        let stake: u64 = 1_000_000_000;
        let _max_uids: u16 = 2000;
        let _extra_uids: u16 = 20;
        let max_allowed_modules: u16 = 100;

        // make sure that the results won´t get affected by burn
        zero_min_burn();
        MaxRegistrationsPerBlock::<Test>::set(1000);
        MaxAllowedModules::<Test>::put(max_allowed_modules);
        for i in 1..(2 * max_allowed_modules) {
            assert_ok!(register_module(netuid, U256::from(i), stake));
            let n = N::<Test>::get(netuid);
            assert!(
                n <= max_allowed_modules,
                "subnet_n {:?} is not less than max_allowed_modules {:?}",
                n,
                max_allowed_modules
            );
        }
    })
}

#[test]
fn test_deregister_subnet_when_overflows_max_allowed_subnets() {
    new_test_ext().execute_with(|| {
        let extra = 1;
        let mut params = SubspaceMod::global_params();
        params.max_allowed_subnets = 3;
        assert_ok!(SubspaceMod::set_global_params(params.clone()));
        // make sure that the results won´t get affected by burn
        zero_min_burn();

        assert_eq!(params.max_allowed_subnets, 3);

        let stakes: Vec<u64> = vec![
            2_000_000_000,
            6_000_000_000,
            3_000_000_000,
            4_000_000_000,
            9_000_000_000,
        ];

        for netuid in 0..params.max_allowed_subnets + extra {
            let stake: u64 = stakes[netuid as usize];
            assert_ok!(register_module(netuid, U256::from(netuid), stake));
        }

        assert_eq!(SubspaceMod::get_total_subnet_stake(1), stakes[1]);
        assert_eq!(SubspaceMod::get_total_subnet_stake(2), stakes[2]);
        assert_eq!(SubspaceMod::get_total_subnet_stake(0), stakes[3]);
        assert_eq!(TotalSubnets::<Test>::get(), 3);
    });
}

#[test]
fn test_emission_distribution_novote() {
    // test if subnet emissions are distributed correctly, even without voting
    new_test_ext().execute_with(|| {
        let netuid_general: u16 = 0; // hold 50% of the networks stake
        let stake_general: u64 = to_nano(500_000);

        let netuid_yuma: u16 = 1; // holds 45% of the networks stake
        let stake_yuma: u64 = to_nano(450_000);

        let netuid_below_threshold: u16 = 2; // holds 5% of the networks stake
        let stake_below_threshold: u64 = to_nano(50_000);

        // making sure the unit emission are set correctly
        UnitEmission::<Test>::put(23148148148);
        zero_min_burn();
        SubnetStakeThreshold::<Test>::put(Percent::from_percent(10));
        let blocks_in_day: u16 = 10_800;
        // this is aprox. the stake we expect at the end of the day with the above unit emission
        let expected_stake_change = to_nano(250_000);
        let expected_stake_change_general = (stake_general as f64
            / ((stake_general + stake_yuma) as f64)
            * expected_stake_change as f64) as u64;
        let expected_stake_change_yuma = (stake_yuma as f64 / ((stake_general + stake_yuma) as f64)
            * expected_stake_change as f64) as u64;
        let expected_stake_change_below = 0;
        let change_tolerance = to_nano(22) as i64; // we tolerate 22 token difference (due to rounding)

        // first register the general subnet
        assert_ok!(register_module(
            netuid_general,
            U256::from(0),
            stake_general
        ));
        FounderShare::<Test>::set(netuid_general, 0);

        // then register the yuma subnet
        assert_ok!(register_module(netuid_yuma, U256::from(1), stake_yuma));

        // then register the below threshold subnet
        assert_ok!(register_module(
            netuid_below_threshold,
            U256::from(2),
            stake_below_threshold
        ));

        FounderShare::<Test>::set(0, 0);
        FounderShare::<Test>::set(1, 0);
        FounderShare::<Test>::set(2, 0);

        step_block(blocks_in_day);

        let general_netuid_stake = from_nano(SubspaceMod::get_total_subnet_stake(netuid_general));
        let yuma_netuid_stake = from_nano(SubspaceMod::get_total_subnet_stake(netuid_yuma));
        let below_threshold_netuid_stake =
            from_nano(SubspaceMod::get_total_subnet_stake(netuid_below_threshold));

        let general_netuid_stake = (general_netuid_stake as f64 / 100.0).round() * 100.0;
        let yuma_netuid_stake = (yuma_netuid_stake as f64 / 100.0).round() * 100.0;
        let below_threshold_netuid_stake =
            (below_threshold_netuid_stake as f64 / 100.0).round() * 100.0;

        let start_stake = stake_general + stake_yuma + stake_below_threshold;
        let end_day_stake = to_nano(
            (general_netuid_stake + yuma_netuid_stake + below_threshold_netuid_stake) as u64,
        );
        let stake_change = end_day_stake - start_stake;
        assert_eq!(stake_change, expected_stake_change);

        // Check the expected difference for the general subnet
        let general_stake_change = to_nano(general_netuid_stake as u64) - stake_general;
        assert!(
            (general_stake_change as i64 - expected_stake_change_general as i64).abs()
                <= change_tolerance
        );

        // Check the expected difference for the yuma subnet
        let yuma_stake_change = to_nano(yuma_netuid_stake as u64) - stake_yuma;
        assert!(
            (yuma_stake_change as i64 - expected_stake_change_yuma as i64).abs()
                <= change_tolerance
        );

        // Check the expected difference for the below threshold subnet
        let below_stake_change =
            to_nano(below_threshold_netuid_stake as u64) - stake_below_threshold;
        assert_eq!(below_stake_change, expected_stake_change_below);
    });
}

#[test]
fn test_yuma_self_vote() {
    new_test_ext().execute_with(|| {
        let netuid_general: u16 = 0;
        let netuid_yuma: u16 = 1;
        let netuid_below_threshold: u16 = 2;
        // this much stake is on the general subnet 0
        let stake_general: u64 = to_nano(500_000);
        // this is how much the first voter on yuma consensus has
        let stake_yuma_voter: u64 = to_nano(440_000);
        // miner
        let stake_yuma_miner: u64 = to_nano(10_000);
        // this is how much the self voter on yuma consensus has
        let stake_yuma_voter_self: u64 = to_nano(400_000);
        let stake_yuma_miner_self: u64 = to_nano(2_000);
        // below threshold subnet, emission distribution should not even start
        let stake_below_threshold: u64 = to_nano(50_000);
        let blocks_in_day: u16 = 10_800;
        let validator_key = U256::from(1);
        let miner_key = U256::from(2);
        let validator_self_key = U256::from(3);
        let miner_self_key = U256::from(4);

        // making sure the unit emission are set correctly
        UnitEmission::<Test>::put(23148148148);
        zero_min_burn();

        assert_ok!(register_module(
            netuid_general,
            U256::from(0),
            stake_general
        ));
        FounderShare::<Test>::set(netuid_general, 0);
        assert_ok!(register_module(
            netuid_yuma,
            validator_key,
            stake_yuma_voter
        ));
        update_params!(netuid_yuma => { max_weight_age: (blocks_in_day + 1) as u64});
        assert_ok!(register_module(netuid_yuma, miner_key, stake_yuma_miner));
        assert_ok!(register_module(
            netuid_yuma,
            validator_self_key,
            stake_yuma_voter_self
        ));
        assert_ok!(register_module(
            netuid_yuma,
            miner_self_key,
            stake_yuma_miner_self
        ));
        step_block(1);
        set_weights(
            netuid_yuma,
            validator_key,
            [SubspaceMod::get_uid_for_key(netuid_yuma, &miner_key)].to_vec(),
            [1].to_vec(),
        );
        set_weights(
            netuid_yuma,
            validator_self_key,
            [SubspaceMod::get_uid_for_key(netuid_yuma, &miner_self_key)].to_vec(),
            [1].to_vec(),
        );
        assert_ok!(register_module(
            netuid_below_threshold,
            U256::from(2),
            stake_below_threshold
        ));

        // Calculate the expected daily change in total stake
        let expected_stake_change = to_nano(250_000);

        FounderShare::<Test>::set(0, 0);
        FounderShare::<Test>::set(1, 0);
        FounderShare::<Test>::set(2, 0);

        step_block(blocks_in_day);

        let stake_validator = SubspaceMod::get_stake(netuid_yuma, &validator_key);
        let stake_miner = SubspaceMod::get_stake(netuid_yuma, &miner_key);
        let stake_validator_self_vote = SubspaceMod::get_stake(netuid_yuma, &validator_self_key);
        let stake_miner_self_vote = SubspaceMod::get_stake(netuid_yuma, &miner_self_key);

        assert!(stake_yuma_voter < stake_validator);
        assert!(stake_yuma_miner < stake_miner);
        assert_eq!(stake_yuma_miner_self, stake_miner_self_vote);
        assert_eq!(stake_yuma_voter_self, stake_validator_self_vote);

        let general_netuid_stake = SubspaceMod::get_total_subnet_stake(netuid_general);
        let yuma_netuid_stake = SubspaceMod::get_total_subnet_stake(netuid_yuma);
        let below_threshold_netuid_stake =
            SubspaceMod::get_total_subnet_stake(netuid_below_threshold);

        assert!(stake_general < general_netuid_stake);
        assert!(stake_yuma_voter < yuma_netuid_stake);
        assert_eq!(stake_below_threshold, below_threshold_netuid_stake);
        // Check the actual daily change in total stake
        let start_stake = stake_below_threshold
            + stake_general
            + stake_yuma_voter
            + stake_yuma_voter_self
            + stake_yuma_miner
            + stake_yuma_miner_self;
        let end_day_stake = general_netuid_stake + yuma_netuid_stake + below_threshold_netuid_stake;
        let actual_stake_change = round_first_five(end_day_stake - start_stake);

        assert_eq!(actual_stake_change, expected_stake_change);
    });
}

#[test]
fn test_emission_activation() {
    new_test_ext().execute_with(|| {
        // Define the subnet stakes
        let subnet_stakes = [
            ("Subnet A", to_nano(10), true),
            ("Subnet B", to_nano(4), false), // This one should not activate
            ("Subnet C", to_nano(86), true),
        ];

        // Set the stake threshold and minimum burn
        SubnetStakeThreshold::<Test>::put(Percent::from_percent(5));
        zero_min_burn();

        // Register the subnets
        for (i, (name, stake, _)) in subnet_stakes.iter().enumerate() {
            assert_ok!(register_module(i as u16, U256::from(i as u64), *stake));
            info!("Registered {name} with stake: {stake}");
        }

        step_block(1_000);

        // Check if subnet rewards have increased, but Subnet B should not have activated
        for (i, (name, initial_stake, should_activate)) in subnet_stakes.iter().enumerate() {
            let current_stake = SubspaceMod::get_total_subnet_stake(i as u16);
            if *should_activate {
                assert!(
                    current_stake > *initial_stake,
                    "{name} should have activated and increased its stake"
                );
            } else {
                assert_eq!(
                    current_stake, *initial_stake,
                    "{name} should not have activated"
                );
            }
            info!("{name} current stake: {current_stake}");
        }
    });
}

// immunity period attack
// this test should ignore, immunity period of subnet under specific conditions
#[test]
fn test_parasite_subnet_registrations() {
    new_test_ext().execute_with(|| {
        let expected_module_amount: u16 = 5;
        MaxAllowedModules::<Test>::put(expected_module_amount);
        MaxRegistrationsPerBlock::<Test>::set(1000);
        let main_subnet_netuid: u16 = 0;
        let main_subnet_stake = to_nano(500_000);
        let main_subnet_key = U256::from(0);

        let parasite_netuid: u16 = 1;
        let parasite_subnet_stake = to_nano(1_000);
        let parasite_subnet_key = U256::from(1);

        // Register the honest subnet.
        assert_ok!(register_module(
            main_subnet_netuid,
            main_subnet_key,
            main_subnet_stake
        ));
        // Set the immunity period of the honest subnet to 1000 blocks.
        update_params!(main_subnet_netuid => { immunity_period: 1000 });

        // Register the parasite subnet
        assert_ok!(register_module(
            parasite_netuid,
            parasite_subnet_key,
            parasite_subnet_stake
        ));
        // Parasite subnet set it's immunity period to 100k blocks.
        update_params!(parasite_netuid => { immunity_period: u16::MAX });

        // Honest subnet will now register another module, so it will have 2 in total.
        assert_ok!(register_module(
            main_subnet_netuid,
            U256::from(2),
            main_subnet_stake
        ));

        // Parasite subnet will now try to register a large number of modules.
        // This is in hope of deregistering modules from the honest subnet.
        for i in 10..50 {
            let result = register_module(parasite_netuid, U256::from(i), parasite_subnet_stake);
            assert_ok!(result);
        }

        // Check if the honest subnet has 2 modules.
        let main_subnet_module_amount = N::<Test>::get(main_subnet_netuid);
        assert_eq!(main_subnet_module_amount, 2);

        // Check if the parasite subnet has 3 modules
        let parasite_subnet_module_amount = N::<Test>::get(parasite_netuid);
        assert_eq!(parasite_subnet_module_amount, 3);
    });
}

// After reaching maximum global modules, subnets will start getting deregisterd
// Test ensures that newly registered subnets will take the "spots" of these deregistered subnets.
// And modules go beyond the global maximum.
#[test]
fn test_subnet_replacing() {
    new_test_ext().execute_with(|| {
        // Defines the maximum number of modules, that can be registered,
        // on all subnets at once.
        let expected_subnet_amount: u16 = 3;
        MaxAllowedModules::<Test>::put(expected_subnet_amount);

        let subnets = [
            (U256::from(0), to_nano(100_000)),
            (U256::from(1), to_nano(5000)),
            (U256::from(2), to_nano(4_000)),
            (U256::from(3), to_nano(1_100)),
        ];

        let random_keys = [U256::from(4), U256::from(5)];

        // Register all subnets
        for (i, (subnet_key, subnet_stake)) in subnets.iter().enumerate() {
            assert_ok!(register_module(i as u16, *subnet_key, *subnet_stake));
        }

        let subnet_amount = TotalSubnets::<Test>::get();
        assert_eq!(subnet_amount, expected_subnet_amount);

        // Register module on the subnet one (netuid 0), this means that subnet
        // subnet two (netuid 1) will be deregistered, as we reached global module limit.
        assert_ok!(register_module(0, random_keys[0], to_nano(1_000)));
        assert_ok!(register_module(4, random_keys[1], to_nano(150_000)));

        let subnet_amount = TotalSubnets::<Test>::get();
        assert_eq!(subnet_amount, expected_subnet_amount - 1);

        // netuid 1 replaced by subnet four
        assert_ok!(register_module(3, subnets[3].0, subnets[3].1));

        let subnet_amount = TotalSubnets::<Test>::get();
        let total_module_amount = SubspaceMod::global_n_modules();
        assert_eq!(subnet_amount, expected_subnet_amount);
        assert_eq!(total_module_amount, expected_subnet_amount);

        let netuids = SubspaceMod::netuids();
        let max_netuid = netuids.iter().max().unwrap();
        assert_eq!(*max_netuid, 2);
    });
}

#[test]
fn test_active_stake() {
    new_test_ext().execute_with(|| {
        SubnetStakeThreshold::<Test>::put(Percent::from_percent(5));
        let max_subnets = 10;
        MaxAllowedSubnets::<Test>::put(max_subnets);

        let general_subnet_stake = to_nano(65_000_000);
        let general_subnet_key = U256::from(0);
        assert_ok!(register_module(0, general_subnet_key, general_subnet_stake));
        step_block(1);
        // register 9 subnets reaching the subnet limit,
        // make sure they can't get emission
        let n: u16 = 9;
        let stake_per_subnet: u64 = to_nano(8_001);
        for i in 1..n + 1 {
            assert_ok!(register_module(
                i,
                U256::from(i),
                stake_per_subnet - (i as u64 * 1000)
            ));
            step_block(1)
        }

        for i in 0..max_subnets {
            assert_eq!(N::<Test>::get(i), 1);
        }

        step_block(200);

        // deregister subnet netuid 9, and get enough emission to produce yuma
        let new_subnet_stake = to_nano(9_900_000);
        assert_ok!(register_module(10, U256::from(10), new_subnet_stake));
        step_block(7);

        for i in 0..max_subnets {
            assert_eq!(N::<Test>::get(i), 1);
        }
        assert!(SubspaceMod::is_registered(9, &U256::from(10)));

        // register another module on the newly re-registered subnet 9,
        // and set weights on it from the key 11
        let miner_key = U256::from(11);
        let miner_stake = to_nano(100_000);
        assert_ok!(register_module(10, miner_key, miner_stake));

        step_block(1);

        assert_eq!(N::<Test>::get(9), 2);

        // set weights from key 11 to miner
        let uids = [1].to_vec();
        let weights = [1].to_vec();

        set_weights(9, U256::from(10), uids, weights);

        step_block(100);

        // register another massive module on the subnet
        let new_module_stake = to_nano(9_000_000);
        assert_ok!(register_module(10, U256::from(12), new_module_stake));

        step_block(1);
        assert!(SubspaceMod::is_registered(9, &U256::from(12)));
        // check if the module is registered
        assert_eq!(N::<Test>::get(9), 3);

        // set weights from key 12 to both modules
        let uids = [0, 1].to_vec();
        let weights = [1, 1].to_vec();

        set_weights(9, U256::from(12), uids, weights);

        let n = 10;
        let stake_per_n = to_nano(20_000_000);
        let start_key = 13;
        // register the n modules
        for i in 0..n {
            assert_ok!(register_module(10, U256::from(i + start_key), stake_per_n));
        }

        assert_eq!(N::<Test>::get(9), 3 + n);
        step_block(100);

        let uid_zero_dividends = SubspaceMod::get_dividends_for_uid(9, 0);
        let uid_two_dividends = SubspaceMod::get_dividends_for_uid(9, 2);
        let total_dividends_sum = Dividends::<Test>::get(9).iter().sum::<u16>();
        let active_dividends = uid_zero_dividends + uid_two_dividends;

        assert!(uid_zero_dividends > 0);
        assert!(uid_two_dividends > 0);
        assert!(uid_zero_dividends > uid_two_dividends);
        assert_eq!(total_dividends_sum, active_dividends);
    });
}

// this test, should confirm that we can update subnet using the same name
#[test]
fn test_update_same_name() {
    new_test_ext().execute_with(|| {
        // general subnet
        let netuid: u16 = 0;
        let key = U256::from(0);
        let stake = to_nano(100_000);

        assert_ok!(register_module(netuid, key, stake));

        assert_eq!(N::<Test>::get(0), 1);

        let mut params = SubspaceMod::subnet_params(netuid);
        let new_tempo = 30;
        params.tempo = new_tempo;
        let result = SubspaceMod::update_subnet(
            get_origin(key),
            netuid,
            params.founder,
            params.founder_share,
            params.immunity_period,
            params.incentive_ratio,
            params.max_allowed_uids,
            params.max_allowed_weights,
            params.min_allowed_weights,
            params.max_weight_age,
            params.min_stake,
            params.name,
            params.tempo,
            params.trust_ratio,
            params.maximum_set_weight_calls_per_epoch,
            params.governance_config.vote_mode,
            params.bonds_ma,
            params.target_registrations_interval,
            params.target_registrations_per_interval,
            params.max_registrations_per_interval,
            params.adjustment_alpha,
        );
        assert_ok!(result);

        let new_params = SubspaceMod::subnet_params(netuid);
        assert_eq!(new_params.tempo, new_tempo);
    });
}

#[test]
fn test_set_weight_rate_limiting() {
    new_test_ext().execute_with(|| {
        let netuid: u16 = 0;
        let key = U256::from(0);
        let stake = to_nano(100_000);

        assert_ok!(register_module(netuid, key, stake));
        assert_ok!(register_module(netuid, 1.into(), stake));

        Tempo::<Test>::set(netuid, 5);

        MaximumSetWeightCallsPerEpoch::<Test>::set(netuid, 1);

        let set_weights = || SubspaceMod::set_weights(get_origin(key), netuid, vec![1], vec![10]);

        assert_ok!(set_weights());
        assert_err!(
            set_weights(),
            Error::<Test>::MaximumSetWeightsPerEpochReached
        );

        step_block(5);

        assert_ok!(set_weights());
        assert_err!(
            set_weights(),
            Error::<Test>::MaximumSetWeightsPerEpochReached
        );

        MaximumSetWeightCallsPerEpoch::<Test>::set(netuid, 0);

        assert_ok!(set_weights());
        assert_ok!(set_weights());
        assert_ok!(set_weights());
        assert_ok!(set_weights());
    });
}

// ----------------
// Weights
// ----------------

// Test ensures that uids -- weights must have the same size.
#[test]
fn test_weights_err_weights_vec_not_equal_size() {
    new_test_ext().execute_with(|| {
        let netuid: u16 = 0;
        let key_account_id = U256::from(55);
        // make sure that the results won´t get affected by burn
        zero_min_burn();
        assert_ok!(register_module(netuid, key_account_id, 1_000_000_000));
        let weights_keys: Vec<u16> = vec![1, 2, 3, 4, 5, 6];
        let weight_values: Vec<u16> = vec![1, 2, 3, 4, 5]; // Uneven sizes
        let result = SubspaceMod::set_weights(
            RuntimeOrigin::signed(key_account_id),
            netuid,
            weights_keys,
            weight_values,
        );
        assert_eq!(result, Err(Error::<Test>::WeightVecNotEqualSize.into()));
    });
}

// Test ensures that uids can have not duplicates
#[test]
fn test_weights_err_has_duplicate_ids() {
    new_test_ext().execute_with(|| {
        let key_account_id = U256::from(666);
        let netuid: u16 = 0;
        // make sure that the results won´t get affected by burn
        zero_min_burn();
        MaxRegistrationsPerBlock::<Test>::set(100);

        assert_ok!(register_module(netuid, key_account_id, 10));
        update_params!(netuid => { max_allowed_uids: 100 });

        // uid 1
        assert_ok!(register_module(netuid, U256::from(1), 100));
        SubspaceMod::get_uid_for_key(netuid, &U256::from(1));

        // uid 2
        assert_ok!(register_module(netuid, U256::from(2), 10000));
        SubspaceMod::get_uid_for_key(netuid, &U256::from(2));

        // uid 3
        assert_ok!(register_module(netuid, U256::from(3), 10000000));
        SubspaceMod::get_uid_for_key(netuid, &U256::from(3));

        assert_eq!(N::<Test>::get(netuid), 4);

        let weights_keys: Vec<u16> = vec![1, 1, 1]; // Contains duplicates
        let weight_values: Vec<u16> = vec![1, 2, 3];
        let result = SubspaceMod::set_weights(
            RuntimeOrigin::signed(key_account_id),
            netuid,
            weights_keys,
            weight_values,
        );
        assert_eq!(result, Err(Error::<Test>::DuplicateUids.into()));
    });
}

// Tests the call requires a valid origin.
#[test]
fn test_no_signature() {
    new_test_ext().execute_with(|| {
        let uids: Vec<u16> = vec![];
        let values: Vec<u16> = vec![];
        let result = SubspaceMod::set_weights(RuntimeOrigin::none(), 1, uids, values);
        assert_eq!(result, Err(DispatchError::BadOrigin));
    });
}

// Tests that set weights fails if you pass invalid uids.
#[test]
fn test_set_weights_err_invalid_uid() {
    new_test_ext().execute_with(|| {
        let key_account_id = U256::from(55);
        let netuid: u16 = 0;
        // make sure that the results won´t get affected by burn
        zero_min_burn();
        assert_ok!(register_module(netuid, key_account_id, 1_000_000_000));
        let weight_keys: Vec<u16> = vec![9999]; // Does not exist
        let weight_values: Vec<u16> = vec![88]; // random value
        let result = SubspaceMod::set_weights(
            RuntimeOrigin::signed(key_account_id),
            netuid,
            weight_keys,
            weight_values,
        );
        assert_eq!(result, Err(Error::<Test>::InvalidUid.into()));
    });
}

// Tests that set weights fails if you dont pass enough values.
#[test]
fn test_set_weight_not_enough_values() {
    new_test_ext().execute_with(|| {
        let netuid: u16 = 0;
        let n = 100;
        MaxRegistrationsPerBlock::<Test>::set(n);
        let account_id = U256::from(0);
        // make sure that the results won´t get affected by burn
        zero_min_burn();
        assert_ok!(register_module(netuid, account_id, 1_000_000_000));

        for i in 1..n {
            assert_ok!(register_module(netuid, U256::from(i), 1_000_000_000));
        }

        update_params!(netuid => { min_allowed_weights: 2 });

        // setting weight below minimim
        let weight_keys: Vec<u16> = vec![1]; // not weight.
        let weight_values: Vec<u16> = vec![88]; // random value.
        let result = SubspaceMod::set_weights(
            RuntimeOrigin::signed(account_id),
            netuid,
            weight_keys,
            weight_values,
        );
        assert_eq!(result, Err(Error::<Test>::InvalidUidsLength.into()));

        update_params!(netuid => { min_allowed_weights: 1 });

        let weight_keys: Vec<u16> = vec![0]; // self weight.
        let weight_values: Vec<u16> = vec![88]; // random value.
        let result = SubspaceMod::set_weights(
            RuntimeOrigin::signed(account_id),
            netuid,
            weight_keys,
            weight_values,
        );
        assert_eq!(result, Err(Error::<Test>::NoSelfWeight.into()));

        // Should pass because we are setting enough values.
        let weight_keys: Vec<u16> = vec![1, 2]; // self weight.
        let weight_values: Vec<u16> = vec![10, 10]; // random value.
        update_params!(netuid => { min_allowed_weights: 1 });
        assert_ok!(SubspaceMod::set_weights(
            RuntimeOrigin::signed(account_id),
            netuid,
            weight_keys,
            weight_values
        ));
    });
}

// Tests that set weights fails if you dont pass enough values.
#[test]
fn test_set_max_allowed_uids() {
    new_test_ext().execute_with(|| {
        let netuid: u16 = 0;
        let n = 100;
        MaxRegistrationsPerBlock::<Test>::set(n);
        let account_id = U256::from(0);
        // make sure that the results won´t get affected by burn
        zero_min_burn();
        assert_ok!(register_module(netuid, account_id, 1_000_000_000));
        for i in 1..n {
            assert_ok!(register_module(netuid, U256::from(i), 1_000_000_000));
        }

        let max_allowed_uids: u16 = 10;

        update_params!(netuid => { max_allowed_weights: max_allowed_uids });

        // Should fail because we are only setting a single value and its not the self weight.
        let weight_keys: Vec<u16> = (1..max_allowed_uids + 1).collect(); // not weight.
        let weight_values: Vec<u16> = vec![1; max_allowed_uids as usize]; // random value.
        let result = SubspaceMod::set_weights(
            RuntimeOrigin::signed(account_id),
            netuid,
            weight_keys,
            weight_values,
        );
        assert_ok!(result);
    });
}

/// Check do nothing path
#[test]
fn test_normalize_weights_does_not_mutate_when_sum_is_zero() {
    new_test_ext().execute_with(|| {
        let max_allowed: u16 = 3;

        let weights: Vec<u16> = Vec::from_iter((0..max_allowed).map(|_| 0));

        let _expected = weights.clone();
        let _result = SubspaceMod::normalize_weights(weights);
    });
}

/// Check do something path
#[test]
fn test_normalize_weights_does_not_mutate_when_sum_not_zero() {
    new_test_ext().execute_with(|| {
        let max_allowed: u16 = 3;

        let weights: Vec<u16> = Vec::from_iter(0..max_allowed);

        let expected = weights.clone();
        let result = SubspaceMod::normalize_weights(weights);

        assert_eq!(expected.len(), result.len(), "Length of weights changed?!");
    });
}

#[test]
fn test_min_weight_stake() {
    new_test_ext().execute_with(|| {
        let mut global_params = SubspaceMod::global_params();
        global_params.min_weight_stake = to_nano(20);
        assert_ok!(SubspaceMod::set_global_params(global_params));
        MaxRegistrationsPerBlock::<Test>::set(1000);

        let netuid: u16 = 0;
        let module_count: u16 = 16;
        let voter_idx: u16 = 0;

        // registers the modules
        for i in 0..module_count {
            assert_ok!(register_module(netuid, U256::from(i), to_nano(10)));
        }

        let uids: Vec<u16> = (0..module_count).filter(|&uid| uid != voter_idx).collect();
        let weights = vec![1; uids.len()];

        assert_err!(
            SubspaceMod::set_weights(
                get_origin(U256::from(voter_idx)),
                netuid,
                uids.clone(),
                weights.clone(),
            ),
            Error::<Test>::NotEnoughStakePerWeight
        );

        increase_stake(U256::from(voter_idx), to_nano(400));

        assert_ok!(SubspaceMod::set_weights(
            get_origin(U256::from(voter_idx)),
            netuid,
            uids,
            weights,
        ));
    });
}

#[test]
fn test_weight_age() {
    new_test_ext().execute_with(|| {
        const NETUID: u16 = 0;
        const MODULE_COUNT: u16 = 16;
        const TEMPO: u64 = 100;
        const PASSIVE_VOTER: u16 = 0;
        const ACTIVE_VOTER: u16 = 1;
        MaxRegistrationsPerBlock::<Test>::set(1000);
        FloorFounderShare::<Test>::put(0);
        // Register modules
        (0..MODULE_COUNT).for_each(|i| {
            assert_ok!(register_module(NETUID, U256::from(i), to_nano(10)));
        });

        let uids: Vec<u16> = (0..MODULE_COUNT)
            .filter(|&uid| uid != PASSIVE_VOTER && uid != ACTIVE_VOTER)
            .collect();
        let weights = vec![1; uids.len()];

        // Set subnet parameters
        update_params!(NETUID => { tempo: TEMPO as u16, max_weight_age: TEMPO * 2 });

        // Set weights for passive and active voters
        assert_ok!(SubspaceMod::set_weights(
            get_origin(U256::from(PASSIVE_VOTER)),
            NETUID,
            uids.clone(),
            weights.clone(),
        ));
        assert_ok!(SubspaceMod::set_weights(
            get_origin(U256::from(ACTIVE_VOTER)),
            NETUID,
            uids.clone(),
            weights.clone(),
        ));

        let passive_stake_before = SubspaceMod::get_total_stake_to(&U256::from(PASSIVE_VOTER));
        let active_stake_before = SubspaceMod::get_total_stake_to(&U256::from(ACTIVE_VOTER));

        step_block((TEMPO as u16) * 2);

        let passive_stake_after = SubspaceMod::get_total_stake_to(&U256::from(PASSIVE_VOTER));
        let active_stake_after = SubspaceMod::get_total_stake_to(&U256::from(ACTIVE_VOTER));

        assert!(
            passive_stake_before < passive_stake_after || active_stake_before < active_stake_after,
            "Stake should be increasing"
        );

        // Set weights again for active voter
        assert_ok!(SubspaceMod::set_weights(
            get_origin(U256::from(ACTIVE_VOTER)),
            NETUID,
            uids,
            weights,
        ));

        step_block((TEMPO as u16) * 2);

        let passive_stake_after_v2 = SubspaceMod::get_total_stake_to(&U256::from(PASSIVE_VOTER));
        let active_stake_after_v2 = SubspaceMod::get_total_stake_to(&U256::from(ACTIVE_VOTER));

        assert_eq!(
            passive_stake_after, passive_stake_after_v2,
            "Stake values should remain the same after maximum weight age"
        );
        assert!(
            active_stake_after < active_stake_after_v2,
            "Stake should be increasing"
        );
    });
}
