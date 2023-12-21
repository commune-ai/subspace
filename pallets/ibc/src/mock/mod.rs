use super::*;

use crate as pallet_ibc;
pub use frame_support::{
	construct_runtime, parameter_types,
	traits::Everything,
	StorageValue,
};
use frame_system as system;
use pallet_ibc_utils::module::DefaultRouter;
use sp_runtime::{
	traits::{IdentityLookup, BlakeTwo256, IdentifyAccount, Verify},
	MultiSignature,
};
use sp_core::{Get, H256, U256};

pub type Signature = MultiSignature;
pub(crate) type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

// Configure a mock runtime to test the pallet.
construct_runtime!(
	pub enum Test {
		System: frame_system,
		PalletTimestamp: pallet_timestamp,
		Ibc: pallet_ibc,
	}
);

/// A hash of some data used by the chain.
pub type Hash = sp_core::H256;

parameter_types! {
	pub const BlockHashCount: u64 = 250;
	pub const SS58Prefix: u8 = 42;
}

/// Index of a transaction in the chain.
pub type Index = u32;
/// An index to a block.
pub type BlockNumber = u32;

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

pub type Balance = u128;
/// Type used for expressing timestamp.
pub type Moment = u64;

pub const MILLISECS_PER_BLOCK: Moment = 6000;

// NOTE: Currently it is not possible to change the slot duration after the chain has started.
//       Attempting to do so will brick block production.
pub const SLOT_DURATION: Moment = MILLISECS_PER_BLOCK;

parameter_types! {
	pub const MinimumPeriod: Moment = SLOT_DURATION / 2;
}

impl pallet_timestamp::Config for Test {
	/// A timestamp: milliseconds since the unix epoch.
	type Moment = Moment;
	type OnTimestampSet = ();
	type MinimumPeriod = MinimumPeriod;
	type WeightInfo = ();
}

parameter_types! {
	pub const MaxAuthorities: u32 = 100;
	pub const MaxKeys: u32 = 10_000;
	pub const MaxPeerInHeartbeats: u32 = 10_000;
	pub const MaxPeerDataEncodingSize: u32 = 1_000;
}

parameter_types! {
	pub const ExpectedBlockTime: u64 = 6;
	pub const ChainVersion: u64 = 0;
}

impl pallet_ibc_utils::module::AddModule for Test {
	fn add_module(router: pallet_ibc_utils::module::Router) -> pallet_ibc_utils::module::Router {
		router
	}
}

impl pallet::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type TimeProvider = pallet_timestamp::Pallet<Test>;
	type ExpectedBlockTime = ExpectedBlockTime;
	const IBC_COMMITMENT_PREFIX: &'static [u8] = b"Ibc";
	type ChainVersion = ChainVersion;
	type IbcModule = DefaultRouter;
	type WeightInfo = ();
}

#[allow(dead_code)]
// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
	system::GenesisConfig::default().build_storage::<Test>().unwrap().into()
}
