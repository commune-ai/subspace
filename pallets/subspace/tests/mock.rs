#![allow(non_camel_case_types)]

use frame_support::{
    assert_ok, parameter_types,
    traits::{Everything, Hooks},
    PalletId,
};
use frame_system as system;
use pallet_governance_api::*;
use pallet_subspace::{
    Address, BurnConfig, Dividends, Emission, Incentive, LastUpdate, MaxRegistrationsPerBlock,
    MaxRegistrationsPerInterval, Name, Stake, Tempo, N,
};
use sp_core::{H256, U256};
use sp_runtime::{
    traits::{AccountIdConversion, BlakeTwo256, IdentityLookup},
    BuildStorage, DispatchResult, Percent,
};

use log::info;

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

#[allow(dead_code)]
pub type AccountId = U256;

// Balance of an account.
pub type Balance = u64;

// An index to a block.
#[allow(dead_code)]
pub type BlockNumber = u64;

parameter_types! {
    pub const SS58Prefix: u8 = 42;
    pub const BlockHashCount: u64 = 250;
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
    type MaxFreezes = frame_support::traits::ConstU32<16>;
    type RuntimeFreezeReason = ();
}

impl system::Config for Test {
    type BaseCallFilter = Everything;
    type Block = Block;
    type BlockWeights = ();
    type BlockLength = ();
    type AccountId = U256;
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
}

impl pallet_subspace::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
    type WeightInfo = ();
    type PalletId = SubspacePalletId;
}

impl GovernanceApi<<Test as frame_system::Config>::AccountId> for Test {
    fn get_dao_treasury_address() -> AccountId {
        SubspacePalletId::get().into_account_truncating()
    }

    fn get_dao_treasury_distribution() -> Percent {
        Percent::from_percent(50u8)
    }

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
        _governance_config: GovernanceConfiguration,
    ) -> DispatchResult {
        Ok(())
    }

    fn update_subnet_governance_configuration(
        _subnet_id: u16,
        _governance_config: GovernanceConfiguration,
    ) -> DispatchResult {
        Ok(())
    }

    fn handle_subnet_removal(_subnet_id: u16) {}

    fn execute_application(_user_id: &AccountId) -> DispatchResult {
        Ok(())
    }

    fn get_general_subnet_application_cost() -> u64 {
        to_nano(1_000)
    }

    fn curator_application_exists(_module_key: &<Test as frame_system::Config>::AccountId) -> bool {
        false
    }

    fn get_curator() -> <Test as frame_system::Config>::AccountId {
        AccountId::default()
    }

    fn set_curator(_key: &<Test as frame_system::Config>::AccountId) {}

    fn set_general_subnet_application_cost(_amount: u64) {}
}

#[allow(dead_code)]
pub fn set_weights(netuid: u16, key: U256, uids: Vec<u16>, values: Vec<u16>) {
    SubspaceModule::set_weights(get_origin(key), netuid, uids.clone(), values.clone()).unwrap();
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
    sp_tracing::try_init_simple();
    frame_system::GenesisConfig::<Test>::default().build_storage().unwrap().into()
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

#[allow(dead_code)]
pub(crate) fn step_epoch(netuid: u16) {
    let tempo: u16 = Tempo::<Test>::get(netuid);
    step_block(tempo);
}

#[allow(dead_code)]
pub(crate) fn block_number() -> u64 {
    System::block_number()
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
    <<Test as frame_system::Config>::RuntimeOrigin>::signed(key)
}

#[allow(dead_code)]
pub fn register_n_modules(netuid: u16, n: u16, stake: u64) {
    for i in 0..n {
        register_module(netuid, U256::from(i), stake).unwrap_or_else(|_| {
            panic!("register module failed for netuid: {netuid:?} key: {i:?} stake: {stake:?}")
        })
    }
}

pub fn register_module(netuid: u16, key: U256, stake: u64) -> DispatchResult {
    let mut network: Vec<u8> = "test".as_bytes().to_vec();
    network.extend(netuid.to_string().as_bytes().to_vec());

    let mut name: Vec<u8> = "module".as_bytes().to_vec();
    name.extend(key.to_string().as_bytes().to_vec());

    let address: Vec<u8> = "0.0.0.0:30333".as_bytes().to_vec();

    let origin = get_origin(key);

    add_balance(key, stake + 1);

    let result = SubspaceModule::register(origin, network, name, address, stake, key, None);
    MaxRegistrationsPerInterval::<Test>::set(netuid, 1000);
    result
}

#[allow(dead_code)]
pub fn check_subnet_storage(netuid: u16) -> bool {
    let n = N::<Test>::get(netuid);
    let uids = SubspaceModule::get_uids(netuid);
    let keys = SubspaceModule::get_keys(netuid);
    let names = SubspaceModule::get_names(netuid);
    let addresses = SubspaceModule::get_addresses(netuid);
    let emissions = Emission::<Test>::get(netuid);
    let incentives = Incentive::<Test>::get(netuid);
    let dividends = Dividends::<Test>::get(netuid);
    let last_update = LastUpdate::<Test>::get(netuid);

    if (n as usize) != uids.len() {
        return false;
    }
    if (n as usize) != keys.len() {
        return false;
    }
    if (n as usize) != names.len() {
        return false;
    }
    if (n as usize) != addresses.len() {
        return false;
    }
    if (n as usize) != emissions.len() {
        return false;
    }
    if (n as usize) != incentives.len() {
        return false;
    }
    if (n as usize) != dividends.len() {
        return false;
    }
    if (n as usize) != last_update.len() {
        return false;
    }

    // length of addresss
    let name_vector: Vec<Vec<u8>> = Name::<Test>::iter_prefix_values(netuid).collect();
    if (n as usize) != name_vector.len() {
        return false;
    }

    // length of addresss
    let address_vector: Vec<Vec<u8>> = Address::<Test>::iter_prefix_values(netuid).collect();
    if (n as usize) != address_vector.len() {
        return false;
    }

    true
}

#[allow(dead_code)]
pub fn get_stake_for_uid(netuid: u16, module_uid: u16) -> u64 {
    let Some(key) = SubspaceModule::get_key_for_uid(netuid, module_uid) else {
        return 0;
    };
    Stake::<Test>::get(netuid, key)
}

#[allow(dead_code)]
pub fn get_emission_for_key(netuid: u16, key: &AccountId) -> u64 {
    let uid = SubspaceModule::get_uid_for_key(netuid, key);
    SubspaceModule::get_emission_for_uid(netuid, uid)
}

#[allow(dead_code)]
pub fn get_stakes(netuid: u16) -> Vec<u64> {
    SubspaceModule::get_uid_key_tuples(netuid)
        .into_iter()
        .map(|(_, key)| SubspaceModule::get_stake(netuid, &key))
        .collect()
}

#[allow(dead_code)]
pub fn get_total_subnet_balance(netuid: u16) -> u64 {
    let keys = SubspaceModule::get_keys(netuid);
    keys.iter().map(SubspaceModule::get_balance_u64).sum()
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

    let origin = get_origin(key);
    let is_new_subnet: bool = !SubspaceModule::if_subnet_exist(netuid);
    if is_new_subnet {
        MaxRegistrationsPerBlock::<Test>::set(1000)
    }

    let balance = SubspaceModule::get_balance(&key);

    if stake >= balance {
        add_balance(key, stake + 1);
    }
    info!("Registering module: network: {network:?}, key: {module_key:?} stake {balance:?}",);

    let result = SubspaceModule::register(
        origin,
        network,
        name.clone(),
        address,
        stake,
        module_key,
        None,
    );

    log::info!("Register ok module: network: {name:?}, module_key: {module_key:?} key: {key:?}",);

    result
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

    let result = SubspaceModule::register(origin, network, name, address, stake, key, None);
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
const TOKEN_DECIMALS: u32 = 9;

#[allow(dead_code)]
pub const fn to_nano(x: u64) -> u64 {
    x * 10u64.pow(TOKEN_DECIMALS)
}

#[allow(dead_code)]
pub const fn from_nano(x: u64) -> u64 {
    x / 10u64.pow(TOKEN_DECIMALS)
}

#[allow(dead_code)]
pub fn round_first_five(num: u64) -> u64 {
    let place_value = 10_u64.pow(num.to_string().len() as u32 - 5);
    let first_five = num / place_value;

    if first_five % 10 >= 5 {
        (first_five / 10 + 1) * place_value * 10
    } else {
        (first_five / 10) * place_value * 10
    }
}

#[allow(dead_code)]
pub fn zero_min_burn() {
    BurnConfig::<Test>::mutate(|cfg| cfg.min_burn = 0);
}

#[macro_export]
macro_rules! update_params {
    ($netuid:expr => {$($f:ident:$v:expr),+}) => {{
        let params = ::pallet_subspace::SubnetParams {
            $($f: $v),+,
            ..SubspaceModule::subnet_params($netuid)
        };
        ::pallet_subspace::subnet::SubnetChangeset::<Test>::update($netuid, params).unwrap().apply($netuid).unwrap();
    }};
    ($netuid:expr => $params:expr) => {{
        ::pallet_subspace::subnet::SubnetChangeset::<Test>::update($netuid, $params).unwrap().apply($netuid).unwrap();
    }};
}
