use crate::mock::*;
use frame_support::assert_noop;
use pallet_subspace::*;
use substrate_fixed::types::I64F64;

#[test]
fn adds_stake_and_removes_to_module_and_calculates_total_stake() {
    new_test_ext().execute_with(|| {
        zero_min_burn();

        let max_uids = 2;
        let netuids = [0, 1];

        let amount_staked_vector: Vec<_> = netuids.iter().map(|_| to_nano(10)).collect();
        let mut total_stake = 0;

        for netuid in netuids {
            let amount_staked = amount_staked_vector[netuid as usize];
            let key_vector: Vec<_> =
                (0..max_uids).map(|i| (i + max_uids * netuid) as u32).collect();

            let mut subnet_stake = 0;

            for key in key_vector.iter() {
                assert_ok!(register_module(netuid, *key, amount_staked, false));

                assert_eq!(SubspaceMod::get_owned_stake(key), amount_staked);
                assert_eq!(SubspaceMod::get_balance(key), 1);

                assert_ok!(SubspaceMod::remove_stake(
                    get_origin(*key),
                    *key,
                    amount_staked
                ));
                assert_eq!(SubspaceMod::get_balance(key), amount_staked + 1);
                assert_eq!(SubspaceMod::get_owned_stake(key), 0);

                assert_ok!(SubspaceMod::add_stake(
                    get_origin(*key),
                    *key,
                    amount_staked,
                ));
                assert_eq!(SubspaceMod::get_owned_stake(key), amount_staked);
                assert_eq!(SubspaceMod::get_balance(key), 1);

                subnet_stake += SubspaceMod::get_owned_stake(key);
            }

            total_stake += subnet_stake;

            assert_eq!(SubspaceMod::get_total_subnet_stake(netuid), subnet_stake);
            assert_eq!(TotalStake::<Test>::get(), total_stake);
        }
    });
}

#[test]
fn transfers_stake_between_keys() {
    new_test_ext().execute_with(|| {
        zero_min_burn();
        MinimumAllowedStake::<Test>::set(0);

        let (key_1, key_2) = (0, 1);
        let stake_amount = to_nano(10);
        let netuid = 0;

        assert_ok!(register_module(netuid, key_1, stake_amount, false));
        assert_ok!(register_module(netuid, key_2, 1, false));

        assert_ok!(SubspaceMod::transfer_stake(
            get_origin(key_1),
            key_1,
            key_2,
            stake_amount,
        ));

        let key1_stake = SubspaceMod::get_total_stake_from(&key_1);
        let key2_stake = SubspaceMod::get_total_stake_from(&key_2);
        assert_eq!(key1_stake, 0);
        assert_eq!(key2_stake, stake_amount + 1);
    });
}

#[test]
fn fails_to_withdraw_zero_stake() {
    new_test_ext().execute_with(|| {
        zero_min_burn();
        MinimumAllowedStake::<Test>::set(0);

        let key = 0;

        assert_ok!(register_module(0, key, 1, false));
        assert_noop!(
            SubspaceMod::do_remove_stake(get_origin(1), key, 1),
            Error::<Test>::NotEnoughStakeToWithdraw
        );
    });
}

#[test]
fn adds_and_removes_stakes_for_a_delegated_module() {
    new_test_ext().execute_with(|| {
        zero_min_burn();
        MinimumAllowedStake::<Test>::set(0);

        let key = 2;
        add_balance(key, 6);

        let module_key = 0u32;
        assert_ok!(register_module(0, module_key, 1, false));

        assert_ok!(SubspaceMod::add_stake(get_origin(key), module_key, 5));
        assert_eq!(SubspaceMod::get_balance_u64(&key), 1);
        assert_eq!(
            *SubspaceMod::get_stake_from_vector(&module_key).get(&key).unwrap(),
            5
        );

        assert_ok!(SubspaceMod::remove_stake(get_origin(key), module_key, 5,));
        assert_eq!(SubspaceMod::get_balance_u64(&key), 6);
        assert!(!SubspaceMod::get_stake_from_vector(&module_key).contains_key(&key));
    });
}

#[test]
fn adds_and_removes_multiple_stakes_for_different_modules() {
    new_test_ext().execute_with(|| {
        zero_min_burn();
        MinimumAllowedStake::<Test>::set(0);

        let key = 2;
        add_balance(key, 11);

        let keys = [0u32, 1];
        register_n_modules(0, keys.len() as u16, 1, false);

        assert_ok!(SubspaceMod::add_stake_multiple(
            get_origin(key),
            keys.to_vec(),
            vec![5, 5],
        ));
        assert_eq!(SubspaceMod::get_balance_u64(&key), 1);
        assert_eq!(
            *SubspaceMod::get_stake_from_vector(&keys[0]).get(&key).unwrap(),
            5
        );
        assert_eq!(
            *SubspaceMod::get_stake_from_vector(&keys[1]).get(&key).unwrap(),
            5
        );

        assert_ok!(SubspaceMod::remove_stake_multiple(
            get_origin(key),
            keys.to_vec(),
            vec![5, 5],
        ));
        assert_eq!(SubspaceMod::get_balance_u64(&key), 11);
        assert!(!SubspaceMod::get_stake_from_vector(&keys[0]).contains_key(&key));
        assert!(!SubspaceMod::get_stake_from_vector(&keys[1]).contains_key(&key));
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

        register_n_modules(netuid, num_modules, 10, false);

        let keys = SubspaceMod::get_keys(netuid);

        for k in &keys {
            let delegate_keys: Vec<u32> =
                (0..num_modules).map(|i| (i + num_modules + 1) as u32).collect();
            for d in delegate_keys.iter() {
                add_balance(*d, stake_per_module + 1);
            }

            let pre_delegate_stake_from_vector = SubspaceMod::get_stake_from_vector(k);
            assert_eq!(pre_delegate_stake_from_vector.len(), 1); // +1 for the module itself, +1 for the delegate key on

            log::info!("KEY: {}", k);
            for (i, d) in delegate_keys.iter().enumerate() {
                log::info!("DELEGATE KEY: {d}");
                assert_ok!(SubspaceMod::add_stake(get_origin(*d), *k, stake_per_module,));
                let stake_from_vector = SubspaceMod::get_stake_from_vector(k);
                assert_eq!(
                    stake_from_vector.len(),
                    pre_delegate_stake_from_vector.len() + i + 1
                );
            }
            let ownership_ratios: Vec<(u32, I64F64)> = SubspaceMod::get_ownership_ratios(netuid, k);

            assert_eq!(ownership_ratios.len(), delegate_keys.len() + 1);
            log::info!("OWNERSHIP RATIOS: {ownership_ratios:?}");

            step_epoch(netuid);

            let stake_from_vector = SubspaceMod::get_stake_from_vector(k);
            let stake: u64 = SubspaceMod::get_delegated_stake(k);
            let sumed_stake: u64 = stake_from_vector.iter().fold(0, |acc, (_a, x)| acc + x);
            let total_stake: u64 = SubspaceMod::get_total_subnet_stake(netuid);

            log::info!("STAKE: {}", stake);
            log::info!("SUMED STAKE: {sumed_stake}");
            log::info!("TOTAL STAKE: {total_stake}");

            assert_eq!(stake, sumed_stake);

            // for (d_a, o) in ownership_ratios.iter() {
            //     info!("OWNERSHIP RATIO: {}", o);

            // }
        }
    });
}
