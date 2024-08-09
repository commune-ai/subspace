use std::collections::BTreeSet;

use crate::mock::*;
use frame_support::{assert_err, assert_noop, dispatch::DispatchResult};
use pallet_subnet_emission::SubnetConsensusType;
use pallet_subnet_emission_api::SubnetConsensus;
use pallet_subspace::*;
use sp_runtime::Percent;

#[test]
fn module_is_registered_correctly() {
    new_test_ext().execute_with(|| {
        MinimumAllowedStake::<Test>::set(0);

        let netuid = 0;
        let max_registrations_per_block = 10;
        let reg_this_block = 100;

        let network = format!("test{netuid}").as_bytes().to_vec();
        let name = b"module".to_vec();
        let address = "0.0.0.0:30333".as_bytes().to_vec();
        let network_string = String::from_utf8(network.clone()).expect("Invalid UTF-8");
        assert_ok!(register_named_subnet(0, 0, network_string));
        // Direct the rootnet netuid to something else than 0
        SubnetConsensusType::<Test>::insert(1, SubnetConsensus::Root);
        assert_noop!(
            SubspaceMod::do_register(get_origin(0), network, name, address, 0, None),
            Error::<Test>::NotEnoughBalanceToRegister
        );
        Burn::<Test>::insert(netuid, 0);
        MaxRegistrationsPerBlock::<Test>::set(max_registrations_per_block);
        step_block(1);
        assert_eq!(RegistrationsPerBlock::<Test>::get(), 0);

        let n = reg_this_block as u32;
        let keys_list: Vec<_> = (1..n).collect();

        for key in keys_list {
            let _ = register_module(netuid, key, 0, false);
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
            assert_ok!(register_module(netuid, i * 2, to_nano(100), false));
            assert_err!(
                register_module(netuid, u32::MAX, to_nano(100), false),
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
                    to_nano(100),
                    false
                ));
                step_block(1);

                ModuleBurnConfig::<Test>::mutate(netuid, |config| {
                    config.max_registrations_per_interval = interval;
                    config.target_registrations_interval = interval;
                });
            }

            assert_ok!(register_module(
                netuid,
                (i * interval + interval) as u32,
                to_nano(100),
                false
            ));

            assert_err!(
                register_module(netuid, u32::MAX, to_nano(100), false),
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
        MinimumAllowedStake::<Test>::set(0);

        let netuid: u16 = 0;
        let key = 1u32;

        add_balance(key, 2);
        register_module(netuid, key, 1, false)
            .unwrap_or_else(|_| panic!("register module failed for key {key:?}"));

        assert_eq!(N::<Test>::get(netuid), 1);

        let module_uid = SubspaceMod::get_uid_for_key(netuid, &key).unwrap();
        assert_eq!(SubspaceMod::get_uid_for_key(netuid, &key).unwrap(), 0);
        assert_eq!(get_stake_for_uid(netuid, module_uid), 1);
    });
}

#[test]
fn cannot_register_same_key_twice_in_a_subnet() {
    new_test_ext().execute_with(|| {
        zero_min_burn();
        MinimumAllowedStake::<Test>::set(0);

        let netuid = 0;
        let stake = 10;
        let key = 1u32;

        assert_ok!(register_module(netuid, key, stake, false));
        assert_err!(
            register_module(netuid, key, stake, false),
            Error::<Test>::KeyAlreadyRegistered
        );
    });
}

#[test]
fn registers_module_delegating_stake() {
    new_test_ext().execute_with(|| {
        zero_min_burn();

        let netuid = 0;
        let n = 10u32;
        let key = n + 1;
        let stake_amount = 10_000_000_000u64;

        SubspaceMod::add_balance_to_account(&key, stake_amount * n as u64);

        for module_key in 0..n {
            delegate_register_module(netuid, key, module_key, stake_amount)
                .expect("delegate register module failed");
            let stake_to_module = SubspaceMod::get_stake_to_module(&key, &module_key);
            assert_eq!(stake_to_module, stake_amount);
        }
    });
}

#[test]
fn deregister_within_subnet_when_limit_is_reached() {
    new_test_ext().execute_with(|| {
        zero_min_burn();

        MaxAllowedModules::<Test>::set(3);
        assert_ok!(register_module(0, 0, to_nano(10_000), false));
        assert_ok!(register_module(1, 1, to_nano(5_000), false));

        assert_eq!(SubspaceMod::get_delegated_stake(&0), to_nano(10_000));
        assert_eq!(SubspaceMod::get_delegated_stake(&1), to_nano(5_000));

        MaxAllowedUids::<Test>::set(0, 1);
        MaxAllowedUids::<Test>::set(1, 1);

        assert_ok!(register_module(0, 2, to_nano(15_000), false));

        assert_eq!(SubspaceMod::get_delegated_stake(&2), to_nano(15_000));
        assert_eq!(SubspaceMod::get_delegated_stake(&1), to_nano(5_000));

        assert_eq!(Emission::<Test>::get(0).len(), 1);
        assert_eq!(Emission::<Test>::get(1).len(), 1);
    });
}

#[test]
fn deregister_globally_when_global_limit_is_reached() {
    new_test_ext().execute_with(|| {
        zero_min_burn();
        SubnetImmunityPeriod::<Test>::set(0);

        let get_emission = |netuid, id| {
            let id = Uids::<Test>::get(netuid, id).unwrap_or(u16::MAX);
            Emission::<Test>::get(netuid).get(id as usize).copied().unwrap_or_default()
        };

        MaxAllowedModules::<Test>::set(2);

        assert_ok!(register_module(0, 0, to_nano(10), true));
        assert_ok!(register_module(1, 1, to_nano(5), true));

        assert_eq!(get_emission(0, 0), to_nano(10));
        assert_eq!(get_emission(1, 1), to_nano(5));

        MaxAllowedUids::<Test>::set(0, 2);
        MaxAllowedUids::<Test>::set(1, 1);

        assert_ok!(register_module(0, 2, to_nano(15), true));

        assert_eq!(get_emission(0, 0), to_nano(10));
        assert_eq!(get_emission(0, 2), to_nano(15));
        assert_eq!(get_emission(1, 1), 0);

        assert_eq!(Emission::<Test>::get(0).len(), 2);
        assert_eq!(Emission::<Test>::get(1).len(), 0);
    });
}

#[test]
fn deregister_subnet_with_dangling_keys() {
    new_test_ext().execute_with(|| {
        zero_min_burn();

        let key_a = 0;
        let stake_a = 100000000000;
        let key_b = 1;
        let stake_b = 100000000000;

        assert_ok!(register_module(0, key_a, stake_a, false));
        assert_ok!(register_module(1, key_a, stake_a, false));
        assert_ok!(register_module(1, key_b, stake_b, false));

        assert_eq!(StakeFrom::<Test>::get(key_a, key_a), stake_a * 2);
        assert_eq!(StakeFrom::<Test>::get(key_b, key_b), stake_b);

        let netuid = SubspaceMod::get_netuid_for_name("test1".as_bytes()).unwrap();
        SubspaceMod::remove_subnet(netuid);

        assert_eq!(StakeFrom::<Test>::get(key_b, key_b), 0);
        assert_eq!(SubspaceMod::get_balance(&key_b), stake_b + 1)
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
        let _ = SubspaceMod::register_subnet(origin.clone(), network.clone(), None);
        SubspaceMod::register(origin, network, name.to_vec(), addr.to_vec(), key, None)
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
            MinimumAllowedStake::<Test>::set(0);
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
            MinimumAllowedStake::<Test>::set(0);

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

mod subnet_validation {
    use super::*;

    #[test]
    fn subnet_registration_validates_subnet_names() {
        new_test_ext().execute_with(|| {
            zero_min_burn();

            let address = b"0.0.0.0".to_vec();
            let module_name = b"test".to_vec();

            let register_subnet = |key, name: Vec<u8>| {
                add_balance(key, 1);

                SubspaceMod::register_subnet(get_origin(key), name.clone(), None)?;
                SubspaceMod::register(
                    get_origin(key),
                    name,
                    module_name.clone(),
                    address.clone(),
                    key,
                    None,
                )
            };

            // Set min name length
            MinNameLength::<Test>::put(2);

            assert_err!(
                register_subnet(0, Vec::new()),
                Error::<Test>::InvalidSubnetName
            );

            // Try registering with a name that is too short (invalid)
            assert_err!(
                register_subnet(1, b"a".to_vec()),
                Error::<Test>::SubnetNameTooShort
            );

            // Try registering with a name that is exactly the minimum length (valid)
            let min_name_length = MinNameLength::<Test>::get();
            assert_ok!(register_subnet(2, vec![b'a'; min_name_length as usize]));

            // Try registering with a name that is exactly the maximum length (valid)
            let max_name_length = MaxNameLength::<Test>::get();
            assert_ok!(register_subnet(3, vec![b'a'; max_name_length as usize]));

            // Try registering with a name that is too long (invalid)
            assert_err!(
                register_subnet(4, vec![b'a'; (max_name_length + 1) as usize]),
                Error::<Test>::SubnetNameTooLong
            );

            // Try registering with an invalid UTF-8 name (invalid)
            assert_err!(
                register_subnet(5, vec![0xFF, 0xFF]),
                Error::<Test>::InvalidSubnetName
            );
        });
    }
}

#[test]
fn new_subnet_reutilized_removed_netuid_if_total_is_bigger_than_removed() {
    new_test_ext().execute_with(|| {
        zero_min_burn();

        // Increase the "subnet_count"
        for i in 0..10 {
            N::<Test>::set(i, 0);
        }
        dbg!(SubspaceMod::get_total_subnets());

        SubnetGaps::<Test>::set(BTreeSet::from([5]));

        SubspaceMod::add_balance_to_account(&0, SubnetBurn::<Test>::get() + 1 + to_nano(10));
        let _ = SubspaceMod::register_subnet(get_origin(0), b"test".to_vec(), None);
        SubspaceMod::register(
            get_origin(0),
            b"test".to_vec(),
            b"test".to_vec(),
            b"test".to_vec(),
            0,
            None,
        )
        .unwrap();

        let module_count = N::<Test>::get(5);
        assert_eq!(module_count, 1);
        assert_eq!(SubnetGaps::<Test>::get(), BTreeSet::from([]));
    });
}

#[test]
fn new_subnet_does_not_reuse_removed_netuid_if_total_is_smaller_than_removed() {
    new_test_ext().execute_with(|| {
        zero_min_burn();

        // Emulate total subnet count of 3
        for i in 0..3 {
            N::<Test>::set(i, 0);
        }

        SubnetGaps::<Test>::set(BTreeSet::from([7]));

        SubspaceMod::add_balance_to_account(&0, SubnetBurn::<Test>::get() + 1 + to_nano(10));
        let _ = SubspaceMod::register_subnet(get_origin(0), b"test".to_vec(), None);
        SubspaceMod::register(
            get_origin(0),
            b"test".to_vec(),
            b"test".to_vec(),
            b"test".to_vec(),
            0,
            None,
        )
        .unwrap();

        let module_count = N::<Test>::get(7);
        assert_eq!(module_count, 1);
        assert_eq!(SubnetGaps::<Test>::get(), BTreeSet::from([]));
    });
}

#[test]
fn new_subnets_on_removed_uids_register_modules_to_the_correct_netuids() {
    macro_rules! assert_subnets {
        ($v:expr) => {
            let v: Vec<_> = $v.iter().map(|(u, n)| (*u, n.as_bytes().to_vec())).collect();
            let names: Vec<_> = SubnetNames::<Test>::iter().filter(|(n, _)| *n > 0).collect();
            assert_eq!(names, v);
        };
    }

    fn register_module(netuid: u16, key: AccountId, stake: u64) {
        let origin = get_origin(key);
        SubnetImmunityPeriod::<Test>::set(0);

        let network = format!("test{netuid}").as_bytes().to_vec();
        let name = format!("module{key}").as_bytes().to_vec();
        let address = "0.0.0.0:30333".as_bytes().to_vec();

        SubspaceMod::add_balance_to_account(&key, stake + SubnetBurn::<Test>::get() + 1);
        let _ = SubspaceMod::register_subnet(origin.clone(), network.clone(), None);
        SubspaceMod::register(origin, network.clone(), name, address, key, None).unwrap();

        let netuid = SubspaceMod::get_netuid_for_name(&network).unwrap();
        let uid = pallet_subspace::Uids::<Test>::get(netuid, key).unwrap();

        Emission::<Test>::mutate(netuid, |v| v[uid as usize] = stake);
        pallet_subnet_emission::SubnetEmission::<Test>::mutate(netuid, |s| *s += stake);
    }

    new_test_ext().execute_with(|| {
        zero_min_burn();

        let add_emission = |netuid, id, emission| {
            let netuid =
                SubspaceMod::get_netuid_for_name(format!("test{netuid}").as_bytes()).unwrap();
            Emission::<Test>::mutate(netuid, |v| {
                v[Uids::<Test>::get(netuid, id).unwrap() as usize] = emission
            });
            pallet_subnet_emission::SubnetEmission::<Test>::mutate(netuid, |s| *s += emission);
        };

        MaxAllowedSubnets::<Test>::put(4);
        register_module(0, 0, to_nano(10_000));

        register_module(1, 0, to_nano(10));
        register_module(2, 1, to_nano(5));
        register_module(3, 2, to_nano(1));
        assert_subnets!(&[(1, "test1"), (2, "test2"), (3, "test3")]);

        register_module(4, 3, to_nano(15));
        assert_subnets!(&[(1, "test1"), (2, "test2"), (3, "test4")]);

        register_module(5, 4, to_nano(20));
        assert_subnets!(&[(1, "test1"), (2, "test5"), (3, "test4")]);

        add_balance(0, to_nano(50));
        add_emission(1, 0, to_nano(10));

        register_module(6, 5, to_nano(17));
        assert_subnets!(&[(1, "test1"), (2, "test5"), (3, "test6")]);

        assert_eq!(N::<Test>::get(1), 1);
        assert_eq!(N::<Test>::get(2), 1);
        assert_eq!(N::<Test>::get(3), 1);
    });
}

#[test]
fn test_subnet_immunity() {
    new_test_ext().execute_with(|| {
        zero_min_burn();
        MaxAllowedSubnets::<Test>::set(1);

        SubspaceMod::add_balance_to_account(&0, SubnetBurn::<Test>::get());
        let _ = SubspaceMod::register_subnet(get_origin(0), b"net1".to_vec(), None);
        assert_ok!(SubspaceMod::register(
            get_origin(0),
            b"net1".to_vec(),
            b"mod1".to_vec(),
            b"127.0.0.1".to_vec(),
            0,
            None,
        ));
        SubspaceMod::increase_stake(&0, &0, 100000000000);

        SubspaceMod::add_balance_to_account(&1, 100000000001 + SubnetBurn::<Test>::get());

        assert_err!(
            SubspaceMod::register_subnet(get_origin(1), b"net2".to_vec(), None),
            sp_runtime::DispatchError::Other("No valid netuid to deregister")
        );
    });
}
