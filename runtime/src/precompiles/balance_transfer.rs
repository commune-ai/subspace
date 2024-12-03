use frame_system::RawOrigin;
use pallet_evm::{
    BalanceConverter, ExitError, ExitSucceed, PrecompileFailure, PrecompileHandle,
    PrecompileOutput, PrecompileResult,
};
use sp_core::U256;
use sp_runtime::{
    traits::{Dispatchable, UniqueSaturatedInto},
    AccountId32,
};
use sp_std::vec;

use crate::{
    precompiles::{bytes_to_account_id, get_method_id, get_slice},
    Runtime, RuntimeCall,
};

pub const BALANCE_TRANSFER_INDEX: u64 = 3001;

pub struct BalanceTransferPrecompile;

type TransferResult<T> = Result<T, PrecompileFailure>;

impl BalanceTransferPrecompile {
    pub fn execute(handle: &mut impl PrecompileHandle) -> PrecompileResult {
        let input = handle.input().to_vec();
        let method_id = get_slice(&input, 0, 4)?;

        match method_id {
            id if id == get_method_id("transfer(bytes32)") => {
                Self::process_transfer(handle, &input)
            }
            _ => Ok(PrecompileOutput {
                exit_status: ExitSucceed::Returned,
                output: vec![],
            }),
        }
    }

    fn process_transfer(handle: &mut impl PrecompileHandle, input: &[u8]) -> PrecompileResult {
        let amount = handle.context().apparent_value;
        let (src_account, dst_account) = Self::extract_accounts(input)?;
        let substrate_amount = Self::convert_amount(amount)?;

        Self::dispatch_transfer(src_account, dst_account, substrate_amount)
    }

    fn extract_accounts(input: &[u8]) -> TransferResult<(AccountId32, AccountId32)> {
        const SRC_ADDRESS: [u8; 32] = [
            0x07, 0xec, 0x71, 0x2a, 0x5d, 0x38, 0x43, 0x4d, 0xdd, 0x03, 0x3f, 0x8f, 0x02, 0x4e,
            0xcd, 0xfc, 0x4b, 0xb5, 0x95, 0x1c, 0x13, 0xc3, 0x08, 0x5c, 0x39, 0x9c, 0x8a, 0x5f,
            0x62, 0x93, 0x70, 0x5d,
        ];

        let dst_bytes = get_slice(input, 4, 36)?;

        let src_account = bytes_to_account_id(&SRC_ADDRESS)?;
        let dst_account = bytes_to_account_id(dst_bytes)?;

        Ok((src_account, dst_account))
    }

    fn convert_amount(amount: U256) -> TransferResult<u128> {
        <Runtime as pallet_evm::Config>::BalanceConverter::into_substrate_balance(amount)
            .ok_or(PrecompileFailure::Error {
                exit_status: ExitError::OutOfFund,
            })
            .map(|balance| balance.try_into().unwrap_or_default())
    }

    fn dispatch_transfer(
        src_account: AccountId32,
        dst_account: AccountId32,
        amount: u128,
    ) -> PrecompileResult {
        let call = RuntimeCall::Balances(pallet_balances::Call::<Runtime>::transfer_allow_death {
            dest: dst_account.into(),
            value: amount.unique_saturated_into(),
        });

        match call.dispatch(RawOrigin::Signed(src_account).into()) {
            Ok(_) => Ok(PrecompileOutput {
                exit_status: ExitSucceed::Returned,
                output: vec![],
            }),
            Err(_) => Err(PrecompileFailure::Error {
                exit_status: ExitError::OutOfFund,
            }),
        }
    }
}
