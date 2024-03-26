use jsonrpsee::{
    core::{Error as JsonRpseeError, RpcResult},
    proc_macros::rpc,
    types::error::{CallError, ErrorObject},
};
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_runtime::{
    traits::{Block as BlockT, IdentifyAccount, Verify},
    MultiSignature,
};
use std::sync::Arc;
pub use subspace_runtime_api::SubspaceRuntimeApi;
use subspace_runtime_api::{GlobalInfo, KeyInfo, ModuleInfo, SubnetInfo};

type Signature = MultiSignature;
type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;

#[rpc(client, server)]
pub trait SubspaceApi<BlockHash, AccountId> {
    #[method(name = "subspace_getGlobalInfo")]
    fn get_global_info(&self, at: Option<BlockHash>) -> RpcResult<GlobalInfo>;

    #[method(name = "subspace_getSubnetInfo")]
    fn get_subnet_info(&self, netuid: u16, at: Option<BlockHash>) -> RpcResult<SubnetInfo>;

    #[method(name = "subspace_getModuleInfo")]
    fn get_module_info(
        &self,
        key: AccountId,
        netuid: u16,
        at: Option<BlockHash>,
    ) -> RpcResult<ModuleInfo>;

    #[method(name = "subspace_getKeyInfo")]
    fn get_key_info(&self, key: AccountId, at: Option<BlockHash>) -> RpcResult<KeyInfo>;
}

pub struct SubspacePallet<C, Block> {
    client: Arc<C>,
    _marker: std::marker::PhantomData<Block>,
}

impl<C, Block> SubspacePallet<C, Block> {
    pub fn new(client: Arc<C>) -> Self {
        Self {
            client,
            _marker: Default::default(),
        }
    }
}

impl<C, Block> SubspaceApiServer<<Block as BlockT>::Hash, AccountId> for SubspacePallet<C, Block>
where
    Block: BlockT,
    C: Send + Sync + 'static + ProvideRuntimeApi<Block> + HeaderBackend<Block>,
    C::Api: SubspaceRuntimeApi<Block>,
{
    fn get_global_info(&self, at: Option<<Block as BlockT>::Hash>) -> RpcResult<GlobalInfo> {
        let api = self.client.runtime_api();
        let at = at.unwrap_or_else(|| self.client.info().best_hash);

        let value = api.get_global_info(at).map_err(runtime_error_into_rpc_err);
        Ok(value.unwrap())
    }

    fn get_subnet_info(
        &self,
        netuid: u16,
        at: Option<<Block as BlockT>::Hash>,
    ) -> RpcResult<SubnetInfo> {
        let api = self.client.runtime_api();
        let at = at.unwrap_or_else(|| self.client.info().best_hash);

        let value = api.get_subnet_info(at, netuid).map_err(runtime_error_into_rpc_err);
        Ok(value.unwrap())
    }

    fn get_module_info(
        &self,
        key: AccountId,
        netuid: u16,
        at: Option<<Block as BlockT>::Hash>,
    ) -> RpcResult<ModuleInfo> {
        let api = self.client.runtime_api();
        let at = at.unwrap_or_else(|| self.client.info().best_hash);

        let value = api.get_module_info(at, key, netuid).map_err(runtime_error_into_rpc_err);
        Ok(value.unwrap())
    }

    fn get_key_info(
        &self,
        key: AccountId,
        at: Option<<Block as BlockT>::Hash>,
    ) -> RpcResult<KeyInfo> {
        let api = self.client.runtime_api();
        let at = at.unwrap_or_else(|| self.client.info().best_hash);

        let value = api.get_key_info(at, key).map_err(runtime_error_into_rpc_err);
        Ok(value.unwrap())
    }
}

const RUNTIME_ERROR: i32 = 1;

/// Converts a runtime trap into an RPC error.
fn runtime_error_into_rpc_err(err: impl std::fmt::Debug) -> JsonRpseeError {
    CallError::Custom(ErrorObject::owned(
        RUNTIME_ERROR,
        "Runtime error",
        Some(format!("{:?}", err)),
    ))
    .into()
}
