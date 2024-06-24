use crate::mock::*;
use frame_support::{assert_err, assert_noop, assert_ok, dispatch::DispatchResult};
use pallet_subspace::*;
use sp_runtime::Percent;

#[test]
fn module_registration_respects_min_stake() {
    new_test_ext().execute_with(|| {
        zero_min_burn();

        let netuid = 0;
        let min_stake = 100_000_000;
        let max_registrations_per_block = 10;
        let reg_this_block = 100;

        let network = format!("test{netuid}").as_bytes().to_vec();
        let name = b"module".to_vec();
        let address = "0.0.0.0:30333".as_bytes().to_vec();

        assert_noop!(
            SubspaceMod::do_register(get_origin(0), network, name, address, 0, 0, None),
            Error::<Test>::NotEnoughBalanceToRegister
        );

        MinStake::<Test>::set(netuid, min_stake);
        MaxRegistrationsPerBlock::<Test>::set(max_registrations_per_block);
        step_block(1);
        assert_eq!(RegistrationsPerBlock::<Test>::get(), 0);

        let n = reg_this_block as u32;
        let keys_list: Vec<_> = (1..n as u32).collect();

        let min_stake_to_register = MinStake::<Test>::get(netuid);

        for key in keys_list {
            let _ = register_module(netuid, key, min_stake_to_register);
        }

        let registrations_this_block = RegistrationsPerBlock::<Test>::get();
        assert_eq!(registrations_this_block, max_registrations_per_block);

        step_block(1);
        assert_eq!(RegistrationsPerBlock::<Test>::get(), 0);
    });
}

#[test]
fn registration_fails_when_max_registrations_per_block_reached() {
    new_test_ext().execute_with(|| {
        zero_min_burn();

        let netuid = 0;

        MaxRegistrationsPerBlock::<Test>::set(1);

        for i in 0..2 {
            assert_ok!(register_module(netuid, i * 2, to_nano(100)));
            assert_err!(
                register_module(netuid, u32::MAX, to_nano(100)),
                Error::<Test>::TooManyRegistrationsPerBlock
            );
            assert_eq!(RegistrationsPerBlock::<Test>::get(), 1);
            step_block(1);
        }
    });
}

#[test]
fn registration_fails_when_max_registrations_per_interval_reached() {
    new_test_ext().execute_with(|| {
        zero_min_burn();

        let netuid = 0;
        let interval = 5;

        for i in 0..2 {
            for block in 0..interval - 1 {
                assert_ok!(register_module(
                    netuid,
                    (i * interval + block + 1) as u32,
                    to_nano(100)
                ));
                step_block(1);

                MaxRegistrationsPerInterval::<Test>::set(netuid, interval);
                TargetRegistrationsInterval::<Test>::set(netuid, interval);
            }

            assert_ok!(register_module(
                netuid,
                (i * interval + interval) as u32,
                to_nano(100)
            ));

            assert_err!(
                register_module(netuid, u32::MAX, to_nano(100)),
                Error::<Test>::TooManyRegistrationsPerInterval
            );

            assert_eq!(RegistrationsThisInterval::<Test>::get(netuid), 5);
            step_block(1);
        }
    });
}

#[test]
fn registers_a_module_and_storage_values_correctly() {
    new_test_ext().execute_with(|| {
        zero_min_burn();

        let netuid: u16 = 0;
        let key = 1u32;

        add_balance(key, 2);
        register_module(netuid, key, 1)
            .unwrap_or_else(|_| panic!("register module failed for key {key:?}"));

        assert_eq!(N::<Test>::get(netuid), 1);

        let module_uid = SubspaceMod::get_uid_for_key(netuid, &key);
        assert_eq!(SubspaceMod::get_uid_for_key(netuid, &key), 0);

        assert_eq!(get_stake_for_uid(netuid, module_uid), 1);
    });
}

#[test]
fn cannot_register_same_key_twice_in_a_subnet() {
    new_test_ext().execute_with(|| {
        zero_min_burn();

        let netuid = 0;
        let stake = 10;
        let key = 1u32;

        assert_ok!(register_module(netuid, key, stake));
        assert_err!(
            register_module(netuid, key, stake),
            Error::<Test>::KeyAlreadyRegistered
        );
    });
}

mod module_validation {
    use super::*;

    fn register_custom(netuid: u16, key: u32, name: &[u8], addr: &[u8]) -> DispatchResult {
        let network: Vec<u8> = format!("test{netuid}").as_bytes().to_vec();

        let origin = get_origin(key);
        let is_new_subnet: bool = !SubspaceMod::if_subnet_exist(netuid);
        if is_new_subnet {
            MaxRegistrationsPerBlock::<Test>::set(1000)
        }

        // make sure there is some balance
        add_balance(key, 2);
        SubspaceMod::register(origin, network, name.to_vec(), addr.to_vec(), 1, key, None)
    }

    fn test_validation_cases(f: impl Fn(&[u8], &[u8]) -> DispatchResult) {
        assert_err!(f(b"", b""), Error::<Test>::InvalidModuleName);
        assert_err!(f(b"o", b""), Error::<Test>::ModuleNameTooShort);
        assert_err!(
            f("o".repeat(100).as_bytes(), b""),
            Error::<Test>::ModuleNameTooLong
        );
        assert_err!(f(b"\xc3\x28", b""), Error::<Test>::InvalidModuleName);

        assert_err!(f(b"test", b""), Error::<Test>::InvalidModuleAddress);
        assert_err!(
            f(b"test", "o".repeat(100).as_bytes()),
            Error::<Test>::ModuleAddressTooLong
        );
        assert_err!(f(b"test", b"\xc3\x28"), Error::<Test>::InvalidModuleAddress);

        assert_ok!(f(b"test", b"abc"));
    }

    #[test]
    fn validates_module_on_registration() {
        new_test_ext().execute_with(|| {
            // make sure that the results won´t get affected by burn
            zero_min_burn();
            test_validation_cases(|name, addr| register_custom(0, 0, name, addr));

            assert_err!(
                register_custom(0, 1, b"test", b"0.0.0.0:1"),
                Error::<Test>::ModuleNameAlreadyExists
            );
        });
    }

    #[test]
    fn validates_module_on_update() {
        new_test_ext().execute_with(|| {
            let subnet = 0;
            let key_0 = 0;
            let origin_0 = get_origin(0);
            // make sure that the results won´t get affected by burn
            zero_min_burn();

            assert_ok!(register_custom(subnet, key_0, b"test", b"0.0.0.0:1"));

            test_validation_cases(|name, addr| {
                SubspaceMod::update_module(
                    origin_0.clone(),
                    subnet,
                    name.to_vec(),
                    addr.to_vec(),
                    None,
                    None,
                )
            });

            let key_1 = 1;
            let origin_1 = get_origin(key_1);
            assert_ok!(register_custom(0, key_1, b"test2", b"0.0.0.0:2"));

            let update_module = |name: &[u8], addr: &[u8]| {
                SubspaceMod::update_module(
                    origin_1.clone(),
                    subnet,
                    name.to_vec(),
                    addr.to_vec(),
                    Some(Percent::from_percent(5)),
                    None,
                )
            };

            assert_err!(
                update_module(b"test", b""),
                Error::<Test>::ModuleNameAlreadyExists
            );
            assert_ok!(update_module(b"test2", b"0.0.0.0:2"));
            assert_ok!(update_module(b"test3", b"0.0.0.0:3"));

            let params = SubspaceMod::module_params(0, &key_1);
            assert_eq!(params.name, b"test3");
            assert_eq!(params.address, b"0.0.0.0:3");

            FloorDelegationFee::<Test>::put(Percent::from_percent(10));
            assert_err!(
                update_module(b"test3", b"0.0.0.0:3"),
                Error::<Test>::InvalidMinDelegationFee
            );
        });
    }
}
