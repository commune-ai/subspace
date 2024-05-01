use super::*;
use frame_support::{pallet_prelude::DispatchResult, storage::with_storage_layer};
use sp_runtime::{Percent, SaturatedConversion};

#[derive(Clone, Debug, TypeInfo, Decode, Encode)]
#[scale_info(skip_type_params(T))]
pub struct Proposal<T: Config> {
    pub id: u64,
    pub proposer: T::AccountId,
    pub expiration_block: u64,
    pub data: ProposalData<T>,
    pub status: ProposalStatus,
    pub votes_for: BTreeSet<T::AccountId>, // account addresses
    pub votes_against: BTreeSet<T::AccountId>, // account addresses
    pub proposal_cost: u64,
    pub creation_block: u64,
    pub finalization_block: Option<u64>,
}

#[derive(Clone, Debug, TypeInfo, Decode, Encode)]
#[scale_info(skip_type_params(T))]
pub struct CuratorApplication<T: Config> {
    pub id: u64,
    pub user_id: T::AccountId,
    pub paying_for: T::AccountId,
    pub data: Vec<u8>,
    pub status: ApplicationStatus,
    pub application_cost: u64,
}

impl<T: Config> Proposal<T> {
    /// Whether the proposal is still active.
    pub fn is_active(&self) -> bool {
        matches!(self.status, ProposalStatus::Pending)
    }

    /// Marks a proposal as accepted and overrides the storage value.
    fn accept(mut self, block_number: u64) {
        assert!(self.is_active());

        self.status = ProposalStatus::Accepted;
        self.finalization_block = Some(block_number);

        Proposals::<T>::insert(self.id, self);
    }

    /// Marks a proposal as refused and overrides the storage value.
    fn refuse(mut self, block_number: u64) {
        assert!(self.is_active());

        self.status = ProposalStatus::Refused;
        self.finalization_block = Some(block_number);

        Proposals::<T>::insert(self.id, self);
    }

    /// Marks a proposal as expired and overrides the storage value.
    fn expire(mut self, block_number: u64) {
        assert!(self.is_active());
        assert!(block_number >= self.expiration_block);

        self.status = ProposalStatus::Expired;
        self.data = ProposalData::Expired;
        self.finalization_block = Some(block_number);
        self.votes_for = Default::default();
        self.votes_against = Default::default();

        Proposals::<T>::insert(self.id, self);
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, TypeInfo, Decode, Encode)]
pub enum ProposalStatus {
    #[default]
    Pending,
    Accepted,
    Refused,
    Expired,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, TypeInfo, Decode, Encode)]
pub enum ApplicationStatus {
    #[default]
    Pending,
    Accepted,
    Refused,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, TypeInfo, Decode, Encode)]
pub enum VoteMode {
    Authority = 0,
    Vote = 1,
}

#[derive(Clone, Debug, PartialEq, Eq, TypeInfo, Decode, Encode)]
#[scale_info(skip_type_params(T))]
pub enum ProposalData<T: Config> {
    Custom(Vec<u8>),
    GlobalParams(GlobalParams<T>),
    SubnetParams {
        netuid: u16,
        params: SubnetParams<T>,
    },
    SubnetCustom {
        netuid: u16,
        data: Vec<u8>,
    },
    Expired,
}

impl<T: Config> ProposalData<T> {
    pub fn netuid(&self) -> Option<u16> {
        match self {
            Self::SubnetParams { netuid, .. } | Self::SubnetCustom { netuid, .. } => Some(*netuid),
            _ => None,
        }
    }
}

impl<T: Config> Pallet<T> {
    // Helper function to get the next proposal ID
    fn get_next_proposal_id() -> u64 {
        match Proposals::<T>::iter_keys().max() {
            Some(id) => id + 1,
            None => 0,
        }
    }

    fn get_next_application_id() -> u64 {
        match CuratorApplications::<T>::iter_keys().max() {
            Some(id) => id + 1,
            None => 0,
        }
    }

    pub fn add_proposal(key: T::AccountId, data: ProposalData<T>) -> DispatchResult {
        // Check if the proposer has enough balance
        let proposal_cost = ProposalCost::<T>::get();
        ensure!(
            Self::has_enough_balance(&key, proposal_cost),
            Error::<T>::NotEnoughBalanceToPropose
        );

        let removed_balance_as_currency = Self::u64_to_balance(proposal_cost);
        ensure!(
            removed_balance_as_currency.is_some(),
            Error::<T>::CouldNotConvertToBalance
        );

        // Get the next proposal ID
        let proposal_id = Self::get_next_proposal_id();

        // Get the proposal expiration value from storage
        let proposal_expiration = ProposalExpiration::<T>::get();

        // Create the proposal
        let current_block = Self::get_current_block_number();
        let expiration_block = current_block + proposal_expiration as u64;

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
            status: ProposalStatus::Pending,
            votes_for: BTreeSet::new(),
            votes_against: BTreeSet::new(),
            proposal_cost,
            creation_block: current_block,
            finalization_block: None,
        };

        // Burn the proposal cost from the proposer's balance
        let removed_balance: bool =
            Self::remove_balance_from_account(&key, removed_balance_as_currency.unwrap());
        ensure!(removed_balance, Error::<T>::BalanceCouldNotBeRemoved);

        // Store the proposal
        Proposals::<T>::insert(proposal_id, proposal);

        Self::deposit_event(Event::<T>::ProposalCreated(proposal_id));
        Ok(())
    }

    pub fn add_application(
        key: T::AccountId,
        application_key: T::AccountId,
        data: Vec<u8>,
    ) -> DispatchResult {
        // Check if the proposer has enough balance
        // re use the same value as for proposals
        let application_cost = ProposalCost::<T>::get();

        ensure!(
            Self::has_enough_balance(&key, application_cost),
            Error::<T>::NotEnoughtBalnceToApply
        );

        let removed_balance_as_currency = Self::u64_to_balance(application_cost);
        ensure!(
            removed_balance_as_currency.is_some(),
            Error::<T>::CouldNotConvertToBalance
        );

        let application_id = Self::get_next_application_id();

        let application = CuratorApplication {
            user_id: application_key,
            paying_for: key.clone(),
            id: application_id,
            data,
            status: ApplicationStatus::Pending,
            application_cost,
        };

        // Burn the application cost from the proposer's balance
        let removed_balance: bool =
            Self::remove_balance_from_account(&key, removed_balance_as_currency.unwrap());
        ensure!(removed_balance, Error::<T>::BalanceCouldNotBeRemoved);

        CuratorApplications::<T>::insert(application_id, application);

        Self::deposit_event(Event::<T>::ApplicationCreated(application_id));
        Ok(())
    }

    pub fn do_refuse_dao_application(
        origin: T::RuntimeOrigin,
        application_id: u64,
    ) -> DispatchResult {
        let key = ensure_signed(origin)?;

        // --- 2. Ensure that the key is the curator multisig.
        ensure!(Self::get_curator() == key, Error::<T>::NotCurator);

        let mut application =
            CuratorApplications::<T>::get(application_id).ok_or(Error::<T>::ApplicationNotFound)?;

        ensure!(
            application.status == ApplicationStatus::Pending,
            Error::<T>::ApplicationNotPending
        );

        // Change the status of application to refused
        application.status = ApplicationStatus::Refused;

        CuratorApplications::<T>::insert(application_id, application);

        Ok(())
    }

    pub fn do_add_dao_application(
        origin: T::RuntimeOrigin,
        application_key: T::AccountId,
        data: Vec<u8>,
    ) -> DispatchResult {
        let key = ensure_signed(origin)?;
        ensure!(!data.is_empty(), Error::<T>::ApplicationTooSmall);
        ensure!(data.len() <= 256, Error::<T>::ApplicationTooLarge);
        sp_std::str::from_utf8(&data).map_err(|_| Error::<T>::InvalidApplication)?;

        Self::add_application(key, application_key, data)
    }

    // Proposal with custom data
    pub fn do_add_custom_proposal(origin: T::RuntimeOrigin, data: Vec<u8>) -> DispatchResult {
        let key = ensure_signed(origin)?;
        ensure!(!data.is_empty(), Error::<T>::ProposalCustomDataTooSmall);
        ensure!(data.len() <= 256, Error::<T>::ProposalCustomDataTooLarge);
        sp_std::str::from_utf8(&data).map_err(|_| Error::<T>::InvalidProposalCustomData)?;

        let proposal_data = ProposalData::Custom(data);
        Self::add_proposal(key, proposal_data)
    }

    // Subnet proposal with custom data
    pub fn do_add_custom_subnet_proposal(
        origin: T::RuntimeOrigin,
        netuid: u16,
        data: Vec<u8>,
    ) -> DispatchResult {
        let key = ensure_signed(origin)?;
        ensure!(!data.is_empty(), Error::<T>::ProposalCustomDataTooSmall);
        ensure!(data.len() <= 256, Error::<T>::ProposalCustomDataTooLarge);
        sp_std::str::from_utf8(&data).map_err(|_| Error::<T>::InvalidProposalCustomData)?;

        let proposal_data = ProposalData::SubnetCustom { netuid, data };
        Self::add_proposal(key, proposal_data)
    }

    /// Proposal to change the global parameters
    pub fn do_add_global_proposal(
        origin: T::RuntimeOrigin,
        params: GlobalParams<T>,
    ) -> DispatchResult {
        let key = ensure_signed(origin)?;

        Self::check_global_params(&params)?;
        let proposal_data = ProposalData::GlobalParams(params);
        Self::add_proposal(key, proposal_data)
    }

    // Proposal to change subnet parameters
    /// Subnet has to be on a "vote" mode, otherwise this proposal will throw an error
    pub fn do_add_subnet_proposal(
        origin: T::RuntimeOrigin,
        netuid: u16,
        params: SubnetParams<T>,
    ) -> DispatchResult {
        let key = ensure_signed(origin)?;

        // Make sure that the subnet is set on `Vote` mode.
        // In Authority mode, only the founder can make changes.
        let vote_mode = VoteModeSubnet::<T>::get(netuid);
        ensure!(vote_mode == VoteMode::Vote, Error::<T>::NotVoteMode);

        Self::check_subnet_params(&params)?;
        let proposal_data = ProposalData::SubnetParams { netuid, params };
        Self::add_proposal(key, proposal_data)
    }

    /// Votes on proposals,
    pub fn do_vote_proposal(
        origin: T::RuntimeOrigin,
        proposal_id: u64,
        agree: bool,
    ) -> DispatchResult {
        let key = ensure_signed(origin)?;

        // Get the proposal from storage
        let Ok(mut proposal) = Proposals::<T>::try_get(proposal_id) else {
            return Err(Error::<T>::ProposalNotFound.into());
        };

        // Check if the proposal is in a valid state for voting
        ensure!(
            proposal.status == ProposalStatus::Pending,
            Error::<T>::InvalidProposalStatus
        );

        // Check if the voter has already voted
        ensure!(
            !proposal.votes_for.contains(&key) && !proposal.votes_against.contains(&key),
            Error::<T>::AlreadyVoted
        );

        // Get the netuid from the proposal data
        let netuid = proposal.data.netuid();

        // Get the voter's stake
        let voter_stake = Self::get_account_stake(&key, netuid);

        // Check if the voter has non-zero stake
        ensure!(voter_stake > 0, Error::<T>::InsufficientStake);

        // Update the proposal based on the vote
        match agree {
            true => proposal.votes_for.insert(key.clone()),
            false => proposal.votes_against.insert(key.clone()),
        };

        // Update the proposal in storage
        Proposals::<T>::insert(proposal_id, proposal);
        Self::deposit_event(Event::<T>::ProposalVoted(proposal_id, key, agree));
        Ok(())
    }

    /// Unregister the vote on a proposal
    pub fn do_unregister_vote(origin: T::RuntimeOrigin, proposal_id: u64) -> DispatchResult {
        let key = ensure_signed(origin)?;

        // Get the proposal from storage
        let Ok(mut proposal) = Proposals::<T>::try_get(proposal_id) else {
            return Err(Error::<T>::ProposalNotFound.into());
        };

        // Check if the proposal is in a valid state for unregistering a vote
        ensure!(
            proposal.status == ProposalStatus::Pending,
            Error::<T>::InvalidProposalStatus
        );

        let removed = proposal.votes_for.remove(&key) || proposal.votes_against.remove(&key);

        // Check if the voter has actually voted on the proposal
        ensure!(removed, Error::<T>::VoteNotFound);

        // Update the proposal in storage
        Proposals::<T>::insert(proposal.id, proposal);
        Self::deposit_event(Event::<T>::ProposalVoteUnregistered(proposal_id, key));
        Ok(())
    }

    pub(crate) fn resolve_proposals(block_number: u64) {
        for proposal in Proposals::<T>::iter_values() {
            let proposal_id = proposal.id;

            if !matches!(proposal.status, ProposalStatus::Pending) {
                continue;
            }

            let netuid = proposal.data.netuid();

            let votes_for: u64 =
                proposal.votes_for.iter().map(|id| Self::get_account_stake(id, netuid)).sum();
            let votes_against: u64 = proposal
                .votes_against
                .iter()
                .map(|id| Self::get_account_stake(id, netuid))
                .sum();

            let total_stake = votes_for + votes_against;
            let minimal_stake_to_execute = Self::get_minimal_stake_to_execute(netuid);

            let res: DispatchResult = with_storage_layer(|| {
                if total_stake >= minimal_stake_to_execute {
                    if votes_against > votes_for {
                        proposal.refuse(block_number);
                    } else {
                        Self::execute_proposal(proposal, block_number)?;
                    }
                } else if block_number >= proposal.expiration_block {
                    proposal.expire(block_number);
                }

                Ok(())
            });

            if let Err(err) = res {
                log::error!("failed to resolve proposal {proposal_id}: {err:?}");
            }
        }
    }

    pub fn execute_application(user_id: &T::AccountId) -> DispatchResult {
        // Perform actions based on the application data type
        // The owners will handle the off-chain logic

        let mut application = CuratorApplications::<T>::iter_values()
            .find(|app| app.user_id == *user_id)
            .ok_or(Error::<T>::ApplicationNotFound)?;

        // Give the proposer back his tokens, if the application passed
        Self::add_balance_to_account(
            &application.paying_for,
            Self::u64_to_balance(application.application_cost).unwrap(),
        );
        application.status = ApplicationStatus::Accepted;

        CuratorApplications::<T>::insert(application.id, application);

        Ok(())
    }

    fn execute_proposal(proposal: Proposal<T>, block_number: u64) -> DispatchResult {
        // Perform actions based on the proposal data type
        match &proposal.data {
            ProposalData::Custom(_) | ProposalData::SubnetCustom { .. } => {
                // No specific action needed for custom proposals
                // The owners will handle the off-chain logic
            }
            ProposalData::GlobalParams(params) => {
                // Update the global parameters
                Self::set_global_params(params.clone());
                // Emit the GlobalParamsUpdated event
                Self::deposit_event(Event::GlobalParamsUpdated(params.clone()));
            }
            ProposalData::SubnetParams { netuid, params } => {
                let changeset = SubnetChangeset::<T>::update(*netuid, params.clone())?;
                changeset.apply(*netuid)?;

                // Emit the SubnetParamsUpdated event
                Self::deposit_event(Event::SubnetParamsUpdated(*netuid));
            }
            ProposalData::Expired => {
                unreachable!("Expired data is illegal at this point")
            }
        }

        // Give the proposer back his tokens, if the proposal passed
        Self::add_balance_to_account(
            &proposal.proposer,
            Self::u64_to_balance(proposal.proposal_cost).unwrap(),
        );

        proposal.accept(block_number);

        Ok(())
    }

    /// Returns how much stake is needed to execute a proposal
    pub fn get_minimal_stake_to_execute(netuid: Option<u16>) -> u64 {
        let threshold: Percent = Self::get_proposal_participation_threshold();

        let stake = match netuid {
            Some(specific_netuid) => TotalStake::<T>::get(specific_netuid),
            None => Self::total_stake(),
        };
        (stake.saturated_into::<u128>() * threshold.deconstruct() as u128 / 100) as u64
    }
}
