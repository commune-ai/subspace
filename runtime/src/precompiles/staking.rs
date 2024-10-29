use frame_system::RawOrigin;
use pallet_evm::{
    AddressMapping, BalanceConverter, ExitError, ExitSucceed, HashedAddressMapping,
    PrecompileFailure, PrecompileHandle, PrecompileOutput, PrecompileResult,
};
use sp_core::{H160, U256};
use sp_runtime::{
    traits::{BlakeTwo256, Dispatchable},
    AccountId32,
};
use sp_std::{vec, vec::Vec};

use crate::{
    precompiles::{get_method_id, get_slice},
    Runtime, RuntimeCall,
};

pub const STAKING_PRECOMPILE_INDEX: u64 = 3002;

pub struct StakingPrecompile;

type StakingResult<T> = Result<T, PrecompileFailure>;

impl StakingPrecompile {
    pub fn execute(handle: &mut impl PrecompileHandle) -> PrecompileResult {
        let input = handle.input();
        let (method_id, method_input) = input.split_at(4);

        match method_id {
            id if id == get_method_id("addStake(bytes32)") => {
                Self::add_stake(handle, method_input.to_vec())
            }
            id if id == get_method_id("removeStake(bytes32,uint256)") => {
                Self::remove_stake(handle, method_input.to_vec())
            }
            _ => Err(PrecompileFailure::Error {
                exit_status: ExitError::InvalidRange,
            }),
        }
    }

    fn add_stake(handle: &mut impl PrecompileHandle, data: Vec<u8>) -> PrecompileResult {
        let key = Self::extract_key(&data)?;
        let amount = Self::convert_amount(handle.context().apparent_value)?;

        Self::dispatch(
            handle,
            RuntimeCall::SubspaceModule(pallet_subspace::Call::<Runtime>::add_stake {
                module_key: key.into(),
                amount: amount as u64,
            }),
        )
    }

    fn remove_stake(handle: &mut impl PrecompileHandle, data: Vec<u8>) -> PrecompileResult {
        let key = Self::extract_key(&data)?;
        let amount =
            data.get(56..64).map(U256::from_big_endian).ok_or(PrecompileFailure::Error {
                exit_status: ExitError::OutOfFund,
            })?;
        let amount = Self::convert_amount(amount)?;

        Self::dispatch(
            handle,
            RuntimeCall::SubspaceModule(pallet_subspace::Call::<Runtime>::remove_stake {
                module_key: key.into(),
                amount: amount as u64,
            }),
        )
    }

    fn extract_key(data: &[u8]) -> StakingResult<[u8; 32]> {
        let mut key = [0u8; 32];
        key.copy_from_slice(get_slice(data, 0, 32)?);
        Ok(key)
    }

    fn convert_amount(amount: U256) -> StakingResult<u128> {
        let balance =
            <Runtime as pallet_evm::Config>::BalanceConverter::into_substrate_balance(amount)
                .ok_or(PrecompileFailure::Error {
                    exit_status: ExitError::OutOfFund,
                })?;
        balance.try_into().map_err(|_| PrecompileFailure::Error {
            exit_status: ExitError::OutOfFund,
        })
    }

    fn dispatch(handle: &mut impl PrecompileHandle, call: RuntimeCall) -> PrecompileResult {
        let caller = HashedAddressMapping::<BlakeTwo256>::into_account_id(handle.context().caller);
        let value = handle.context().apparent_value;

        if !value.is_zero() {
            Self::transfer_back_to_caller(&caller, value)?;
        }

        match call.dispatch(RawOrigin::Signed(caller).into()) {
            Ok(post_info) => {
                log::info!("Dispatch succeeded. Post info: {:?}", post_info);
                Ok(PrecompileOutput {
                    exit_status: ExitSucceed::Returned,
                    output: vec![],
                })
            }
            Err(e) => {
                log::error!("Dispatch failed. Error: {:?}", e);
                Err(PrecompileFailure::Error {
                    exit_status: ExitError::Other("Subspace call failed".into()),
                })
            }
        }
    }

    fn transfer_back_to_caller(account_id: &AccountId32, amount: U256) -> StakingResult<()> {
        let precompile_account = HashedAddressMapping::<BlakeTwo256>::into_account_id(
            H160::from_low_u64_be(STAKING_PRECOMPILE_INDEX),
        );

        let amount = Self::convert_amount(amount)?;

        let transfer =
            RuntimeCall::Balances(pallet_balances::Call::<Runtime>::transfer_allow_death {
                dest: account_id.clone().into(),
                value: amount as u64,
            });

        match transfer.dispatch(RawOrigin::Signed(precompile_account).into()) {
            Ok(_) => Ok(()),
            Err(e) => {
                log::error!("Transfer back to caller failed. Error: {:?}", e);
                Err(PrecompileFailure::Error {
                    exit_status: ExitError::Other("Transfer back to caller failed".into()),
                })
            }
        }
    }
}
