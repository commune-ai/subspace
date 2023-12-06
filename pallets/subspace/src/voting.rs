use core::ops::Add;

use frame_support::{pallet_prelude::DispatchResult};
use scale_info::prelude::string::String;

use super::*;
use crate::utils::{is_vec_str};

impl<T: Config> Pallet<T> {

    pub fn num_proposals() -> u64 {
        return Proposals::<T>::iter().count() as u64;
    }

    pub fn next_proposal_id() -> u64 {
        let mut next_proposal_id: u64 = 0;
        while Self::proposal_exists(next_proposal_id) {
            next_proposal_id = next_proposal_id + 1;
        }
        return next_proposal_id;
    }

    pub fn has_max_proposals() -> bool {
        return Self::num_proposals() <  MaxProposals::<T>::get()
    }

    pub fn check_proposal(proposal: Proposal<T>) -> DispatchResult {
        
        // remove lowest voted proposal
        if Self::has_max_proposals() {
            let mut least_voted_proposal_id: u64 = 0;
            let mut least_votes: u64 = 0;
    
            for (proposal_id, proposal) in Proposals::<T>::iter() {
                if proposal.votes < least_votes {
                    least_votes = proposal.votes;
                    least_voted_proposal_id = proposal_id;
                }
            }

            assert!(proposal.votes > least_votes);
            Proposals::<T>::remove(least_voted_proposal_id);

        }

        let mode = proposal.mode.clone();
        
        // check if proposal is valid
        if is_vec_str(mode.clone(), "global") {
            Self::check_global_params(proposal.global_params)?;
        } else if is_vec_str(mode.clone(), "subnet") {

            Self::check_subnet_params(proposal.subnet_params.clone())?;
            //  check if vote mode is valid
            assert!(
                is_vec_str(proposal.subnet_params.vote_mode.clone(),"stake") ||
                is_vec_str(proposal.subnet_params.vote_mode.clone(),"quadratic")
            );
        } else {
            assert!(proposal.data.len() > 0);
        }
        // check if proposal is valid
        assert!(proposal.data.len() < 256); 
        // avoid an exploit with large data, cap it at 256 bytes
        Ok(())
    }

    pub fn is_proposal_owner(
        // check if the key is the owner of the proposal
        key: &T::AccountId,
        proposal_id: u64,
    ) -> bool {
        let proposal: Proposal<T> = Proposals::<T>::get(proposal_id);
        return proposal.participants.contains(key);
    }

    pub fn do_update_proposal(
        origin: T::RuntimeOrigin,
        proposal_id: u64,
        proposal: Proposal<T>,
    ) -> DispatchResult {
        // update proposal
        let key = ensure_signed(origin)?;
        let mut proposal = proposal;
        assert!( Self::is_proposal_owner(&key, proposal_id), "not proposal owner");
        Self::check_proposal(proposal.clone())?; // check if proposal is valid
        Proposals::<T>::insert(proposal_id, proposal);
        Ok(())
    }

    pub fn do_add_proposal(
        origin: T::RuntimeOrigin,
        mut proposal:Proposal<T>,
    ) -> DispatchResult {
        let key =  ensure_signed(origin)?;

        if is_vec_str(proposal.mode.clone(),"subnet") {
            proposal.votes = Self::get_total_stake_to(proposal.netuid, &key);
        }
        else {
            proposal.votes = Self::get_total_global_stake(&key);
        } 
        // add the proposal owner to the participants
        proposal.participants.push(key.clone());

        Self::check_proposal(proposal.clone())?; // check if proposal is valid
        Proposals::<T>::insert(Self::next_proposal_id(), proposal);
        Ok(())
    }

    pub fn do_vote_proposal(
        origin: T::RuntimeOrigin,
        proposal_id: u64
    ) -> DispatchResult {
        let key = ensure_signed(origin)?;
        let proposal = Proposals::<T>::get(proposal_id);
        ensure!(
            Self::is_vote_available(&key, proposal_id),
            Error::<T>::UpdateProposalVoteNotAvailable
        );

        let mut voting_power : u64;
        let mut stake_threshold: u64; 
        let mut total_stake : u64 = Self::total_stake();
        let mut voting_power = Self::get_total_global_stake(&key);

        let current_global_params: GlobalParams = Self::global_params();
        let current_subnet_params: SubnetParams = Self::subnet_params(proposal.netuid);
        
        let mut stake_threshold: u64;

        if is_vec_str(proposal.mode.clone(),"subnet") {
            
            total_stake = Self::get_total_subnet_stake(proposal.netuid);
            voting_power = Self::get_total_stake_to(proposal.netuid, &key);
            stake_threshold = (total_stake * current_subnet_params.vote_threshold as u64) / 100;

        } else {
            stake_threshold = (total_stake * current_global_params.vote_threshold as u64) / 100;
        }

        Proposals::<T>::mutate(proposal_id, |proposal| {
            proposal.votes += voting_power;
            proposal.participants.push(key.clone());
        });

        

        let total_stake = Self::total_stake();
        let proposal = Proposals::<T>::get(proposal_id);

        if proposal.votes >  stake_threshold  {

            Proposals::<T>::remove(proposal_id);
            
            if is_vec_str(proposal.mode.clone(), "subnet") {
                Self::set_subnet_params(proposal.netuid, proposal.subnet_params);
                Self::deposit_event(Event::SubnetProposalAccepted(proposal_id, proposal.netuid));
    
            } else if is_vec_str(proposal.mode.clone(), "global") {
                Self::set_global_params(proposal.global_params);
                Self::deposit_event(Event::GlobalProposalAccepted(proposal_id));

            } 
        }

        Ok(())
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

