extern crate alloc;

use alloc::vec::Vec;
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use fp_evm::{ExitError, ExitSucceed, LinearCostPrecompile, PrecompileFailure};

use crate::precompiles::get_slice;

pub const ED25519_PRECOMPILE_INDEX: u64 = 3000;

type PrecompileResult<T> = Result<T, PrecompileFailure>;
type VerificationResult = Result<bool, &'static str>;

pub struct Ed25519VerifyPrecompile;

impl LinearCostPrecompile for Ed25519VerifyPrecompile {
    const BASE: u64 = 15;
    const WORD: u64 = 3;

    fn execute(input: &[u8], _: u64) -> PrecompileResult<(ExitSucceed, Vec<u8>)> {
        let required_len = 132;
        if input.len() < required_len {
            return Err(PrecompileFailure::Error {
                exit_status: ExitError::Other("input must contain 128 bytes".into()),
            });
        }

        let verification_result =
            verify_signature(input).map_err(|e| PrecompileFailure::Error {
                exit_status: ExitError::Other(e.into()),
            })?;

        let mut result = [0u8; 32];
        result[31] = u8::from(verification_result);

        Ok((ExitSucceed::Returned, result.to_vec()))
    }
}

fn verify_signature(input: &[u8]) -> VerificationResult {
    let msg = get_slice(input, 4, 36).map_err(|_| "Failed to get message slice")?;

    let public_key = get_slice(input, 36, 68)
        .map_err(|_| "Failed to get public key slice")
        .and_then(|pk| VerifyingKey::try_from(pk).map_err(|_| "Public key recover failed"))?;

    let signature = get_slice(input, 68, 132)
        .map_err(|_| "Failed to get signature slice")
        .and_then(|sig| Signature::try_from(sig).map_err(|_| "Signature recover failed"))?;

    Ok(public_key.verify(msg, &signature).is_ok())
}
