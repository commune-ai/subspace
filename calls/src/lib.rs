#![feature(associated_type_bounds)]

use pallet_balances::AccountData;
use pallet_ibc::Event;
use sp_core::H256;
use sp_runtime::generic::Header;
use sp_runtime::traits::{BlakeTwo256, IdentifyAccount, Verify};
use sp_runtime::{MultiSignature, OpaqueExtrinsic};
use substrate_subxt::{
    balances::{Balances, BalancesEventTypeRegistry},
    contracts::{Contracts, ContractsEventTypeRegistry},
    extrinsic::DefaultExtra,
    register_default_type_sizes,
    session::{Session, SessionEventTypeRegistry},
    staking::{Staking, StakingEventTypeRegistry},
    system::{System, SystemEventTypeRegistry},
    BasicSessionKeys, EventTypeRegistry, Runtime,
};

pub mod ibc;
pub mod template;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct NodeRuntime;

impl Runtime for NodeRuntime {
    type Signature = MultiSignature;
    type Extra = DefaultExtra<Self>;

    fn register_type_sizes(event_type_registry: &mut EventTypeRegistry<Self>) {
        event_type_registry.with_system();
        event_type_registry.with_balances();
        event_type_registry.with_staking();
        event_type_registry.with_session();
        event_type_registry.register_type_size::<H256>("H256");
        event_type_registry.register_type_size::<u64>("TAssetBalance");
        event_type_registry.register_type_size::<pallet_ibc::event::primitive::Height>("Height");
        event_type_registry
            .register_type_size::<pallet_ibc::event::primitive::ClientType>("ClientType");
        event_type_registry
            .register_type_size::<pallet_ibc::event::primitive::ClientId>("ClientId");
        event_type_registry
            .register_type_size::<pallet_ibc::event::primitive::ConnectionId>("ConnectionId");
        register_default_type_sizes(event_type_registry);
    }
}

impl System for NodeRuntime {
    type Index = u32;
    type BlockNumber = u32;
    type Hash = sp_core::H256;
    type Hashing = BlakeTwo256;
    type AccountId = <<MultiSignature as Verify>::Signer as IdentifyAccount>::AccountId;
    type Address = sp_runtime::MultiAddress<Self::AccountId, u32>;
    type Header = Header<Self::BlockNumber, BlakeTwo256>;
    type Extrinsic = OpaqueExtrinsic;
    type AccountData = AccountData<<Self as Balances>::Balance>;
}

impl Balances for NodeRuntime {
    type Balance = u128;
}

impl Session for NodeRuntime {
    type ValidatorId = <Self as System>::AccountId;
    type Keys = BasicSessionKeys;
}
impl Staking for NodeRuntime {}

impl Contracts for NodeRuntime {}

impl ibc::Ibc for NodeRuntime {}

impl template::TemplateModule for NodeRuntime {}
