use crate::{
    proposal::{check_senate_approval, ProposalStatus},
    *,
};
use frame_support::pallet_prelude::DispatchResult;
use frame_system::ensure_signed;

impl<T: Config> Pallet<T> {
    /// Votes on proposals,
    pub fn do_vote_proposal(
        origin: T::RuntimeOrigin,
        proposal_id: u64,
        agree: bool,
    ) -> DispatchResult {
        let key = ensure_signed(origin)?;
        let is_senate = Self::is_senate_member(key.clone());

        let Ok(mut proposal) = Proposals::<T>::try_get(proposal_id) else {
            return Err(Error::<T>::ProposalNotFound.into());
        };

        let ProposalStatus::Open {
            votes_for,
            votes_against,
            ..
        } = &mut proposal.status
        else {
            return Err(Error::<T>::ProposalClosed.into());
        };

        ensure!(
            !votes_for.contains(&key) && !votes_against.contains(&key),
            Error::<T>::AlreadyVoted
        );

        if !is_senate {
            // Skip stake requirements for senate

            let voter_delegated_stake = pallet_subspace::Pallet::<T>::get_delegated_stake(&key);
            let voter_owned_stake = pallet_subspace::Pallet::<T>::get_owned_stake(&key);

            ensure!(
                voter_delegated_stake > 0 || voter_owned_stake > 0,
                Error::<T>::InsufficientStake
            );

            if !NotDelegatingVotingPower::<T>::get().contains(&key) && voter_delegated_stake == 0 {
                return Err(Error::<T>::VoterIsDelegatingVotingPower.into());
            }
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

        Proposals::<T>::insert(proposal_id, &proposal);
        Self::deposit_event(Event::<T>::ProposalVoted(proposal_id, key, agree));

        if is_senate {
            check_senate_approval(proposal)
        } else {
            Ok(())
        }
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
            ..
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
