#![cfg(feature = "runtime-benchmarks")]

use crate::{Pallet as SubspaceMod, *};
use frame_benchmarking::{account, benchmarks};
use frame_system::RawOrigin;
pub use pallet::*;
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

const REMOVE_WHEN_STAKING: u64 = 500;

benchmarks! {
    // ---------------------------------
    // Consensus operations
    // ---------------------------------

    // 0
    set_weights {
        log::info!("Running set_weights benchmark");
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
        log::info!("Running add_stake benchmark");
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
        log::info!("Running remove_stake benchmark");
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
        log::info!("Running add_stake_multiple benchmark");
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
        log::info!("Running remove_stake_multiple benchmark");
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
        log::info!("Running transfer_stake benchmark");
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
        log::info!("Running transfer_multiple benchmark");
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
        log::info!("Running register benchmark");
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
        log::info!("Running deregister benchmark");
        let caller: T::AccountId = account("Alice", 0, 1);
        let netuid = 0;
        let stake = 100000000000000u64;
        register_mock::<T>(caller.clone(), caller.clone(), stake, "test".as_bytes().to_vec())?;
    }: deregister(RawOrigin::Signed(caller), netuid)

    // ---------------------------------
    // Updating
    // ---------------------------------

    // ---------------------------------
    // Subnet 0 DAO
    // ---------------------------------

    // ---------------------------------
    // Adding proposals
    // ---------------------------------

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
