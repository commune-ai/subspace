#![cfg(feature = "runtime-benchmarks")]

use crate::*;
use frame_benchmarking::{account, benchmarks};
use frame_system::RawOrigin;
pub use pallet::*;
use pallet_subspace::{vec, MinValidatorStake, Pallet as SubspaceMod, SubnetBurn};
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

benchmarks! {
    set_weights {
        let module_key: T::AccountId = account("ModuleKey", 0, 2);
        let module_key2: T::AccountId = account("ModuleKey2", 0, 3);

        register_mock::<T>(module_key.clone(), module_key.clone(), "test".as_bytes().to_vec())?;
        register_mock::<T>(module_key2.clone(), module_key2.clone(), "test1".as_bytes().to_vec())?;
        let netuid = SubspaceMod::<T>::get_netuid_for_name("testnet".as_bytes()).unwrap();
        let uids = vec![0];
        let weights = vec![10];
        pallet_subspace::UseWeightsEncryption::<T>::set(netuid, false);
        MinValidatorStake::<T>::set(netuid, 0);
    }: set_weights(RawOrigin::Signed(module_key2), netuid, uids, weights)

    set_weights_encrypted {
        let module_key: T::AccountId = account("ModuleKey", 0, 2);
        let module_key2: T::AccountId = account("ModuleKey2", 0, 3);

        register_mock::<T>(module_key.clone(), module_key.clone(), "test".as_bytes().to_vec())?;
        register_mock::<T>(module_key2.clone(), module_key2.clone(), "test1".as_bytes().to_vec())?;
        let netuid = SubspaceMod::<T>::get_netuid_for_name("testnet".as_bytes()).unwrap();

        let weights = vec![10u8];
        let hash = vec![10u8];
        MinValidatorStake::<T>::set(netuid, 0);
    }: set_weights_encrypted(RawOrigin::Signed(module_key2), netuid, weights, hash)

    delegate_weight_control {
        let module_key: T::AccountId = account("ModuleKey", 0, 2);
        let module_key2: T::AccountId = account("ModuleKey2", 0, 3);

        register_mock::<T>(module_key.clone(), module_key.clone(), "test".as_bytes().to_vec())?;
        register_mock::<T>(module_key2.clone(), module_key2.clone(), "test1".as_bytes().to_vec())?;
        let netuid = SubspaceMod::<T>::get_netuid_for_name("testnet".as_bytes()).unwrap();
        
    }: delegate_weight_control(RawOrigin::Signed(module_key), netuid, module_key2.clone())

    remove_weight_control {
        // we first delegate
        let module_key: T::AccountId = account("ModuleKey", 0, 2);
        let module_key2: T::AccountId = account("ModuleKey2", 0, 3);
    
        register_mock::<T>(module_key.clone(), module_key.clone(), "test".as_bytes().to_vec())?;
        register_mock::<T>(module_key2.clone(), module_key2.clone(), "test1".as_bytes().to_vec())?;
    
        let netuid = SubspaceMod::<T>::get_netuid_for_name("testnet".as_bytes()).unwrap();
    
        let _ = Pallet::<T>::delegate_weight_control(RawOrigin::Signed(module_key.clone()).into(), netuid, module_key2.clone());
    
    }: remove_weight_control(RawOrigin::Signed(module_key), netuid)
}
