use crate::{dispatch, Call, Config, Pallet};
use frame_support::{
    dispatch::{DispatchInfo, PostDispatchInfo},
    traits::{Currency, IsSubType},
};
use frame_system::Config as SystemConfig;
use parity_scale_codec::{Decode, Encode};
use scale_info::TypeInfo;
use sp_runtime::{
    traits::{DispatchInfoOf, Dispatchable, PostDispatchInfoOf, SignedExtension},
    transaction_validity::{TransactionValidity, TransactionValidityError, ValidTransaction},
};
use sp_std::marker::PhantomData;

#[derive(Debug, PartialEq, Default)]
pub enum CallType {
    AddStake,
    TransferStakeMultiple,
    TransferMultiple,
    TransferStake,
    AddStakeMultiple,
    RemoveStakeMultiple,
    RemoveStake,
    AddDelegate,
    Register,
    AddNetwork,
    Update,
    #[default]
    Other,
}

#[derive(Encode, Decode, Clone, Eq, PartialEq, TypeInfo)]
pub struct SubspaceSignedExtension<T: Config + Send + Sync + TypeInfo>(pub PhantomData<T>);

impl<T: Config + Send + Sync + TypeInfo> SubspaceSignedExtension<T>
where
    T::RuntimeCall: Dispatchable<Info = DispatchInfo, PostInfo = PostDispatchInfo>,
    <T as SystemConfig>::RuntimeCall: IsSubType<Call<T>>,
{
    pub fn new() -> Self {
        Self(Default::default())
    }

    pub fn get_priority_vanilla(who: &T::AccountId) -> u64 {
        let current_block_number = Pallet::<T>::get_current_block_number();
        let balance = Pallet::<T>::get_balance_u64(who);
        current_block_number.saturating_add(balance)
    }

    #[must_use]
    pub fn u64_to_balance(
        input: u64,
    ) -> Option<<<T as Config>::Currency as Currency<<T as SystemConfig>::AccountId>>::Balance>
    {
        input.try_into().ok()
    }
}

impl<T: Config + Send + Sync + TypeInfo> Default for SubspaceSignedExtension<T>
where
    T::RuntimeCall: Dispatchable<Info = DispatchInfo, PostInfo = PostDispatchInfo>,
    <T as SystemConfig>::RuntimeCall: IsSubType<Call<T>>,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Config + Send + Sync + TypeInfo> sp_std::fmt::Debug for SubspaceSignedExtension<T> {
    fn fmt(&self, f: &mut sp_std::fmt::Formatter) -> sp_std::fmt::Result {
        write!(f, "SubspaceSignedExtension")
    }
}

impl<T: Config + Send + Sync + TypeInfo> SignedExtension for SubspaceSignedExtension<T>
where
    T::RuntimeCall: Dispatchable<Info = DispatchInfo, PostInfo = PostDispatchInfo>,
    <T as SystemConfig>::RuntimeCall: IsSubType<Call<T>>,
{
    const IDENTIFIER: &'static str = "SubspaceSignedExtension";

    type AccountId = T::AccountId;
    type Call = T::RuntimeCall;
    type AdditionalSigned = ();
    type Pre = (CallType, u64, Self::AccountId);

    fn additional_signed(&self) -> Result<Self::AdditionalSigned, TransactionValidityError> {
        Ok(())
    }

    fn validate(
        &self,
        who: &Self::AccountId,
        _call: &Self::Call,
        _info: &DispatchInfoOf<Self::Call>,
        _len: usize,
    ) -> TransactionValidity {
        Ok(ValidTransaction {
            priority: Self::get_priority_vanilla(who),
            ..Default::default()
        })
    }

    fn pre_dispatch(
        self,
        who: &Self::AccountId,
        call: &Self::Call,
        _info: &DispatchInfoOf<Self::Call>,
        _len: usize,
    ) -> Result<Self::Pre, TransactionValidityError> {
        let who = who.clone();
        match call.is_sub_type() {
            Some(Call::add_stake { .. }) => Ok((CallType::AddStake, 0, who)),
            Some(Call::add_stake_multiple { .. }) => Ok((CallType::AddStakeMultiple, 0, who)),
            Some(Call::remove_stake { .. }) => Ok((CallType::RemoveStake, 0, who)),
            Some(Call::remove_stake_multiple { .. }) => Ok((CallType::RemoveStakeMultiple, 0, who)),
            Some(Call::transfer_stake { .. }) => Ok((CallType::TransferStake, 0, who)),
            Some(Call::transfer_multiple { .. }) => Ok((CallType::TransferMultiple, 0, who)),
            Some(Call::register { .. }) => Ok((CallType::Register, 0, who)),
            Some(Call::update_module { .. }) => Ok((CallType::Update, 0, who)),
            _ => Ok((CallType::Other, 0, who)),
        }
    }

    fn post_dispatch(
        maybe_pre: Option<Self::Pre>,
        _info: &DispatchInfoOf<Self::Call>,
        _post_info: &PostDispatchInfoOf<Self::Call>,
        _len: usize,
        _result: &dispatch::DispatchResult,
    ) -> Result<(), TransactionValidityError> {
        if let Some((call_type, _transaction_fee, _who)) = maybe_pre {
            match call_type {
                CallType::AddStake => {
                    log::debug!("Not Implemented! Need to add potential transaction fees here.");
                }
                CallType::AddStakeMultiple => {
                    log::debug!("Not Implemented! Need to add potential transaction fees here.");
                }
                CallType::RemoveStake => {
                    log::debug!("Not Implemented! Need to add potential transaction fees here.");
                }
                CallType::RemoveStakeMultiple => {
                    log::debug!("Not Implemented! Need to add potential transaction fees here.");
                }
                CallType::TransferStake => {
                    log::debug!("Not Implemented! Need to add potential transaction fees here.");
                }
                CallType::TransferStakeMultiple => {
                    log::debug!("Not Implemented! Need to add potential transaction fees here.");
                }
                CallType::TransferMultiple => {
                    log::debug!("Not Implemented! Need to add potential transaction fees here.");
                }
                CallType::AddNetwork => {
                    log::debug!("Not Implemented! Need to add potential transaction fees here.");
                }
                CallType::Register => {
                    log::debug!("Not Implemented!");
                }
                _ => {
                    log::debug!("Not Implemented!");
                }
            }
        }
        Ok(())
    }
}
