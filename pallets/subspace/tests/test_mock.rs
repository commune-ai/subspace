// disable all warnings
#![allow(warnings)]
use frame_support::{
	assert_ok, parameter_types,
	traits::{ConstU32, ConstU64, Everything, Hooks, StorageMapShim},
	weights,
};
use frame_system as system;
use frame_system::{limits, Config, EnsureNever, EnsureRoot, RawOrigin};
use pallet_subspace::autogen_weights::SubstrateWeight;
use sp_core::{Get, H256, U256};
use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, IdentityLookup},
	BuildStorage, DispatchResult,
};
use sp_std::hash::Hash;

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
	pub enum Test {
		System: frame_system,
		Balances: pallet_balances,
		SubspaceModule: pallet_subspace,
	}
);

#[allow(dead_code)]
pub type SubspaceCall = pallet_subspace::Call<Test>;

#[allow(dead_code)]
pub type BalanceCall = pallet_balances::Call<Test>;

#[allow(dead_code)]
pub type TestRuntimeCall = frame_system::Call<Test>;

parameter_types! {
	pub const BlockHashCount: u64 = 250;
	pub const SS58Prefix: u8 = 42;
}

#[allow(dead_code)]
pub type AccountId = U256;

// The address format for describing accounts.
pub type Address = AccountId;

// Balance of an account.
#[allow(dead_code)]
pub type Balance = u64;

// An index to a block.
#[allow(dead_code)]
pub type BlockNumber = u64;

parameter_types! {
	pub const ExistentialDeposit: Balance = 1;
	pub const MaxLocks: u32 = 50;
	pub const MaxReserves: u32 = 50;
}

impl pallet_balances::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type AccountStore = System;
	type Balance = u64;
	type DustRemoval = ();
	type ExistentialDeposit = ExistentialDeposit;
	type MaxLocks = MaxLocks;
	type WeightInfo = ();
	type MaxReserves = MaxReserves;
	type ReserveIdentifier = ();
    type RuntimeHoldReason = ();
    type FreezeIdentifier = ();
    type MaxHolds = frame_support::traits::ConstU32<16>;
	type MaxFreezes = frame_support::traits::ConstU32<16>;
}

impl system::Config for Test {
	type BaseCallFilter = Everything;
	type Block = Block;
	type BlockWeights = ();
	type BlockLength = ();
	type AccountId = U256;
	type RuntimeCall = RuntimeCall;
	type Nonce = u64;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type Lookup = IdentityLookup<Self::AccountId>;
	type RuntimeEvent = RuntimeEvent;
	type RuntimeOrigin = RuntimeOrigin;
	type BlockHashCount = BlockHashCount;
	type DbWeight = ();
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = pallet_balances::AccountData<Balance>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
	type SS58Prefix = SS58Prefix;
	type OnSetCode = ();
	type MaxConsumers = frame_support::traits::ConstU32<16>;
}

impl pallet_subspace::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type WeightInfo = ();
}

// Build genesis storage according to the mock runtime.
//pub fn new_test_ext() -> sp_io::TestExternalities {
//	system::GenesisConfig::default().build_storage().unwrap().into()
//}

#[allow(dead_code)]
pub fn set_weights(netuid: u16, key: U256, uids: Vec<u16>, values: Vec<u16>) {
	SubspaceModule::set_weights(get_origin(key), netuid, uids.clone(), values.clone()).unwrap();
}

// Build genesis storage according to the mock runtime.
#[allow(dead_code)]
pub fn new_test_ext() -> sp_io::TestExternalities {
	sp_tracing::try_init_simple();
	frame_system::GenesisConfig::<Test>::default().build_storage().unwrap().into()
}

#[allow(dead_code)]
pub fn test_ext_with_balances(balances: Vec<(U256, u128)>) -> sp_io::TestExternalities {
	sp_tracing::try_init_simple();
	let mut t = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap();

	pallet_balances::GenesisConfig::<Test> {
		balances: balances.iter().map(|(a, b)| (*a, *b as u64)).collect::<Vec<(U256, u64)>>(),
	}
	.assimilate_storage(&mut t)
	.unwrap();

	t.into()
}

#[allow(dead_code)]
pub(crate) fn step_block(n: u16) {
	for _ in 0..n {
		SubspaceModule::on_finalize(System::block_number());
		System::on_finalize(System::block_number());
		System::set_block_number(System::block_number() + 1);
		System::on_initialize(System::block_number());
		SubspaceModule::on_initialize(System::block_number());
	}
}

pub(crate) fn step_epoch(netuid: u16) {
	let tempo: u16 = SubspaceModule::get_tempo(netuid);
	step_block(tempo);
}

#[allow(dead_code)]
pub(crate) fn block_number() -> u64 {
	return System::block_number()
}

#[allow(dead_code)]
pub(crate) fn run_to_block(n: u64) {
	while System::block_number() < n {
		SubspaceModule::on_finalize(System::block_number());
		System::on_finalize(System::block_number());
		System::set_block_number(System::block_number() + 1);
		System::on_initialize(System::block_number());
		SubspaceModule::on_initialize(System::block_number());
	}
}

#[allow(dead_code)]
pub fn add_balance(key: U256, balance: u64) {
	SubspaceModule::add_balance_to_account(&key, balance);
}
#[allow(dead_code)]
pub fn increase_stake(netuid: u16, key: U256, stake: u64) {
	SubspaceModule::increase_stake(netuid, &key, &key, stake);
}

#[allow(dead_code)]
pub fn delegate_stake(netuid: u16, key: U256, module_key: U256, stake: u64) {
	SubspaceModule::increase_stake(netuid, &key, &module_key, stake);
}

#[allow(dead_code)]
pub fn decrease_stake(netuid: u16, key: U256, stake: u64) {
	SubspaceModule::decrease_stake(netuid, &key, &key, stake);
}

pub fn get_origin(key: U256) -> RuntimeOrigin {
	let origin = <<Test as frame_system::Config>::RuntimeOrigin>::signed(key);
	return origin
}

pub fn register_n_modules(netuid: u16, n: u16, stake: u64) {
	for i in 0..n {
		register_module(netuid, U256::from(i), stake);
	}
}

#[allow(dead_code)]
pub fn register_module(netuid: u16, key: U256, stake: u64) -> DispatchResult {
	// can i format the test in rus

	let mut network: Vec<u8> = "test".as_bytes().to_vec();
	network.extend(netuid.to_string().as_bytes().to_vec());

	let mut name: Vec<u8> = "module".as_bytes().to_vec();
	name.extend(key.to_string().as_bytes().to_vec());

	let address: Vec<u8> = "0.0.0.0:30333".as_bytes().to_vec();

	let block_number: u64 = SubspaceModule::get_current_block_as_u64();
	let origin = get_origin(key);
	let is_new_subnet: bool = !SubspaceModule::if_subnet_exist(netuid);
	if is_new_subnet {
		SubspaceModule::set_max_registrations_per_block(1000)
	}

	add_balance(key, stake + 1);
	let balance = SubspaceModule::get_balance(&key);

	let result = SubspaceModule::register(origin, network, name.clone(), address, stake, key);
	return result
}

#[allow(dead_code)]
pub fn delegate_register_module(
	netuid: u16,
	key: U256,
	module_key: U256,
	stake: u64,
) -> DispatchResult {
	// can i format the test in rus

	let mut network: Vec<u8> = "test".as_bytes().to_vec();
	network.extend(netuid.to_string().as_bytes().to_vec());

	let mut name: Vec<u8> = "module".as_bytes().to_vec();
	name.extend(module_key.to_string().as_bytes().to_vec());

	let address: Vec<u8> = "0.0.0.0:30333".as_bytes().to_vec();

	let block_number: u64 = SubspaceModule::get_current_block_as_u64();
	let origin = get_origin(key);
	let is_new_subnet: bool = !SubspaceModule::if_subnet_exist(netuid);
	if is_new_subnet {
		SubspaceModule::set_max_registrations_per_block(1000)
	}

	let balance = SubspaceModule::get_balance(&key);

	if stake >= balance {
		add_balance(key, stake + 1);
	}
	println!(
		"Registering module: network: {:?}, key: {:?} stake {:?}",
		network, module_key, balance
	);

	let result =
		SubspaceModule::register(origin, network, name.clone(), address, stake, module_key);

	log::info!(
		"Register ok neuron: network: {:?}, module_key: {:?} key: {:?}",
		name.clone(),
		module_key,
		key
	);

	return result
}

#[allow(dead_code)]
pub fn register(netuid: u16, key: U256, stake: u64) {
	// can i format the test in rus
	let mut network: Vec<u8> = "test".as_bytes().to_vec();
	network.extend(netuid.to_string().as_bytes().to_vec());
	let mut name: Vec<u8> = "module".as_bytes().to_vec();
	name.extend(key.to_string().as_bytes().to_vec());
	let address: Vec<u8> = "0.0.0.0:30333".as_bytes().to_vec();
	let origin = get_origin(key);
	let is_new_subnet: bool = !SubspaceModule::if_subnet_exist(netuid);

	let result = SubspaceModule::register(origin, network, name.clone(), address, stake, key);
	assert_ok!(result);
}

#[allow(dead_code)]
pub fn add_network(netuid: u16, founder: U256) {
	let network: Vec<u8> = netuid.to_string().as_bytes().to_vec();
	let stake: u64 = 1_000_000_000;
	let result = SubspaceModule::add_network_from_registration(network, stake, &founder);
}

#[allow(dead_code)]
pub fn remove_network(netuid: u16, key: U256) {
	let origin = get_origin(key);
	let result = SubspaceModule::do_remove_network(origin, netuid);
	assert_ok!(result);
}

#[allow(dead_code)]
pub fn remove_stake(netuid: u16, key: U256, amount: u64) {
	let origin = get_origin(key);
	let result = SubspaceModule::remove_stake(origin, netuid, key, amount);

	assert_ok!(result);
}
#[allow(dead_code)]
pub fn add_stake(netuid: u16, key: U256, amount: u64) {
	let origin = get_origin(key);
	let result = SubspaceModule::add_stake(origin, netuid, key, amount);

	assert_ok!(result);
}

#[allow(dead_code)]
pub fn add_stake_and_balance(netuid: u16, key: U256, amount: u64) {
	let origin = get_origin(key);
	add_balance(key, amount);
	let result = SubspaceModule::add_stake(origin, netuid, key, amount);

	assert_ok!(result);
}
