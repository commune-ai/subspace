use super::*;
use frame_support::pallet_prelude::DispatchResult;

impl<T: Config> Pallet<T> {
    // cancels the vote on the proposal
    pub fn do_unregister_voter(origin: T::RuntimeOrigin) -> DispatchResult {
        todo!()
    }

    // adds proposal to change the globala parameters
    pub fn do_add_global_proposal(
        origin: T::RuntimeOrigin,
        params: GlobalParams,
    ) -> DispatchResult {
        todo!()
    }

    pub fn do_add_custom_proposal(origin: T::RuntimeOrigin, data: Vec<u8>) -> DispatchResult {
        todo!()
    }

    pub fn do_add_subnet_proposal(
        origin: T::RuntimeOrigin,
        netuid: u16,
        params: SubnetParams<T>,
    ) -> DispatchResult {
        todo!()
    }

    pub fn do_vote_proposal(origin: T::RuntimeOrigin, proposal_id: u64) -> DispatchResult {
        todo!()
    }

    pub fn num_proposals() -> u64 {
        todo!()
    }

    pub fn unregister_voter(key: &T::AccountId) {
        todo!()
    }
}
