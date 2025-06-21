use frame_support::{dispatch::DispatchResult, parameter_types, traits::ConstU32};
use frame_system as system;
use pallet_balances;
use pallet_governance_api::{GovernanceApi, GovernanceConfiguration};
use pallet_subnet_emission_api::{SubnetConsensus, SubnetEmissionApi};
use pallet_subspace;
use sp_core::H256;
use sp_runtime::{
    traits::{AccountIdConversion, BlakeTwo256, IdentityLookup},
    BuildStorage,
};

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
    pub const BlockHashCount: u64 = 250;
    pub BlockWeights: frame_system::limits::BlockWeights =
        frame_system::limits::BlockWeights::simple_max(1024.into());
}

impl system::Config for Test {
    type BaseCallFilter = frame_support::traits::Everything;
    type BlockWeights = BlockWeights;
    type BlockLength = ();
    type DbWeight = ();
    type RuntimeOrigin = RuntimeOrigin;
    type RuntimeCall = RuntimeCall;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type AccountId = u64;
    type Lookup = IdentityLookup<Self::AccountId>;
    type RuntimeEvent = RuntimeEvent;
    type BlockHashCount = BlockHashCount;
    type Version = ();
    type PalletInfo = PalletInfo;
    type AccountData = pallet_balances::AccountData<u64>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type SS58Prefix = ();
    type OnSetCode = ();
    type MaxConsumers = ConstU32<16>;
    type RuntimeTask = ();
    type Nonce = u32;
    type Block = Block;
    type SingleBlockMigrations = ();
    type MultiBlockMigrator = ();
    type PreInherents = ();
    type PostInherents = ();
    type PostTransactions = ();
}

parameter_types! {
    pub const ExistentialDeposit: u64 = 1;
    pub const MaxLocks: u32 = 50;
}

impl pallet_balances::Config for Test {
    type MaxLocks = MaxLocks;
    type MaxReserves = ();
    type ReserveIdentifier = [u8; 8];
    type Balance = u64;
    type RuntimeEvent = RuntimeEvent;
    type DustRemoval = ();
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
    type WeightInfo = ();
    type FreezeIdentifier = ();
    type MaxFreezes = ();
    type RuntimeFreezeReason = ();
    type RuntimeHoldReason = ();
}

parameter_types! {
    pub const GovernancePalletId: frame_support::PalletId = frame_support::PalletId(*b"gov/trea");
}

parameter_types! {
    pub const SubspacePalletId: frame_support::PalletId = frame_support::PalletId(*b"sub/spce");
    pub const DefaultMaxRegistrationsPerInterval: u16 = 10;
    pub const DefaultMaxSubnetRegistrationsPerInterval: u16 = 5;
    pub const DefaultModuleMinBurn: u64 = 1000;
    pub const DefaultSubnetMinBurn: u64 = 5000;
    pub const DefaultMinValidatorStake: u64 = 10000;
}

impl pallet_subspace::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
    type WeightInfo = ();
    type PalletId = SubspacePalletId;
    type DefaultMaxRegistrationsPerInterval = DefaultMaxRegistrationsPerInterval;
    type DefaultMaxSubnetRegistrationsPerInterval = DefaultMaxSubnetRegistrationsPerInterval;
    type DefaultModuleMinBurn = DefaultModuleMinBurn;
    type DefaultSubnetMinBurn = DefaultSubnetMinBurn;
    type DefaultMinValidatorStake = DefaultMinValidatorStake;
    type EnforceWhitelist = frame_support::traits::ConstBool<false>;
    type DefaultUseWeightsEncryption = frame_support::traits::ConstBool<false>;
}

impl pallet_governance::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
    type PalletId = GovernancePalletId;
    type WeightInfo = ();
}

pub type BlockNumber = u64;

pub fn new_test_ext() -> sp_io::TestExternalities {
    let mut t = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap();

    // Get the treasury account ID from the PalletId
    let treasury = GovernancePalletId::get().into_account_truncating();

    pallet_balances::GenesisConfig::<Test> {
        balances: vec![
            (1, 100_000),
            (2, 100_000),
            (treasury, 1_000_000), // Ensure treasury has enough funds for payments
        ],
    }
    .assimilate_storage(&mut t)
    .unwrap();

    let mut ext = sp_io::TestExternalities::new(t);
    ext.execute_with(|| System::set_block_number(1));
    ext
}

impl SubnetEmissionApi<u64> for Test {
    fn get_lowest_emission_netuid(_ignore_subnet_immunity: bool) -> Option<u16> {
        Some(0)
    }

    fn set_subnet_emission_storage(_netuid: u16, _emission: u64) {}

    fn create_yuma_subnet(_netuid: u16) {}

    fn can_remove_subnet(_netuid: u16) -> bool {
        true
    }

    fn is_mineable_subnet(_netuid: u16) -> bool {
        true
    }

    fn get_consensus_netuid(_subnet_consensus: SubnetConsensus) -> Option<u16> {
        Some(0)
    }

    fn get_subnet_consensus_type(_netuid: u16) -> Option<SubnetConsensus> {
        Some(SubnetConsensus::Root)
    }

    fn set_subnet_consensus_type(_netuid: u16, _subnet_consensus: Option<SubnetConsensus>) {}

    fn get_weights(_netuid: u16, _uid: u16) -> Option<Vec<(u16, u16)>> {
        Some(vec![])
    }

    fn set_weights(
        _netuid: u16,
        _uid: u16,
        _weights: Option<Vec<(u16, u16)>>,
    ) -> Option<Vec<(u16, u16)>> {
        Some(vec![])
    }

    fn clear_subnet_includes(_netuid: u16) {}

    fn clear_module_includes(
        _netuid: u16,
        _uid: u16,
        _replace_uid: u16,
        _module_key: &u64,
        _replace_key: &u64,
    ) -> DispatchResult {
        Ok(())
    }
}

impl GovernanceApi<u64> for Test {
    fn get_dao_treasury_address() -> u64 {
        1
    }

    fn get_global_governance_configuration() -> GovernanceConfiguration {
        GovernanceConfiguration::default()
    }

    fn get_subnet_governance_configuration(_subnet_id: u16) -> GovernanceConfiguration {
        GovernanceConfiguration::default()
    }

    fn update_global_governance_configuration(
        _config: GovernanceConfiguration,
    ) -> Result<(), sp_runtime::DispatchError> {
        Ok(())
    }

    fn update_subnet_governance_configuration(
        _subnet_id: u16,
        _config: GovernanceConfiguration,
    ) -> Result<(), sp_runtime::DispatchError> {
        Ok(())
    }

    fn is_delegating_voting_power(_account: &u64) -> bool {
        false
    }

    fn update_delegating_voting_power(
        _account: &u64,
        _delegating: bool,
    ) -> Result<(), sp_runtime::DispatchError> {
        Ok(())
    }

    fn execute_application(_account: &u64) -> Result<(), sp_runtime::DispatchError> {
        Ok(())
    }

    fn get_general_subnet_application_cost() -> u64 {
        1000
    }

    fn curator_application_exists(_account: &u64) -> bool {
        false
    }

    fn whitelisted_keys() -> std::collections::BTreeSet<u64> {
        std::collections::BTreeSet::new()
    }

    fn get_curator() -> u64 {
        1
    }

    fn set_curator(_account: &u64) {}

    fn set_general_subnet_application_cost(_cost: u64) {}

    fn clear_subnet_includes(_subnet_id: u16) {}
}
