use core::ops::Add;

use frame_support::{pallet_prelude::DispatchResult};

use super::*;

impl<T: Config> Pallet<T> {
    pub fn proposal_global_update(
        origin: T::RuntimeOrigin,
        max_name_length: u16,
		max_allowed_subnets: u16,
		max_allowed_modules: u16,
		max_registrations_per_block: u16,
		unit_emission: u64, 
		tx_rate_limit: u64
    ) -> DispatchResult {
        ensure_signed(origin)?;

        assert!(max_name_length > 0, "Invalid max_name_length");
        assert!(max_allowed_subnets > 0, "Invalid max_allowed_subnets");
        assert!(max_allowed_modules > 0, "Invalid max_allowed_modules");
        assert!(max_registrations_per_block > 0, "Invalid max_registrations_per_block");
        assert!(unit_emission > 0, "Invalid unit_emission");
        assert!(tx_rate_limit > 0, "Invalid tx_rate_limit");

        let last_id = GlobalUpdateProposalLastId::<T>::get();

        ensure!(
            last_id < u64::MAX,
            Error::<T>::TooMuchUpdateProposals
        );

        GlobalUpdateProposalLastId::<T>::mutate(|last_id| {
            *last_id = last_id.add(1);
        });

        GlobalUpdateProposals::<T>::insert(last_id,
            GlobalUpdateProposal {
                params: GlobalParams {
                    max_name_length,
                    max_allowed_subnets,
                    max_allowed_modules,
                    max_registrations_per_block,
                    unit_emission,
                    tx_rate_limit
                },
                votes: 0,
                participants: vec![],
                accepted: false
            }
        );

        Ok(())
    }

    pub fn stake_global_update(
        origin: T::RuntimeOrigin,
        proposal_id: u64
    ) -> DispatchResult {
        let key = ensure_signed(origin)?;

        let last_proposal_id = GlobalUpdateProposalLastId::<T>::get();

        ensure!(
            proposal_id < last_proposal_id,
            Error::<T>::InvalidProposalId
        );

        ensure!(
            Self::is_vote_available(&key, proposal_id, true),
            Error::<T>::UpdateProposalVoteNotAvailable
        );

        let total_stake_to = Self::get_total_global_stake(&key);

        GlobalUpdateProposals::<T>::mutate(proposal_id, |proposal| {
            proposal.votes += total_stake_to;
            proposal.participants.push(key.clone());
        });

        Ok(())
    }

    pub fn do_global_update(
        origin: T::RuntimeOrigin,
        proposal_id: u64
    ) -> DispatchResult {
        ensure_signed(origin)?;

        let last_proposal_id = GlobalUpdateProposalLastId::<T>::get();

        ensure!(
            proposal_id < last_proposal_id,
            Error::<T>::InvalidProposalId
        );

        let total_stake = Self::total_stake();
        let proposal = GlobalUpdateProposals::<T>::get(proposal_id);

        ensure!(
            proposal.votes > total_stake / 2, Error::<T>::NotEnoughVotesToAccept
        );

        let params = proposal.params;

        Self::set_max_name_length(params.max_name_length);
        Self::set_max_allowed_subnets(params.max_allowed_subnets);
        Self::set_max_allowed_modules(params.max_allowed_modules);
        Self::set_max_registrations_per_block(params.max_registrations_per_block);
        Self::set_unit_emission(params.unit_emission);
        Self::set_tx_rate_limit(params.tx_rate_limit);

        GlobalUpdateProposals::<T>::mutate(proposal_id, |proposal| {
            proposal.accepted = true;
        });

        Self::deposit_event(
            Event::GlobalUpdate(
                params.max_name_length,
                params.max_allowed_subnets,
                params.max_allowed_modules,
                params.max_registrations_per_block,
                params.unit_emission,
                params.tx_rate_limit
            )
        );

        Ok(())
    }

    pub fn proposal_network_update(
        origin: T::RuntimeOrigin,
		netuid: u16,
		name: Vec<u8>,
		tempo: u16,
		immunity_period: u16,
		min_allowed_weights: u16,
		max_allowed_weights: u16,
		max_allowed_uids: u16,
        burn_rate: u16,
		min_stake: u64,
		vote_period: u16,
		vote_threshold: u16,
    ) -> DispatchResult {
        ensure_signed(origin)?;

        ensure!(
            Self::if_subnet_netuid_exists(netuid),
            Error::<T>::SubnetNameNotExists
        );

        let last_id = SubnetUpdateProposalLastId::<T>::get();

        ensure!(
            last_id < u64::MAX,
            Error::<T>::TooMuchUpdateProposals
        );

        SubnetUpdateProposalLastId::<T>::mutate(|last_id| {
            *last_id = last_id.add(1);
        });

        let founder = Founder::<T>::get(netuid);

        SubnetUpdateProposals::<T>::insert(last_id,
            SubnetUpdateProposal {
                params: SubnetParams {
                    name,
                    tempo,
                    immunity_period,
                    min_allowed_weights,
                    max_allowed_weights,
                    max_allowed_uids,
                    burn_rate,
                    min_stake,
                    founder,
                    vote_period,
                    vote_threshold,
                },
                votes: 0,
                participants: vec![],
                accepted: false
            }
        );

        Ok(())
    }

    pub fn stake_subnet_update(
        origin: T::RuntimeOrigin,
        proposal_id: u64
    ) -> DispatchResult {
        let key = ensure_signed(origin)?;

        let last_proposal_id = SubnetUpdateProposalLastId::<T>::get();

        ensure!(
            proposal_id < last_proposal_id,
            Error::<T>::InvalidProposalId
        );

        ensure!(
            Self::is_vote_available(&key, proposal_id, false),
            Error::<T>::UpdateProposalVoteNotAvailable
        );

        let total_stake_to = Self::get_total_global_stake(&key);
        SubnetUpdateProposals::<T>::mutate(proposal_id, |proposal| {
            proposal.votes += total_stake_to;
            proposal.participants.push(key.clone());
        });

        Ok(())
    }

    pub fn do_subnet_update(
        origin: T::RuntimeOrigin,
        proposal_id: u64
    ) -> DispatchResult {
        ensure_signed(origin)?;

        let last_proposal_id = SubnetUpdateProposalLastId::<T>::get();

        ensure!(
            proposal_id < last_proposal_id,
            Error::<T>::InvalidProposalId
        );

        let proposal = SubnetUpdateProposals::<T>::get(proposal_id);
        let netuid = Self::get_netuid_for_name(proposal.params.name.clone());

        let total_stake = Self::get_total_subnet_stake(netuid);

        ensure!(
            proposal.votes > total_stake / 2,
            Error::<T>::NotEnoughVotesToAccept
        );

        let params = proposal.params;

        Self::update_network_for_netuid(
			netuid,
            params.name,
            params.tempo,
            params.immunity_period,
            params.min_allowed_weights,
            params.max_allowed_weights,
            params.max_allowed_uids,
            params.burn_rate,
            params.min_stake,
            params.founder,
		);

        SubnetUpdateProposals::<T>::mutate(proposal_id, |proposal| {
            proposal.accepted = true;
        });

        Ok(())
    }

    pub fn is_vote_available(
        key: &T::AccountId,
        proposal_id: u64,
        is_global: bool
    ) -> bool {
        if is_global {
            !GlobalUpdateProposals::<T>::get(proposal_id).participants.contains(key)
            && !GlobalUpdateProposals::<T>::get(proposal_id).accepted
        } else {
            !SubnetUpdateProposals::<T>::get(proposal_id).participants.contains(key)
            && !SubnetUpdateProposals::<T>::get(proposal_id).accepted
        }
    }

    pub fn get_total_global_stake(
        key: &T::AccountId,
    ) -> u64 {
        let total_networks: u16 = TotalSubnets::<T>::get();
        let mut total_stake_to = 0;

        for netuid in 0..total_networks {
            total_stake_to += Self::get_total_stake_to(netuid, key);
        }

        total_stake_to
    }
}
