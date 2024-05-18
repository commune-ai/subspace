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

}
