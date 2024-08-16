#![cfg(feature = "runtime-benchmarks")]

use crate::{Pallet as SubspaceMod, *};
use frame_benchmarking::{account, benchmarks};
use frame_system::RawOrigin;
pub use pallet::*;
use sp_arithmetic::per_things::Percent;
use sp_std::vec::Vec;

fn register_mock<T: Config>(
    key: T::AccountId,
    module_key: T::AccountId,
    name: Vec<u8>,
) -> Result<(), &'static str> {
    let address = "test".as_bytes().to_vec();
    let network = "testnet".as_bytes().to_vec();

    let enough_stake = 10000000000000u64;
    SubspaceMod::<T>::add_balance_to_account(
        &key,
        SubspaceMod::<T>::u64_to_balance(SubnetBurn::<T>::get() + enough_stake).unwrap(),
    );
    let network_metadata = Some("networkmetadata".as_bytes().to_vec());
    let metadata = Some("metadata".as_bytes().to_vec());
    let _ = SubspaceMod::<T>::register_subnet(
        RawOrigin::Signed(key.clone()).into(),
        network.clone(),
        network_metadata,
    );
    SubspaceMod::<T>::register(
        RawOrigin::Signed(key.clone()).into(),
        network,
        name,
        address,
        module_key.clone(),
        metadata,
    )?;
    SubspaceMod::<T>::increase_stake(&key, &module_key, enough_stake);
    Ok(())
}

const REMOVE_WHEN_STAKING: u64 = 500;

benchmarks! {
    // ---------------------------------
    // Consensus operations
    // ---------------------------------

    // 0
    set_weights {
        let module_key: T::AccountId = account("ModuleKey", 0, 2);
        let module_key2: T::AccountId = account("ModuleKey2", 0, 3);

        register_mock::<T>(module_key.clone(), module_key.clone(), "test".as_bytes().to_vec())?;
        register_mock::<T>(module_key2.clone(), module_key2.clone(), "test1".as_bytes().to_vec())?;
        let netuid = SubspaceMod::<T>::get_netuid_for_name("testnet".as_bytes()).unwrap();
        let uids = vec![0];
        let weights = vec![10];
    }: set_weights(RawOrigin::Signed(module_key2), netuid, uids, weights)

    // ---------------------------------
    // Stake operations
    // ---------------------------------

    // 1
    add_stake {
        let key: T::AccountId = account("Alice", 0, 1);
        let module_key: T::AccountId = account("ModuleKey", 0, 2);
        let stake = 100000000000000u64;
        SubspaceMod::<T>::add_balance_to_account(
            &key,
            SubspaceMod::<T>::u64_to_balance(stake + 2000).unwrap(),
        );
        register_mock::<T>(module_key.clone(), module_key.clone(), "test".as_bytes().to_vec())?;
    }: add_stake(RawOrigin::Signed(key), module_key, stake)

    // 2
    remove_stake {
        let caller: T::AccountId = account("Alice", 0, 1);
        let module_key: T::AccountId = account("ModuleKey", 0, 2);
        let stake = 100000000000000u64;
        register_mock::<T>(module_key.clone(), module_key.clone(), "test".as_bytes().to_vec())?;
        let amount = 1000000000000;
        SubspaceMod::<T>::add_balance_to_account(
            &caller,
            SubspaceMod::<T>::u64_to_balance(amount).unwrap(),
        );
        SubspaceMod::<T>::add_stake(RawOrigin::Signed(caller.clone()).into(), module_key.clone(), amount - REMOVE_WHEN_STAKING)?;
    }: remove_stake(RawOrigin::Signed(caller), module_key, amount - REMOVE_WHEN_STAKING)

    // ---------------------------------
    // Bulk stake operations
    // ---------------------------------

    // 3
    add_stake_multiple {
        let caller: T::AccountId = account("Alice", 0, 1);
        let module_key1: T::AccountId = account("ModuleKey1", 0, 2);
        let module_key2: T::AccountId = account("ModuleKey2", 0, 3);
        register_mock::<T>(module_key1.clone(), module_key1.clone(),"test".as_bytes().to_vec())?;
        register_mock::<T>(module_key2.clone(), module_key2.clone(), "test1".as_bytes().to_vec())?;
        let module_keys = vec![module_key1, module_key2];
        let mut amounts = vec![100000000000000, 100000000000000];
        SubspaceMod::<T>::add_balance_to_account(
            &caller,
            SubspaceMod::<T>::u64_to_balance(amounts.iter().sum::<u64>()).unwrap(),
        );
        // remove REMOVE_WHEN_STAKING from all amounts
        amounts.iter_mut().for_each(|x| *x -= REMOVE_WHEN_STAKING);
    }: add_stake_multiple(RawOrigin::Signed(caller), module_keys, amounts)

    // 4
    remove_stake_multiple {
        let caller: T::AccountId = account("Alice", 0, 1);
        let module_key1: T::AccountId = account("ModuleKey1", 0, 2);
        let module_key2: T::AccountId = account("ModuleKey2", 0, 3);
        register_mock::<T>(module_key1.clone(), module_key1.clone(), "test".as_bytes().to_vec())?;
        register_mock::<T>(module_key2.clone(), module_key2.clone(), "test1".as_bytes().to_vec())?;
        let module_keys = vec![module_key1.clone(), module_key2.clone()];
        let mut amounts = vec![100000000000000, 100000000000000];
        SubspaceMod::<T>::add_balance_to_account(
            &caller,
            SubspaceMod::<T>::u64_to_balance(amounts.iter().sum::<u64>()).unwrap(),
        );
        // remove REMOVE_WHEN_STAKING from all amounts
        amounts.iter_mut().for_each(|x| *x -= REMOVE_WHEN_STAKING);
        SubspaceMod::<T>::add_stake_multiple(RawOrigin::Signed(caller.clone()).into(), module_keys.clone(), amounts.clone())?;
    }: remove_stake_multiple(RawOrigin::Signed(caller), module_keys, amounts)

    // ---------------------------------
    // Transfers
    // ---------------------------------

    // 5
    transfer_stake {
        let caller: T::AccountId = account("Alice", 0, 1);
        let module_key: T::AccountId = account("ModuleKey", 0, 2);
        let new_module_key: T::AccountId = account("NewModuleKey", 0, 3);
        register_mock::<T>(module_key.clone(), module_key.clone(), "test".as_bytes().to_vec())?;
        register_mock::<T>(new_module_key.clone(), new_module_key.clone(), "test1".as_bytes().to_vec())?;
        let amount = 50000000000000;
        SubspaceMod::<T>::add_balance_to_account(
            &caller,
            SubspaceMod::<T>::u64_to_balance(amount).unwrap(),
        );
        SubspaceMod::<T>::add_stake(RawOrigin::Signed(caller.clone()).into(), module_key.clone(), amount - REMOVE_WHEN_STAKING)?;
    }: transfer_stake(RawOrigin::Signed(caller), module_key, new_module_key, amount - REMOVE_WHEN_STAKING)

    // 6
    transfer_multiple {
        let caller: T::AccountId = account("Alice", 0, 1);
        let dest1: T::AccountId = account("Dest1", 0, 2);
        let dest2: T::AccountId = account("Dest2", 0, 3);
        let destinations = vec![dest1.clone(), dest2.clone()];
        let mut amounts = vec![100000000000000, 100000000000000];
        SubspaceMod::<T>::add_balance_to_account(
            &caller,
            SubspaceMod::<T>::u64_to_balance(amounts.iter().sum()).unwrap(),
        );
        // Reduce by REMOVE_WHEN_STAKING
        amounts.iter_mut().for_each(|x| *x -= REMOVE_WHEN_STAKING);

    }: transfer_multiple(RawOrigin::Signed(caller), destinations, amounts)

    // ---------------------------------
    // Registereing / Deregistering
    // ---------------------------------

    // 7
    register {
        let key: T::AccountId = account("Alice", 0, 1);
        let module_key: T::AccountId = account("ModuleKey", 0, 2);
        let stake = 100000000000000u64;
        SubspaceMod::<T>::add_balance_to_account(
            &key,
            SubspaceMod::<T>::u64_to_balance(stake + SubnetBurn::<T>::get() + 2000).unwrap(),
        );
    }: register(RawOrigin::Signed(key.clone()), "test".as_bytes().to_vec(), "test".as_bytes().to_vec(), "test".as_bytes().to_vec(),  module_key.clone(), Some("metadata".as_bytes().to_vec()))

    // 8
    deregister {
        let caller: T::AccountId = account("Alice", 0, 1);
        register_mock::<T>(caller.clone(), caller.clone(), "test".as_bytes().to_vec())?;
        let netuid = SubspaceMod::<T>::get_netuid_for_name("testnet".as_bytes()).unwrap();
    }: deregister(RawOrigin::Signed(caller), netuid)

    // ---------------------------------
    // Updating
    // ---------------------------------

    // 9
    update_module {
        let caller: T::AccountId = account("Alice", 0, 1);
        let stake = 100000000000000u64;
        register_mock::<T>(caller.clone(), caller.clone(), "test".as_bytes().to_vec())?;
        let name = "updated_name".as_bytes().to_vec();
        let address = "updated_address".as_bytes().to_vec();
        let delegation_fee = Some(Percent::from_percent(5));
        let metadata = Some("updated_metadata".as_bytes().to_vec());
        let netuid = SubspaceMod::<T>::get_netuid_for_name("testnet".as_bytes()).unwrap();
    }: update_module(RawOrigin::Signed(caller), netuid, name, address, delegation_fee, metadata)


   // 10
   update_subnet {
        let caller: T::AccountId = account("Alice", 0, 1);
        register_mock::<T>(caller.clone(), caller.clone(), "test".as_bytes().to_vec())?;
        let netuid = SubspaceMod::<T>::get_netuid_for_name("testnet".as_bytes()).unwrap();
        let params = SubspaceMod::<T>::subnet_params(netuid);
    }: update_subnet(
        RawOrigin::Signed(caller),
        netuid,
        params.founder,
        params.founder_share,
        params.name.clone(),
        params.metadata.clone(),
        params.immunity_period,
        params.incentive_ratio,
        params.max_allowed_uids,
        params.max_allowed_weights,
        params.min_allowed_weights,
        params.max_weight_age,
        params.tempo,
        params.trust_ratio,
        params.maximum_set_weight_calls_per_epoch,
        params.governance_config.vote_mode,
        params.bonds_ma,
        params.module_burn_config,
        params.min_validator_stake,
        params.max_allowed_validators
    )
    // 11
    delegate_rootnet_control {
        use pallet_subnet_emission_api::SubnetConsensus;
        let module_key: T::AccountId = account("ModuleKey", 0, 2);
        let module_key2: T::AccountId = account("ModuleKey2", 0, 3);

        register_mock::<T>(module_key.clone(), module_key.clone(), "test".as_bytes().to_vec())?;
        register_mock::<T>(module_key2.clone(), module_key2.clone(), "test1".as_bytes().to_vec())?;
        let netuid = SubspaceMod::<T>::get_netuid_for_name(b"testnet").unwrap();
        T::set_subnet_consensus_type(netuid, Some(SubnetConsensus::Root));
    }: delegate_rootnet_control(RawOrigin::Signed(module_key), module_key2)

    // 12
    register_subnet {
        let key: T::AccountId = account("Alice", 0, 1);
        let stake = 100000000000000u64;
        SubspaceMod::<T>::add_balance_to_account(
            &key,
            SubspaceMod::<T>::u64_to_balance(stake + SubnetBurn::<T>::get() + 2000).unwrap(),
        );
    }: register_subnet(RawOrigin::Signed(key.clone()), "testnet".as_bytes().to_vec(), Some(b"testmetadata".to_vec()))

    add_blacklist {
        let owner: T::AccountId = account("Alice", 0, 1);
        let stake = 100000000000000u64;
        let blacklisted: T::AccountId = account("Module", 0, 2);
        register_mock::<T>(owner.clone(), owner.clone(), b"owner".to_vec()).unwrap();
        register_mock::<T>(owner.clone(), owner.clone(), b"blacklisted".to_vec()).unwrap();
        let netuid = SubspaceMod::<T>::get_netuid_for_name(b"testnet").unwrap();
    }: add_blacklist(RawOrigin::Signed(owner), netuid, blacklisted)

    remove_blacklist {
        let owner: T::AccountId = account("Alice", 0, 1);
        let stake = 100000000000000u64;
        let blacklisted: T::AccountId = account("Module", 0, 2);
        register_mock::<T>(owner.clone(), owner.clone(), b"owner".to_vec()).unwrap();
        register_mock::<T>(owner.clone(), owner.clone(), b"blacklisted".to_vec()).unwrap();
        let netuid = SubspaceMod::<T>::get_netuid_for_name(b"testnet").unwrap();
        SubspaceMod::<T>::add_blacklist(RawOrigin::Signed(owner.clone()).into(), netuid, blacklisted.clone()).unwrap();
    }: remove_blacklist(RawOrigin::Signed(owner), netuid, blacklisted)
}
