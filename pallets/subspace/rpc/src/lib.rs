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
use subspace_runtime_api::ModuleInfo;
pub use subspace_runtime_api::SubspaceRuntimeApi;

type Signature = MultiSignature;
type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;

#[derive(serde::Deserialize, serde::Serialize)]
pub struct Custom {
    code: u32,
    burn_rate: u16,
}

#[rpc(client, server)]
pub trait SubspaceApi<BlockHash, AccountId> {
    #[method(name = "subspace_getBurnRate")]
    fn get_burn_rate(&self, at: Option<BlockHash>) -> RpcResult<Custom>;

    #[method(name = "subspace_getModuleInfo")]
    fn get_module_info(
        &self,
        key: AccountId,
        netuid: u16,
        at: Option<BlockHash>,
    ) -> RpcResult<ModuleInfo>;
}

pub struct SubspacePallet<C, Block> {
    client: Arc<C>,
    _marker: std::marker::PhantomData<Block>,
}

impl<C, Block> SubspacePallet<C, Block> {
    pub fn new(client: Arc<C>) -> Self {
        Self { client, _marker: Default::default() }
    }
}

impl<C, Block> SubspaceApiServer<<Block as BlockT>::Hash, AccountId> for SubspacePallet<C, Block>
where
    Block: BlockT,
    C: Send + Sync + 'static + ProvideRuntimeApi<Block> + HeaderBackend<Block>,
    C::Api: SubspaceRuntimeApi<Block>,
{
    fn get_burn_rate(&self, at: Option<<Block as BlockT>::Hash>) -> RpcResult<Custom> {
        let api = self.client.runtime_api();
        let at = at.unwrap_or_else(|| self.client.info().best_hash);

        let value = api.get_burn_rate(at).map_err(runtime_error_into_rpc_err);
        Ok(Custom { code: 200, burn_rate: value.unwrap() })
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
