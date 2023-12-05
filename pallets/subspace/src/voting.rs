use core::ops::Add;

use frame_support::{pallet_prelude::DispatchResult};
use scale_info::prelude::string::String;

use super::*;

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

    pub fn string2vec(s: &str) -> Vec<u8> {
        let mut v: Vec<u8> = Vec::new();
        for c in s.chars() {
            v.push(c as u8);
        }
        return v;
    }

    pub fn is_string_equal(s1: &str, s2: &str) -> bool {
        let v1: Vec<u8> = Self::string2vec(s1);
        let v2: Vec<u8> = Self::string2vec(s2);
        return v1 == v2;
    }

    pub fn is_string_vec(s1: &str, v2: Vec<u8>) -> bool {
        let v1: Vec<u8> = Self::string2vec(s1);
        return v1 == v2.clone();
    }

    pub fn is_vec_str(v1: Vec<u8>, s2: &str) -> bool {
        let v2: Vec<u8> = Self::string2vec(s2);
        return v1 == v2.clone();
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
        
        if Self::is_vec_str(mode.clone(), "global") {
            Self::check_global_params(proposal.global_params)?;
        } else if Self::is_vec_str(mode.clone(), "subnet") {
            Self::check_subnet_params(proposal.subnet_params)?;
        } else {
            assert!(proposal.data.len() > 0);
        }

        assert!(proposal.data.len() < 256); // avoid an exploit with large data
        Ok(())
    }

    pub fn do_add_proposal(
        origin: T::RuntimeOrigin,
        mut proposal:Proposal<T>,
    ) -> DispatchResult {
        let key =  ensure_signed(origin)?;
        
        let mut total_vote_power: u64 ; 

        if Self::is_vec_str(proposal.mode.clone(),"subnet") {
            assert!(
                    Self::is_vec_str(proposal.subnet_params.vote_mode.clone(),"stake") ||
                    Self::is_vec_str(proposal.subnet_params.vote_mode.clone(),"quadratic")
                );
            proposal.mode = "subnet".as_bytes().to_vec();
            total_vote_power = Self::get_total_stake_to(proposal.netuid, &key);
        }
        else if Self::is_vec_str(proposal.mode.clone(),"global") {
            // assert!(
            //         Self::is_vec_str(proposal.global_params.vote_mode.clone(),"stake") ||
            //         Self::is_vec_str(proposal.global_params.vote_mode.clone(),"quadratic")
            //     );
            // if its a global proposal, we need to set the mode to global
            total_vote_power = Self::get_total_global_stake(&key);
        } else {
            // if its a custom proposal, we need to set the mode to custom
            total_vote_power = Self::get_total_global_stake(&key);
        } 
        proposal.votes = total_vote_power;
        proposal.participants.push(key.clone());
        

        Self::check_proposal(proposal.clone())?; // check if proposal is valid
        let next_proposal_id: u64 = Self::next_proposal_id(); // get next proposal id

        Proposals::<T>::insert(next_proposal_id, proposal);
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
        
        let mut stake_threshold: u64 = (total_stake * current_global_params.vote_threshold as u64) / 100;

        if Self::is_vec_str(proposal.mode.clone(),"subnet") {
            
            total_stake = Self::get_total_subnet_stake(proposal.netuid);
            voting_power = Self::get_total_stake_to(proposal.netuid, &key);
            stake_threshold = (total_stake * current_subnet_params.vote_threshold as u64) / 100;

        } 

        Proposals::<T>::mutate(proposal_id, |proposal| {
            proposal.votes += voting_power;
            proposal.participants.push(key.clone());
        });

        

        let total_stake = Self::total_stake();
        let proposal = Proposals::<T>::get(proposal_id);

        if proposal.votes >  stake_threshold  {

            Proposals::<T>::mutate(proposal_id, |proposal| {
                proposal.accepted = true;
                proposal.participants = Vec::new();
                proposal.votes = 0;
            });
    
            if Self::is_vec_str(proposal.mode.clone(), "subnet") {
                Self::set_subnet_params(proposal.netuid, proposal.subnet_params);
    
            } else if Self::is_vec_str(proposal.mode.clone(), "global") {
                Self::set_global_params(proposal.global_params);
            } 
        }

        Ok(())
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

