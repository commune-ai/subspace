#![cfg(feature = "runtime-benchmarks")]

use crate::{voting::VoteMode, Pallet as SubspaceMod, *};
use frame_benchmarking::{account, benchmarks};
use frame_system::RawOrigin;
pub use pallet::*;
use sp_arithmetic::per_things::Percent;
use sp_std::vec::Vec;

fn register_mock<T: Config>(
    key: T::AccountId,
    module_key: T::AccountId,
    stake: u64,
    name: Vec<u8>,
) -> Result<(), &'static str> {
    let address = "test".as_bytes().to_vec();
    let network = "testnet".as_bytes().to_vec();
    BurnConfig::<T>::mutate(|cfg| cfg.min_burn = 0);
    SubspaceMod::<T>::add_balance_to_account(
        &key,
        SubspaceMod::<T>::u64_to_balance(stake + 2000).unwrap(),
    );
    let metadata = Some("metadata".as_bytes().to_vec());
    SubspaceMod::<T>::register(
        RawOrigin::Signed(key).into(),
        network,
        name,
        address,
        stake.into(),
        module_key,
        metadata,
    )?;
    Ok(())
}

fn submit_dao_application<T: Config>() -> Result<(), &'static str> {
    // First add the application
    let caller: T::AccountId = account("Alice", 0, 1);
    let application_key: T::AccountId = account("Bob", 0, 2);
    SubspaceMod::<T>::add_balance_to_account(
        &caller,
        SubspaceMod::<T>::u64_to_balance(1_000_000_000_000_000).unwrap(),
    );
    let data = "test".as_bytes().to_vec();
    SubspaceMod::<T>::add_dao_application(RawOrigin::Signed(caller).into(), application_key, data)?;
    Ok(())
}

const REMOVE_WHEN_STAKING: u64 = 500;

benchmarks! {
    // ---------------------------------
    // Consensus operations
    // ---------------------------------

    // 0
    set_weights {
        let netuid = 0;
        let module_key: T::AccountId = account("ModuleKey", 0, 2);
        let module_key2: T::AccountId = account("ModuleKey2", 0, 3);
        let stake = 100000000000000u64;
        register_mock::<T>(module_key.clone(), module_key.clone(), stake, "test".as_bytes().to_vec())?;
        register_mock::<T>(module_key2.clone(), module_key2.clone(), stake, "test1".as_bytes().to_vec())?;
        let uids = vec![0];
        let weights = vec![10];
    }: set_weights(RawOrigin::Signed(module_key2), netuid, uids, weights)

    // ---------------------------------
    // Stake operations
    // ---------------------------------

    // 1
    add_stake {
        let key: T::AccountId = account("Alice", 0, 1);
        let netuid = 0;
        let module_key: T::AccountId = account("ModuleKey", 0, 2);
        let stake = 100000000000000u64;
        SubspaceMod::<T>::add_balance_to_account(
            &key,
            SubspaceMod::<T>::u64_to_balance(stake + 2000).unwrap(),
        );
        register_mock::<T>(module_key.clone(), module_key.clone(), stake.clone(), "test".as_bytes().to_vec())?;
    }: add_stake(RawOrigin::Signed(key), netuid, module_key, stake)

    // 2
    remove_stake {
        let caller: T::AccountId = account("Alice", 0, 1);
        let netuid = 0;
        let module_key: T::AccountId = account("ModuleKey", 0, 2);
        let stake = 100000000000000u64;
        register_mock::<T>(module_key.clone(), module_key.clone(), stake, "test".as_bytes().to_vec())?;
        let amount = 100000;
        SubspaceMod::<T>::add_balance_to_account(
            &caller,
            SubspaceMod::<T>::u64_to_balance(amount).unwrap(),
        );
        SubspaceMod::<T>::add_stake(RawOrigin::Signed(caller.clone()).into(), netuid, module_key.clone(), amount - REMOVE_WHEN_STAKING)?;
    }: remove_stake(RawOrigin::Signed(caller), netuid, module_key, amount - REMOVE_WHEN_STAKING)

    // ---------------------------------
    // Bulk stake operations
    // ---------------------------------

    // 3
    add_stake_multiple {
        let caller: T::AccountId = account("Alice", 0, 1);
        let netuid = 0;
        let module_key1: T::AccountId = account("ModuleKey1", 0, 2);
        let module_key2: T::AccountId = account("ModuleKey2", 0, 3);
        let stake = 100000000000000u64;
        register_mock::<T>(module_key1.clone(), module_key1.clone(), stake, "test".as_bytes().to_vec())?;
        register_mock::<T>(module_key2.clone(), module_key2.clone(), stake, "test1".as_bytes().to_vec())?;
        let module_keys = vec![module_key1, module_key2];
        let mut amounts = vec![10000, 20000];
        SubspaceMod::<T>::add_balance_to_account(
            &caller,
            SubspaceMod::<T>::u64_to_balance(amounts.iter().sum::<u64>()).unwrap(),
        );
        // remove REMOVE_WHEN_STAKING from all amounts
        amounts.iter_mut().for_each(|x| *x -= REMOVE_WHEN_STAKING);
    }: add_stake_multiple(RawOrigin::Signed(caller), netuid, module_keys, amounts)

    // 4
    remove_stake_multiple {
        let caller: T::AccountId = account("Alice", 0, 1);
        let netuid = 0;
        let module_key1: T::AccountId = account("ModuleKey1", 0, 2);
        let module_key2: T::AccountId = account("ModuleKey2", 0, 3);
        let stake = 100000000000000u64;
        register_mock::<T>(module_key1.clone(), module_key1.clone(), stake, "test".as_bytes().to_vec())?;
        register_mock::<T>(module_key2.clone(), module_key2.clone(), stake, "test1".as_bytes().to_vec())?;
        let module_keys = vec![module_key1.clone(), module_key2.clone()];
        let mut amounts = vec![1000, 2000];
        SubspaceMod::<T>::add_balance_to_account(
            &caller,
            SubspaceMod::<T>::u64_to_balance(amounts.iter().sum::<u64>()).unwrap(),
        );
        // remove REMOVE_WHEN_STAKING from all amounts
        amounts.iter_mut().for_each(|x| *x -= REMOVE_WHEN_STAKING);
        SubspaceMod::<T>::add_stake_multiple(RawOrigin::Signed(caller.clone()).into(), netuid, module_keys.clone(), amounts.clone())?;
    }: remove_stake_multiple(RawOrigin::Signed(caller), netuid, module_keys, amounts)

    // ---------------------------------
    // Transfers
    // ---------------------------------

    // 5
    transfer_stake {
        let caller: T::AccountId = account("Alice", 0, 1);
        let netuid = 0;
        let module_key: T::AccountId = account("ModuleKey", 0, 2);
        let new_module_key: T::AccountId = account("NewModuleKey", 0, 3);
        let stake = 100000000000000u64;
        register_mock::<T>(module_key.clone(), module_key.clone(), stake, "test".as_bytes().to_vec())?;
        register_mock::<T>(new_module_key.clone(), new_module_key.clone(), stake, "test1".as_bytes().to_vec())?;
        let amount = 10000;
        SubspaceMod::<T>::add_balance_to_account(
            &caller,
            SubspaceMod::<T>::u64_to_balance(amount).unwrap(),
        );
        SubspaceMod::<T>::add_stake(RawOrigin::Signed(caller.clone()).into(), netuid, module_key.clone(), amount - REMOVE_WHEN_STAKING)?;
    }: transfer_stake(RawOrigin::Signed(caller), netuid, module_key, new_module_key, amount - REMOVE_WHEN_STAKING)

    // 6
    transfer_multiple {
        let caller: T::AccountId = account("Alice", 0, 1);
        let dest1: T::AccountId = account("Dest1", 0, 2);
        let dest2: T::AccountId = account("Dest2", 0, 3);
        let destinations = vec![dest1.clone(), dest2.clone()];
        let mut amounts = vec![10000, 20000];
        SubspaceMod::<T>::add_balance_to_account(
            &caller,
            SubspaceMod::<T>::u64_to_balance(amounts.iter().sum()).unwrap(),
        );
        // Reduce by REMOVE_WHEN_STAKING
        amounts.iter_mut().for_each(|x| *x -= REMOVE_WHEN_STAKING);

    }: transfer_multiple(RawOrigin::Signed(caller), destinations, amounts)

    // ---------------------------------
    // Registereing / Deregistering
    // ---------------------------------

    // 7
    register {
        let key: T::AccountId = account("Alice", 0, 1);
        let module_key: T::AccountId = account("ModuleKey", 0, 2);
        let stake = 100000000000000u64;
        SubspaceMod::<T>::add_balance_to_account(
            &key,
            SubspaceMod::<T>::u64_to_balance(stake + 2000).unwrap(),
        );
    }: register(RawOrigin::Signed(key.clone()), "test".as_bytes().to_vec(), "test".as_bytes().to_vec(), "test".as_bytes().to_vec(), stake.into(), module_key.clone(), Some("metadata".as_bytes().to_vec()))

    // 8
    deregister {
        let caller: T::AccountId = account("Alice", 0, 1);
        let netuid = 0;
        let stake = 100000000000000u64;
        register_mock::<T>(caller.clone(), caller.clone(), stake, "test".as_bytes().to_vec())?;
    }: deregister(RawOrigin::Signed(caller), netuid)

    // ---------------------------------
    // Updating
    // ---------------------------------

    // 9
    update_module {
        let caller: T::AccountId = account("Alice", 0, 1);
        let netuid = 0;
        let stake = 100000000000000u64;
        register_mock::<T>(caller.clone(), caller.clone(), stake, "test".as_bytes().to_vec())?;
        let name = "updated_name".as_bytes().to_vec();
        let address = "updated_address".as_bytes().to_vec();
        let delegation_fee = Some(Percent::from_percent(5));
        let metadata = Some("updated_metadata".as_bytes().to_vec());
    }: update_module(RawOrigin::Signed(caller), netuid, name, address, delegation_fee, metadata)


    // 10
    update_subnet {
        let caller: T::AccountId = account("Alice", 0, 1);
        let netuid = 0;
        let stake = 100000000000000u64;
        register_mock::<T>(caller.clone(), caller.clone(), stake, "test".as_bytes().to_vec())?;
        let params = SubspaceMod::<T>::subnet_params(netuid);
        }: update_subnet(
        RawOrigin::Signed(caller),
        netuid,
        params.founder,
        params.founder_share,
        params.immunity_period,
        params.incentive_ratio,
        params.max_allowed_uids,
        params.max_allowed_weights,
        params.min_allowed_weights,
        params.max_weight_age,
        params.min_stake,
        params.name.clone(),
        params.tempo,
        params.trust_ratio,
        params.maximum_set_weight_calls_per_epoch,
        params.vote_mode,
        params.bonds_ma
    )
    // ---------------------------------
    // Subnet 0 DAO
    // ---------------------------------

    // 11
    add_dao_application {
        let caller: T::AccountId = account("Alice", 0, 1);
        let application_key: T::AccountId = account("Bob", 0, 2);
        SubspaceMod::<T>::add_balance_to_account(&caller, SubspaceMod::<T>::u64_to_balance(1_000_000_000_000_000).unwrap());
        let data = "test".as_bytes().to_vec();
    }: add_dao_application(RawOrigin::Signed(caller), application_key, data)

    // 12
    refuse_dao_application {
        // First add the application
        submit_dao_application::<T>()?;
        let caller: T::AccountId = account("Alice", 0, 1);
        Curator::<T>::set(caller.clone());
    }: refuse_dao_application(RawOrigin::Signed(caller), 0)

    // 13
    add_to_whitelist {
        // First add the application
        submit_dao_application::<T>()?;
        let caller: T::AccountId = account("Alice", 0, 1);
        let application_key: T::AccountId = account("Bob", 0, 2);
        Curator::<T>::set(caller.clone());
    }: add_to_whitelist(RawOrigin::Signed(caller), application_key, 1)

    // 14
    remove_from_whitelist {
        // First add the application
        submit_dao_application::<T>()?;
        let caller: T::AccountId = account("Alice", 0, 1);
        let application_key: T::AccountId = account("Bob", 0, 2);
        Curator::<T>::set(caller.clone());
        // Now add it to whitelist
        SubspaceMod::<T>::add_to_whitelist(RawOrigin::Signed(caller.clone()).into(), application_key.clone(), 1)?;
    }: remove_from_whitelist(RawOrigin::Signed(caller), application_key)

    // ---------------------------------
    // Adding proposals
    // ---------------------------------

    // 15
    add_global_proposal {
        let caller: T::AccountId = account("Alice", 0, 1);
        // Add alice funds to submit the proposal
        SubspaceMod::<T>::add_balance_to_account(&caller, SubspaceMod::<T>::u64_to_balance(1_000_000_000_000_000).unwrap());
        // Get the current current parameters
        let params = SubspaceMod::<T>::global_params();
    }: add_global_proposal(
        RawOrigin::Signed(caller),
        params.max_name_length, // max_name_length: max length of a network name
        params.min_name_length, // min_name_length: min length of a network name
        params.max_allowed_subnets, // max_allowed_subnets: max number of subnets allowed
        params.max_allowed_modules, // max_allowed_modules: max number of modules allowed per subnet
        params.max_registrations_per_block, // max_registrations_per_block: max number of registrations per block
        params.max_allowed_weights, // max_allowed_weights: max number of weights per module
        params.burn_config.max_burn, // max_burn: max burn allowed to register
        params.burn_config.min_burn, // min_burn: min burn required to register
        params.floor_delegation_fee, // floor_delegation_fee: min delegation fee
        params.floor_founder_share, // floor_founder_share: min founder share
        params.min_weight_stake, // min_weight_stake: min weight stake required
        params.burn_config.expected_registrations, // target_registrations_per_interval: desired number of registrations per interval
        params.burn_config.adjustment_interval, // target_registrations_interval: the number of blocks that defines the registration interval
        params.burn_config.adjustment_alpha, // adjustment_alpha: adjustment alpha
        params.unit_emission, // unit_emission: emission per block
        params.curator, // curator: subnet 0 dao multisig
        params.subnet_stake_threshold, // subnet_stake_threshold: stake needed to start subnet emission
        params.proposal_cost, // proposal_cost: amount of $COMAI to create a proposal, returned if proposal gets accepted
        params.proposal_expiration, // proposal_expiration: the block number, proposal expires at
        params.proposal_participation_threshold, // proposal_participation_threshold: minimum stake of the overall network stake, in order for proposal to get executed
        params.general_subnet_application_cost // general_subnet_application_cost
    )

    // 16
    add_subnet_proposal {
        let caller: T::AccountId = account("Alice", 0, 1);

        // register the subnet
        register_mock::<T>(caller.clone(), caller.clone(), 100000000000000u64, "test".as_bytes().to_vec())?;

        // Switch the vote mode to vote
        let params = SubspaceMod::<T>::subnet_params(0);

        SubspaceMod::<T>::update_subnet(
            RawOrigin::Signed(caller.clone()).into(),
            0,
            params.founder.clone(),
            params.founder_share,
            params.immunity_period,
            params.incentive_ratio,
            params.max_allowed_uids,
            params.max_allowed_weights,
            params.min_allowed_weights,
            params.max_weight_age,
            params.min_stake,
            params.name.clone(),
            params.tempo,
            params.trust_ratio,
            params.maximum_set_weight_calls_per_epoch,
            VoteMode::Vote,
            params.bonds_ma
        )?;

        // add balance to submit the proposal
        SubspaceMod::<T>::add_balance_to_account(&caller, SubspaceMod::<T>::u64_to_balance(1_000_000_000_000_000).unwrap());

        let founder = params.founder.clone();
        let name = params.name.clone();

    }: add_subnet_proposal(
        RawOrigin::Signed(caller.clone()),
        0,
        founder, // founder: the address of the founder
        name, // name: the name of the subnet
        params.founder_share, // founder_share: the share of the founder
        params.immunity_period, // immunity_period: the period of immunity
        params.incentive_ratio, // incentive_ratio: the incentive ratio
        params.max_allowed_uids, // max_allowed_uids: the max allowed uids
        params.max_allowed_weights, // max_allowed_weights: the max allowed weights
        params.min_allowed_weights, // min_allowed_weights: the min allowed weights
        params.min_stake, // min_stake: the min stake
        params.max_weight_age, // max_weight_age: the max weight age
        params.tempo, // tempo: the tempo
        params.trust_ratio, // trust_ratio: the trust ratio
        params.maximum_set_weight_calls_per_epoch, // maximum_set_weight_calls_per_epoch: the maximum set weight calls per epoch
        params.vote_mode, // vote_mode: the vote mode
        params.bonds_ma // bonds_ma: the bonds ma
    )

    // 17
    add_custom_proposal {
        let caller: T::AccountId = account("Alice", 0, 1);
        // Add alice fund to submit the proposal
        SubspaceMod::<T>::add_balance_to_account(&caller, SubspaceMod::<T>::u64_to_balance(1_000_000_000_000_000).unwrap());
        let data = "test".as_bytes().to_vec();
    }: add_custom_proposal(RawOrigin::Signed(caller), data)


    // 18
    add_custom_subnet_proposal {
        let caller: T::AccountId = account("Alice", 0, 1);
        // The subnet has to exist
        register_mock::<T>(caller.clone(), caller.clone(), 100000000000000u64, "test".as_bytes().to_vec())?;
        // Add alice fund to submit the proposal
        SubspaceMod::<T>::add_balance_to_account(&caller, SubspaceMod::<T>::u64_to_balance(1_000_000_000_000_000).unwrap());
        let data = "test".as_bytes().to_vec();
    }: add_custom_subnet_proposal(RawOrigin::Signed(caller), 0, data)

    // 19
    add_transfer_dao_treasury_proposal {
        let caller: T::AccountId = account("Alice", 0, 1);
        // Add alice fund to submit the proposal
        SubspaceMod::<T>::add_balance_to_account(&caller, SubspaceMod::<T>::u64_to_balance(1_000_000_000_000_000).unwrap());
        let data = "test".as_bytes().to_vec();
        let amount = 0;
        let destinations: T::AccountId = account("Bob", 0, 2);
    }: add_transfer_dao_treasury_proposal(RawOrigin::Signed(caller), data, amount, destinations)

    // ---------------------------------
    // Voting / Unvoting proposals
    // ---------------------------------

    // 20
    vote_proposal {
        let caller: T::AccountId = account("Alice", 0, 1);
        // Register alice such that she has funds to vote
        register_mock::<T>(caller.clone(), caller.clone(), 100000000000000u64, "test".as_bytes().to_vec())?;

        // Add alice fund to submit the proposal
        SubspaceMod::<T>::add_balance_to_account(&caller, SubspaceMod::<T>::u64_to_balance(1_000_000_000_000_000).unwrap());
        // Submit a custom proposal
        let data = "test".as_bytes().to_vec();
        SubspaceMod::<T>::add_custom_proposal(RawOrigin::Signed(caller.clone()).into(), data)?;
        let proposal_id = 0;
        let vote = true;
    }: vote_proposal(RawOrigin::Signed(caller), proposal_id, vote)

    // 21
    unvote_proposal {
        let caller: T::AccountId = account("Alice", 0, 1);
        // Register alice such that she has funds to vote
        register_mock::<T>(caller.clone(), caller.clone(), 100000000000000u64, "test".as_bytes().to_vec())?;
        // Add alice fund to submit the proposal
        SubspaceMod::<T>::add_balance_to_account(&caller, SubspaceMod::<T>::u64_to_balance(1_000_000_000_000_000).unwrap());
        // Submit a custom proposal
        let data = "test".as_bytes().to_vec();
        SubspaceMod::<T>::add_custom_proposal(RawOrigin::Signed(caller.clone()).into(), data)?;
        let proposal_id = 0;
        // Let alice vote on the proposal
        SubspaceMod::<T>::vote_proposal(RawOrigin::Signed(caller.clone()).into(), proposal_id, true)?;
    }: unvote_proposal(RawOrigin::Signed(caller), proposal_id)

    // ---------------------------------
    // Profit sharing
    // ---------------------------------

    // 22
    add_profit_shares {
        let caller: T::AccountId = account("Alice", 0, 1);
        // Register caller
        register_mock::<T>(caller.clone(), caller.clone(), 100000000000000u64, "test".as_bytes().to_vec())?;
        let bob = account("Bob", 0, 2);
        let cecilia = account("Cecilia", 0, 3);
        let shares: Vec<u16> = vec![50, 50];
        let keys = vec![bob, cecilia];
    }: add_profit_shares(RawOrigin::Signed(caller), keys, shares)

    // ---------------------------------
    // Testnet
    // ---------------------------------

    // 23
    // TODO: Add testnet benchmarks later
}
