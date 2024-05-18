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
    stake: u64,
    name: Vec<u8>,
) -> Result<(), &'static str> {
    let address = "test".as_bytes().to_vec();
    let network = "testnet".as_bytes().to_vec();
    BurnConfig::<T>::mutate(|cfg| cfg.min_burn = 0);
    SubspaceMod::<T>::add_balance_to_account(
        &key,
        SubspaceMod::<T>::u64_to_balance(stake + 2000).unwrap(),
    );
    let metadata = Some("metadata".as_bytes().to_vec());
    SubspaceMod::<T>::register(
        RawOrigin::Signed(key).into(),
        network,
        name,
        address,
        stake.into(),
        module_key,
        metadata,
    )?;
    Ok(())
}

fn submit_dao_application<T: Config>() -> Result<(), &'static str> {
    // First add the application
    let caller: T::AccountId = account("Alice", 0, 1);
    let application_key: T::AccountId = account("Bob", 0, 2);
    SubspaceMod::<T>::add_balance_to_account(
        &caller,
        SubspaceMod::<T>::u64_to_balance(1_000_000_000_000_000).unwrap(),
    );
    let data = "test".as_bytes().to_vec();
    SubspaceMod::<T>::add_dao_application(RawOrigin::Signed(caller).into(), application_key, data)?;
    Ok(())
}

const REMOVE_WHEN_STAKING: u64 = 500;

benchmarks! {
    // ---------------------------------
    // Consensus operations
    // ---------------------------------

    // 0
    set_weights {
        let netuid = 0;
        let module_key: T::AccountId = account("ModuleKey", 0, 2);
        let module_key2: T::AccountId = account("ModuleKey2", 0, 3);
        let stake = 100000000000000u64;
        register_mock::<T>(module_key.clone(), module_key.clone(), stake, "test".as_bytes().to_vec())?;
        register_mock::<T>(module_key2.clone(), module_key2.clone(), stake, "test1".as_bytes().to_vec())?;
        let uids = vec![0];
        let weights = vec![10];
    }: set_weights(RawOrigin::Signed(module_key2), netuid, uids, weights)

    // ---------------------------------
    // Stake operations
    // ---------------------------------

    // 1
    add_stake {
        let key: T::AccountId = account("Alice", 0, 1);
        let netuid = 0;
        let module_key: T::AccountId = account("ModuleKey", 0, 2);
        let stake = 100000000000000u64;
        SubspaceMod::<T>::add_balance_to_account(
            &key,
            SubspaceMod::<T>::u64_to_balance(stake + 2000).unwrap(),
        );
        register_mock::<T>(module_key.clone(), module_key.clone(), stake.clone(), "test".as_bytes().to_vec())?;
    }: add_stake(RawOrigin::Signed(key), netuid, module_key, stake)

    // 2
    remove_stake {
        let caller: T::AccountId = account("Alice", 0, 1);
        let netuid = 0;
        let module_key: T::AccountId = account("ModuleKey", 0, 2);
        let stake = 100000000000000u64;
        register_mock::<T>(module_key.clone(), module_key.clone(), stake, "test".as_bytes().to_vec())?;
        let amount = 100000;
        SubspaceMod::<T>::add_balance_to_account(
            &caller,
            SubspaceMod::<T>::u64_to_balance(amount).unwrap(),
        );
        SubspaceMod::<T>::add_stake(RawOrigin::Signed(caller.clone()).into(), netuid, module_key.clone(), amount - REMOVE_WHEN_STAKING)?;
    }: remove_stake(RawOrigin::Signed(caller), netuid, module_key, amount - REMOVE_WHEN_STAKING)

    // ---------------------------------
    // Bulk stake operations
    // ---------------------------------

    // 3
    add_stake_multiple {
        let caller: T::AccountId = account("Alice", 0, 1);
        let netuid = 0;
        let module_key1: T::AccountId = account("ModuleKey1", 0, 2);
        let module_key2: T::AccountId = account("ModuleKey2", 0, 3);
        let stake = 100000000000000u64;
        register_mock::<T>(module_key1.clone(), module_key1.clone(), stake, "test".as_bytes().to_vec())?;
        register_mock::<T>(module_key2.clone(), module_key2.clone(), stake, "test1".as_bytes().to_vec())?;
        let module_keys = vec![module_key1, module_key2];
        let mut amounts = vec![10000, 20000];
        SubspaceMod::<T>::add_balance_to_account(
            &caller,
            SubspaceMod::<T>::u64_to_balance(amounts.iter().sum::<u64>()).unwrap(),
        );
        // remove REMOVE_WHEN_STAKING from all amounts
        amounts.iter_mut().for_each(|x| *x -= REMOVE_WHEN_STAKING);
    }: add_stake_multiple(RawOrigin::Signed(caller), netuid, module_keys, amounts)

    // 4
    remove_stake_multiple {
        let caller: T::AccountId = account("Alice", 0, 1);
        let netuid = 0;
        let module_key1: T::AccountId = account("ModuleKey1", 0, 2);
        let module_key2: T::AccountId = account("ModuleKey2", 0, 3);
        let stake = 100000000000000u64;
        register_mock::<T>(module_key1.clone(), module_key1.clone(), stake, "test".as_bytes().to_vec())?;
        register_mock::<T>(module_key2.clone(), module_key2.clone(), stake, "test1".as_bytes().to_vec())?;
        let module_keys = vec![module_key1.clone(), module_key2.clone()];
        let mut amounts = vec![1000, 2000];
        SubspaceMod::<T>::add_balance_to_account(
            &caller,
            SubspaceMod::<T>::u64_to_balance(amounts.iter().sum::<u64>()).unwrap(),
        );
        // remove REMOVE_WHEN_STAKING from all amounts
        amounts.iter_mut().for_each(|x| *x -= REMOVE_WHEN_STAKING);
        SubspaceMod::<T>::add_stake_multiple(RawOrigin::Signed(caller.clone()).into(), netuid, module_keys.clone(), amounts.clone())?;
    }: remove_stake_multiple(RawOrigin::Signed(caller), netuid, module_keys, amounts)

    // ---------------------------------
    // Transfers
    // ---------------------------------

    // 5
    transfer_stake {
        let caller: T::AccountId = account("Alice", 0, 1);
        let netuid = 0;
        let module_key: T::AccountId = account("ModuleKey", 0, 2);
        let new_module_key: T::AccountId = account("NewModuleKey", 0, 3);
        let stake = 100000000000000u64;
        register_mock::<T>(module_key.clone(), module_key.clone(), stake, "test".as_bytes().to_vec())?;
        register_mock::<T>(new_module_key.clone(), new_module_key.clone(), stake, "test1".as_bytes().to_vec())?;
        let amount = 10000;
        SubspaceMod::<T>::add_balance_to_account(
            &caller,
            SubspaceMod::<T>::u64_to_balance(amount).unwrap(),
        );
        SubspaceMod::<T>::add_stake(RawOrigin::Signed(caller.clone()).into(), netuid, module_key.clone(), amount - REMOVE_WHEN_STAKING)?;
    }: transfer_stake(RawOrigin::Signed(caller), netuid, module_key, new_module_key, amount - REMOVE_WHEN_STAKING)

    // 6
    transfer_multiple {
        let caller: T::AccountId = account("Alice", 0, 1);
        let dest1: T::AccountId = account("Dest1", 0, 2);
        let dest2: T::AccountId = account("Dest2", 0, 3);
        let destinations = vec![dest1.clone(), dest2.clone()];
        let mut amounts = vec![10000, 20000];
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
            SubspaceMod::<T>::u64_to_balance(stake + 2000).unwrap(),
        );
    }: register(RawOrigin::Signed(key.clone()), "test".as_bytes().to_vec(), "test".as_bytes().to_vec(), "test".as_bytes().to_vec(), stake.into(), module_key.clone(), Some("metadata".as_bytes().to_vec()))

    // 8
    deregister {
        let caller: T::AccountId = account("Alice", 0, 1);
        let netuid = 0;
        let stake = 100000000000000u64;
        register_mock::<T>(caller.clone(), caller.clone(), stake, "test".as_bytes().to_vec())?;
    }: deregister(RawOrigin::Signed(caller), netuid)

    // ---------------------------------
    // Updating
    // ---------------------------------

    // 9
    update_module {
        let caller: T::AccountId = account("Alice", 0, 1);
        let netuid = 0;
        let stake = 100000000000000u64;
        register_mock::<T>(caller.clone(), caller.clone(), stake, "test".as_bytes().to_vec())?;
        let name = "updated_name".as_bytes().to_vec();
        let address = "updated_address".as_bytes().to_vec();
        let delegation_fee = Some(Percent::from_percent(5));
        let metadata = Some("updated_metadata".as_bytes().to_vec());
    }: update_module(RawOrigin::Signed(caller), netuid, name, address, delegation_fee, metadata)


    // 10
    update_subnet {
        // Register a new subnet
        let caller: T::AccountId = account("Alice", 0, 1);
        let netuid = 0;
        let stake = 100000000000000u64;
        register_mock::<T>(caller.clone(), caller.clone(), stake, "test".as_bytes().to_vec())?;

        // Get the parameters of the subnet
        let params = SubspaceMod::<T>::subnet_params(netuid);
        let name = params.name;
        let founder = params.founder;
        let founder_share = params.founder_share;
        let immunity_period = params.immunity_period;
        let incentive_ratio = params.incentive_ratio;
        let max_allowed_uids = params.max_allowed_uids;
        let max_allowed_weights = params.max_allowed_weights;
        let min_allowed_weights = params.min_allowed_weights;
        let max_weight_age = params.max_weight_age;
        let min_stake = params.min_stake;
        let tempo = params.tempo;
        let trust_ratio = params.trust_ratio;
        let maximum_set_weight_calls_per_epoch = params.maximum_set_weight_calls_per_epoch;
        let vote_mode = params.vote_mode;
        let bonds_ma = params.bonds_ma;
    }: update_subnet(
        RawOrigin::Signed(caller),
        netuid,
        founder,
        founder_share,
        immunity_period,
        incentive_ratio,
        max_allowed_uids,
        max_allowed_weights,
        min_allowed_weights,
        max_weight_age,
        min_stake,
        name.clone(),
        tempo,
        trust_ratio,
        maximum_set_weight_calls_per_epoch,
        vote_mode,
        bonds_ma
    )

    // ---------------------------------
    // Subnet 0 DAO
    // ---------------------------------

    // 11
    add_dao_application {
        let caller: T::AccountId = account("Alice", 0, 1);
        let application_key: T::AccountId = account("Bob", 0, 2);
        SubspaceMod::<T>::add_balance_to_account(&caller, SubspaceMod::<T>::u64_to_balance(1_000_000_000_000_000).unwrap());
        let data = "test".as_bytes().to_vec();
    }: add_dao_application(RawOrigin::Signed(caller), application_key, data)

    // 12
    refuse_dao_application {
        // First add the application
        submit_dao_application::<T>()?;
        let caller: T::AccountId = account("Alice", 0, 1);
        Curator::<T>::set(caller.clone());
    }: refuse_dao_application(RawOrigin::Signed(caller), 0)

    // 13
    add_to_whitelist {
        // First add the application
        submit_dao_application::<T>()?;
        let caller: T::AccountId = account("Alice", 0, 1);
        let application_key: T::AccountId = account("Bob", 0, 2);
        Curator::<T>::set(caller.clone());
    }: add_to_whitelist(RawOrigin::Signed(caller), application_key, 1)

    // 14
    remove_from_whitelist {
        // First add the application
        submit_dao_application::<T>()?;
        let caller: T::AccountId = account("Alice", 0, 1);
        let application_key: T::AccountId = account("Bob", 0, 2);
        Curator::<T>::set(caller.clone());
        // Now add it to whitelist
        SubspaceMod::<T>::add_to_whitelist(RawOrigin::Signed(caller.clone()).into(), application_key.clone(), 1)?;
    }: remove_from_whitelist(RawOrigin::Signed(caller), application_key)

    // ---------------------------------
    // Adding proposals
    // ---------------------------------

    // 15
    add_global_proposal {
        let caller: T::AccountId = account("Alice", 0, 1);
        // Add alice funds to submit the proposal
        add_balance_to_account::<T>(&caller, 10_000)?;
    }

    // ---------------------------------
    // Voting / Unvoting proposals
    // ---------------------------------

    // ---------------------------------
    // Profit sharing
    // ---------------------------------

    // ---------------------------------
    // Testnet
    // ---------------------------------

    // 21
    // TODO: Add testnet benchmarks later
}
