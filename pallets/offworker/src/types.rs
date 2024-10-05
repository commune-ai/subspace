use super::*;

pub struct ShouldDecryptResult<T> {
    pub should_decrypt: bool,
    pub delta: I64F64,
    pub simulation_result: ConsensusSimulationResult<T>,
}
