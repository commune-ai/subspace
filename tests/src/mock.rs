#![allow(non_camel_case_types)]

use frame_support::{
    ensure, parameter_types,
    traits::{Currency, Everything, Get, Hooks},
    PalletId,
};
use frame_system as system;
use pallet_governance::GlobalGovernanceConfig;
use pallet_governance_api::*;
use pallet_subnet_emission_api::{SubnetConsensus, SubnetEmissionApi};
use scale_info::prelude::collections::BTreeSet;
use sp_core::{ConstU16, H256};
use std::cell::RefCell;

use pallet_subspace::{
    subnet::SubnetChangeset, Address, BurnConfig, DefaultKey, DefaultSubnetParams, Dividends,
    Emission, Incentive, LastUpdate, MaxRegistrationsPerBlock, Name, SubnetBurn, SubnetBurnConfig,
    SubnetParams, Tempo, TotalStake, N,
};
use sp_runtime::{
    traits::{AccountIdConversion, BlakeTwo256, IdentityLookup},
    BuildStorage, DispatchError, DispatchResult,
};

type Block = frame_system::mocking::MockBlock<Test>;

frame_support::construct_runtime!(
    pub enum Test {
        System: frame_system,
        Balances: pallet_balances,
        SubnetEmissionMod: pallet_subnet_emission,
        SubspaceMod: pallet_subspace,
        GovernanceMod: pallet_governance,
    }
);

#[allow(dead_code)]
pub type BalanceCall = pallet_balances::Call<Test>;

#[allow(dead_code)]
pub type TestRuntimeCall = frame_system::Call<Test>;

parameter_types! {
    pub const BlockHashCount: u64 = 250;
    pub const SS58Prefix: u8 = 42;
}

pub type AccountId = u32;

// Balance of an account.
pub type Balance = u64;

// An index to a block.
#[allow(dead_code)]
pub type BlockNumber = u64;

parameter_types! {
    pub const ExistentialDeposit: Balance = 1;
    pub const MaxLocks: u32 = 50;
    pub const MaxReserves: u32 = 50;
}

pub const PALLET_ID: PalletId = PalletId(*b"py/subsp");

pub struct SubspacePalletId;

impl Get<PalletId> for SubspacePalletId {
    fn get() -> PalletId {
        PALLET_ID
    }
}

impl pallet_subspace::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
    type WeightInfo = ();
    type DefaultMaxRegistrationsPerInterval = ConstU16<{ u16::MAX }>;
    type DefaultMaxSubnetRegistrationsPerInterval = ConstU16<{ u16::MAX }>;
    type PalletId = SubspacePalletId;
}

impl GovernanceApi<<Test as frame_system::Config>::AccountId> for Test {
    fn get_dao_treasury_address() -> AccountId {
        SubspacePalletId::get().into_account_truncating()
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

    fn update_global_governance_configuration(config: GovernanceConfiguration) -> DispatchResult {
        pallet_governance::Pallet::<Test>::update_global_governance_configuration(config)
    }

    fn update_subnet_governance_configuration(
        subnet_id: u16,
        config: GovernanceConfiguration,
    ) -> DispatchResult {
        pallet_governance::Pallet::<Test>::update_subnet_governance_configuration(subnet_id, config)
    }

    fn handle_subnet_removal(subnet_id: u16) {
        pallet_governance::Pallet::<Test>::handle_subnet_removal(subnet_id)
    }

    fn execute_application(user_id: &AccountId) -> DispatchResult {
        pallet_governance::Pallet::<Test>::execute_application(user_id)
    }

    fn get_general_subnet_application_cost() -> u64 {
        to_nano(1_000)
    }

    fn curator_application_exists(module_key: &<Test as frame_system::Config>::AccountId) -> bool {
        pallet_governance::Pallet::<Test>::curator_application_exists(module_key)
    }

    fn whitelisted_keys() -> BTreeSet<AccountId> {
        Default::default()
    }

    fn get_curator() -> <Test as frame_system::Config>::AccountId {
        AccountId::default()
    }

    fn set_curator(_key: &<Test as frame_system::Config>::AccountId) {}

    fn set_general_subnet_application_cost(_amount: u64) {}
}

impl SubnetEmissionApi for Test {
    fn get_unit_emission() -> u64 {
        pallet_subnet_emission::UnitEmission::<Test>::get()
    }

    fn set_unit_emission(unit_emission: u64) {
        pallet_subnet_emission::UnitEmission::<Test>::set(unit_emission);
    }

    fn get_lowest_emission_netuid(ignore_subnet_immunity: bool) -> Option<u16> {
        pallet_subnet_emission::Pallet::<Test>::get_lowest_emission_netuid(ignore_subnet_immunity)
    }

    fn remove_subnet_emission_storage(subnet_id: u16) {
        pallet_subnet_emission::Pallet::<Test>::remove_subnet_emission_storage(subnet_id)
    }

    fn set_subnet_emission_storage(subnet_id: u16, emission: u64) {
        pallet_subnet_emission::Pallet::<Test>::set_subnet_emission_storage(subnet_id, emission)
    }

    fn create_yuma_subnet(netuid: u16) {
        pallet_subnet_emission::Pallet::<Test>::create_yuma_subnet(netuid)
    }

    fn remove_yuma_subnet(netuid: u16) {
        pallet_subnet_emission::Pallet::<Test>::remove_yuma_subnet(netuid)
    }

    fn can_remove_subnet(netuid: u16) -> bool {
        pallet_subnet_emission::Pallet::<Test>::can_remove_subnet(netuid)
    }

    fn is_mineable_subnet(netuid: u16) -> bool {
        pallet_subnet_emission::Pallet::<Test>::is_mineable_subnet(netuid)
    }

    fn get_consensus_netuid(subnet_consensus: SubnetConsensus) -> Option<u16> {
        pallet_subnet_emission::Pallet::<Test>::get_consensus_netuid(subnet_consensus)
    }

    fn get_pending_emission(netuid: u16) -> u64 {
        pallet_subnet_emission::PendingEmission::<Test>::get(netuid)
    }

    fn set_pending_emission(netuid: u16, pending_emission: u64) {
        pallet_subnet_emission::PendingEmission::<Test>::set(netuid, pending_emission);
    }

    fn get_subnet_emission(netuid: u16) -> u64 {
        pallet_subnet_emission::SubnetEmission::<Test>::get(netuid)
    }

    fn set_subnet_emission(netuid: u16, subnet_emission: u64) {
        pallet_subnet_emission::SubnetEmission::<Test>::set(netuid, subnet_emission);
    }

    fn get_subnet_consensus_type(
        netuid: u16,
    ) -> Option<pallet_subnet_emission_api::SubnetConsensus> {
        pallet_subnet_emission::SubnetConsensusType::<Test>::get(netuid)
    }

    fn set_subnet_consensus_type(
        netuid: u16,
        subnet_consensus: Option<pallet_subnet_emission_api::SubnetConsensus>,
    ) {
        pallet_subnet_emission::SubnetConsensusType::<Test>::set(netuid, subnet_consensus)
    }
}

impl pallet_subnet_emission::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
    type Decimals = Decimals;
    type HalvingInterval = HalvingInterval;
    type MaxSupply = MaxSupply;
}

impl pallet_governance::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
    type PalletId = SubspacePalletId;
    type WeightInfo = ();
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

pub struct MockEmissionConfig {
    pub decimals: u8,
    pub halving_interval: u64,
    pub max_supply: u64,
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

impl Default for MockEmissionConfig {
    fn default() -> Self {
        Self {
            decimals: TOKEN_DECIMALS as u8,
            halving_interval: 250_000_000,
            max_supply: 1_000_000_000,
        }
    }
}

pub struct Decimals;
impl Get<u8> for Decimals {
    fn get() -> u8 {
        MOCK_EMISSION_CONFIG.with(|config| config.borrow().decimals)
    }
}

pub struct HalvingInterval;
impl Get<u64> for HalvingInterval {
    fn get() -> u64 {
        MOCK_EMISSION_CONFIG.with(|config| config.borrow().halving_interval)
    }
}

pub struct MaxSupply;
impl Get<u64> for MaxSupply {
    fn get() -> u64 {
        MOCK_EMISSION_CONFIG.with(|config| config.borrow().max_supply)
    }
}

thread_local! {
    static MOCK_EMISSION_CONFIG: RefCell<MockEmissionConfig> = RefCell::new(MockEmissionConfig::default());
}

#[allow(dead_code)]
pub fn set_emission_config(decimals: u8, halving_interval: u64, max_supply: u64) {
    MOCK_EMISSION_CONFIG.with(|config| {
        *config.borrow_mut() = MockEmissionConfig {
            decimals,
            halving_interval,
            max_supply,
        };
    });
}

pub fn add_balance(key: AccountId, amount: Balance) {
    let _ = <Balances as Currency<AccountId>>::deposit_creating(&key, amount);
}

pub fn delegate(account: u32) {
    assert_ok!(GovernanceMod::enable_vote_power_delegation(get_origin(
        account
    )));
}

#[allow(dead_code)]
pub fn get_stakes(netuid: u16) -> Vec<u64> {
    SubspaceMod::get_uid_key_tuples(netuid)
        .into_iter()
        .map(|(_, key)| SubspaceMod::get_delegated_stake(&key))
        .collect()
}

pub fn stake(account: u32, module: u32, stake: u64) {
    if get_balance(account) <= stake {
        add_balance(account, stake + to_nano(1));
    }

    assert_ok!(SubspaceMod::do_add_stake(
        get_origin(account),
        module,
        stake
    ));
}

pub fn increase_stake(key: AccountId, stake: u64) {
    SubspaceMod::increase_stake(&key, &key, stake);
}

pub fn set_total_issuance(total_issuance: u64) {
    let key = DefaultKey::<Test>::get();
    // Reset the issuance (completelly nuke the key's balance)
    <Test as pallet_subspace::Config>::Currency::make_free_balance_be(&key, 0u32.into());
    // Add the total_issuance to the key's balance
    SubspaceMod::add_balance_to_account(&key, total_issuance);
}

pub fn new_test_ext() -> sp_io::TestExternalities {
    sp_tracing::try_init_simple();
    let t = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap();
    let mut ext = sp_io::TestExternalities::new(t);
    ext.execute_with(|| {});
    ext
}

pub fn new_test_ext_with_block(block: u64) -> sp_io::TestExternalities {
    sp_tracing::try_init_simple();
    let t = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap();
    let mut ext = sp_io::TestExternalities::new(t);
    ext.execute_with(|| System::set_block_number(block));
    ext
}

pub fn get_origin(key: AccountId) -> RuntimeOrigin {
    <<Test as frame_system::Config>::RuntimeOrigin>::signed(key)
}

#[allow(dead_code)]
pub fn get_total_subnet_balance(netuid: u16) -> u64 {
    let keys = SubspaceMod::get_keys(netuid);
    keys.iter().map(SubspaceMod::get_balance_u64).sum()
}

#[allow(dead_code)]
pub(crate) fn step_block(n: u16) {
    for _ in 0..n {
        SubspaceMod::on_finalize(System::block_number());
        System::on_finalize(System::block_number());
        System::set_block_number(System::block_number() + 1);
        System::on_initialize(System::block_number());
        SubspaceMod::on_initialize(System::block_number());
        SubnetEmissionMod::on_initialize(System::block_number());
        GovernanceMod::on_initialize(System::block_number());
    }
}

#[allow(dead_code)]
pub(crate) fn run_to_block(n: u64) {
    while System::block_number() < n {
        SubspaceMod::on_finalize(System::block_number());
        System::on_finalize(System::block_number());
        System::set_block_number(System::block_number() + 1);
        System::on_initialize(System::block_number());
        SubspaceMod::on_initialize(System::block_number());
        SubnetEmissionMod::on_initialize(System::block_number());
        GovernanceMod::on_initialize(System::block_number());
    }
}

#[allow(dead_code)]
pub(crate) fn step_epoch(netuid: u16) {
    let tempo: u16 = Tempo::<Test>::get(netuid);
    step_block(tempo);
}

#[allow(dead_code)]
pub fn set_weights(netuid: u16, key: AccountId, uids: Vec<u16>, values: Vec<u16>) {
    SubspaceMod::set_weights(get_origin(key), netuid, uids.clone(), values.clone()).unwrap();
}

#[allow(dead_code)]
pub fn get_stake_for_uid(netuid: u16, module_uid: u16) -> u64 {
    let Some(key) = SubspaceMod::get_key_for_uid(netuid, module_uid) else {
        return 0;
    };
    SubspaceMod::get_delegated_stake(&key)
}

#[allow(dead_code)]
pub fn get_emission_for_key(netuid: u16, key: &AccountId) -> u64 {
    let uid = SubspaceMod::get_uid_for_key(netuid, key).unwrap();
    SubspaceMod::get_emission_for_uid(netuid, uid)
}

pub fn delegate_register_module(
    netuid: u16,
    key: AccountId,
    module_key: AccountId,
    stake: u64,
) -> DispatchResult {
    // can i format the test in rus

    let origin = get_origin(key);
    let network = format!("test{netuid}").as_bytes().to_vec();
    let name = format!("module{module_key}").as_bytes().to_vec();
    let address = "0.0.0.0:30333".as_bytes().to_vec();

    let is_new_subnet = !SubspaceMod::if_subnet_exist(netuid);
    if is_new_subnet {
        MaxRegistrationsPerBlock::<Test>::set(1000)
    }

    let balance = SubspaceMod::get_balance(&key);
    if stake >= balance {
        SubspaceMod::add_balance_to_account(&key, stake + 1);
    }

    let result = SubspaceMod::register(origin, network, name.clone(), address, module_key, None);
    SubspaceMod::increase_stake(&key, &module_key, stake);

    log::info!("Register ok module: network: {name:?}, module_key: {module_key} key: {key}");

    result
}

#[allow(dead_code)]
pub fn check_subnet_storage(netuid: u16) -> bool {
    let n = N::<Test>::get(netuid);
    let uids = SubspaceMod::get_uids(netuid);
    let keys = SubspaceMod::get_keys(netuid);
    let names = SubspaceMod::get_names(netuid);
    let addresses = SubspaceMod::get_addresses(netuid);
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
pub fn register_n_modules(netuid: u16, n: u16, stake: u64, increase_emission: bool) {
    for i in 0..n {
        register_module(netuid, i as u32, stake, increase_emission).unwrap_or_else(|err| {
            panic!(
                "register module failed for netuid: {netuid} key: {i} stake: {stake}. err: {err:?}"
            )
        });
    }
}

pub fn register_named_subnet(key: AccountId, netuid: u16, name: impl ToString) -> DispatchResult {
    ensure!(
        !N::<Test>::contains_key(netuid),
        "subnet id already registered"
    );

    let name = name.to_string().as_bytes().to_vec();
    let params = SubnetParams {
        name: name.try_into().unwrap(),
        founder: key,
        ..DefaultSubnetParams::<Test>::get()
    };

    SubspaceMod::add_subnet(SubnetChangeset::<Test>::new(params).unwrap(), Some(netuid)).unwrap();

    Ok(())
}

pub fn register_subnet(key: AccountId, netuid: u16) -> DispatchResult {
    register_named_subnet(key, netuid, format!("test{netuid}"))
}

#[allow(dead_code)]
pub fn register_module(
    netuid: u16,
    key: AccountId,
    stake: u64,
    increase_emission: bool,
) -> Result<u16, DispatchError> {
    let origin = get_origin(key);

    let network = format!("test{netuid}").as_bytes().to_vec();
    let name = format!("module{key}").as_bytes().to_vec();
    let address = "0.0.0.0:30333".as_bytes().to_vec();

    let _ = register_subnet(key, netuid);

    SubspaceMod::add_balance_to_account(&key, SubnetBurn::<Test>::get() + 1);
    SubspaceMod::register(origin, network.clone(), name, address, key, None)?;
    SubspaceMod::increase_stake(&key, &key, stake);

    let netuid = SubspaceMod::get_netuid_for_name(&network).ok_or("netuid is missing")?;
    let uid = pallet_subspace::Uids::<Test>::get(netuid, key).ok_or("uid is missing")?;

    if increase_emission {
        Emission::<Test>::mutate(netuid, |v| v[uid as usize] = stake);
        pallet_subnet_emission::SubnetEmission::<Test>::mutate(netuid, |s| *s += stake);
    }

    Ok(uid)
}

#[allow(dead_code)]
pub fn register_root_validator(key: AccountId, stake: u64) -> Result<u16, DispatchError> {
    let origin = get_origin(key);
    let network = b"Rootnet".to_vec();
    let name = format!("module{key}").as_bytes().to_vec();
    let address = "0.0.0.0:30333".as_bytes().to_vec();

    SubspaceMod::register(origin, network.clone(), name, address, key, None)?;
    SubspaceMod::increase_stake(&key, &key, stake);

    let netuid = SubspaceMod::get_netuid_for_name(&network).ok_or("netuid is missing")?;
    if netuid != 0 {
        return Err("rootnet id is not 0".into());
    }
    pallet_subspace::Uids::<Test>::get(netuid, key).ok_or("uid is missing".into())
}

pub fn get_balance(key: AccountId) -> Balance {
    <Balances as Currency<AccountId>>::free_balance(&key)
}

pub fn get_total_issuance() -> u64 {
    let total_staked_balance = TotalStake::<Test>::get();
    let total_free_balance = pallet_balances::Pallet::<Test>::total_issuance();
    total_staked_balance + total_free_balance
}

pub fn vote(account: u32, proposal_id: u64, agree: bool) {
    assert_ok!(GovernanceMod::do_vote_proposal(
        get_origin(account),
        proposal_id,
        agree
    ));
}

pub fn config(proposal_cost: u64, proposal_expiration: u32) {
    GlobalGovernanceConfig::<Test>::set(GovernanceConfiguration {
        proposal_cost,
        proposal_expiration,
        vote_mode: pallet_governance_api::VoteMode::Vote,
        ..Default::default()
    });
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
    SubnetBurnConfig::<Test>::mutate(|cfg| cfg.min_burn = 0);
}

macro_rules! update_params {
    ($netuid:expr => { $($f:ident: $v:expr),+ }) => {{
        let params = ::pallet_subspace::SubnetParams {
            $($f: $v),+,
            ..SubspaceMod::subnet_params($netuid)
        };
        ::pallet_subspace::subnet::SubnetChangeset::<Test>::update($netuid, params).unwrap().apply($netuid).unwrap();
    }};
    ($netuid:expr => $params:expr) => {{
        ::pallet_subspace::subnet::SubnetChangeset::<Test>::update($netuid, $params).unwrap().apply($netuid).unwrap();
    }};
}

macro_rules! assert_ok {
    ( $x:expr $(,)? ) => {
        match $x {
            Ok(v) => v,
            is => panic!("Expected Ok(_). Got {is:#?}"),
        }
    };
    ( $x:expr, $y:expr $(,)? ) => {
        assert_eq!($x, Ok($y));
    };
}

macro_rules! assert_in_range {
    ($value:expr, $expected:expr, $margin:expr) => {
        assert!(
            ($expected - $margin..=$expected + $margin).contains(&$value),
            "value {} is out of range {}..={}",
            $value,
            $expected,
            $margin
        );
    };
}

pub(crate) use assert_in_range;
pub(crate) use assert_ok;
pub(crate) use update_params;
