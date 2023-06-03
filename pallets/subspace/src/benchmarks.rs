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
    assert_ok!( Subspace::<T>::do_add_network( RawOrigin::Root.into(), netuid.try_into().unwrap(), name.into(),  tempo.into()));
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
        let name: Vec<u8> = "DefaultModule".as_bytes().to_vec();
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
    let name: Vec<u8> = "DefaultModule".as_bytes().to_vec();


    assert_ok!(Subspace::<T>::do_add_network( RawOrigin::Root.into(), netuid.try_into().unwrap(), name.into(), tempo.into()));
    
    let block_number: u64 = Subspace::<T>::get_current_block_as_u64();
    let key: T::AccountId = account("Alice", 0, seed);
        
  }: register( RawOrigin::Signed( caller.clone() ), netuid  )

  benchmark_set_weights {
    
    // This is a whitelisted caller who can make transaction without weights.
    let caller: T::AccountId = whitelisted_caller::<AccountIdOf<T>>(); 
    let caller_origin = <T as frame_system::Config>::RuntimeOrigin::from(RawOrigin::Signed(caller.clone()));
    let netuid: u16 = 1;
    let tempo: u16 = 1;
    let name: Vec<u8> = "DefaultModule".as_bytes().to_vec();
   
    assert_ok!( Subspace::<T>::do_add_network( RawOrigin::Root.into(), name.into(),  tempo.into(), n.into()));
    Subspace::<T>::set_max_allowed_uids( netuid, 4096 ); 

   assert_ok!(Subspace::<T>::do_sudo_set_max_registrations_per_block(RawOrigin::Root.into(), netuid.try_into().unwrap(), 4096 ));
    
    let mut seed : u32 = 1; 
    let mut dests: Vec<u16> = vec![];
    let mut weights: Vec<u16> = vec![];
    let signer : T::AccountId = account("Alice", 0, seed);
    let name : Vec<u8> = "Alice".as_bytes().to_vec();
    let ip: Vec<u8> = "0.0.0.0".as_bytes().to_vec();
    let port: u16 = 30333;

    for id in 0..4096 as u16 {
      let block_number: u64 = Subspace::<T>::get_current_block_as_u64();
      let start_nonce: u64 = (39420842u64 + 100u64*id as u64).into();
      
      let key: T::AccountId = account("Alice", 0, seed);
      seed = seed +1;
    
      
      let block_number: u64 = Subspace::<T>::get_current_block_as_u64();
      
      assert_ok!( Subspace::<T>::do_registration(caller_origin.clone(), netuid.try_into().unwrap(), name.clone(), ip.clone(), port )); 

      let uid = Subspace::<T>::get_uid_for_net_and_key(netuid, &key.clone()).unwrap();
      dests.push(id.clone());
      weights.push(id.clone());
    }

  }: set_weights(RawOrigin::Signed( signer.clone() ), netuid, dests, weights)


  benchmark_add_stake {
    let caller: T::AccountId = whitelisted_caller::<AccountIdOf<T>>(); 
    let caller_origin = <T as frame_system::Config>::RuntimeOrigin::from(RawOrigin::Signed(caller.clone()));
    let tempo: u16 = 1;
    let n : u32 = 4096;
    let name: Vec<u8> = "DefaultModule".as_bytes().to_vec();

    assert_ok!( Subspace::<T>::do_add_network( RawOrigin::Root.into(), name.into(),  tempo, n ));
    Subspace::<T>::set_max_allowed_uids( netuid, 4096 ); 
    assert_eq!(Subspace::<T>::get_max_allowed_uids(netuid), 4096);

    let block_number: u64 = Subspace::<T>::get_current_block_as_u64();
    let start_nonce: u64 = (39420842u64 + 100u64*netuid as u64).into();
    let mut seed : u32 = 1;
    let key: T::AccountId = account("Alice", 0, seed);

    assert_ok!( Subspace::<T>::do_registration(caller_origin.clone(), netuid.try_into().unwrap() , name.clone(), ip.clone(), port ));

    let amount: u64 = 1;
    let amoun_to_be_staked = Subspace::<T>::u64_to_balance( 1000000000);

    Subspace::<T>::add_balance_to_account(&key.clone(), amoun_to_be_staked.unwrap());

  }: add_stake(RawOrigin::Signed( key.clone() ), amount)

  benchmark_remove_stake{
    let caller: T::AccountId = whitelisted_caller::<AccountIdOf<T>>(); 
    let caller_origin = <T as frame_system::Config>::RuntimeOrigin::from(RawOrigin::Signed(caller.clone()));
    let netuid: u16 = 1;
    let tempo: u16 = 1;
    let name: Vec<u8> = "DefaultModule".as_bytes().to_vec();
    let network: Vec<u8> = "commune".as_bytes().to_vec();
    let netuid = Subspace::<T>::get_netuid_for_name(name.clone()).unwrap();

    assert_ok!( Subspace::<T>::do_add_network( RawOrigin::Root.into(), name.into(),  tempo.into(), n.into()));
    Subspace::<T>::set_max_allowed_uids( netuid, 4096 ); 
    assert_eq!(Subspace::<T>::get_max_allowed_uids(netuid), 4096);

    let block_number: u64 = Subspace::<T>::get_current_block_as_u64();
    let start_nonce: u64 = (39420842u64 + 100u64*netuid as u64).into();
    let mut seed : u32 = 1;
    let key: T::AccountId = caller; 

    assert_ok!( Subspace::<T>::do_registration(caller_origin.clone(), netuid.try_into().unwrap() , name.clone(), ip.clone(), port));

    let amoun_to_be_staked = Subspace::<T>::u64_to_balance( 1000000000);
    Subspace::<T>::add_balance_to_account(&key.clone(), amoun_to_be_staked.unwrap());

    assert_ok!( Subspace::<T>::add_stake(RawOrigin::Signed( key.clone() ).into(), 1000));

    let amount_unstaked: u64 = 1;

  }: remove_stake(RawOrigin::Signed( key.clone() ), key.clone(), amount_unstaked)


