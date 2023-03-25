//! Subspace pallet benchmarking.

#![cfg(feature = "runtime-benchmarks")]
//mod benchmarking;


use crate::*;
use crate::Pallet as Subspace;
use frame_benchmarking::{benchmarks, whitelisted_caller, account};
use frame_system::RawOrigin;
use frame_support::sp_std::vec;
use frame_support::inherent::Vec;
pub use pallet::*;
use frame_support::assert_ok;
//use mock::{Test, new_test_ext};

benchmarks! {
   
  // Add individual benchmarks here
  benchmark_epoch_without_weights { 

    // This is a whitelisted caller who can make transaction without weights.
    let caller: T::AccountId = whitelisted_caller::<AccountIdOf<T>>(); 
    let caller_origin = <T as frame_system::Config>::RuntimeOrigin::from(RawOrigin::Signed(caller.clone()));

    // Lets create a single network.
    let n: u16 = 4096;
    let netuid: u16 = 11; //11 is the benchmark network.
    let tempo: u16 = 1;
    let name: Vec<u8> = "DefaultModule".as_bytes().to_vec();
    assert_ok!( Subspace::<T>::do_add_network( RawOrigin::Root.into(), netuid.try_into().unwrap(), name.into(), tempo.into()));
    Subspace::<T>::set_max_allowed_uids( netuid, n ); 

    // Lets fill the network with 100 UIDS and no weights.
    let mut seed : u32 = 1;
    for uid in 0..n as u16 {
        let block_number: u64 = Subspace::<T>::get_current_block_as_u64();
        let key: T::AccountId = account("Alice", 0, seed);
        Subspace::<T>::append_neuron( netuid, &key, block_number );
        seed = seed + 1;
    }

  }: _( RawOrigin::Signed( caller.clone() ) )

  // Add individual benchmarks here
  /*benchmark_drain_emission { 

    // This is a whitelisted caller who can make transaction without weights.
    let caller: T::AccountId = whitelisted_caller::<AccountIdOf<T>>(); 
    let caller_origin = <T as frame_system::Config>::RuntimeOrigin::from(RawOrigin::Signed(caller.clone()));

    // Lets create a single network.
    let n: u16 = 4096;
    let netuid: u16 = 11; //11 is the benchmark network.
    let tempo: u16 = 1;
    let name: Vec<u8> = b"default".to_vec();
    Subspace::<T>::do_add_network( caller_origin.clone(), netuid.try_into().unwrap(), name.into(), tempo.into());
    Subspace::<T>::set_max_allowed_uids( netuid, n ); 
    Subspace::<T>::set_tempo( netuid, tempo );

    // Lets fill the network with 100 UIDS and no weights.
    let mut seed : u32 = 1;
    let mut emission: Vec<(T::AccountId, u64)> = vec![];
    for uid in 0..n as u16 {
        let block_number: u64 = Subspace::<T>::get_current_block_as_u64();
        let key: T::AccountId = account("Alice", 0, SEED);
        Subspace::<T>::append_neuron( netuid, &key, block_number );
        SEED = SEED + 1;
        emission.push( ( key, 1 ) );
    }
    Subspace::<T>::sink_emission( netuid, emission );
 
  }: _( RawOrigin::Signed( caller.clone() ) )  */


  benchmark_register { 

    // This is a whitelisted caller who can make transaction without weights.
    let caller: T::AccountId = whitelisted_caller::<AccountIdOf<T>>(); 

    // Lets create a single network.
    let n: u16 = 10;
    let netuid: u16 = 1; //11 is the benchmark network.
    let tempo: u16 = 1;
    let name: Vec<u8> = b"default".to_vec();

    let block_number: u64 = Subspace::<T>::get_current_block_as_u64();
    let start_nonce: u64 = (39420842u64 + 100u64*netuid as u64).into();

    assert_ok!(Subspace::<T>::do_add_network( RawOrigin::Root.into(), netuid.try_into().unwrap(), name.into(), tempo.into()));
    
    let mut seed : u32 = 1;
    let block_number: u64 = Subspace::<T>::get_current_block_as_u64();
    let key: T::AccountId = account("Alice", 0, seed);
        
  }: register( RawOrigin::Signed( caller.clone() ), netuid  )

 benchmark_epoch_with_weights { 
    let caller: T::AccountId = whitelisted_caller::<AccountIdOf<T>>(); 
    let caller_origin = <T as frame_system::Config>::RuntimeOrigin::from(RawOrigin::Signed(caller.clone()));
    Subspace::<T>::create_network_with_weights(
      caller_origin.clone(), 
      11u16.into(), // netuid
      4096u16.into(), // n
      1000u16.into(), // tempo
      100u16.into(), // n_vals
      1000u16.into() // nweights
    );
  }: _( RawOrigin::Signed( caller.clone() ) ) 

  benchmark_set_weights {
    
    // This is a whitelisted caller who can make transaction without weights.
    let caller: T::AccountId = whitelisted_caller::<AccountIdOf<T>>(); 
    let caller_origin = <T as frame_system::Config>::RuntimeOrigin::from(RawOrigin::Signed(caller.clone()));
    let netuid: u16 = 1;
    let version_key: u64 = 1;
    let tempo: u16 = 1;
    let name: Vec<u8> = b"default".to_vec();
   
    assert_ok!( Subspace::<T>::do_add_network( RawOrigin::Root.into(), name.into(), netuid.try_into().unwrap()));
    Subspace::<T>::set_max_allowed_uids( netuid, 4096 ); 

   assert_ok!(Subspace::<T>::do_sudo_set_max_registrations_per_block(RawOrigin::Root.into(), netuid.try_into().unwrap(), 4096 ));
    
    let mut seed : u32 = 1; 
    let mut dests: Vec<u16> = vec![];
    let mut weights: Vec<u16> = vec![];
    let signer : T::AccountId = account("Alice", 0, seed);

    for id in 0..4096 as u16 {
      let block_number: u64 = Subspace::<T>::get_current_block_as_u64();
      let start_nonce: u64 = (39420842u64 + 100u64*id as u64).into();
      
        let key: T::AccountId = account("Alice", 0, seed);
        seed = seed +1;
      
      
      let block_number: u64 = Subspace::<T>::get_current_block_as_u64();
      
      assert_ok!( Subspace::<T>::do_registration(caller_origin.clone(), netuid.try_into().unwrap() )); 

      let uid = Subspace::<T>::get_uid_for_net_and_key(netuid, &key.clone()).unwrap();
      Subspace::<T>::set_validator_permit_for_uid(netuid, uid.clone(), true);
      dests.push(id.clone());
      weights.push(id.clone());
    }

  }: set_weights(RawOrigin::Signed( signer.clone() ), netuid, dests, weights, version_key)


  benchmark_add_stake {
    let caller: T::AccountId = whitelisted_caller::<AccountIdOf<T>>(); 
    let caller_origin = <T as frame_system::Config>::RuntimeOrigin::from(RawOrigin::Signed(caller.clone()));
    let netuid: u16 = 1;
    let version_key: u64 = 1;
    let tempo: u16 = 1;
    let name: Vec<u8> = b"default".to_vec();

    assert_ok!( Subspace::<T>::do_add_network( RawOrigin::Root.into(), name.into(), netuid.try_into().unwrap()));
    Subspace::<T>::set_max_allowed_uids( netuid, 4096 ); 
    assert_eq!(Subspace::<T>::get_max_allowed_uids(netuid), 4096);

    let block_number: u64 = Subspace::<T>::get_current_block_as_u64();
    let start_nonce: u64 = (39420842u64 + 100u64*netuid as u64).into();
    let mut seed : u32 = 1;
    let key: T::AccountId = account("Alice", 0, seed);

    assert_ok!( Subspace::<T>::do_registration(caller_origin.clone(), netuid.try_into().unwrap() ));

    let amount: u64 = 1;
    let amoun_to_be_staked = Subspace::<T>::u64_to_balance( 1000000000);

    Subspace::<T>::add_balance_to_account(&key.clone(), amoun_to_be_staked.unwrap());

  }: add_stake(RawOrigin::Signed( key.clone() ), amount)

  benchmark_remove_stake{
    let caller: T::AccountId = whitelisted_caller::<AccountIdOf<T>>(); 
    let caller_origin = <T as frame_system::Config>::RuntimeOrigin::from(RawOrigin::Signed(caller.clone()));
    let netuid: u16 = 1;
    let version_key: u64 = 1;
    let tempo: u16 = 1;
    let name: Vec<u8> = b"default".to_vec();

    assert_ok!( Subspace::<T>::do_add_network( RawOrigin::Root.into(), name.into(), netuid.try_into().unwrap()));
    Subspace::<T>::set_max_allowed_uids( netuid, 4096 ); 
    assert_eq!(Subspace::<T>::get_max_allowed_uids(netuid), 4096);

    let block_number: u64 = Subspace::<T>::get_current_block_as_u64();
    let start_nonce: u64 = (39420842u64 + 100u64*netuid as u64).into();
    let mut seed : u32 = 1;
    let key: T::AccountId = caller; 

    assert_ok!( Subspace::<T>::do_registration(caller_origin.clone(), netuid.try_into().unwrap() ));

    let amoun_to_be_staked = Subspace::<T>::u64_to_balance( 1000000000);
    Subspace::<T>::add_balance_to_account(&key.clone(), amoun_to_be_staked.unwrap());

    assert_ok!( Subspace::<T>::add_stake(RawOrigin::Signed( key.clone() ).into(), 1000));

    let amount_unstaked: u64 = 1;

  }: remove_stake(RawOrigin::Signed( key.clone() ), key.clone(), amount_unstaked)

  benchmark_serve_axon{
    let caller: T::AccountId = whitelisted_caller::<AccountIdOf<T>>(); 
    let caller_origin = <T as frame_system::Config>::RuntimeOrigin::from(RawOrigin::Signed(caller.clone()));
    let netuid: u16 = 1;
    let version: u32 =  2;
    let ip: u128 = 1676056785;
    let port: u16 = 128;
    let ip_type: u8 = 4;
    let protocol: u8 = 0;
    let placeholder1: u8 = 0;
    let placeholder2: u8 = 0;

    Subspace::<T>::set_serving_rate_limit(netuid, 0);

  }: serve_axon(RawOrigin::Signed( caller.clone() ), netuid, version, ip, port, ip_type, protocol, placeholder1, placeholder2)

  benchmark_serve_prometheus {
    let caller: T::AccountId = whitelisted_caller::<AccountIdOf<T>>(); 
    let caller_origin = <T as frame_system::Config>::RuntimeOrigin::from(RawOrigin::Signed(caller.clone()));
    let netuid: u16 = 1;
    let version: u32 = 2;
    let ip: u128 = 1676056785;
    let port: u16 = 128;
    let ip_type: u8 = 4;
    

    Subspace::<T>::set_serving_rate_limit(netuid, 0);

  }: serve_prometheus(RawOrigin::Signed( caller.clone() ), netuid, version, ip, port, ip_type)


  benchmark_sudo_add_network {
    let netuid: u16 = 1;
    let tempo: u16 = 1;
    let name: Vec<u8> = b"default".to_vec();

  }: sudo_add_network(RawOrigin::<AccountIdOf<T>>::Root, netuid, name, tempo,)

  benchmark_sudo_remove_network {
    let netuid: u16 = 1;
    let tempo: u16 = 0;
    let name: Vec<u8> = b"default".to_vec();

    assert_ok!( Subspace::<T>::do_add_network( RawOrigin::Root.into(), name.into(), netuid.try_into().unwrap()));

  }: sudo_remove_network(RawOrigin::<AccountIdOf<T>>::Root, netuid)

  benchmark_sudo_set_emission_values{
    let tempo: u16 = 1;
    let name: Vec<u8> = b"default".to_vec();
    let netuids: Vec<u16> = vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9];
    let emission: Vec<u64> = vec![100000000, 100000000, 100000000, 100000000, 100000000, 100000000, 100000000, 100000000, 100000000, 100000000];

    assert_ok!( Subspace::<T>::do_add_network( RawOrigin::Root.into(), 0, name.into(), tempo.into()));
    assert_ok!( Subspace::<T>::do_add_network( RawOrigin::Root.into(), 1, name.into(), tempo.into()));
    assert_ok!( Subspace::<T>::do_add_network( RawOrigin::Root.into(), 2, name.into(), tempo.into()));
    assert_ok!( Subspace::<T>::do_add_network( RawOrigin::Root.into(), 4, name.into(), tempo.into()));
    assert_ok!( Subspace::<T>::do_add_network( RawOrigin::Root.into(), 5, name.into(), tempo.into()));
    assert_ok!( Subspace::<T>::do_add_network( RawOrigin::Root.into(), 6, name.into(), tempo.into()));
    assert_ok!( Subspace::<T>::do_add_network( RawOrigin::Root.into(), 7, name.into(), tempo.into()));
    assert_ok!( Subspace::<T>::do_add_network( RawOrigin::Root.into(), 8, name.into(), tempo.into()));
    assert_ok!( Subspace::<T>::do_add_network( RawOrigin::Root.into(), 9, name.into(), tempo.into())); 

  }: sudo_set_emission_values(RawOrigin::<AccountIdOf<T>>::Root, netuids, emission)

  benchmark_sudo_add_network_connection_requirement {
    let netuid_a: u16 = 1; 
    let netuid_b: u16 = 2; 
    let tempo: u16 = 1;
    let name: Vec<u8> = b"default".to_vec();
    let requirement: u16 = 1;

    assert_ok!( Subspace::<T>::do_add_network( RawOrigin::Root.into(), netuid_a.try_into().unwrap(), name.into(), tempo.into()));
    assert_ok!( Subspace::<T>::do_add_network( RawOrigin::Root.into(), netuid_b.try_into().unwrap(), name.into(), tempo.into()));

  }: sudo_add_network_connection_requirement(RawOrigin::<AccountIdOf<T>>::Root, netuid_a, netuid_b, requirement)

  benchmark_sudo_remove_network_connection_requirement {
    let netuid_a: u16 = 1; 
    let netuid_b: u16 = 2; 
    let tempo: u16 = 1;
    let name: Vec<u8> = b"default".to_vec();
    let requirement: u16 = 1;

    assert_ok!( Subspace::<T>::do_add_network( RawOrigin::Root.into(), netuid_a.try_into().unwrap(), name.into(), tempo.into()));
    assert_ok!( Subspace::<T>::do_add_network( RawOrigin::Root.into(), netuid_b.try_into().unwrap(), name.into(), tempo.into()));

  }: sudo_remove_network_connection_requirement(RawOrigin::<AccountIdOf<T>>::Root, netuid_a, netuid_b)

  benchmark_sudo_set_default_take {
    let default_take: u16 = 100; 

  }: sudo_set_default_take(RawOrigin::<AccountIdOf<T>>::Root, default_take)

  benchmark_sudo_set_serving_rate_limit {
    let serving_rate_limit: u64 = 100;
    let netuid: u16 = 1;

  }: sudo_set_serving_rate_limit(RawOrigin::<AccountIdOf<T>>::Root, netuid, serving_rate_limit)

  benchmark_sudo_set_weights_set_rate_limit {
    let netuid: u16 = 1; 
    let weights_set_rate_limit: u64 = 3;
    let tempo: u16 = 1;
    let name: Vec<u8> = b"default".to_vec();

    assert_ok!( Subspace::<T>::do_add_network( RawOrigin::Root.into(), name.into(), netuid.try_into().unwrap()));

  }: sudo_set_weights_set_rate_limit(RawOrigin::<AccountIdOf<T>>::Root, netuid, weights_set_rate_limit)

  benchmark_sudo_set_weights_version_key {
    let netuid: u16 = 1; 
    let weights_version_key: u64 = 1;
    let tempo: u16 = 1;
    let name: Vec<u8> = b"default".to_vec();

    assert_ok!( Subspace::<T>::do_add_network( RawOrigin::Root.into(), name.into(), netuid.try_into().unwrap()));

  }: sudo_set_weights_version_key(RawOrigin::<AccountIdOf<T>>::Root, netuid, weights_version_key)


  benchmark_sudo_set_max_allowed_validators {
    let netuid: u16 = 1;
    let tempo: u16 = 1;
    let name: Vec<u8> = b"default".to_vec();
    let max_allowed_validators: u16 = 10;

    assert_ok!( Subspace::<T>::do_add_network( RawOrigin::Root.into(), name.into(), netuid.try_into().unwrap()));

  }: sudo_set_max_allowed_validators(RawOrigin::<AccountIdOf<T>>::Root, netuid, max_allowed_validators)


  benchmark_sudo_set_adjustment_interval {
    let netuid: u16 = 1;
    let tempo: u16 = 1;
    let name: Vec<u8> = b"default".to_vec();
    let adjustment_interval: u16 = 12;

    assert_ok!( Subspace::<T>::do_add_network( RawOrigin::Root.into(), name.into(), netuid.try_into().unwrap()));

  }: sudo_set_adjustment_interval(RawOrigin::<AccountIdOf<T>>::Root, netuid, adjustment_interval)

  benchmark_sudo_set_target_registrations_per_interval {
    let netuid: u16 = 1;
    let tempo: u16 = 1;
    let name: Vec<u8> = b"default".to_vec();
    let target_registrations_per_interval: u16 = 300;

    assert_ok!( Subspace::<T>::do_add_network( RawOrigin::Root.into(), name.into(), netuid.try_into().unwrap()));

  }: sudo_set_target_registrations_per_interval(RawOrigin::<AccountIdOf<T>>::Root, netuid, target_registrations_per_interval)

  benchmark_sudo_set_activity_cutoff {
    let netuid: u16 = 1;
    let tempo: u16 = 1;
    let name: Vec<u8> = b"default".to_vec();
    let activity_cutoff: u16 = 300;

    assert_ok!( Subspace::<T>::do_add_network( RawOrigin::Root.into(), name.into(), netuid.try_into().unwrap()));

  }: sudo_set_activity_cutoff(RawOrigin::<AccountIdOf<T>>::Root, netuid, activity_cutoff)


  benchmark_sudo_set_max_allowed_uids {
    let netuid: u16 = 1;
    let tempo: u16 = 1;
    let name: Vec<u8> = b"default".to_vec();
    let max_allowed_uids: u16 = 4096;

    assert_ok!( Subspace::<T>::do_add_network( RawOrigin::Root.into(), name.into(), netuid.try_into().unwrap()));

  }: sudo_set_max_allowed_uids(RawOrigin::<AccountIdOf<T>>::Root, netuid, max_allowed_uids)

  benchmark_sudo_set_min_allowed_weights {
    let netuid: u16 = 1;
    let tempo: u16 = 1;
    let name: Vec<u8> = b"default".to_vec();
    let min_allowed_weights: u16 = 10;

    assert_ok!( Subspace::<T>::do_add_network( RawOrigin::Root.into(), name.into(), netuid.try_into().unwrap()));

  }: sudo_set_min_allowed_weights(RawOrigin::<AccountIdOf<T>>::Root, netuid, min_allowed_weights)

  benchmark_sudo_set_validator_batch_size{
    let netuid: u16 = 1;
    let tempo: u16 = 1;
    let name: Vec<u8> = b"default".to_vec();
    let validator_batch_size: u16 = 10;

    assert_ok!( Subspace::<T>::do_add_network( RawOrigin::Root.into(), name.into(), netuid.try_into().unwrap()));

  }: sudo_set_validator_batch_size(RawOrigin::<AccountIdOf<T>>::Root, netuid, validator_batch_size)

  benchmark_sudo_set_validator_epochs_per_reset {
    let netuid: u16 = 1;
    let tempo: u16 = 1;
    let name: Vec<u8> = b"default".to_vec();
    let validator_epochs_per_reset: u16 = 10;

    assert_ok!( Subspace::<T>::do_add_network( RawOrigin::Root.into(), name.into(), netuid.try_into().unwrap()));

  }: sudo_set_validator_epochs_per_reset(RawOrigin::<AccountIdOf<T>>::Root, netuid, validator_epochs_per_reset)


  benchmark_sudo_set_validator_prune_len {
    let netuid: u16 = 1;
    let tempo: u16 = 1;
    let name: Vec<u8> = b"default".to_vec();
    let validator_prune_len: u64 = 10;

    assert_ok!( Subspace::<T>::do_add_network( RawOrigin::Root.into(), name.into(), netuid.try_into().unwrap()));

  }: sudo_set_validator_prune_len(RawOrigin::<AccountIdOf<T>>::Root, netuid, validator_prune_len)


  benchmark_sudo_set_immunity_period {
    let netuid: u16 = 1;
    let tempo: u16 = 1;
    let name: Vec<u8> = b"default".to_vec();
    let immunity_period: u16 = 100;

    assert_ok!( Subspace::<T>::do_add_network( RawOrigin::Root.into(), name.into(), netuid.try_into().unwrap()));

  }: sudo_set_immunity_period(RawOrigin::<AccountIdOf<T>>::Root, netuid, immunity_period)

  benchmark_sudo_set_max_weight_limit {
    let netuid: u16 = 1;
    let tempo: u16 = 1;
    let name: Vec<u8> = b"default".to_vec();
    let max_weight_limit: u16 = 100;

    assert_ok!( Subspace::<T>::do_add_network( RawOrigin::Root.into(), name.into(), netuid.try_into().unwrap()));

  }: sudo_set_max_weight_limit(RawOrigin::<AccountIdOf<T>>::Root, netuid, max_weight_limit)

  benchmark_sudo_set_max_registrations_per_block {
    let netuid: u16 = 1;
    let tempo: u16 = 1;
    let name: Vec<u8> = b"default".to_vec();
    let max_registrations_per_block: u16 = 100;

    assert_ok!( Subspace::<T>::do_add_network( RawOrigin::Root.into(), netuid.try_into().unwrap(),  name.into(), tempo.into()));

  }: sudo_set_max_registrations_per_block(RawOrigin::<AccountIdOf<T>>::Root, netuid, max_registrations_per_block)

  benchmark_sudo_set_validator_epoch_length {
    let netuid: u16 = 1;
    let tempo: u16 = 1;
    let validator_epoch_len: u16 = 10;

    assert_ok!( Subspace::<T>::do_add_network( RawOrigin::Root.into(), netuid.try_into().unwrap(),  name.into(), tempo.into()));

  }: sudo_set_validator_epoch_len(RawOrigin::<AccountIdOf<T>>::Root, netuid, validator_epoch_len)


