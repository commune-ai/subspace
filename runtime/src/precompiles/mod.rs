use core::marker::PhantomData;
use sp_core::{hashing::keccak_256, H160};
use sp_runtime::AccountId32;

use pallet_evm::{
    ExitError, IsPrecompileResult, Precompile, PrecompileFailure, PrecompileHandle,
    PrecompileResult, PrecompileSet,
};
use pallet_evm_precompile_modexp::Modexp;
use pallet_evm_precompile_sha3fips::Sha3FIPS256;
use pallet_evm_precompile_simple::{ECRecover, ECRecoverPublicKey, Identity, Ripemd160, Sha256};

mod balance_transfer;
mod ed25519;
mod staking;

use balance_transfer::{BalanceTransferPrecompile, BALANCE_TRANSFER_INDEX};
use ed25519::{Ed25519VerifyPrecompile as Ed25519Verify, ED25519_PRECOMPILE_INDEX};
use staking::{StakingPrecompile, STAKING_PRECOMPILE_INDEX};

type PrecompileAddress = H160;
type AccountConversionResult = Result<AccountId32, PrecompileFailure>;
type SliceResult<'a> = Result<&'a [u8], PrecompileFailure>;

const ECRECOVER_ADDRESS: u64 = 1;
const SHA256_ADDRESS: u64 = 2;
const RIPEMD160_ADDRESS: u64 = 3;
const IDENTITY_ADDRESS: u64 = 4;
const MODEXP_ADDRESS: u64 = 5;
const SHA3FIPS256_ADDRESS: u64 = 1024;
const ECRECOVER_PUBKEY_ADDRESS: u64 = 1025;

pub struct FrontierPrecompiles<R>(PhantomData<R>);

impl<R> Default for FrontierPrecompiles<R>
where
    R: pallet_evm::Config,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<R> FrontierPrecompiles<R>
where
    R: pallet_evm::Config,
{
    pub fn new() -> Self {
        Self(Default::default())
    }

    pub fn used_addresses() -> [PrecompileAddress; 10] {
        [
            hash(ECRECOVER_ADDRESS),
            hash(SHA256_ADDRESS),
            hash(RIPEMD160_ADDRESS),
            hash(IDENTITY_ADDRESS),
            hash(MODEXP_ADDRESS),
            hash(SHA3FIPS256_ADDRESS),
            hash(ECRECOVER_PUBKEY_ADDRESS),
            hash(ED25519_PRECOMPILE_INDEX),
            hash(BALANCE_TRANSFER_INDEX),
            hash(STAKING_PRECOMPILE_INDEX),
        ]
    }

    fn match_precompile(
        address: PrecompileAddress,
        handle: &mut impl PrecompileHandle,
    ) -> Option<PrecompileResult> {
        match address {
            // Ethereum precompiles
            a if a == hash(ECRECOVER_ADDRESS) => Some(ECRecover::execute(handle)),
            a if a == hash(SHA256_ADDRESS) => Some(Sha256::execute(handle)),
            a if a == hash(RIPEMD160_ADDRESS) => Some(Ripemd160::execute(handle)),
            a if a == hash(IDENTITY_ADDRESS) => Some(Identity::execute(handle)),
            a if a == hash(MODEXP_ADDRESS) => Some(Modexp::execute(handle)),
            // Additional precompiles
            a if a == hash(SHA3FIPS256_ADDRESS) => Some(Sha3FIPS256::execute(handle)),
            a if a == hash(ECRECOVER_PUBKEY_ADDRESS) => Some(ECRecoverPublicKey::execute(handle)),
            a if a == hash(ED25519_PRECOMPILE_INDEX) => Some(Ed25519Verify::execute(handle)),
            // Custom precompiles
            a if a == hash(BALANCE_TRANSFER_INDEX) => {
                Some(BalanceTransferPrecompile::execute(handle))
            }
            a if a == hash(STAKING_PRECOMPILE_INDEX) => Some(StakingPrecompile::execute(handle)),
            _ => None,
        }
    }
}

impl<R> PrecompileSet for FrontierPrecompiles<R>
where
    R: pallet_evm::Config,
{
    fn execute(&self, handle: &mut impl PrecompileHandle) -> Option<PrecompileResult> {
        Self::match_precompile(handle.code_address(), handle)
    }

    fn is_precompile(&self, address: PrecompileAddress, _gas: u64) -> IsPrecompileResult {
        IsPrecompileResult::Answer {
            is_precompile: Self::used_addresses().contains(&address),
            extra_cost: 0,
        }
    }
}

fn hash(a: u64) -> PrecompileAddress {
    PrecompileAddress::from_low_u64_be(a)
}

pub fn get_method_id(method_signature: &str) -> [u8; 4] {
    let hash = keccak_256(method_signature.as_bytes());
    hash[..4].try_into().expect("slice will always be 4 bytes")
}

pub fn bytes_to_account_id(account_id_bytes: &[u8]) -> AccountConversionResult {
    AccountId32::try_from(account_id_bytes).map_err(|_| {
        log::info!("Error parsing account id bytes {:?}", account_id_bytes);
        PrecompileFailure::Error {
            exit_status: ExitError::InvalidRange,
        }
    })
}

pub fn get_slice(data: &[u8], from: usize, to: usize) -> SliceResult {
    data.get(from..to).ok_or(PrecompileFailure::Error {
        exit_status: ExitError::InvalidRange,
    })
}
