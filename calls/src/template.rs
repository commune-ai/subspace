//! Implements support for the template module.
use codec::Encode;
use core::marker::PhantomData;
use sp_core::H256;
use sp_finality_grandpa::{AuthorityList, SetId};
use substrate_subxt::{module, system::System, Call};

/// The subset of the `template::Trait` that a client must implement.
#[module]
pub trait TemplateModule: System {}

/// Arguments for creating test client.
#[derive(Encode, Call)]
pub struct TestCreateClientCall<T: TemplateModule> {
    pub _runtime: PhantomData<T>,
    pub identifier: H256,
    pub height: u32,
    pub set_id: SetId,
    pub authority_list: AuthorityList,
    pub root: H256,
}

/// Arguments for opening connection.
#[derive(Encode, Call)]
pub struct TestConnOpenInitCall<T: TemplateModule> {
    pub _runtime: PhantomData<T>,
    pub identifier: H256,
    pub desired_counterparty_connection_identifier: H256,
    pub client_identifier: H256,
    pub counterparty_client_identifier: H256,
}

/// Arguments for binding port.
#[derive(Encode, Call)]
pub struct TestBindPortCall<T: TemplateModule> {
    pub _runtime: PhantomData<T>,
    pub identifier: Vec<u8>,
}

/// Arguments for releasing port.
#[derive(Encode, Call)]
pub struct TestReleasePortCall<T: TemplateModule> {
    pub _runtime: PhantomData<T>,
    pub identifier: Vec<u8>,
}

/// Arguments for opening channel.
#[derive(Encode, Call)]
pub struct TestChanOpenInitCall<T: TemplateModule> {
    pub _runtime: PhantomData<T>,
    pub unordered: bool,
    pub connection_hops: Vec<H256>,
    pub port_identifier: Vec<u8>,
    pub channel_identifier: H256,
    pub counterparty_port_identifier: Vec<u8>,
    pub counterparty_channel_identifier: H256,
}

/// Arguments for sending packet.
#[derive(Encode, Call)]
pub struct TestSendPacketCall<T: TemplateModule> {
    pub _runtime: PhantomData<T>,
    pub sequence: u64,
    pub timeout_height: u32,
    pub source_port: Vec<u8>,
    pub source_channel: H256,
    pub dest_port: Vec<u8>,
    pub dest_channel: H256,
    pub data: Vec<u8>,
}
