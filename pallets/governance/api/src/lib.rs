#![no_std]

use frame_support::DebugNoBound;
use parity_scale_codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use sp_runtime::{DispatchResult, Percent};

#[derive(Clone, Copy, Debug, PartialEq, Eq, TypeInfo, Decode, Encode, MaxEncodedLen)]
pub enum VoteMode {
    Authority = 0,
    Vote = 1,
}

#[derive(Clone, TypeInfo, Decode, Encode, PartialEq, Eq, DebugNoBound, MaxEncodedLen)]
pub struct GovernanceConfiguration {
    pub proposal_cost: u64,
    pub proposal_expiration: u32,
    pub vote_mode: VoteMode,
    pub proposal_reward_treasury_allocation: Percent,
    pub max_proposal_reward_treasury_allocation: u64,
    pub proposal_reward_interval: u64,
}

impl Default for GovernanceConfiguration {
    fn default() -> Self {
        Self {
            proposal_cost: 10_000_000_000_000,
            proposal_expiration: 130_000,
            vote_mode: VoteMode::Vote,
            // Maximum allocate 2% of the treasury.
            proposal_reward_treasury_allocation: Percent::from_percent(2),
            // Maximum reward allocation 10K.
            max_proposal_reward_treasury_allocation: 10_000_000_000_000,
            proposal_reward_interval: 75_600,
        }
    }
}

pub trait GovernanceApi<AccountId> {
    /// Gets the account address for the DAO treasury.
    fn get_dao_treasury_address() -> AccountId;

    /// Gets the percentage of the treasury to be distributed in a step.
    fn get_dao_treasury_distribution() -> Percent;

    /// Gets global governance configuration.
    fn get_global_governance_configuration() -> GovernanceConfiguration;

    /// Gets the governance configuration for a given subnet.
    fn get_subnet_governance_configuration(subnet_id: u16) -> GovernanceConfiguration;

    /// Updates the governance configuration of the global network if in authority mode.
    fn update_global_governance_configuration(
        governance_config: GovernanceConfiguration,
    ) -> DispatchResult;

    /// Updates the governance configuration of a subnet in authority mode.
    fn update_subnet_governance_configuration(
        subnet_id: u16,
        governance_config: GovernanceConfiguration,
    ) -> DispatchResult;

    /// Returns whether this account is delegating their voting power to the modules it has stakes
    /// on.
    fn is_delegating_voting_power(delegator: &AccountId) -> bool;

    /// Defines whether this account will delegate their voting power or not. This decision is
    /// global.
    fn update_delegating_voting_power(delegator: &AccountId, delegating: bool) -> DispatchResult;

    /// Handles the deregistration of a subnet.
    fn handle_subnet_removal(subnet_id: u16);

    /// Executes the application in DAO.
    fn execute_application(user_id: &AccountId) -> DispatchResult;

    /// Returns the cost of an application in general subnet.
    fn get_general_subnet_application_cost() -> u64;

    /// Returns bool indicating whether the application on SN0 exists.
    fn curator_application_exists(module_key: &AccountId) -> bool;

    fn get_curator() -> AccountId;

    fn set_curator(key: &AccountId);

    fn set_general_subnet_application_cost(amount: u64);
}
