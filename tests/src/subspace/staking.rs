use crate::mock::*;
use frame_support::{assert_err, assert_noop};
use pallet_subspace::*;

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
                assert_ok!(register_module(netuid, *key, amount_staked));

                assert_eq!(Stake::<Test>::get(key), amount_staked);
                assert_eq!(SubspaceMod::get_balance(key), 1);

                assert_ok!(SubspaceMod::remove_stake(
                    get_origin(*key),
                    *key,
                    amount_staked
                ));
                assert_eq!(SubspaceMod::get_balance(key), amount_staked + 1);
                assert_eq!(Stake::<Test>::get(key), 0);

                assert_ok!(SubspaceMod::add_stake(
                    get_origin(*key),
                    *key,
                    amount_staked,
                ));
                assert_eq!(Stake::<Test>::get(key), amount_staked);
                assert_eq!(SubspaceMod::get_balance(key), 1);

                subnet_stake += Stake::<Test>::get(key);
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

        assert_ok!(register_module(netuid, key_1, stake_amount));
        assert_ok!(register_module(netuid, key_2, 1));

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
fn fails_to_register_if_stake_is_below_minimum() {
    new_test_ext().execute_with(|| {
        zero_min_burn();

        let netuid = 0;
        let num_modules = 10;
        let min_stake = to_nano(10);

        assert_ok!(register_module(netuid, num_modules, min_stake));
        update_params!(netuid => { min_stake: min_stake + 100 });
        assert_err!(
            register_module(netuid, num_modules, min_stake),
            Error::<Test>::NotEnoughStakeToRegister
        );
    });
}

#[test]
fn fails_to_withdraw_zero_stake() {
    new_test_ext().execute_with(|| {
        zero_min_burn();
        MinimumAllowedStake::<Test>::set(0);

        let key = 0;

        assert_ok!(register_module(0, key, 1));
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
        assert_ok!(register_module(0, module_key, 1));

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
        register_n_modules(0, keys.len() as u16, 1);

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
