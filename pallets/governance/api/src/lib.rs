#![no_std]

use sp_runtime::DispatchResult;

pub trait GovernanceApi<AccountId> {
    /// Returns whether this account is delegating their voting power to the modules it has stakes
    /// on.
    fn is_delegating_voting_power(delegator: &AccountId) -> bool;

    /// Defines whether this account will delegate their voting power or not. This decision is
    /// global.
    fn update_delegating_voting_power(delegator: &AccountId, delegating: bool) -> DispatchResult;
}
