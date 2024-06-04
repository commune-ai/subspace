use crate::{proposal::ProposalStatus, *};
use frame_support::pallet_prelude::DispatchResult;
use frame_system::ensure_signed;
use pallet_subspace::Pallet as PalletSubspace;

impl<T: Config> Pallet<T> {
    /// Votes on proposals,
    pub fn do_vote_proposal(
        origin: T::RuntimeOrigin,
        proposal_id: u64,
        agree: bool,
    ) -> DispatchResult {
        let key = ensure_signed(origin)?;

        let Ok(mut proposal) = Proposals::<T>::try_get(proposal_id) else {
            return Err(Error::<T>::ProposalNotFound.into());
        };

        let subnet_id = proposal.subnet_id();
        let ProposalStatus::Open {
            votes_for,
            votes_against,
        } = &mut proposal.status
        else {
            return Err(Error::<T>::ProposalClosed.into());
        };

        ensure!(
            !votes_for.contains(&key) && !votes_against.contains(&key),
            Error::<T>::AlreadyVoted
        );

        let voter_stake = dbg!(PalletSubspace::<T>::get_account_stake(&key, subnet_id));

        ensure!(voter_stake > 0, Error::<T>::InsufficientStake);

        let has_stake_from = || {
            pallet_subspace::StakeFrom::<T>::iter()
                .any(|(_, k, stakes)| k == key && !stakes.is_empty())
        };

        if DelegatingVotingPower::<T>::get().contains(&key) && !has_stake_from() {
            return Err(Error::<T>::VoterIsDelegatingVotingPower.into());
        }

        if agree {
            votes_for
                .try_insert(key.clone())
                .map_err(|_| Error::<T>::InvalidProposalVotingParameters)?;
        } else {
            votes_against
                .try_insert(key.clone())
                .map_err(|_| Error::<T>::InvalidProposalVotingParameters)?;
        }

        Proposals::<T>::insert(proposal_id, proposal);
        Self::deposit_event(Event::<T>::ProposalVoted(proposal_id, key, agree));
        Ok(())
    }

    /// Unregister the vote on a proposal
    pub fn do_remove_vote_proposal(origin: T::RuntimeOrigin, proposal_id: u64) -> DispatchResult {
        let key = ensure_signed(origin)?;

        let Ok(mut proposal) = Proposals::<T>::try_get(proposal_id) else {
            return Err(Error::<T>::ProposalNotFound.into());
        };

        let ProposalStatus::Open {
            votes_for,
            votes_against,
        } = &mut proposal.status
        else {
            return Err(Error::<T>::ProposalClosed.into());
        };

        let removed = votes_for.remove(&key) || votes_against.remove(&key);

        // Check if the voter has actually voted on the proposal
        ensure!(removed, Error::<T>::NotVoted);

        // Update the proposal in storage
        Proposals::<T>::insert(proposal.id, proposal);
        Self::deposit_event(Event::<T>::ProposalVoteUnregistered(proposal_id, key));
        Ok(())
    }
}
