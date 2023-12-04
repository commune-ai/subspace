
#![cfg(feature = "runtime-benchmarks")]

use super::*;

use super::*;

use crate::Pallet;
use frame_benchmarking::v2::*;
use frame_system::RawOrigin;

const SEED: u32 = 1;
const BALANCE: u64 = 1_000_000_000_000_000_000;
const MIN_STAKE: u64 = 1_00_000_000_000;

fn set_user_balance<T: Config>(user: &T::AccountId) {
    T::Currency::deposit_creating(user, <Pallet<T>>::u64_to_balance(BALANCE).unwrap());
}

fn default_register_helper<T: Config>() -> (
    Vec<u8>,
    Vec<u8>,
    Vec<u8>,
    T::AccountId,
    u16
) {
    let network: Vec<u8> = b"network".to_vec();
    let name: Vec<u8> = b"name".to_vec();
    let address: Vec<u8> = b"address".to_vec();
    let module_key: T::AccountId = account("key", 0, SEED);

    let netuid = register_helper::<T>(
        network.clone(),
        name.clone(),
        address.clone(),
        module_key.clone()
    );

    (network, name, address, module_key, netuid)
}

fn register_helper<T: Config>(
    network: Vec<u8>,
    name: Vec<u8>,
    address: Vec<u8>,
    module_key: T::AccountId,
) -> u16 {
    set_user_balance::<T>(&module_key);

    <Pallet<T>>::register(
        RawOrigin::Signed(module_key.clone()).into(),
        network.clone(),
        name.clone(),
        address.clone(),
        MIN_STAKE,
        module_key.clone(),
    );

    let netuid = <Pallet<T>>::get_netuid_for_name(network.clone());
    
    netuid
}

fn add_stake_helper<T: Config>(
    network: Vec<u8>,
    name: Vec<u8>,
    address: Vec<u8>,
    module_key: T::AccountId,
    amount: u64
) -> u16 {
    let netuid = register_helper::<T>(
        network,
        name.clone(),
        address.clone(),
        module_key.clone()
    );

    <Pallet<T>>::add_stake(
    RawOrigin::Signed(module_key.clone()).into(),
        netuid,
        module_key,
        amount,
    );

    netuid
}

fn add_stake_multiple_helper<T: Config>(
    caller: T::AccountId,
) -> (
    u16,
    Vec<T::AccountId>,
    Vec<u64>
) {
    let network: Vec<u8> = b"network".to_vec();
    let address: Vec<u8> = b"address".to_vec();

    let module_keys: Vec<T::AccountId> = (0..10).map(|i| account("key", i, SEED)).collect();
    let amounts: Vec<u64> = (0..10).map(|i| i + MIN_STAKE).collect();
    
    let mut netuid: u16 = 0;
    
    for (index, module_key) in module_keys.iter().enumerate() {
        let mut name: Vec<u8> = b"name".to_vec();
        name.extend(vec![index as u8]);

        netuid = register_helper::<T>(
            network.clone(),
            name,
            address.clone(),
            module_key.clone()
        );
    }

    set_user_balance::<T>(&caller);

    <Pallet<T>>::add_stake_multiple(
    RawOrigin::Signed(caller).into(),
        netuid,
        module_keys.clone(),
        amounts.clone(),
    );

    (netuid, module_keys, amounts)
}


#[benchmarks]
mod benchmarks {
    use super::*;

    #[benchmark]
    fn register() -> Result<(), BenchmarkError> {
        let network: Vec<u8> = b"network".to_vec();
        let name: Vec<u8> = b"name".to_vec();
        let address: Vec<u8> = b"address".to_vec();
        let module_key: T::AccountId = account("key", 0, SEED);

        set_user_balance::<T>(&module_key);

        #[extrinsic_call]
		register(
            RawOrigin::Signed(module_key.clone()),
			network.clone(),
            name,
            address,
            MIN_STAKE,
			module_key.clone(),
        );

        let netuid = <Pallet<T>>::get_netuid_for_name(network);

        assert!(
            <Pallet<T>>::is_registered(netuid, &module_key),
            "Register failed"
        );

        Ok(())
    }

    #[benchmark]
    fn set_weights() -> Result<(), BenchmarkError> {
        let network: Vec<u8> = b"network".to_vec();
        let name: Vec<u8> = b"name".to_vec();
        let address: Vec<u8> = b"address".to_vec();
        let caller: T::AccountId = account("key", 0, SEED);

        add_stake_helper::<T>(
            network.clone(),
            name.clone(),
            address.clone(),
            caller.clone(),
            MIN_STAKE
        );

        let (netuid, _, _) = add_stake_multiple_helper::<T>(caller.clone());

        let uids = <Pallet<T>>::get_uids(netuid);

        #[extrinsic_call]
		set_weights(
            RawOrigin::Signed(caller.clone()),
            netuid,
            uids.clone(),
            vec![1u16;uids.len()]
        );

        Ok(())
    }

    #[benchmark]
    fn add_stake() -> Result<(), BenchmarkError> {
        let (network, name, address, module_key, netuid) = default_register_helper::<T>();

        #[extrinsic_call]
		add_stake(
            RawOrigin::Signed(module_key.clone()),
			netuid,
			module_key.clone(),
            MIN_STAKE,
        );

        Ok(())
    }

    #[benchmark]
    fn add_stake_multiple() -> Result<(), BenchmarkError> {
        let caller: T::AccountId = account("caller", 0, SEED);
        
        let network: Vec<u8> = b"network".to_vec();
        let address: Vec<u8> = b"address".to_vec();

        let module_keys: Vec<T::AccountId> = (0..10).map(|i| account("key", i, SEED)).collect();
        let amounts: Vec<u64> = (0..10).map(|i| i + MIN_STAKE).collect();
        
        let mut netuid: u16 = 0;
        
        for (index, module_key) in module_keys.iter().enumerate() {
            let mut name: Vec<u8> = b"name".to_vec();
            name.extend(vec![index as u8]);

            netuid = register_helper::<T>(
                network.clone(),
                name,
                address.clone(),
                module_key.clone()
            );
        }

        set_user_balance::<T>(&caller);

        #[extrinsic_call]
		add_stake_multiple(
            RawOrigin::Signed(caller),
			netuid,
			module_keys,
            amounts,
        );

        Ok(())
    }

    #[benchmark]
    fn transfer_stake() -> Result<(), BenchmarkError> {
        let (_, _, _, new_module_key, _) = default_register_helper::<T>();

        let network: Vec<u8> = b"network".to_vec();
        let old_name: Vec<u8> = b"old_name".to_vec();
        let old_address: Vec<u8> = b"old_address".to_vec();
        let old_module_key: T::AccountId = account("old_key", 0, SEED);

        let netuid = add_stake_helper::<T>(
            network.clone(),
            old_name.clone(),
            old_address.clone(),
            old_module_key.clone(),
            MIN_STAKE
        );

        #[extrinsic_call]
		transfer_stake(
            RawOrigin::Signed(old_module_key.clone()),
			netuid,
			old_module_key.clone(),
            new_module_key.clone(),
            MIN_STAKE,
        );

        Ok(())
    }

    #[benchmark]
    fn transfer_multiple() -> Result<(), BenchmarkError> {
        let network: Vec<u8> = b"network".to_vec();
        let name: Vec<u8> = b"name".to_vec();
        let address: Vec<u8> = b"address".to_vec();
        let module_key: T::AccountId = account("key", 0, SEED);

        let netuid = add_stake_helper::<T>(
            network.clone(),
            name.clone(),
            address.clone(),
            module_key.clone(),
            MIN_STAKE
        );

        let new_module_keys: Vec<T::AccountId> = (0..10).map(|i| account("new_key", i, SEED)).collect();
        let amounts: Vec<u64> = (0..10).map(|i| i + MIN_STAKE).collect();

        for (index, new_module_key) in new_module_keys.iter().enumerate() {
            let mut new_name: Vec<u8> = b"name".to_vec();
            new_name.extend(vec![index as u8]);
    
            register_helper::<T>(
                network.clone(),
                new_name,
                address.clone(),
                new_module_key.clone()
            );
        }

        #[extrinsic_call]
		transfer_multiple(
            RawOrigin::Signed(module_key.clone()),
			new_module_keys,
            amounts,
        );

        Ok(())
    }

    #[benchmark]
    fn remove_stake() -> Result<(), BenchmarkError> {
        let network: Vec<u8> = b"network".to_vec();
        let name: Vec<u8> = b"name".to_vec();
        let address: Vec<u8> = b"address".to_vec();
        let module_key: T::AccountId = account("key", 0, SEED);

        let netuid = add_stake_helper::<T>(
            network.clone(),
            name.clone(),
            address.clone(),
            module_key.clone(),
            MIN_STAKE
        );

        #[extrinsic_call]
		remove_stake(
            RawOrigin::Signed(module_key.clone()),
			netuid,
			module_key.clone(),
            MIN_STAKE,
        );

        Ok(())
    }

    #[benchmark]
    fn remove_stake_multiple() -> Result<(), BenchmarkError> {
        let caller: T::AccountId = account("caller", 0, SEED);

        let (netuid, module_keys, amounts) = add_stake_multiple_helper::<T>(caller.clone());

        #[extrinsic_call]
		remove_stake_multiple(
            RawOrigin::Signed(caller.clone()),
			netuid,
			module_keys,
            amounts,
        );

        Ok(())
    }

    #[benchmark]
    fn update_network() -> Result<(), BenchmarkError> {
        let (network, name, address, module_key, netuid) = default_register_helper::<T>();

        let subnet_params = <Pallet<T>>::subnet_params(netuid);
        let tempo = 5;
		let min_stake = 0;

        #[extrinsic_call]
		update_network(
            RawOrigin::Signed(module_key.clone()),
			netuid,
			subnet_params.name.clone(),
			tempo,
			subnet_params.immunity_period,
			subnet_params.min_allowed_weights,
			subnet_params.max_allowed_weights,
			subnet_params.max_allowed_uids,
            subnet_params.burn_rate,
            min_stake,
			subnet_params.founder,
        );

        Ok(())
    }

    #[benchmark]
    fn remove_network() -> Result<(), BenchmarkError> {
        let (network, name, address, module_key, netuid) = default_register_helper::<T>();

        #[extrinsic_call]
		remove_network(
            RawOrigin::Signed(module_key.clone()),
			netuid
        );

        Ok(())
    }

    #[benchmark]
    fn update_module() -> Result<(), BenchmarkError> {
        let (network, name, address, module_key, netuid) = default_register_helper::<T>();

        let mut new_name = name.clone();
        new_name.extend(vec![1u8]);

        #[extrinsic_call]
		update_module(
            RawOrigin::Signed(module_key.clone()),
			netuid,
            new_name,
            address,
            Option::None
        );
        

        Ok(())
    }

    #[benchmark]
    fn update_global() -> Result<(), BenchmarkError> {
        #[extrinsic_call]
		update_global(
            RawOrigin::Root,
			1,
            1,
            1,
            1,
            1,
            1
        );

        Ok(())
    }

    #[benchmark]
    fn add_global_update() -> Result<(), BenchmarkError> {
        #[extrinsic_call]
		add_global_update(
            RawOrigin::Root,
			1,
            1,
            1,
            1,
            1,
            1
        );

        Ok(())
    }

    #[benchmark]
    fn vote_global_update() -> Result<(), BenchmarkError> {
		<Pallet<T>>::add_global_update(
            RawOrigin::Root.into(),
			1,
            1,
            1,
            1,
            1,
            1
        );

        let network: Vec<u8> = b"network".to_vec();
        let name: Vec<u8> = b"name".to_vec();
        let address: Vec<u8> = b"address".to_vec();
        let caller: T::AccountId = account("key", 0, SEED);

        add_stake_helper::<T>(
            network.clone(),
            name.clone(),
            address.clone(),
            caller.clone(),
            MIN_STAKE
        );

        #[extrinsic_call]
        vote_global_update(
            RawOrigin::Signed(caller.clone()),
            1
        );

        Ok(())
    }

    #[benchmark]
    fn accept_global_update() -> Result<(), BenchmarkError> {
		<Pallet<T>>::add_global_update(
            RawOrigin::Root.into(),
			1,
            1,
            1,
            1,
            1,
            1
        );

        let network: Vec<u8> = b"network".to_vec();
        let name: Vec<u8> = b"name".to_vec();
        let address: Vec<u8> = b"address".to_vec();
        let caller: T::AccountId = account("key", 0, SEED);

        add_stake_helper::<T>(
            network.clone(),
            name.clone(),
            address.clone(),
            caller.clone(),
            MIN_STAKE
        );

        <Pallet<T>>::vote_global_update(
            RawOrigin::Signed(caller.clone()).into(),
            1
        );

        #[extrinsic_call]
        accept_global_update(
            RawOrigin::Signed(caller.clone()),
            1
        );

        Ok(())
    }

    #[benchmark]
    fn add_subnet_update() -> Result<(), BenchmarkError> {
        let network: Vec<u8> = b"network".to_vec();
        let name: Vec<u8> = b"name".to_vec();
        let address: Vec<u8> = b"address".to_vec();
        let caller: T::AccountId = account("key", 0, SEED);

        add_stake_helper::<T>(
            network.clone(),
            name.clone(),
            address.clone(),
            caller.clone(),
            MIN_STAKE
        );

        let netuid = <Pallet<T>>::get_netuid_for_name(network.clone());

        #[extrinsic_call]
		add_subnet_update(
            RawOrigin::Root,
			netuid,
            name,
            1,
            1,
            1,
            1,
            1,
            1,
            1,
            1,
            1,
        );

        Ok(())
    }

    #[benchmark]
    fn vote_subnet_update() -> Result<(), BenchmarkError> {
        let network: Vec<u8> = b"network".to_vec();
        let name: Vec<u8> = b"name".to_vec();
        let address: Vec<u8> = b"address".to_vec();
        let caller: T::AccountId = account("key", 0, SEED);

        add_stake_helper::<T>(
            network.clone(),
            name.clone(),
            address.clone(),
            caller.clone(),
            MIN_STAKE
        );

        let netuid = <Pallet<T>>::get_netuid_for_name(network.clone());

		<Pallet<T>>::add_subnet_update(
            RawOrigin::Root.into(),
			netuid,
            name,
            1,
            1,
            1,
            1,
            1,
            1,
            1,
            1,
            1,
        );
        

        #[extrinsic_call]
        vote_subnet_update(
            RawOrigin::Signed(caller.clone()),
            1
        );

        Ok(())
    }

    #[benchmark]
    fn accept_subnet_update() -> Result<(), BenchmarkError> {
        let network: Vec<u8> = b"network".to_vec();
        let name: Vec<u8> = b"name".to_vec();
        let address: Vec<u8> = b"address".to_vec();
        let caller: T::AccountId = account("key", 0, SEED);

        add_stake_helper::<T>(
            network.clone(),
            name.clone(),
            address.clone(),
            caller.clone(),
            MIN_STAKE
        );

        let netuid = <Pallet<T>>::get_netuid_for_name(network.clone());

		<Pallet<T>>::add_subnet_update(
            RawOrigin::Root.into(),
			netuid,
            name,
            1,
            1,
            1,
            1,
            1,
            1,
            1,
            1,
            1,
        );
        

        <Pallet<T>>::vote_subnet_update(
            RawOrigin::Signed(caller.clone()).into(),
            1
        );

        #[extrinsic_call]
        accept_subnet_update(
            RawOrigin::Signed(caller.clone()),
            1
        );

        Ok(())
    }

    impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::mock::Test);
}
