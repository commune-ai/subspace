#![cfg(feature = "runtime-benchmarks")]

use crate::{Pallet as GovernanceMod, *};
use frame_benchmarking::{account, benchmarks};
use frame_system::RawOrigin;
pub use pallet::*;
use pallet_subspace::{Pallet as SubspaceMod, SubnetBurn};
use sp_std::vec::Vec;

fn submit_dao_application<T: Config>() -> Result<(), &'static str> {
    // First add the application
    let caller: T::AccountId = account("Alice", 0, 1);
    let application_key: T::AccountId = account("Bob", 0, 2);
    SubspaceMod::<T>::add_balance_to_account(
        &caller,
        SubspaceMod::<T>::u64_to_balance(1_000_000_000_000_000).unwrap(),
    );
    let data = "test".as_bytes().to_vec();
    GovernanceMod::<T>::add_dao_application(
        RawOrigin::Signed(caller).into(),
        application_key,
        data,
    )?;
    Ok(())
}

fn register_mock<T: Config>(
    key: T::AccountId,
    module_key: T::AccountId,
    name: Vec<u8>,
) -> Result<(), &'static str> {
    let address = "test".as_bytes().to_vec();
    let network = "testnet".as_bytes().to_vec();

    let enough_stake = 10000000000000u64;
    SubspaceMod::<T>::add_balance_to_account(
        &key,
        SubspaceMod::<T>::u64_to_balance(SubnetBurn::<T>::get() + enough_stake).unwrap(),
    );
    let metadata = Some("metadata".as_bytes().to_vec());
    SubspaceMod::<T>::register(
        RawOrigin::Signed(key.clone()).into(),
        network,
        name,
        address,
        module_key.clone(),
        metadata,
    )?;
    SubspaceMod::<T>::increase_stake(&key, &module_key, enough_stake);
    Ok(())
}
benchmarks! {
    //---------------------------------
    //Adding proposals
    //---------------------------------

    // 0
    add_global_params_proposal {
        let caller: T::AccountId = account("Alice", 0, 1);
        // Add Alice's funds to submit the proposal
        SubspaceMod::<T>::add_balance_to_account(
            &caller,
            SubspaceMod::<T>::u64_to_balance(1_000_000_000_000_000).unwrap()
        );

        // Get the current global parameters
        let params = SubspaceMod::<T>::global_params();
        let data = "ipfshash".as_bytes().to_vec();

        // Submit the proposal with the global parameters
        }: add_global_params_proposal(
            RawOrigin::Signed(caller),
            data,
            params.max_name_length,                // max_name_length: max length of a network name
            params.min_name_length,                // min_name_length: min length of a network name
            params.max_allowed_subnets,            // max_allowed_subnets: max number of subnets allowed
            params.max_allowed_modules,            // max_allowed_modules: max number of modules allowed per subnet
            params.max_registrations_per_block,    // max_registrations_per_block: max number of registrations per block
            params.max_allowed_weights,            // max_allowed_weights: max number of weights per module
            params.floor_delegation_fee,           // floor_delegation_fee: min delegation fee
            params.floor_founder_share,            // floor_founder_share: min founder share
            params.min_weight_stake,               // min_weight_stake: min weight stake required
            params.curator,                            // curator: subnet 0 dao multisig
            params.governance_config.proposal_cost,                      // proposal_cost: amount of $COMAI to create a proposal, returned if proposal gets accepted
            params.governance_config.proposal_expiration,                // proposal_expiration: the block number, proposal expires at
            params.general_subnet_application_cost,     // general_subnet_application_cost
            params.kappa,
            params.rho,
            params.subnet_immunity_period
        )


    // 1
    add_subnet_params_proposal {
        let caller: T::AccountId = account("Alice", 0, 1);

        // register the subnet
        register_mock::<T>(caller.clone(), caller.clone(),
    "test".as_bytes().to_vec())?;
        let netuid = SubspaceMod::<T>::get_netuid_for_name("testnet".as_bytes()).unwrap();

        // Switch the vote mode to vote
        let params = SubspaceMod::<T>::subnet_params(netuid);
        let data = "ipfshash".as_bytes().to_vec();

        SubspaceMod::<T>::update_subnet(
            RawOrigin::Signed(caller.clone()).into(),
            netuid,
            params.founder.clone(),
            params.founder_share,
            params.name.clone(),
            params.metadata.clone(),
            params.immunity_period,
            params.incentive_ratio,
            params.max_allowed_uids,
            params.max_allowed_weights,
            params.min_allowed_weights,
            params.max_weight_age,
            params.tempo,
            params.maximum_set_weight_calls_per_epoch,
            VoteMode::Vote,
            params.bonds_ma,
            params.module_burn_config.clone(),
            params.min_validator_stake,
            params.max_allowed_validators
        )?;

        // add balance to submit the proposal
        SubspaceMod::<T>::add_balance_to_account(&caller,
        SubspaceMod::<T>::u64_to_balance(1_000_000_000_000_000).unwrap());

        let founder = params.founder.clone();
        let name = params.name.clone();

    }: add_subnet_params_proposal(
        RawOrigin::Signed(caller),
        netuid,
        data,
        params.founder.clone(),
        params.founder_share,
        params.name.clone(),
        params.metadata.clone(),
        params.immunity_period,
        params.incentive_ratio,
        params.max_allowed_uids,
        params.max_allowed_weights,
        params.min_allowed_weights,
        params.max_weight_age,
        params.tempo,
        params.maximum_set_weight_calls_per_epoch,
        VoteMode::Vote,
        params.bonds_ma,
        params.module_burn_config,
        params.min_validator_stake,
        params.max_allowed_validators
    )

    // 2
    add_global_custom_proposal {
        let caller: T::AccountId = account("Alice", 0, 1);
        // Add alice fund to submit the proposal
        SubspaceMod::<T>::add_balance_to_account(&caller,
        SubspaceMod::<T>::u64_to_balance(1_000_000_000_000_000).unwrap());
        let data =  "test".as_bytes().to_vec(); }: add_global_custom_proposal(RawOrigin::Signed(caller), data)

    // 3
    add_subnet_custom_proposal {
        let caller: T::AccountId = account("Alice", 0, 1);
        // The subnet has to exist
        register_mock::<T>(caller.clone(), caller.clone(),
    "test".as_bytes().to_vec())?;     // Add alice fund to submit the proposal
        SubspaceMod::<T>::add_balance_to_account(&caller,
    SubspaceMod::<T>::u64_to_balance(1_000_000_000_000_000).unwrap());
    let data = "test".as_bytes().to_vec();
 }: add_subnet_custom_proposal(RawOrigin::Signed(caller), 0, data)

    // 4
    add_transfer_dao_treasury_proposal {
        let caller: T::AccountId = account("Alice", 0, 1);
        // Add alice fund to submit the proposal
        SubspaceMod::<T>::add_balance_to_account(&caller,
    SubspaceMod::<T>::u64_to_balance(1_000_000_000_000_000).unwrap());     let amount = 1000;
        // Add the amount to treasury funds
        let treasury_address: T::AccountId = DaoTreasuryAddress::<T>::get();
        SubspaceMod::<T>::add_balance_to_account(&treasury_address,
    SubspaceMod::<T>::u64_to_balance(amount).unwrap());

        let data = "test".as_bytes().to_vec();
        let destinations: T::AccountId = account("Bob", 0, 2);
    }: add_transfer_dao_treasury_proposal(RawOrigin::Signed(caller), data, amount, destinations)

    // ---------------------------------
    // Voting / Unvoting proposals
    // ---------------------------------


    // 5
    vote_proposal {
        let caller: T::AccountId = account("Alice", 0, 1);
        // Register Alice such that she has funds to vote
        register_mock::<T>(caller.clone(), caller.clone(), "test".as_bytes().to_vec())?;

        // Add Alice's funds to submit the proposal
        SubspaceMod::<T>::add_balance_to_account(&caller, SubspaceMod::<T>::u64_to_balance(1_000_000_000_000_000).unwrap());

        // Submit a custom proposal
        let data = "test".as_bytes().to_vec();
        GovernanceMod::<T>::add_global_custom_proposal(RawOrigin::Signed(caller.clone()).into(), data)?;

        let proposal_id = 0;
        let vote = true;
    }: vote_proposal(RawOrigin::Signed(caller), proposal_id, vote)

    // 6
    remove_vote_proposal {
        let caller: T::AccountId = account("Alice", 0, 1);
        // Register Alice such that she has funds to vote
        register_mock::<T>(caller.clone(), caller.clone(), "test".as_bytes().to_vec())?;

        // Add Alice's funds to submit the proposal
        SubspaceMod::<T>::add_balance_to_account(&caller, SubspaceMod::<T>::u64_to_balance(1_000_000_000_000_000).unwrap());

        // Submit a custom proposal
        let data = "test".as_bytes().to_vec();
        GovernanceMod::<T>::add_global_custom_proposal(RawOrigin::Signed(caller.clone()).into(), data)?;

        let proposal_id = 0;

        // Let Alice vote on the proposal
        GovernanceMod::<T>::vote_proposal(RawOrigin::Signed(caller.clone()).into(), proposal_id, true)?;

    }: remove_vote_proposal(RawOrigin::Signed(caller), proposal_id)

    // 7
    enable_vote_power_delegation {
        let caller: T::AccountId = account("Alice", 0, 1);
    }: enable_vote_power_delegation(RawOrigin::Signed(caller))

    // 8
    disable_vote_power_delegation {
        let caller: T::AccountId = account("Alice", 0, 1);
    }: disable_vote_power_delegation(RawOrigin::Signed(caller))

    // ---------------------------------
    // Subnet 0 DAO
    // ---------------------------------

    // 9
    add_dao_application {
        let caller: T::AccountId = account("Alice", 0, 1);
        let application_key: T::AccountId = account("Bob", 0, 2);
        SubspaceMod::<T>::add_balance_to_account(&caller,
    SubspaceMod::<T>::u64_to_balance(1_000_000_000_000_000).unwrap());     let data =
    "test".as_bytes().to_vec(); }: add_dao_application(RawOrigin::Signed(caller), application_key,
    data)

    // 10
    refuse_dao_application {
        // First add the application
        submit_dao_application::<T>()?;
        let caller: T::AccountId = account("Alice", 0, 1);
        Curator::<T>::set(caller.clone());
    }: refuse_dao_application(RawOrigin::Signed(caller), 0)

    // 11
    add_to_whitelist {
        // First add the application
        submit_dao_application::<T>()?;
        let caller: T::AccountId = account("Alice", 0, 1);
        let application_key: T::AccountId = account("Bob", 0, 2);
        Curator::<T>::set(caller.clone());
    }: add_to_whitelist(RawOrigin::Signed(caller), application_key)

    // 12
    remove_from_whitelist {
        // First add the application
        submit_dao_application::<T>()?;
        let caller: T::AccountId = account("Alice", 0, 1);
        let application_key: T::AccountId = account("Bob", 0, 2);
        Curator::<T>::set(caller.clone());
        // Now add it to whitelist
        GovernanceMod::<T>::add_to_whitelist(RawOrigin::Signed(caller.clone()).into(),
    application_key.clone())?; }: remove_from_whitelist(RawOrigin::Signed(caller),
    application_key)

}
