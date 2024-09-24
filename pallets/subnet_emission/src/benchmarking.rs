#![cfg(feature = "runtime-benchmarks")]

use crate::{Pallet as SubnetEmissionMod, *};
use frame_benchmarking::{account, benchmarks};
use frame_system::RawOrigin;
pub use pallet::*;
use pallet_subspace::{Pallet as SubspaceMod, SubnetBurn};
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
    let metadata = Some("metadata".as_bytes().to_vec());
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
    }: set_weights(RawOrigin::Signed(module_key2), netuid, uids, weights)

    set_weights_encrypted {
        let module_key: T::AccountId = account("ModuleKey", 0, 2);
        let module_key2: T::AccountId = account("ModuleKey2", 0, 3);

        register_mock::<T>(module_key.clone(), module_key.clone(), "test".as_bytes().to_vec())?;
        register_mock::<T>(module_key2.clone(), module_key2.clone(), "test1".as_bytes().to_vec())?;
        let netuid = SubspaceMod::<T>::get_netuid_for_name("testnet".as_bytes()).unwrap();
        pallet_subspace::UseWeightsEncrytyption::<T>::set(netuid, true);

        let weights = vec![10u8];
        let hash = vec![10u8];
    }: set_weights_encrypted(RawOrigin::Signed(module_key2), netuid, weights, hash)

    delegate_rootnet_control {
        use pallet_subnet_emission_api::SubnetConsensus;
        let module_key: T::AccountId = account("ModuleKey", 0, 2);
        let module_key2: T::AccountId = account("ModuleKey2", 0, 3);

        register_mock::<T>(module_key.clone(), module_key.clone(), "test".as_bytes().to_vec())?;
        register_mock::<T>(module_key2.clone(), module_key2.clone(), "test1".as_bytes().to_vec())?;
        let netuid = SubspaceMod::<T>::get_netuid_for_name(b"testnet").unwrap();
        T::set_subnet_consensus_type(netuid, Some(SubnetConsensus::Root));
    }: delegate_rootnet_control(RawOrigin::Signed(module_key), module_key2)
}
