use super::*;
use frame_support::pallet_prelude::DispatchResult;
use sp_runtime::{DispatchError, Percent, SaturatedConversion};

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
        T::Currency::withdraw(
            &key,
            stake_amount.into(),
            WithdrawReasons::TRANSFER,
            ExistenceRequirement::KeepAlive,
        )?;

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

    // Proposal to change the global parameters
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
    // Subnet has to be on a "vote" mode, otherwise this proposal will throw an error
    pub fn do_add_subnet_proposal(
        origin: T::RuntimeOrigin,
        netuid: u16,
        params: SubnetParams<T>,
    ) -> DispatchResult {
        let key = ensure_signed(origin)?;
        // TODO: Luiz pls change the data type of the subnet vote mode to enum
        // make sure that vote mode is authority, if not throw an error
        let vote_mode = VoteModeSubnet::<T>::get(netuid);
        /*
                #[pallet::type_value]
        pub fn DefaultVoteMode<T: Config>() -> Vec<u8> {
            "authority".as_bytes().to_vec()
        }
        #[pallet::storage] // --- MAP ( netuid ) --> epoch
        pub type VoteModeSubnet<T> =
            StorageMap<_, Identity, u16, Vec<u8>, ValueQuery, DefaultVoteMode<T>>;
         */

        Self::check_subnet_params(params.clone())?;

        let proposal_data = ProposalData::SubnetParams { netuid, params };
        Self::add_proposal(key, proposal_data)
    }

    // Votes on proposals,
    pub fn do_vote_proposal(
        origin: T::RuntimeOrigin,
        proposal_id: u64,
        agree: bool,
    ) -> DispatchResult {
        let key = ensure_signed(origin)?;

        // Check if the proposal exists
        ensure!(
            Proposals::<T>::contains_key(&proposal_id),
            Error::<T>::ProposalNotFound
        );

        // Get the proposal from storage
        let mut proposal = Proposals::<T>::get(&proposal_id);

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
            ProposalData::SubnetParams { netuid, .. } => Some(netuid),
            _ => None,
        };

        // Get the voter's stake

        let voter_stake = Self::get_account_stake(&key, netuid);

        // Check if the voter has non-zero stake
        ensure!(voter_stake > 0, Error::<T>::InsufficientStake);

        // Update the proposal based on the vote
        if agree {
            proposal.votes_for.insert(key.clone());
        } else {
            proposal.votes_against.insert(key.clone());
        }

        // Update the proposal in storage
        Proposals::<T>::insert(&proposal_id, proposal);

        Ok(())
    }
    // Unregister the vote on a proposal
    // Unregister the vote on a proposal
    pub fn do_unregister_vote(origin: T::RuntimeOrigin, proposal_id: u64) -> DispatchResult {
        let key = ensure_signed(origin)?;

        // Check if the proposal exists
        ensure!(
            Proposals::<T>::contains_key(&proposal_id),
            Error::<T>::ProposalNotFound
        );

        // Get the proposal from storage
        let mut proposal = Proposals::<T>::get(&proposal_id);

        // Check if the proposal is in a valid state for unregistering a vote
        ensure!(
            proposal.proposal_status == ProposalStatus::Pending,
            Error::<T>::InvalidProposalStatus
        );

        // Check if the voter has actually voted on the proposal
        ensure!(
            proposal.votes_for.contains(&key) || proposal.votes_against.contains(&key),
            Error::<T>::VoteNotFound
        );

        // Remove the voter's vote from the proposal
        if proposal.votes_for.contains(&key) {
            proposal.votes_for.remove(&key);
        } else if proposal.votes_against.contains(&key) {
            proposal.votes_against.remove(&key);
        }

        // Update the proposal in storage
        Proposals::<T>::insert(&proposal_id, proposal);

        Ok(())
    }

    pub fn execute_proposal(proposal_id: u64, is_approved: bool) -> DispatchResult {
        // Get the proposal from storage
        let mut proposal = Proposals::<T>::get(proposal_id).ok_or(Error::<T>::ProposalNotFound)?;

        // Update the proposal status based on the approval
        proposal.proposal_status = if is_approved {
            ProposalStatus::Accepted
        } else {
            ProposalStatus::Refused
        };

        // Perform actions based on the proposal data type
        match proposal.data {
            ProposalData::Custom(_) => {
                // No specific action needed for custom proposals
                // The owners will handle the off-chain logic
            }
            ProposalData::GlobalParams(params) => {
                // Update the global parameters
                Self::set_global_params(params);

                // Emit the GlobalParamsUpdated event
                Self::deposit_event(Event::GlobalParamsUpdated(params));
            }
            ProposalData::SubnetParams { netuid, params } => {
                // Update the subnet parameters
                Self::set_subnet_params(netuid, params);

                // Emit the SubnetParamsUpdated event
                Self::deposit_event(Event::SubnetParamsUpdated(netuid));
            }
        }

        // Update the proposal in storage
        Proposals::<T>::insert(proposal_id, proposal);

        Ok(())
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
