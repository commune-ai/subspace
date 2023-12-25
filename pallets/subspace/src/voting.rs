use core::ops::Add;

use frame_support::{pallet_prelude::DispatchResult};
use scale_info::prelude::string::String;
use super::*;
use crate::utils::{is_vec_str};

impl<T: Config> Pallet<T> {

    pub fn do_unregister_voter(
        origin: T::RuntimeOrigin,
    ) -> DispatchResult {
        let key = ensure_signed(origin)?;
        ensure!(Self::is_voter_registered(&key), Error::<T>::VoterIsNotRegistered);
        Self::unregister_voter(&key);
        ensure!(!Self::is_voter_registered(&key), Error::<T>::VoterIsRegistered);
        Ok(())
    }

    pub fn do_add_global_proposal(
        origin: T::RuntimeOrigin,
        // params
        params: GlobalParams,
    ) -> DispatchResult {        
        let mut proposal = Self::default_proposal();
        proposal.global_params = params;
        proposal.mode = "global".as_bytes().to_vec();
        Self::do_add_proposal(origin,  proposal)?;
        Ok(())
    }

    pub fn do_add_custom_proposal(
        origin: T::RuntimeOrigin,
        // params
        data: Vec<u8>,
    ) -> DispatchResult {        
        let mut proposal = Self::default_proposal();
        proposal.data = data.clone();
        proposal.mode = "custom".as_bytes().to_vec();
        Self::do_add_proposal(origin,  proposal)?;
        Ok(())
    }


    pub fn do_add_subnet_proposal(
        origin: T::RuntimeOrigin,
        // params
        netuid: u16,
        params: SubnetParams<T>,
    ) -> DispatchResult {        
        let mut proposal = Self::default_proposal();
        proposal.subnet_params = params;
        proposal.netuid = netuid;
        proposal.mode = "subnet".as_bytes().to_vec();
        Self::do_add_proposal(origin,  proposal)?;
        Ok(())
    }



    pub fn is_proposal_participant(
        key: &T::AccountId,
        proposal_id: u64,
    ) -> bool {
        let proposal: Proposal<T> = Proposals::<T>::get(proposal_id);
        return proposal.participants.contains(key);
    }


    pub fn do_add_proposal(
        origin: T::RuntimeOrigin,
        mut proposal:Proposal<T>,
    ) -> DispatchResult {
        let key =  ensure_signed(origin)?;
        // get the voting power of the proposal owner
        if Self::is_voter_registered(&key.clone()) {
            // unregister voter if they are already registered
            Self::unregister_voter(&key.clone());
        }

        let proposal_id = Self::next_proposal_id();
        let voting_power = Self::get_voting_power(&key, proposal.clone());
        let mut voter_info = Voter2Info::<T>::get(key.clone());
        voter_info.proposal_id = proposal_id;
        voter_info.participant_index = proposal.participants.len() as u16;
        voter_info.votes = voting_power;

        // register the voter to avoid double voting
        proposal.participants.push(key.clone());
        proposal.votes = proposal.votes.saturating_add(voting_power);

        Self::check_proposal(proposal.clone())?; // check if proposal is valid

        // update the proposal
        Voter2Info::<T>::insert(key, voter_info);
        Proposals::<T>::insert(proposal_id, proposal);

        Self::check_proposal_approval(proposal_id);
        Ok(())
    }



    pub fn do_vote_proposal(
        origin: T::RuntimeOrigin,
        proposal_id: u64
    ) -> DispatchResult {
        let key = ensure_signed(origin)?;

        ensure!(Self::proposal_exists(proposal_id), Error::<T>::ProposalDoesNotExist);

        // if you vote the proposal on a subnet, you are no longer a participant

        if Self::is_voter_registered(&key.clone()) {
            // unregister voter
            Self::unregister_voter(&key.clone());
        }

        let mut proposal = Proposals::<T>::get(proposal_id);
        

        let mut voting_power : u64 = Self::get_voting_power(&key, proposal.clone());
        ensure!(voting_power > 0, Error::<T>::VotingPowerIsZero);

        // register the voter to avoid double voting


        let mut voter_info = Voter2Info::<T>::get(key.clone());
        voter_info.proposal_id = proposal_id;
        voter_info.participant_index = proposal.participants.len() as u16;
        voter_info.votes = voting_power;
        
        // register the voter to avoid double voting
        proposal.participants.push(key.clone());
        proposal.votes = proposal.votes.saturating_add(voting_power);


        // update the proposal
        Voter2Info::<T>::insert(key, voter_info);
        Proposals::<T>::insert(proposal_id, proposal);


        Self::check_proposal_approval(proposal_id);

        Ok(())
    }
    pub fn num_proposals() -> u64 {
        return Proposals::<T>::iter().count() as u64;
    }

    pub fn next_proposal_id() -> u64 {
        let mut next_proposal_id: u64 = 0;
        // add proposal id until it is not in the map
        while Self::proposal_exists(next_proposal_id) {
            next_proposal_id = next_proposal_id + 1;
        }
        return next_proposal_id;
    }

    pub fn has_max_proposals() -> bool {
        return Self::num_proposals() >=  Self::get_max_proposals();
    }


    pub fn check_proposal(proposal: Proposal<T>) -> DispatchResult {
        
        // remove lowest voted proposal
        if Self::has_max_proposals() {
            let mut least_voted_proposal_id: u64 = u64::MAX;
            let mut least_votes: u64 = u64::MAX;
    
            for (proposal_id, proposal) in Proposals::<T>::iter() {

                // if proposal is accepted, remove it
                if proposal.accepted || proposal.votes == 0{
                    least_votes = 0;
                    least_voted_proposal_id = proposal_id;
                    break
                }

                if proposal.votes < least_votes {
                    least_votes = proposal.votes;
                    least_voted_proposal_id = proposal_id;
                }
            }

            ensure!(proposal.votes > least_votes, Error::<T>::TooFewVotesForNewProposal);

            // remove proposal participants
            let proposal = Proposals::<T>::get(least_voted_proposal_id);
            for participant in proposal.participants {
                Voter2Info::<T>::remove(participant);
            }
            Proposals::<T>::remove(least_voted_proposal_id);
        }

        let mode = proposal.mode.clone();
        
        // check if proposal is valid
        if is_vec_str(mode.clone(), "global") {
            Self::check_global_params(proposal.global_params)?;
        } else if is_vec_str(mode.clone(), "subnet") {

            Self::check_subnet_params(proposal.subnet_params.clone())?;
            //  check if vote mode is valid
            let subnet_params: SubnetParams<T> = Self::subnet_params(proposal.netuid);
            ensure!(
                is_vec_str(subnet_params.vote_mode.clone(),"stake") ||
                is_vec_str(subnet_params.vote_mode.clone(),"quadratic")
            , Error::<T>::InvalidVoteMode);
        } else {
            ensure!(proposal.data.len() > 0, Error::<T>::InvalidProposalData);
        }
        // check if proposal is valid
        ensure!(proposal.data.len() < 256, Error::<T>::ProposalDataTooLarge); 
        // avoid an exploit with large data, cap it at 256 bytes
        Ok(())
    }

    pub fn is_proposal_owner(
        // check if the key is the owner of the proposal
        key: &T::AccountId,
        proposal_id: u64,
    ) -> bool {
        let proposal: Proposal<T> = Proposals::<T>::get(proposal_id);
        if proposal.participants.len() == 0 {
            return false;
        }
        return proposal.participants[0] == *key;
    }
    pub fn default_proposal() -> Proposal<T> {
        let mut proposal = Proposals::<T>::get(u64::MAX);
        return proposal;
    }

    pub fn get_proposal(
        proposal_id: u64,
    ) -> Proposal<T> {
        return Proposals::<T>::get(proposal_id);
    }


    pub fn unregister_voter(key: &T::AccountId) {
        // unregister voter

        // get the proposal id for the voter
        let voter_info = Self::get_voter_info(key);
        // update the proposal votes
        let mut proposal = Self::get_proposal(voter_info.proposal_id);
        
        // remove the voter from the participants
        let index = voter_info.participant_index as usize;
        proposal.participants.remove(index);

        // update the votes
        proposal.votes = proposal.votes.saturating_sub(voter_info.votes);

        // remove proposal if there are no participants
        if proposal.participants.len() == 0 || proposal.votes == 0 {
            // remove proposal if there are no participants
            Proposals::<T>::remove(voter_info.proposal_id);
        } else {
            // update proposal
            Proposals::<T>::insert(voter_info.proposal_id, proposal);
        }

        Voter2Info::<T>::remove(key);
    }



    pub fn is_voter_registered(key: &T::AccountId) -> bool {
        // check if voter is registered
        return Voter2Info::<T>::contains_key(key);
    }

    pub fn get_voter_info(key: &T::AccountId) -> VoterInfo{
        // get the proposal id for the voter
        return Voter2Info::<T>::get(key);
    }



    pub fn get_voting_power(
        key: &T::AccountId,
        proposal: Proposal<T>,
    ) -> u64 {
        let mut voting_power: u64 = 0;
        if is_vec_str(proposal.mode.clone(),"subnet") {
            voting_power = Self::get_total_stake_to(proposal.netuid, key);
        } else {
            // get all of the stake for the key
            voting_power = Self::get_global_stake_to(key);
        }
        return voting_power;
    }

    pub fn get_proposal_vote_threshold(
        proposal_id: u64,
    ) -> u64 {
        let proposal: Proposal<T> = Proposals::<T>::get(proposal_id);
        let mut vote_threshold: u64 = 0;
        if is_vec_str(proposal.mode.clone(),"subnet") {
            let total_stake = Self::get_total_subnet_stake(proposal.netuid);
            vote_threshold = (total_stake * proposal.subnet_params.vote_threshold as u64) / 100;
        } else {
            let total_stake = Self::total_stake();
            vote_threshold = (total_stake * proposal.global_params.vote_threshold as u64) / 100;
        }
        return vote_threshold;
    }



    pub fn check_proposal_approval(proposal_id: u64) {

        let proposal = Proposals::<T>::get(proposal_id);
        let mut stake_threshold: u64 = Self::get_proposal_vote_threshold(proposal_id); 
        if proposal.votes >  stake_threshold  {
            //  unregister all voters

            for participant in proposal.participants {
                Voter2Info::<T>::remove(participant);
            }
            Proposals::<T>::mutate(proposal_id, |proposal| {
                proposal.accepted = true;
                proposal.participants = Vec::new();
            });

            if is_vec_str(proposal.mode.clone(), "subnet") {
                Self::set_subnet_params(proposal.netuid, proposal.subnet_params);
                Self::deposit_event(Event::SubnetProposalAccepted(proposal_id, proposal.netuid));
    
            } else if is_vec_str(proposal.mode.clone(), "global") {
                Self::set_global_params(proposal.global_params);
                Self::deposit_event(Event::GlobalProposalAccepted(proposal_id));
            } else {
                Self::deposit_event(Event::CustomProposalAccepted(proposal_id));
            }
        }
    }


    pub fn get_subnet_proposals(
        netuid: u16
    ) -> Vec<Proposal<T>> {
        let mut proposals: Vec<Proposal<T>> = Vec::new();
        for (proposal_id, proposal) in Proposals::<T>::iter() {
            if is_vec_str(proposal.mode.clone(), "subnet") && proposal.netuid == netuid {
                proposals.push(proposal);
            }
        }
        return proposals;
    }

    pub fn get_global_proposals() -> Vec<Proposal<T>> {
        let mut proposals: Vec<Proposal<T>> = Vec::new();
        for (proposal_id, proposal) in Proposals::<T>::iter() {
            if is_vec_str(proposal.mode.clone(), "global") {
                proposals.push(proposal);
            }
        }
        return proposals;
    }

    

    pub fn num_subnet_proposals(
        netuid: u16
    ) -> u64 {
        let subnet_proposals = Self::get_subnet_proposals(netuid);
        return subnet_proposals.len() as u64;
    }

    pub fn num_global_proposals() -> u64 {
        let global_proposals = Self::get_global_proposals();
        return global_proposals.len() as u64;
    }


    pub fn proposal_exists(
        proposal_id: u64
    ) -> bool {
        Proposals::<T>::contains_key(proposal_id)
    }

    pub fn is_vote_available(
        key: &T::AccountId,
        proposal_id: u64,
    ) -> bool {
        let proposal: Proposal<T> = Proposals::<T>::get(proposal_id);
        let is_vote_available: bool = !proposal.participants.contains(key) && !proposal.accepted; 
        return is_vote_available;
    }
}

