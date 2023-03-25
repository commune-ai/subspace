//! RPC interface for the custom Subspace rpc methods

use jsonrpsee::{
	core::RpcResult,
	proc_macros::rpc,
	types::error::{CallError, ErrorObject},
};
use sp_blockchain::HeaderBackend;
use sp_runtime::{
	generic::BlockId,
	traits::{Block as BlockT}
};
use std::sync::Arc;


use sp_api::ProvideRuntimeApi;

pub use subspace_custom_rpc_runtime_api::NeuronInfoRuntimeApi;
pub use subspace_custom_rpc_runtime_api::SubnetInfoRuntimeApi;

#[rpc(client, server)]
pub trait SubspaceCustomApi<BlockHash> {

	#[method(name = "neuronInfo_getNeurons")]
	fn get_neurons(&self, netuid: u16, at: Option<BlockHash>) -> RpcResult<Vec<u8>>;
	#[method(name = "neuronInfo_getNeuron")]
	fn get_neuron(&self, netuid: u16, uid: u16, at: Option<BlockHash>) -> RpcResult<Vec<u8>>;

	#[method(name = "subnetInfo_getSubnetInfo")]
	fn get_subnet_info(&self, netuid: u16, at: Option<BlockHash>) -> RpcResult<Vec<u8>>;
	#[method(name = "subnetInfo_getSubnetsInfo")]
	fn get_subnets_info(&self, at: Option<BlockHash>) -> RpcResult<Vec<u8>>;
}

pub struct SubspaceCustom<C, P> {
	/// Shared reference to the client.
	client: Arc<C>,
	_marker: std::marker::PhantomData<P>,
}

impl<C, P> SubspaceCustom<C, P> {
	/// Creates a new instance of the TransactionPayment Rpc helper.
	pub fn new(client: Arc<C>) -> Self {
		Self { client, _marker: Default::default() }
	}
}

/// Error type of this RPC api.
pub enum Error {
	/// The call to runtime failed.
	RuntimeError,
}

impl From<Error> for i32 {
	fn from(e: Error) -> i32 {
		match e {
			Error::RuntimeError => 1,
		}
	}
}

impl<C, Block> SubspaceCustomApiServer<<Block as BlockT>::Hash> for SubspaceCustom<C, Block>
where
	Block: BlockT,
	C: ProvideRuntimeApi<Block> + HeaderBackend<Block> + Send + Sync + 'static,
	C::Api: NeuronInfoRuntimeApi<Block>,
	C::Api: SubnetInfoRuntimeApi<Block>,
	{ 






	fn get_neurons(
		&self,
		netuid: u16,
		at: Option<<Block as BlockT>::Hash>
	) -> RpcResult<Vec<u8>> {
		let api = self.client.runtime_api();
		let at = at.unwrap_or_else(|| self.client.info().best_hash);

		api.get_neurons(at, netuid).map_err(|e| {
			CallError::Custom(ErrorObject::owned(
				Error::RuntimeError.into(),
				"Unable to get neurons info.",
				Some(e.to_string()),
			)).into()
		})
	}

	fn get_neuron(
		&self,
		netuid: u16,
		uid: u16, at: Option<<Block as BlockT>::Hash>
	) -> RpcResult<Vec<u8>> {
		let api = self.client.runtime_api();
		let at = at.unwrap_or_else(|| self.client.info().best_hash);

		api.get_neuron(at, netuid, uid).map_err(|e| {
			CallError::Custom(ErrorObject::owned(
				Error::RuntimeError.into(),
				"Unable to get neuron info.",
				Some(e.to_string()),
			)).into()
		})
	}
	
	fn get_subnet_info(&self, netuid: u16, at: Option<<Block as BlockT>::Hash>) -> RpcResult<Vec<u8>> {
		let api = self.client.runtime_api();
		let at = at.unwrap_or_else(|| self.client.info().best_hash);

		api.get_subnet_info(at, netuid).map_err(|e| {
			CallError::Custom(ErrorObject::owned(
				Error::RuntimeError.into(),
				"Unable to get subnet info.",
				Some(e.to_string()),
			)).into()
		})
	}

	fn get_subnets_info(
		&self,
		at: Option<<Block as BlockT>::Hash>
	) -> RpcResult<Vec<u8>> {
		let api = self.client.runtime_api();
		let at = at.unwrap_or_else(|| self.client.info().best_hash);

		api.get_subnets_info(at).map_err(|e| {
			CallError::Custom(ErrorObject::owned(
			Error::RuntimeError.into(),
			"Unable to get subnets info.",
			Some(e.to_string()),
			)).into()
		})
	}
}
