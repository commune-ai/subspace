// disable all warnings
#![allow(warnings)]
use crate as pallet_subspace;
use frame_support::{
	assert_ok, parameter_types,
	traits::{Everything, Hash, Hooks, StorageMapShim},
	weights,
};
use frame_system as system;
use frame_system::{limits, Config, EnsureNever, EnsureRoot, RawOrigin};
use sp_core::{ConstU32, Get, H256, U256};
use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, IdentityLookup},
	BuildStorage, DispatchResult,
};

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

parameter_types! {
	pub const ExistentialDeposit: Balance = 1;
	pub const MaxLocks: u32 = 50;
	pub const MaxReserves: u32 = 50;
}

impl pallet_balances::Config for Test {
	type Balance = Balance;
	type RuntimeEvent = RuntimeEvent;
	type DustRemoval = ();
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = System;
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
	type DbWeight = ();
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeCall = RuntimeCall;
	type Nonce = u32;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = U256;
	type Lookup = IdentityLookup<Self::AccountId>;
	type RuntimeEvent = RuntimeEvent;
	type BlockHashCount = BlockHashCount;
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
#[allow(dead_code)]
pub fn new_test_ext() -> sp_io::TestExternalities {
	sp_tracing::try_init_simple();
	<frame_system::GenesisConfig<Test>>::default().build_storage().unwrap().into()
}
