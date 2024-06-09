# Changelog

## Version 1.7.8

`spec_version: 117`

This version delivers on the [GovernanceProposal](https://governance.communeai.org/proposal/4).

### Introducing the new `GovernanceModule` pallet

This pallet now holds **ALL** governance-related storages and extrinsics.

#### Treasury address

`DaoTreasury` is now `DaoTreasuryAddress`.

This means that the DAO **treasury is now a regular wallet address**. You can query its balance or send funds to it.

#### Proposals

- The following proposal names have been changed:

```txt
add_global_params_proposal
add_subnet_params_proposal
add_global_custom_proposal
add_subnet_custom_proposal
```

The proposals struct keys are now

```py
['id', 'proposer', 'expiration_block', 'data', 'status', 'metadata', 'proposal_cost', 'creation_block']
```

- Parameter proposals now require at least 40% of the network's or subnet's stake to execute.
- Custom proposals remain at 50%.
- Proposals are resolved at the time of their expiration, not after reaching enough participation.
  - Previously, only a maximum of 50% of the network's stake could participate in a proposal. Now, this can be up to 100%.
  - The relevant factor is whether the proposal reached the execution threshold at the time of its expiration.

#### Proposal Rewards

Governance participants are now motivated to participate in governance actions by being allocated rewards, which are funded from the DAO Treasury Address.

The maximum reward allocation per proposal is 10,000 $COMAI (split across the users), with a dynamic decay based on the number of proposals that occurred within the `proposal_reward_interval`. After this interval is finished, the allocation decay is restarted.

#### Delegation of Voting Power

**By default, all users delegate their voting power to the validator they stake to** (this is not the case for the validators themselves, who manage this voting power). If you are not comfortable with a validator managing your voting power, you can always toggle this off and on by calling one of the two extrinsics:

- To disable the delegation:

```rs
#[pallet::call_index(8)]
#[pallet::weight((Weight::zero(), DispatchClass::Normal, Pays::No))]
pub fn disable_vote_power_delegation(origin: OriginFor<T>) -> DispatchResult {
    let key = ensure_signed(origin)?;
    Self::update_delegating_voting_power(&key, false)
}
```

- To enable the delegation:

```rs
#[pallet::call_index(7)]
#[pallet::weight((Weight::zero(), DispatchClass::Normal, Pays::No))]
pub fn enable_vote_power_delegation(origin: OriginFor<T>) -> DispatchResult {
    let key = ensure_signed(origin)?;
    Self::update_delegating_voting_power(&key, true)
}
```

Note that if you delegate voting power to a validator, the validator will be the one receiving the voting rewards, not you as a delegator.

#### Chain Safety

All of the core functions in the on-initialize hook should now be written in much robust way, to avoid possible panics.

#### Subnet & Global Params

- Adjustement alpha is moving from global parameteres to subnet parameters.
- Moving the `MinBurn` and `MaxBurn` storage values to a single storage value, that can be queried at `BurnConfig`.
- Moving `VoteModeSubne` mode to a GovernanceModule and StorageMap `SubnetGovernanceConfig`.

#### Migrating Subnet Owner Fee

The subnet owner fee **floor** is now 16%, with subnet 0 taking 20% (of the subnet emission allocated to the treasury) to motivate subnet staking.
Old values Floor 8%, SN0 12%.

## Version 1.7.6

- Bumps all substrate versions and node to new versions
- Removed the ethereum and EVM pallets
- Fixed max allowed modules
- Fixed panic on YUMA

## Version 1.7.5

Fixing migration

Spec version: `114`

- Moving Dao Treasury to a normal chain account
  - The `GlobalDaoTreasury` will get deleted in the next release,
  currently used only for migration.
- Deleted, or moved useless code and values of:
  - Burn rate (global parameter)
  - Min Stake (global parameter)
- The following storage values were deleted and are now accessible through `BurnConfig`:
  - `MinBurn`
  - `MaxBurn`
- The storage value `RemovedSubnets` is now called `SubnetGaps`
- Moved Adjustment Alpha parameter under the `SubnetParams` so that subnet owners,
can adjust this value at runtime.

## Version 1.7.5

Fixing migration

## Version 1.7.4

Spec version `114`

- Moved global parameters, of:
  - `target_registration_interval`
  - `target_registration_per_interval`
    To subnet owner control
- Decreased `MaxRegistrationPerBlock` from `5`-> `3`
- Introduced a new SubnetParam `MaxRegistrationsPerInterval`,
which defines how many registraions per `target_registration_interval` can happen,
above are rate limited.

## Version 1.7.4

Spec version `114`

- Moved global parameters, of:
  - `target_registration_interval`
  - `target_registration_per_interval`
    To subnet owner control
- Decreased `MaxRegistrationPerBlock` from `5`-> `3`
- Introduced a new SubnetParam `MaxRegistrationsPerInterval`,
which defines how many registraions per `target_registration_interval` can happen,
above are rate limited.

## Version 1.7.3

Spec version: `113`

- Fix s0 whitelist application cost

## Version 1.7.2

Spec version: `112`

- Fix total active subnet stake calculation.

## Version 1.7.1

Spec version: `111`

- Adding log messages.

## Version 1.7.0

Spec version: `110`

- Introducing minimum founder share.
- DAO treasury.
