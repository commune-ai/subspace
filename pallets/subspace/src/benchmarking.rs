#![cfg(feature = "runtime-benchmarks")]

use super::*;

use super::*;

use crate::Pallet;
use frame_benchmarking::v2::*;
use frame_system::RawOrigin;

const SEED: u32 = 1;
const BALANCE: u64 = 10_000_000_000_000_000_000;
const MIN_STAKE: u64 = 1_00_000_000_000;
const BATCH: u64 = 250;

fn set_user_balance<T: Config>(user: &T::AccountId) {
	T::Currency::deposit_creating(user, <Pallet<T>>::u64_to_balance(BALANCE).unwrap());
}

fn single_register_helper<T: Config>() -> (Vec<u8>, Vec<u8>, Vec<u8>, T::AccountId, u16) {
	let network: Vec<u8> = b"network".to_vec();
	let name: Vec<u8> = b"name".to_vec();
	let address: Vec<u8> = b"address".to_vec();
	let module_key: T::AccountId = account("module_key", 0, SEED);

	set_user_balance::<T>(&module_key);

	let netuid = register_helper::<T>(
		network.clone(),
		name.clone(),
		address.clone(),
		MIN_STAKE,
		module_key.clone(),
	);

	(network, name, address, module_key, netuid)
}

fn multiple_register_helper<T: Config>() -> (u16, Vec<T::AccountId>, Vec<u64>) {
	let network: Vec<u8> = b"network".to_vec();

	let module_keys: Vec<T::AccountId> =
		(0..BATCH).map(|i| account("module_key", i as u32, SEED)).collect();
	let amounts: Vec<u64> = (0..BATCH).map(|i| MIN_STAKE).collect();

	let mut netuid: u16 = 0;

	for (index, module_key) in module_keys.iter().enumerate() {
		let mut address: Vec<u8> = b"address".to_vec();
		let mut name: Vec<u8> = b"name".to_vec();

		address.extend(vec![index as u8]);
		name.extend(vec![index as u8]);

		set_user_balance::<T>(&module_key);

		netuid =
			register_helper::<T>(network.clone(), name, address, MIN_STAKE, module_key.clone());
	}

	(netuid, module_keys, amounts)
}

fn register_helper<T: Config>(
	network: Vec<u8>,
	name: Vec<u8>,
	address: Vec<u8>,
	stake: u64,
	module_key: T::AccountId,
) -> u16 {
	<Pallet<T>>::register(
		RawOrigin::Signed(module_key.clone()).into(),
		network.clone(),
		name.clone(),
		address.clone(),
		stake,
		module_key.clone(),
	);

	let netuid = <Pallet<T>>::get_netuid_for_name(network.clone());

	netuid
}

fn add_stake_helper<T: Config>(
	caller: T::AccountId,
	network: Vec<u8>,
	name: Vec<u8>,
	address: Vec<u8>,
	module_key: T::AccountId,
	stake: u64,
) -> u16 {
	let netuid =
		register_helper::<T>(network, name.clone(), address.clone(), MIN_STAKE, module_key.clone());

	<Pallet<T>>::add_stake(RawOrigin::Signed(caller).into(), netuid, module_key, stake);

	netuid
}

fn add_stake_multiple_helper<T: Config>(
	caller: T::AccountId,
) -> (u16, Vec<T::AccountId>, Vec<u64>) {
	let (netuid, module_keys, amounts) = multiple_register_helper::<T>();

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

	// #[benchmark]
	// fn set_weights() -> Result<(), BenchmarkError> {
	//     let caller: T::AccountId = account("caller", 0, SEED);
	//     let network: Vec<u8> = b"network".to_vec();
	//     let address: Vec<u8> = b"address".to_vec();
	//     let name: Vec<u8> = b"name".to_vec();

	//     set_user_balance::<T>(&caller);

	//     let _netuid = register_helper::<T>(
	//         network.clone(),
	//         name.clone(),
	//         address.clone(),
	//         MIN_STAKE * BATCH,
	//         caller.clone(),
	//     );

	//     let (netuid, _module_keys, _amounts) = multiple_register_helper::<T>();

	//     let uids = <Pallet<T>>::get_uids(netuid);

	//     #[extrinsic_call]
	// 	set_weights(
	//         RawOrigin::Signed(caller.clone()),
	//         netuid,
	//         uids[1..].to_vec(),
	//         vec![1u16;uids.len() - 1]
	//     );

	//     Ok(())
	// }

	// #[benchmark]
	// fn add_stake() -> Result<(), BenchmarkError> {
	//     let caller: T::AccountId = account("caller", 0, SEED);

	//     let (network, name, address, module_key, netuid) = single_register_helper::<T>();

	//     set_user_balance::<T>(&caller);

	//     #[extrinsic_call]
	// 	add_stake(
	//         RawOrigin::Signed(caller),
	// 		netuid,
	// 		module_key,
	//         MIN_STAKE,
	//     );

	//     Ok(())
	// }

	// #[benchmark]
	// fn add_stake_multiple() -> Result<(), BenchmarkError> {
	//     let caller: T::AccountId = account("caller", 0, SEED);

	//     let network: Vec<u8> = b"network".to_vec();

	//     let module_keys: Vec<T::AccountId> = (0..BATCH).map(|i| account("module_key", i as u32,
	// SEED)).collect();     let amounts: Vec<u64> = (0..BATCH).map(|i| MIN_STAKE).collect();

	//     let mut netuid: u16 = 0;

	//     set_user_balance::<T>(&caller);

	//     for (index, module_key) in module_keys.iter().enumerate() {
	//         let mut address: Vec<u8> = b"address".to_vec();
	//         let mut name: Vec<u8> = b"name".to_vec();

	//         address.extend(vec![index as u8]);
	//         name.extend(vec![index as u8]);

	//         set_user_balance::<T>(&module_key);

	//         netuid = register_helper::<T>(
	//             network.clone(),
	//             name,
	//             address,
	//             MIN_STAKE,
	//             module_key.clone()
	//         );
	//     }

	//     #[extrinsic_call]
	// 	add_stake_multiple(
	//         RawOrigin::Signed(caller),
	// 		netuid,
	// 		module_keys,
	//         amounts,
	//     );

	//     Ok(())
	// }

	// #[benchmark]
	// fn remove_stake() -> Result<(), BenchmarkError> {
	//     let caller: T::AccountId = account("caller", 0, SEED);
	//     let network: Vec<u8> = b"network".to_vec();
	//     let name: Vec<u8> = b"name".to_vec();
	//     let address: Vec<u8> = b"address".to_vec();
	//     let module_key: T::AccountId = account("module_key", 0, SEED);

	//     set_user_balance::<T>(&caller);
	//     set_user_balance::<T>(&module_key);

	//     let netuid = add_stake_helper::<T>(
	//         caller.clone(),
	//         network.clone(),
	//         name.clone(),
	//         address.clone(),
	//         module_key.clone(),
	//         MIN_STAKE
	//     );

	//     #[extrinsic_call]
	// 	remove_stake(
	//         RawOrigin::Signed(caller.clone()),
	// 		netuid,
	// 		module_key.clone(),
	//         MIN_STAKE,
	//     );

	//     Ok(())
	// }

	// #[benchmark]
	// fn remove_stake_multiple() -> Result<(), BenchmarkError> {
	//     let caller: T::AccountId = account("caller", 0, SEED);

	//     set_user_balance::<T>(&caller);

	//     let (netuid, module_keys, amounts) = add_stake_multiple_helper::<T>(caller.clone());

	//     #[extrinsic_call]
	// 	remove_stake_multiple(
	//         RawOrigin::Signed(caller.clone()),
	// 		netuid,
	// 		module_keys,
	//         amounts,
	//     );

	//     Ok(())
	// }

	// #[benchmark]
	// fn transfer_stake() -> Result<(), BenchmarkError> {
	//     let caller: T::AccountId = account("caller", 0, SEED);
	//     let network: Vec<u8> = b"network".to_vec();

	//     let old_name: Vec<u8> = b"old_name".to_vec();
	//     let old_address: Vec<u8> = b"old_address".to_vec();
	//     let old_module_key: T::AccountId = account("old_key", 0, SEED);

	//     let new_name: Vec<u8> = b"new_name".to_vec();
	//     let new_address: Vec<u8> = b"new_address".to_vec();
	//     let new_module_key: T::AccountId = account("new_key", 0, SEED);

	//     set_user_balance::<T>(&caller);
	//     set_user_balance::<T>(&old_module_key);
	//     set_user_balance::<T>(&new_module_key);

	//     let _ = add_stake_helper::<T>(
	//         caller.clone(),
	//         network.clone(),
	//         old_name.clone(),
	//         old_address.clone(),
	//         old_module_key.clone(),
	//         MIN_STAKE
	//     );

	//     let netuid = register_helper::<T>(
	//         network.clone(),
	//         new_name,
	//         new_address,
	//         MIN_STAKE,
	//         new_module_key.clone()
	//     );

	//     #[extrinsic_call]
	// 	transfer_stake(
	//         RawOrigin::Signed(caller.clone()),
	// 		netuid,
	// 		old_module_key.clone(),
	//         new_module_key.clone(),
	//         MIN_STAKE,
	//     );

	//     Ok(())
	// }

	// #[benchmark]
	// fn transfer_multiple() -> Result<(), BenchmarkError> {
	//     let caller: T::AccountId = account("caller", 0, SEED);

	//     set_user_balance::<T>(&caller);

	//     let (netuid, module_keys, amounts) = multiple_register_helper::<T>();

	//     #[extrinsic_call]
	// 	transfer_multiple(
	//         RawOrigin::Signed(caller),
	// 		module_keys,
	//         amounts,
	//     );

	//     Ok(())
	// }

	// #[benchmark]
	// fn update_module() -> Result<(), BenchmarkError> {
	//     let (network, name, address, module_key, netuid) = single_register_helper::<T>();

	//     let mut new_name = name.clone();
	//     new_name.extend(vec![1u8]);

	//     #[extrinsic_call]
	// 	update_module(
	//         RawOrigin::Signed(module_key.clone()),
	// 		netuid,
	//         new_name,
	//         address,
	//         Option::None
	//     );

	//     Ok(())
	// }

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

		Ok(())
	}

	// #[benchmark]
	// fn deregister() -> Result<(), BenchmarkError> {
	//     let (_, _, _, module_key, netuid) = single_register_helper::<T>();

	//     #[extrinsic_call]
	// 	deregister(
	//         RawOrigin::Signed(module_key.clone()),
	// 		netuid
	//     );

	//     Ok(())
	// }

	// #[benchmark]
	// fn add_profit_shares() -> Result<(), BenchmarkError> {
	//     let (_, _, _, module_key, netuid) = single_register_helper::<T>();

	//     let keys: Vec<T::AccountId> = (0..BATCH).map(|i| account("key", i as u32,
	// SEED)).collect();     let shares: Vec<u16> = (0..BATCH).map(|i| i as u16 + 1).collect();

	//     #[extrinsic_call]
	// 	add_profit_shares(
	//         RawOrigin::Signed(module_key.clone()),
	// 		keys,
	//         shares
	//     );

	//     Ok(())
	// }

	// #[benchmark]
	// fn update_global() -> Result<(), BenchmarkError> {
	//     #[extrinsic_call]
	// 	update_global(
	//         RawOrigin::Root,
	// 		1, // burn_rate
	//         1, // max_allowed_modules
	//         1, // max_allowed_subnets
	//         1, // max_name_length
	//         1, // max_proposals
	//         1, // max_registrations_per_block
	//         0, // min_burn
	//         0, // min_stake
	//         0, // min_weight_stake
	//         1, // tx_rate_limit
	//         1, // unit_emission
	//         b"stake".to_vec(), // vote_mode
	//         1, // vote_threshold
	//     );

	//     Ok(())
	// }

	// #[benchmark]
	// fn add_global_proposal() -> Result<(), BenchmarkError> {
	//     let caller: T::AccountId = account("caller", 0, SEED);

	//     #[extrinsic_call]
	// 	add_global_proposal(
	//         RawOrigin::Signed(caller),
	// 		1, // burn_rate
	//         1, // max_allowed_modules
	//         1, // max_allowed_subnets
	//         1, // max_name_length
	//         1, // max_proposals
	//         1, // max_registrations_per_block
	//         0, // min_burn
	//         0, // min_stake
	//         0, // min_weight_stake
	//         1, // unit_emission
	//         1, // tx_rate_limit
	//         1, // vote_threshold
	//         b"stake".to_vec(), // vote_mode
	//     );

	//     Ok(())
	// }

	// #[benchmark]
	// fn update_subnet() -> Result<(), BenchmarkError> {
	//     let (network, name, address, module_key, netuid) = single_register_helper::<T>();

	//     let subnet_params = <Pallet<T>>::subnet_params(netuid);

	//     #[extrinsic_call]
	// 	update_subnet(
	//         RawOrigin::Signed(module_key.clone()),
	// 		netuid,
	// 		subnet_params.founder,
	//         subnet_params.founder_share,
	// 		subnet_params.immunity_period,
	//         subnet_params.incentive_ratio,
	//         subnet_params.max_allowed_uids,
	//         subnet_params.max_allowed_weights,
	//         subnet_params.max_stake,
	//         subnet_params.min_allowed_weights,
	//         subnet_params.min_stake,
	// 		b"new_name".to_vec(),
	//         subnet_params.self_vote,
	//         subnet_params.tempo,
	//         subnet_params.trust_ratio,
	//         subnet_params.vote_mode,
	//         subnet_params.vote_threshold,
	//     );

	//     Ok(())
	// }

	// #[benchmark]
	// fn add_subnet_proposal() -> Result<(), BenchmarkError> {
	//     let caller: T::AccountId = account("key", 0, SEED);

	//     let (_, _, _, _, netuid) = single_register_helper::<T>();

	//     #[extrinsic_call]
	// 	add_subnet_proposal(
	//         RawOrigin::Signed(caller.clone()),
	// 		netuid,
	//         caller.clone(), // founder
	//         1, // founder_share
	//         1, // immunity_period
	//         1, // incentive_ratio
	//         u16::MAX, // max_allowed_uids
	//         1, // max_allowed_weights
	//         u64::MAX, // max_stake
	//         1, // min_allowed_weights
	//         0, // min_stake
	//         b"new_name".to_vec(), // name
	//         true, // self_vote
	//         1, // tempo
	//         1, // trust_ratio
	//         b"stake".to_vec(), // vote_mode
	//         1, // vote_threshold
	//     );

	//     Ok(())
	// }

	// #[benchmark]
	// fn add_custom_proposal() -> Result<(), BenchmarkError> {
	//     let caller: T::AccountId = account("key", 0, SEED);

	//     #[extrinsic_call]
	// 	add_custom_proposal(
	//         RawOrigin::Signed(caller.clone()),
	//         b"custom_proposal_data".to_vec(), // data
	//     );

	//     Ok(())
	// }

	// #[benchmark]
	// fn vote_proposal() -> Result<(), BenchmarkError> {
	//     let proposer: T::AccountId = account("proposer", 0, SEED);
	//     let voter: T::AccountId = account("voter", 0, SEED);

	//     set_user_balance::<T>(&proposer);
	//     set_user_balance::<T>(&voter);

	//     let (network, name, address, module_key, netuid) = single_register_helper::<T>();

	//     let _ = add_stake_helper::<T>(
	//         proposer.clone(),
	//         network.clone(),
	//         name.clone(),
	//         address.clone(),
	//         module_key.clone(),
	//         MIN_STAKE
	//     );

	//     let _ = add_stake_helper::<T>(
	//         voter.clone(),
	//         network.clone(),
	//         name.clone(),
	//         address.clone(),
	//         module_key.clone(),
	//         MIN_STAKE
	//     );

	// 	<Pallet<T>>::add_subnet_proposal(
	//         RawOrigin::Signed(proposer.clone()).into(),
	// 		netuid,
	//         proposer.clone(), // founder
	//         1, // founder_share
	//         1, // immunity_period
	//         1, // incentive_ratio
	//         u16::MAX, // max_allowed_uids
	//         1, // max_allowed_weights
	//         u64::MAX, // max_stake
	//         1, // min_allowed_weights
	//         0, // min_stake
	//         b"new_name".to_vec(), // name
	//         true, // self_vote
	//         1, // tempo
	//         1, // trust_ratio
	//         b"stake".to_vec(), // vote_mode
	//         1, // vote_threshold
	//     );

	//     #[extrinsic_call]
	// 	vote_proposal(
	//         RawOrigin::Signed(voter.clone()),
	// 		0
	//     );

	//     Ok(())
	// }

	// #[benchmark]
	// fn unvote_proposal() -> Result<(), BenchmarkError> {
	//     let proposer: T::AccountId = account("proposer", 0, SEED);
	//     let voter: T::AccountId = account("voter", 0, SEED);

	//     set_user_balance::<T>(&proposer);
	//     set_user_balance::<T>(&voter);

	//     let (network, name, address, module_key, netuid) = single_register_helper::<T>();

	//     let _ = add_stake_helper::<T>(
	//         proposer.clone(),
	//         network.clone(),
	//         name.clone(),
	//         address.clone(),
	//         module_key.clone(),
	//         MIN_STAKE
	//     );

	//     let _ = add_stake_helper::<T>(
	//         voter.clone(),
	//         network.clone(),
	//         name.clone(),
	//         address.clone(),
	//         module_key.clone(),
	//         MIN_STAKE
	//     );

	// 	<Pallet<T>>::add_subnet_proposal(
	//         RawOrigin::Signed(proposer.clone()).into(),
	// 		netuid,
	//         proposer.clone(), // founder
	//         1, // founder_share
	//         1, // immunity_period
	//         1, // incentive_ratio
	//         u16::MAX, // max_allowed_uids
	//         1, // max_allowed_weights
	//         u64::MAX, // max_stake
	//         1, // min_allowed_weights
	//         0, // min_stake
	//         b"new_name".to_vec(), // name
	//         true, // self_vote
	//         1, // tempo
	//         1, // trust_ratio
	//         b"stake".to_vec(), // vote_mode
	//         100, // vote_threshold
	//     );

	// 	<Pallet<T>>::vote_proposal(
	//         RawOrigin::Signed(voter.clone()).into(),
	// 		0
	//     );

	//     #[extrinsic_call]
	//     unvote_proposal(
	//         RawOrigin::Signed(voter.clone()),
	//     );

	//     Ok(())
	// }

	impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::mock::Test);
}
