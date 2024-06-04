#![allow(dead_code, unused_imports)]

use frame_support::{
    dispatch::DispatchResult,
    parameter_types,
    sp_runtime::{
        testing::H256,
        traits::{BlakeTwo256, IdentityLookup},
    },
    traits::{Currency, Everything, OnFinalize, OnInitialize},
    PalletId,
};
use pallet_governance_api::*;
use pallet_subspace::BurnConfig;
use sp_runtime::BuildStorage;

pub use frame_support::{assert_err, assert_ok};

type AccountId = u32;
type Balance = u64;
type Block = frame_system::mocking::MockBlock<Test>;

frame_support::construct_runtime!(
    pub enum Test {
        System: frame_system,
        Balances: pallet_balances,
        Subspace: pallet_subspace,
        Governance: pallet_governance,
    }
);

parameter_types! {
    pub const SS58Prefix: u8 = 42;
    pub const BlockHashCount: u64 = 250;
    pub const ExistentialDeposit: Balance = 1;
    pub const MaxLocks: u32 = 50;
    pub const MaxReserves: u32 = 50;
    pub const MaxFreezes: u32 = 16;
}

impl pallet_balances::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type AccountStore = System;
    type Balance = Balance;
    type DustRemoval = ();
    type ExistentialDeposit = ExistentialDeposit;
    type MaxLocks = MaxLocks;
    type WeightInfo = ();
    type MaxReserves = MaxReserves;
    type ReserveIdentifier = ();
    type RuntimeHoldReason = ();
    type FreezeIdentifier = ();
    type MaxFreezes = MaxFreezes;
    type RuntimeFreezeReason = ();
}

impl frame_system::Config for Test {
    type BaseCallFilter = Everything;
    type Block = Block;
    type BlockWeights = ();
    type BlockLength = ();
    type AccountId = AccountId;
    type RuntimeCall = RuntimeCall;
    type Nonce = u32;
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

    type RuntimeTask = ();
    type SingleBlockMigrations = ();
    type MultiBlockMigrator = ();
    type PreInherents = ();
    type PostInherents = ();
    type PostTransactions = ();
}

parameter_types! {
    pub const SubspacePalletId: PalletId = PalletId(*b"py/subsp");
    pub const DefaultProposalCost: u64 = 10_000_000_000_000;
}

impl pallet_subspace::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
    type WeightInfo = ();
    type PalletId = SubspacePalletId;
}

impl pallet_governance::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
    type DefaultProposalCost = DefaultProposalCost;
}

impl GovernanceApi<<Test as frame_system::Config>::AccountId> for Test {
    fn is_delegating_voting_power(_delegator: &AccountId) -> bool {
        false
    }

    fn update_delegating_voting_power(_delegator: &AccountId, _delegating: bool) -> DispatchResult {
        Ok(())
    }

    fn get_global_governance_configuration() -> GovernanceConfiguration {
        Default::default()
    }

    fn get_subnet_governance_configuration(_subnet_id: u16) -> GovernanceConfiguration {
        Default::default()
    }

    fn update_global_governance_configuration(
        governance_config: GovernanceConfiguration,
    ) -> DispatchResult {
        Governance::update_global_governance_configuration(governance_config)
    }

    fn update_subnet_governance_configuration(
        _subnet_id: u16,
        _governance_config: GovernanceConfiguration,
    ) -> DispatchResult {
        Ok(())
    }

    fn handle_subnet_removal(_subnet_id: u16) {}
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
    sp_tracing::try_init_simple();
    <frame_system::GenesisConfig<Test> as BuildStorage>::build_storage(&Default::default())
        .unwrap()
        .into()
}

pub fn step_block(n: usize) {
    for _ in 0..n {
        Governance::on_finalize(System::block_number());
        System::on_finalize(System::block_number());
        System::set_block_number(System::block_number() + 1);
        System::on_initialize(System::block_number());
        Governance::on_initialize(System::block_number());
    }
}

pub fn get_origin(key: AccountId) -> RuntimeOrigin {
    <<Test as frame_system::Config>::RuntimeOrigin>::signed(key)
}

const TOKEN_DECIMALS: u32 = 9;

pub const fn to_nano(x: Balance) -> Balance {
    x * 10u64.pow(TOKEN_DECIMALS)
}

pub const fn from_nano(x: Balance) -> Balance {
    x / 10u64.pow(TOKEN_DECIMALS)
}

pub fn add_balance(key: AccountId, amount: Balance) {
    let _ = <Balances as Currency<AccountId>>::deposit_creating(&key, amount);
}

pub fn get_balance(key: AccountId) -> Balance {
    <Balances as Currency<AccountId>>::free_balance(&key)
}

pub fn zero_min_burn() {
    BurnConfig::<Test>::mutate(|cfg| cfg.min_burn = 0);
}
