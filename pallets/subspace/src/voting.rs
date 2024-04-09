use super::*;
use frame_support::pallet_prelude::DispatchResult;

/*
# Voting, holds all of the governance logic for the blockchain

## Logical Technical Details

- A proposal can be of 3 types:
    - Custom Proposal
    - Global Proposal
    - Subnet Proposal


*/

impl<T: Config> Pallet<T> {
    // helper function to add a proposal
    pub fn add_proposal(data: ProposalData<T>) -> DispatchResult {
        todo!()
    }

    // Proposal with custom text in it
    pub fn do_add_custom_proposal(origin: T::RuntimeOrigin, data: Vec<u8>) -> DispatchResult {
        // check for length of data and ensure valid contents
        todo!()
    }

    // Proposal to change the globala parameters
    // changing the blockchain ownership multisignature is not `YET` allowed
    pub fn do_add_global_proposal(
        origin: T::RuntimeOrigin,
        params: GlobalParams,
    ) -> DispatchResult {
        let key = ensure_signed(origin)?;
        todo!()
    }

    // Proposal to change subnet parameters
    // Subnet has to be on a "vote" mode, otherwise this proposal will throw an error
    pub fn do_add_subnet_proposal(
        origin: T::RuntimeOrigin,
        netuid: u16,
        params: SubnetParams<T>,
    ) -> DispatchResult {
        todo!()
    }

    // Votes on proposals,
    // ! Important:
    // With all proposals stake
    pub fn do_vote_proposal(origin: T::RuntimeOrigin, proposal_id: u64) -> DispatchResult {
        todo!()
    }

    // Unregister the vote on a proposal
    pub fn do_unregister_vote(origin: T::RuntimeOrigin) -> DispatchResult {

        Ok(())
    }

    pub fn execute_proposal(proposal_id: u64) -> DispatchResult {
        todo!()
    }

    pub fn get_minimal_stake_to_execute(netuid: Option<u16>) -> u64 {
        todo!()
    }
}
