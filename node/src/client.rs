use node_subspace_runtime::opaque::Block;
use sc_executor::WasmExecutor;

/// Full backend.
pub type FullBackend = sc_service::TFullBackend<Block>;
pub type WasmClient =
    sc_service::TFullClient<Block, node_subspace_runtime::RuntimeApi, WasmExecutor<HostFunctions>>;

/// Only enable the benchmarking host functions when we actually want to benchmark.
#[cfg(feature = "runtime-benchmarks")]
pub type HostFunctions = frame_benchmarking::benchmarking::HostFunctions;
/// Otherwise we use empty host functions for ext host functions.
#[cfg(not(feature = "runtime-benchmarks"))]
pub type HostFunctions = sp_io::SubstrateHostFunctions;
