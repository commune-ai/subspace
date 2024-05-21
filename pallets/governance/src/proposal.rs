use crate::{
    Config, Error, Event, GovernanceConfig, GovernanceConfiguration, Pallet, Percent, Proposals,
    SubnetId,
};
use frame_support::{
    dispatch::DispatchResult, ensure, storage::with_storage_layer, traits::ConstU32,
    BoundedBTreeSet, BoundedVec, DebugNoBound,
};
use frame_system::ensure_signed;
use pallet_subspace::{
    subnet::SubnetChangeset, voting::VoteMode, DaoTreasuryAddress, GlobalParams,
    Pallet as PalletSubspace, SubnetParams, VoteModeSubnet,
};
use parity_scale_codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use sp_std::vec::Vec;

pub type ProposalId = u64;

#[derive(DebugNoBound, TypeInfo, Decode, Encode, MaxEncodedLen)]
#[scale_info(skip_type_params(T))]
pub struct Proposal<T: Config> {
    pub id: ProposalId,
    pub proposer: T::AccountId,
    pub expiration_block: u64,
    pub data: ProposalData<T>,
    pub status: ProposalStatus<T>,
    pub metadata: BoundedVec<u8, ConstU32<256>>,
    pub proposal_cost: u64,
    pub creation_block: u64,
}

impl<T: Config> Proposal<T> {
    /// Whether the proposal is still active.
    #[must_use]
    pub fn is_active(&self) -> bool {
        matches!(self.status, ProposalStatus::Open { .. })
    }

    /// Returns the subnet ID that this proposal impact.s
    #[must_use]
    pub fn subnet_id(&self) -> Option<u16> {
        match &self.data {
            ProposalData::SubnetParams { subnet_id, .. }
            | ProposalData::SubnetCustom { subnet_id, .. } => Some(*subnet_id),
            _ => None,
        }
    }

    /// Marks a proposal as accepted and overrides the storage value.
    pub fn accept(mut self, block_number: u64) -> DispatchResult {
        ensure!(self.is_active(), Error::<T>::ProposalIsFinished);

        self.status = ProposalStatus::Accepted {
            block: block_number,
            stake_for: 0,
            stake_against: 0,
        };

        Proposals::<T>::insert(self.id, &self);
        Pallet::<T>::deposit_event(Event::ProposalAccepted(self.id));

        execute_proposal(self)?;

        Ok(())
    }

    /// Marks a proposal as refused and overrides the storage value.
    pub fn refuse(mut self, block_number: u64) -> DispatchResult {
        ensure!(self.is_active(), Error::<T>::ProposalIsFinished);

        self.status = ProposalStatus::Refused {
            block: block_number,
            stake_for: 0,
            stake_against: 0,
        };

        Proposals::<T>::insert(self.id, &self);
        Pallet::<T>::deposit_event(Event::ProposalRefused(self.id));

        Ok(())
    }

    /// Marks a proposal as expired and overrides the storage value.
    pub fn expire(mut self, block_number: u64) -> DispatchResult {
        ensure!(self.is_active(), Error::<T>::ProposalIsFinished);
        ensure!(
            block_number >= self.expiration_block,
            Error::<T>::InvalidProposalFinalizationParameters
        );

        self.status = ProposalStatus::Expired;

        Proposals::<T>::insert(self.id, &self);
        Pallet::<T>::deposit_event(Event::ProposalExpired(self.id));

        Ok(())
    }
}

#[derive(Clone, DebugNoBound, TypeInfo, Decode, Encode, MaxEncodedLen, PartialEq, Eq)]
#[scale_info(skip_type_params(T))]
pub enum ProposalStatus<T: Config> {
    Open {
        votes_for: BoundedBTreeSet<T::AccountId, ConstU32<{ u32::MAX }>>,
        votes_against: BoundedBTreeSet<T::AccountId, ConstU32<{ u32::MAX }>>,
    },
    Accepted {
        block: u64,
        stake_for: u64,
        stake_against: u64,
    },
    Refused {
        block: u64,
        stake_for: u64,
        stake_against: u64,
    },
    Expired,
}

#[derive(DebugNoBound, TypeInfo, Decode, Encode, MaxEncodedLen, PartialEq, Eq)]
#[scale_info(skip_type_params(T))]
pub enum ProposalData<T: Config> {
    GlobalCustom,
    GlobalParams(pallet_subspace::GlobalParams<T>),
    SubnetCustom {
        subnet_id: SubnetId,
    },
    SubnetParams {
        subnet_id: SubnetId,
        params: pallet_subspace::SubnetParams<T>,
    },
    TransferDaoTreasury {
        account: T::AccountId,
        amount: u64,
    },
}

impl<T: Config> ProposalData<T> {
    /// The required amount of stake each of the proposal types requires in order to pass.
    #[must_use]
    pub fn required_stake(&self) -> Percent {
        match self {
            Self::GlobalCustom | Self::SubnetCustom { .. } | Self::TransferDaoTreasury { .. } => {
                Percent::from_parts(50)
            }
            Self::GlobalParams(_) | Self::SubnetParams { .. } => Percent::from_parts(40),
        }
    }
}

impl<T: Config> Pallet<T> {
    fn get_next_proposal_id() -> u64 {
        match Proposals::<T>::iter_keys().max() {
            Some(id) => id + 1,
            None => 0,
        }
    }

    pub fn add_proposal(
        key: T::AccountId,
        metadata: BoundedVec<u8, ConstU32<256>>,
        data: ProposalData<T>,
    ) -> DispatchResult {
        let GovernanceConfiguration {
            proposal_cost,
            expiration,
            ..
        } = GovernanceConfig::<T>::get();

        ensure!(
            PalletSubspace::<T>::has_enough_balance(&key, proposal_cost),
            Error::<T>::NotEnoughBalanceToPropose
        );

        let removed_balance_as_currency = PalletSubspace::<T>::u64_to_balance(proposal_cost);
        ensure!(
            removed_balance_as_currency.is_some(),
            Error::<T>::InvalidCurrencyConversionValue
        );

        let proposal_id = Self::get_next_proposal_id();

        let current_block = PalletSubspace::<T>::get_current_block_number();
        let expiration_block = current_block + expiration as u64;

        // TODO: extract rounding function
        let expiration_block = if expiration_block % 100 == 0 {
            expiration_block
        } else {
            expiration_block + 100 - (expiration_block % 100)
        };

        let proposal = Proposal {
            id: proposal_id,
            proposer: key.clone(),
            expiration_block,
            data,
            status: ProposalStatus::Open {
                votes_for: BoundedBTreeSet::new(),
                votes_against: BoundedBTreeSet::new(),
            },
            proposal_cost,
            creation_block: current_block,
            metadata,
        };

        PalletSubspace::<T>::remove_balance_from_account(
            &key,
            removed_balance_as_currency.unwrap(),
        )?;

        Proposals::<T>::insert(proposal_id, proposal);

        Self::deposit_event(Event::<T>::ProposalCreated(proposal_id));
        Ok(())
    }

    pub fn do_add_custom_proposal(origin: T::RuntimeOrigin, data: Vec<u8>) -> DispatchResult {
        let key = ensure_signed(origin)?;
        ensure!(!data.is_empty(), Error::<T>::ProposalDataTooSmall);
        ensure!(data.len() <= 256, Error::<T>::ProposalDataTooLarge);
        sp_std::str::from_utf8(&data).map_err(|_| Error::<T>::InvalidProposalData)?;

        let proposal_data = ProposalData::GlobalCustom;
        Self::add_proposal(key, BoundedVec::truncate_from(data), proposal_data)
    }

    pub fn do_add_custom_subnet_proposal(
        origin: T::RuntimeOrigin,
        netuid: u16,
        data: Vec<u8>,
    ) -> DispatchResult {
        let key = ensure_signed(origin)?;
        ensure!(!data.is_empty(), Error::<T>::ProposalDataTooSmall);
        ensure!(data.len() <= 256, Error::<T>::ProposalDataTooLarge);
        sp_std::str::from_utf8(&data).map_err(|_| Error::<T>::InvalidProposalData)?;

        let proposal_data = ProposalData::SubnetCustom { subnet_id: netuid };
        Self::add_proposal(key, BoundedVec::truncate_from(data), proposal_data)
    }

    pub fn do_add_transfer_dao_treasury_proposal(
        origin: T::RuntimeOrigin,
        data: Vec<u8>,
        value: u64,
        dest: T::AccountId,
    ) -> DispatchResult {
        let key = ensure_signed(origin)?;
        ensure!(!data.is_empty(), Error::<T>::ProposalDataTooSmall);
        ensure!(data.len() <= 256, Error::<T>::ProposalDataTooLarge);
        ensure!(
            PalletSubspace::<T>::has_enough_balance(&DaoTreasuryAddress::<T>::get(), value),
            Error::<T>::InsufficientDaoTreasuryFunds
        );
        sp_std::str::from_utf8(&data).map_err(|_| Error::<T>::InvalidProposalData)?;

        let proposal_data = ProposalData::TransferDaoTreasury {
            amount: value,
            account: dest,
        };
        Self::add_proposal(key, BoundedVec::truncate_from(data), proposal_data)
    }

    pub fn do_add_global_proposal(
        origin: T::RuntimeOrigin,
        data: Vec<u8>,
        params: GlobalParams<T>,
    ) -> DispatchResult {
        let key = ensure_signed(origin)?;

        ensure!(!data.is_empty(), Error::<T>::ProposalDataTooSmall);
        ensure!(data.len() <= 256, Error::<T>::ProposalDataTooLarge);
        PalletSubspace::check_global_params(&params)?;

        let proposal_data = ProposalData::GlobalParams(params);
        Self::add_proposal(key, BoundedVec::truncate_from(data), proposal_data)
    }

    pub fn do_add_subnet_proposal(
        origin: T::RuntimeOrigin,
        netuid: u16,
        data: Vec<u8>,
        params: SubnetParams<T>,
    ) -> DispatchResult {
        let key = ensure_signed(origin)?;

        let vote_mode = VoteModeSubnet::<T>::get(netuid);
        ensure!(vote_mode == VoteMode::Vote, Error::<T>::NotVoteMode);

        ensure!(!data.is_empty(), Error::<T>::ProposalDataTooSmall);
        ensure!(data.len() <= 256, Error::<T>::ProposalDataTooLarge);

        SubnetChangeset::<T>::update(netuid, params.clone())?;
        let proposal_data = ProposalData::SubnetParams {
            subnet_id: netuid,
            params,
        };
        Self::add_proposal(key, BoundedVec::truncate_from(data), proposal_data)
    }
}

pub fn tick_proposals<T: Config>(block_number: u64) {
    for (id, proposal) in Proposals::<T>::iter().filter(|(_, p)| p.is_active()) {
        let res = with_storage_layer(|| tick_proposal(block_number, proposal));
        if let Err(err) = res {
            log::error!("failed to tick proposal {id}: {err:?}, skipping...");
        }
    }
}

pub fn tick_proposal<T: Config>(block_number: u64, proposal: Proposal<T>) -> DispatchResult {
    use pallet_subspace::Pallet as SubspacePallet;

    let subnet_id = proposal.subnet_id();

    let ProposalStatus::Open {
        votes_for,
        votes_against,
    } = &proposal.status
    else {
        return Err(Error::<T>::ProposalIsFinished.into());
    };

    let votes_for: u64 = votes_for
        .iter()
        .map(|id| SubspacePallet::<T>::get_account_stake(id, subnet_id))
        .sum();
    let votes_against: u64 = votes_against
        .iter()
        .map(|id| SubspacePallet::<T>::get_account_stake(id, subnet_id))
        .sum();

    let total_stake = votes_for + votes_against;
    let minimal_stake_to_execute =
        SubspacePallet::<T>::get_minimal_stake_to_execute_with_percentage(
            proposal.data.required_stake(),
            subnet_id,
        );

    if total_stake >= minimal_stake_to_execute {
        if votes_against > votes_for {
            proposal.refuse(block_number)
        } else {
            proposal.accept(block_number)
        }
    } else if block_number >= proposal.expiration_block {
        proposal.expire(block_number)
    } else {
        Ok(())
    }
}

pub fn execute_proposal<T: Config>(proposal: Proposal<T>) -> DispatchResult {
    use pallet_subspace::{
        subnet::SubnetChangeset, Error as SubspaceError, Event as SubspaceEvent,
        Pallet as SubspacePallet,
    };

    match &proposal.data {
        ProposalData::GlobalCustom | ProposalData::SubnetCustom { .. } => {
            // No specific action needed for custom proposals
            // The owners will handle the off-chain logic
        }
        ProposalData::GlobalParams(params) => {
            SubspacePallet::<T>::set_global_params(params.clone());
            SubspacePallet::<T>::deposit_event(SubspaceEvent::GlobalParamsUpdated(params.clone()));
        }
        ProposalData::SubnetParams { subnet_id, params } => {
            let changeset = SubnetChangeset::<T>::update(*subnet_id, params.clone())?;
            changeset.apply(*subnet_id)?;
            SubspacePallet::<T>::deposit_event(SubspaceEvent::SubnetParamsUpdated(*subnet_id));
        }
        ProposalData::TransferDaoTreasury { account, amount } => {
            PalletSubspace::<T>::remove_balance_from_account(
                &DaoTreasuryAddress::<T>::get(),
                PalletSubspace::<T>::u64_to_balance(*amount)
                    .ok_or(Error::<T>::InvalidCurrencyConversionValue)?,
            )?;

            let amount = SubspacePallet::<T>::u64_to_balance(*amount)
                .ok_or(SubspaceError::<T>::CouldNotConvertToBalance)?;
            SubspacePallet::<T>::add_balance_to_account(account, amount);
        }
    }

    SubspacePallet::<T>::add_balance_to_account(
        &proposal.proposer,
        SubspacePallet::<T>::u64_to_balance(proposal.proposal_cost).unwrap(),
    );

    Ok(())
}
