//! Implements support for the pallet_ibc module.
use codec::Decode;
use codec::Encode;
use core::marker::PhantomData;
use pallet_ibc::event::primitive::{ClientId, ClientType, ConnectionId, Height};
use sp_core::H256;
use substrate_subxt::{balances::Balances, module, system::System, Call, Store};
use substrate_subxt_proc_macro::Event;

/// The subset of the `pallet_ibc::Trait` that a client must implement.
#[module]
pub trait Ibc: System + Balances {}

// #[derive(Encode, Store)]
// pub struct ClientStatesV2Store<T: Ibc> {
//     #[store(returns = Vec<u8>)]
//     pub key: Vec<u8>,
//     pub _runtime: PhantomData<T>,
// }
//
// #[derive(Encode, Store)]
// pub struct ConsensusStatesV2Store<T: Ibc> {
//     #[store(returns = Vec<u8>)]
//     pub key: (Vec<u8>, Vec<u8>),
//     pub _runtime: PhantomData<T>,
// }
//
// #[derive(Encode, Store)]
// pub struct ClientStatesStore<T: Ibc> {
//     #[store(returns = pallet_ibc::grandpa::client_state::ClientState)]
//     pub key: H256,
//     pub _runtime: PhantomData<T>,
// }
//
// #[derive(Encode, Store)]
// pub struct ConsensusStatesStore<T: Ibc> {
//     #[store(returns = pallet_ibc::grandpa::consensus_state::ConsensusState)]
//     pub key: (H256, u32),
//     pub _runtime: PhantomData<T>,
// }
//
// #[derive(Encode, Store)]
// pub struct ConnectionsStore<T: Ibc> {
//     #[store(returns = pallet_ibc::ConnectionEnd)]
//     pub key: H256,
//     pub _runtime: PhantomData<T>,
// }
//
// #[derive(Encode, Store)]
// pub struct ChannelsStore<T: Ibc> {
//     #[store(returns = pallet_ibc::ChannelEnd)]
//     pub key: (Vec<u8>, H256),
//     pub _runtime: PhantomData<T>,
// }
//
// #[derive(Encode, Store)]
// pub struct PacketsStore<T: Ibc> {
//     #[store(returns = H256)]
//     pub key: (Vec<u8>, H256, u64),
//     pub _runtime: PhantomData<T>,
// }
//
// #[derive(Encode, Store)]
// pub struct AcknowledgementsStore<T: Ibc> {
//     #[store(returns = H256)]
//     pub key: (Vec<u8>, H256, u64),
//     pub _runtime: PhantomData<T>,
// }
//
// #[derive(Encode, Call)]
// pub struct SubmitDatagramCall<T: Ibc> {
//     pub _runtime: PhantomData<T>,
//     pub datagram: pallet_ibc::Datagram,
// }

#[derive(Clone, Debug, Eq, PartialEq, Event, Decode)]
pub struct CreateClientEvent<T: Ibc> {
    pub _runtime: PhantomData<T>,
    pub height: Height,
    pub client_id: ClientId,
    pub client_type: ClientType,
    pub consensus_height: Height,
}

#[derive(Clone, Debug, Eq, PartialEq, Event, Decode)]
pub struct OpenInitConnectionEvent<T: Ibc> {
    pub _runtime: PhantomData<T>,
    pub height: Height,
    pub connection_id: Option<ConnectionId>,
    pub client_id: ClientId,
    pub counterparty_connection_id: Option<ConnectionId>,
    pub counterparty_client_id: ClientId,
}

#[derive(Encode, Call)]
pub struct DeliverCall<T: Ibc> {
    pub _runtime: PhantomData<T>,
    pub messages: Vec<pallet_ibc::Any>,
    pub tmp: u8,
}
