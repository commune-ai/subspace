use frame_support::assert_err;
use pallet_subnet_emission::{
    subnet_pricing::root::RootPricing, PendingEmission, SubnetConsensusType, SubnetEmission,
    UnitEmission,
};
use pallet_subnet_emission_api::{SubnetConsensus, SubnetEmissionApi};
use pallet_subspace::{
    Error, Kappa, Keys, MaxAllowedUids, MaxAllowedValidators, MaxRegistrationsPerBlock,
    MinimumAllowedStake, ModuleBurnConfig, Rho, StakeFrom, Tempo,
};

pub use crate::mock::*;

const ROOT_NETUID: u16 = 0;

#[test]
fn test_root_pricing() {
    new_test_ext().execute_with(|| {
        zero_min_validator_stake();
        zero_min_burn();

        MaxRegistrationsPerBlock::<Test>::set(6);

        ModuleBurnConfig::<Test>::mutate(0, |config| config.max_registrations_per_interval = 3);

        assert_ok!(register_named_subnet(u32::MAX, 0, "Rootnet"));
        Test::set_subnet_consensus_type(0, Some(SubnetConsensus::Root));

        let net1_id = 1;
        let net2_id = 2;
        let net3_id = 3;

        let val1_id = 101;
        let val2_id = 102;
        let val3_id = 103;

        let val1_stake = to_nano(20_000);
        let val2_stake = to_nano(40_000);
        let val3_stake = to_nano(40_000);

        assert_ok!(register_module(net1_id, val1_id, val1_stake, false));
        assert_ok!(register_module(net2_id, val2_id, val2_stake, false));
        assert_ok!(register_module(net3_id, val3_id, val3_stake, false));

        let _ = assert_ok!(register_root_validator(val1_id, val1_stake));
        let _ = assert_ok!(register_root_validator(val2_id, val2_stake));
        let _ = assert_ok!(register_root_validator(val3_id, val3_stake));

        set_weights(
            0,
            val1_id,
            vec![1, 2, 3],
            vec![u16::MAX, u16::MIN, u16::MIN],
        );
        set_weights(
            0,
            val2_id,
            vec![1, 2, 3],
            vec![u16::MIN, 655 /* ~1% */, 64879u16 /* ~99% */],
        );
        set_weights(
            0,
            val3_id,
            vec![1, 2, 3],
            vec![u16::MIN, u16::MAX, u16::MIN],
        );

        let distributed = to_nano(1_000);
        let priced_subnets = assert_ok!(RootPricing::<Test>::new(0, to_nano(1_000)).run());

        let net1_emission = *priced_subnets.get(&net1_id).unwrap();
        let net2_emission = *priced_subnets.get(&net2_id).unwrap();
        let net3_emission = *priced_subnets.get(&net3_id).unwrap();

        let net1_perc = net1_emission as f32 / distributed as f32;
        let net2_perc = net2_emission as f32 / distributed as f32;
        let net3_perc = net3_emission as f32 / distributed as f32;

        assert_in_range!(net1_perc, 0.04f32, 0.03f32);
        assert_in_range!(net2_perc, 0.78f32, 0.03f32);
        assert_in_range!(net3_perc, 0.18f32, 0.04f32);
    });
}

#[test]
fn test_emission() {
    new_test_ext_with_block(1).execute_with(|| {
        zero_min_validator_stake();
        zero_min_burn();
        MinimumAllowedStake::<Test>::set(0);

        assert_ok!(register_named_subnet(u32::MAX, 0, "Rootnet"));
        Test::set_subnet_consensus_type(0, Some(SubnetConsensus::Root));

        let n = 10;
        MaxRegistrationsPerBlock::<Test>::set(n * 2);
        ModuleBurnConfig::<Test>::mutate(ROOT_NETUID, |config| {
            config.target_registrations_per_interval = n
        });
        MaxAllowedUids::<Test>::set(ROOT_NETUID, n);
        UnitEmission::<Test>::set(1000000000);
        Rho::<Test>::set(30);
        Kappa::<Test>::set(32767);

        for i in 0..n {
            let key_id: u32 = i as u32;
            let key_origin = get_origin(key_id);

            SubspaceMod::add_balance_to_account(&key_id, 1_000_000_000_000_000);
            assert_ok!(SubspaceMod::register(
                key_origin,
                b"Rootnet".to_vec(),
                format!("test{}", i).as_bytes().to_vec(),
                b"0.0.0.0:30333".to_vec(),
                key_id,
                None,
            ));
            SubspaceMod::increase_stake(&key_id, &key_id, 1000);
        }

        for i in 1..n {
            let key_id: u32 = i as u32 + 100;
            let key_origin = get_origin(key_id);
            SubspaceMod::add_balance_to_account(&key_id, 1_000_000_000_000_000);
            let _ = SubspaceMod::register_subnet(
                key_origin.clone(),
                format!("net{}", i).as_bytes().to_vec(),
                None,
            );
            assert_ok!(SubspaceMod::register(
                key_origin,
                format!("net{}", i).as_bytes().to_vec(),
                format!("test{}", i).as_bytes().to_vec(),
                b"0.0.0.0:30333".to_vec(),
                key_id,
                None,
            ));
            SubspaceMod::increase_stake(&key_id, &key_id, 1000);
        }

        for i in 0..n {
            let key_id: u32 = i as u32;
            let key_origin = get_origin(key_id);
            let uids: Vec<u16> = vec![i];
            let values: Vec<u16> = vec![1];
            assert_ok!(SubspaceMod::set_weights(
                key_origin,
                ROOT_NETUID,
                uids,
                values
            ));
        }

        Tempo::<Test>::set(0, 1);

        let _ = SubnetEmissionMod::get_subnet_pricing(1_000_000_000);
        for netuid in 1..n {
            let emission = SubnetEmission::<Test>::get(netuid);
            println!(
                "expected emission for {}: 99_999_999, got {}",
                netuid, &emission
            );

            assert_eq!(emission, 99_999_999);
        }
        step_block(2);
        println!("stepped 2 blocks");

        for netuid in 1..n {
            let pending_emission = PendingEmission::<Test>::get(netuid);
            println!(
                "expected pending emission for {}: 199_999_998, got {}",
                netuid, &pending_emission
            );
            assert_eq!(pending_emission, 199_999_998);
        }

        step_block(1);
        println!("stepped 1 block");
        for netuid in 1..n {
            let pending_emission = PendingEmission::<Test>::get(netuid);
            println!(
                "expected pending emission for {}: 299_999_997, got {}",
                netuid, &pending_emission
            );
            assert_eq!(pending_emission, 299_999_997);
        }

        let step =
            SubspaceMod::blocks_until_next_epoch(10, SubspaceMod::get_current_block_number());
        step_block(step as u16);
        assert_eq!(PendingEmission::<Test>::get(10), 0);
    });
}

/// Test manipulative minority validators can not control rootnet emission
/// distribution.
#[test]
fn test_sigmoid_distribution() {
    new_test_ext().execute_with(|| {
        zero_min_burn();
        let key_zero = 0;
        let key_one = 1;
        let stake_zero = to_nano(26_000_000);
        let stake_one = to_nano(60_000_000);

        // Set kappa to 37k
        Kappa::<Test>::put(37_000);
        // Set rho to 12
        Rho::<Test>::put(12);
        assert_ok!(register_root_validator(key_one, stake_one));
        assert_ok!(register_root_validator(key_zero, stake_zero));
        SubnetConsensusType::<Test>::insert(0, SubnetConsensus::Root);

        // Register subnet 1 and 2
        let subnet_one_key = 2;
        assert_ok!(register_named_subnet(subnet_one_key, 1, "s1"));
        let subnet_two_key = 3;
        assert_ok!(register_named_subnet(subnet_two_key, 2, "s2"));
        // Set the weights on the subnets
        set_weights(0, key_zero, vec![1, 2], vec![1, 0]);
        set_weights(0, key_one, vec![1, 2], vec![0, 1]);

        step_block(1);

        let subnet_one_emission = SubnetEmission::<Test>::get(1);
        let subnet_two_emission = SubnetEmission::<Test>::get(2);

        let total_emission = subnet_one_emission + subnet_two_emission;
        let subnet_one_perc = subnet_one_emission as f32 / total_emission as f32;
        // s1 gets < 6% emissions
        assert!(
            subnet_one_perc < 0.06,
            "Subnet 1 emission percentage should be less than 6%, but was {:.2}%",
            subnet_one_perc * 100.0
        );
    });
}

/// Test rootnet registration requirements:
/// 1. We need to have more stake than least staked key registered there.
/// 2. Try to register without having more stake than the lest staked key, and expect an error.
/// 3. Check deregistered key has no stake and is not present in the registered keys.
#[test]
fn test_rootnet_registration_requirements() {
    new_test_ext().execute_with(|| {
        zero_min_burn();

        // Set up initial conditions
        MaxRegistrationsPerBlock::<Test>::set(100);
        ModuleBurnConfig::<Test>::mutate(ROOT_NETUID, |config| {
            config.max_registrations_per_interval = 5
        });

        assert_ok!(register_named_subnet(u32::MAX, ROOT_NETUID, "Rootnet"));
        // Rootnet configuration
        Test::set_subnet_consensus_type(ROOT_NETUID, Some(SubnetConsensus::Root));
        MaxAllowedValidators::<Test>::set(ROOT_NETUID, Some(5));
        MaxAllowedUids::<Test>::set(ROOT_NETUID, 5);

        // Register initial validators
        let initial_stake = to_nano(10_000);
        let last_stake = 5;
        for i in 1..=last_stake {
            assert_ok!(register_root_validator(i, initial_stake + i as u64));
        }

        let lowest_stake_key_exists = Keys::<Test>::iter().any(|(_, _, account_id)| {
            account_id == <u32 as Into<<Test as frame_system::Config>::AccountId>>::into(1)
        });
        assert!(lowest_stake_key_exists);

        // Try to register with less stake than the least staked key
        let less_stake = to_nano(9_999);
        assert_err!(
            register_root_validator(6, less_stake),
            Error::<Test>::NotEnoughStakeToRegister
        );

        // Register with more stake than the least staked key
        let more_stake = initial_stake + to_nano(last_stake as u64);

        // Manually increase the stake from on the non-registered key
        StakeFrom::<Test>::insert(6, 6, more_stake);
        assert_ok!(register_root_validator(6, 0));

        // Make sure the first key to register has no stake, as it should be deregistered
        assert_eq!(SubspaceMod::get_total_stake_from(&1), 0);
        // Now make sure it is deregistered
        let key_exists = Keys::<Test>::iter().any(|(_, _, account_id)| {
            account_id == <u32 as Into<<Test as frame_system::Config>::AccountId>>::into(1)
        });
        assert!(!key_exists);
        // Make sure the second key has the inital stake + 2
        assert_eq!(SubspaceMod::get_total_stake_from(&2), initial_stake + 2);
    });
}
