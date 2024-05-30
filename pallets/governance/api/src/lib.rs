#![no_std]

use sp_runtime::DispatchError;

pub trait GovernanceApi<AccountId> {
    fn set_delegated_voting_power(
        subnet_id: u16,
        staked: AccountId,
        staker: AccountId,
    ) -> Result<(), DispatchError>;

    fn remove_delegated_voting_power(subnet_id: u16, staked: AccountId, staker: AccountId);

    fn deregister_delegated_voting_power_on_module(subnet_id: u16, staked: AccountId);

    fn deregister_delegated_voting_power_on_subnet(subnet_id: u16);
}
