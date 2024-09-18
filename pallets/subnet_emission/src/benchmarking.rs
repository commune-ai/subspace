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
    process_subnets {
        let caller: T::AccountId = account("Alice", 0, 1);
        // Add Alice's funds to submit the proposal
        SubspaceMod::<T>::add_balance_to_account(
            &caller,
            SubspaceMod::<T>::u64_to_balance(1_000_000_000_000_000).unwrap()
        );

        register_mock::<T>(caller.clone(), caller.clone(),
    "test".as_bytes().to_vec())?;

        reigster_mock()
    }: process_subnets()
}
