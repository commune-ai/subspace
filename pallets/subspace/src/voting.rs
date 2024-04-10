use super::*;
use frame_support::pallet_prelude::DispatchResult;
use sp_runtime::{DispatchError, Percent, SaturatedConversion};

#[derive(Clone, Copy, Debug, PartialEq, Eq, TypeInfo, Decode, Encode)]
pub enum VoteMode {
    Authority = 0,
    Vote = 1,
}

impl<T: Config> Pallet<T> {
    // Helper function to get the next proposal ID
    fn get_next_proposal_id() -> Result<u64, DispatchError> {
        let proposal_id = match Proposals::<T>::iter().last() {
            Some((id, _)) => id + 1,
            None => 0,
        };
        Ok(proposal_id)
    }

    pub fn add_proposal(key: T::AccountId, data: ProposalData<T>) -> DispatchResult {
        // Check if the proposer has enough balance
        let stake_amount = ProposalCost::<T>::get();
        ensure!(
            Self::has_enough_balance(&key, stake_amount),
            "Insufficient balance"
        );

        // Get the next proposal ID
        let proposal_id = Self::get_next_proposal_id()?;

        // Get the proposal expiration value from storage
        let proposal_expiration = ProposalExpiration::<T>::get();

        // Create the proposal
        let proposal = Proposal {
            id: proposal_id,
            proposer: key.clone(),
            expiration_block: Self::get_current_block_as_u64() + proposal_expiration as u64,
            data,
            proposal_status: ProposalStatus::Pending,
            votes_for: BTreeSet::new(),
            votes_against: BTreeSet::new(),
        };

        // Store the proposal
        Proposals::<T>::insert(proposal_id, proposal);

        // Burn the proposal cost from the proposer's balance
        let _ = T::Currency::withdraw(
            &key,
            Self::u64_to_balance(stake_amount).unwrap(),
            WithdrawReasons::TRANSFER,
            ExistenceRequirement::KeepAlive,
        )?;

        Event::<T>::ProposalCreated(proposal_id);
        Ok(())
    }

    // Proposal with custom IPFS data
    pub fn do_add_custom_proposal(origin: T::RuntimeOrigin, data: Vec<u8>) -> DispatchResult {
        // Validate the data as a link to an IPFS document
        let key = ensure_signed(origin)?;
        let ipfs_link = sp_std::str::from_utf8(&data).map_err(|_| "Invalid IPFS link")?;
        ensure!(ipfs_link.starts_with("ipfs://"), "Invalid IPFS link format");
        ensure!(ipfs_link.len() <= 150, "IPFS link exceeds maximum length"); // 150 character limit should be more than enough

        let proposal_data = ProposalData::Custom(data);
        Self::add_proposal(key, proposal_data)
    }

    /// Proposal to change the global parameters
    pub fn do_add_global_proposal(
        origin: T::RuntimeOrigin,
        params: GlobalParams,
    ) -> DispatchResult {
        let key = ensure_signed(origin)?;

        Self::check_global_params(params.clone())?;

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
        let vote_mode = VoteModeSubnet::<T>::get(netuid);
        // make sure that the subnet is set on `Vote`,
        // in authority only the founder can make changes
        ensure!(vote_mode == VoteMode::Vote, Error::<T>::NotVoteMode,);

        Self::check_subnet_params(params.clone())?;

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
        let Ok(mut proposal) = Proposals::<T>::try_get(&proposal_id) else {
            return Err(Error::<T>::ProposalNotFound.into());
        };

        // Check if the proposal is in a valid state for voting
        ensure!(
            proposal.proposal_status == ProposalStatus::Pending,
            Error::<T>::InvalidProposalStatus
        );

        // Check if the voter has already voted
        ensure!(
            !proposal.votes_for.contains(&key) && !proposal.votes_against.contains(&key),
            Error::<T>::AlreadyVoted
        );

        // Get the netuid from the proposal data
        let netuid = match &proposal.data {
            ProposalData::SubnetParams { netuid, .. } => Some(*netuid),
            _ => None,
        };

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
        Proposals::<T>::insert(&proposal_id, proposal);
        Event::<T>::ProposalVoted(proposal_id, key, agree);
        Ok(())
    }

    /// Unregister the vote on a proposal
    pub fn do_unregister_vote(origin: T::RuntimeOrigin, proposal_id: u64) -> DispatchResult {
        let key = ensure_signed(origin)?;

        // Get the proposal from storage
        let Ok(mut proposal) = Proposals::<T>::try_get(&proposal_id) else {
            return Err(Error::<T>::ProposalNotFound.into());
        };

        // Check if the proposal is in a valid state for unregistering a vote
        ensure!(
            proposal.proposal_status == ProposalStatus::Pending,
            Error::<T>::InvalidProposalStatus
        );

        let removed = proposal.votes_for.remove(&key) || proposal.votes_against.remove(&key);

        // Check if the voter has actually voted on the proposal
        ensure!(removed, Error::<T>::VoteNotFound);

        // Update the proposal in storage
        Proposals::<T>::insert(&proposal_id, proposal);
        Event::<T>::ProposalVoteUnregistered(proposal_id, key);
        Ok(())
    }

    pub(crate) fn resolve_proposals(block_number: u64) {
        for (proposal_id, proposal) in Proposals::<T>::iter() {
            if !matches!(proposal.proposal_status, ProposalStatus::Pending) {
                continue;
            }

            let netuid = match &proposal.data {
                ProposalData::SubnetParams { netuid, .. } => Some(*netuid),
                _ => None,
            };

            let votes_for: u64 =
                proposal.votes_for.iter().map(|id| Self::get_account_stake(id, netuid)).sum();
            let votes_against: u64 = proposal
                .votes_against
                .iter()
                .map(|id| Self::get_account_stake(id, netuid))
                .sum();

            let total_stake = votes_for + votes_against;
            let minimal_stake_to_execute = Self::get_minimal_stake_to_execute(netuid);
            let is_approved = votes_for >= votes_against;

            if total_stake > minimal_stake_to_execute {
                // use the result to check for err
                Self::execute_proposal(proposal, is_approved);
            } else if block_number >= proposal.expiration_block {
                Proposals::<T>::remove(&proposal_id);
            }
        }
    }

    fn execute_proposal(mut proposal: Proposal<T>, is_approved: bool) {
        // Update the proposal status based on the approval
        proposal.proposal_status = if is_approved {
            ProposalStatus::Accepted
        } else {
            ProposalStatus::Refused
        };

        // Perform actions based on the proposal data type
        match &proposal.data {
            ProposalData::Custom(_) => {
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
                // Update the subnet parameters
                Self::set_subnet_params(*netuid, params.clone());

                // Emit the SubnetParamsUpdated event
                Self::deposit_event(Event::SubnetParamsUpdated(*netuid));
            }
        }

        // Update the proposal in storage
        Proposals::<T>::insert(proposal.id, proposal);
    }

    // returns how much stake is needed to execute a proposal
    pub fn get_minimal_stake_to_execute(netuid: Option<u16>) -> u64 {
        // in Percent
        let threshold: Percent = Self::get_proposal_participation_threshold();

        let needed_stake = match netuid {
            Some(specific_netuid) => {
                let subnet_stake = TotalStake::<T>::get(specific_netuid);
                (subnet_stake.saturated_into::<u128>() * threshold.deconstruct() as u128 / 100)
                    as u64
            }
            None => {
                let global_stake = Self::total_stake();
                (global_stake.saturated_into::<u128>() * threshold.deconstruct() as u128 / 100)
                    as u64
            }
        };

        needed_stake
    }
}
