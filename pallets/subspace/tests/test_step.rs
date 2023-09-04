use crate::test_mock::*;
use rand::{Rng, thread_rng, SeedableRng, rngs::StdRng, seq::SliceRandom, distributions::Uniform};
use sp_core::U256;
use substrate_fixed::types::{I32F32, I64F64};
use substrate_fixed::transcendental::{PI, cos, ln, sqrt};
use frame_system::Config;
use frame_support::assert_ok;
use std::time::Instant;
mod test_mock;




fn check_network_stats(netuid:u16) {

    let emission_buffer : u64 = 1_000; // the numbers arent perfect but we want to make sure they fall within a range (10_000 / 2**64)

    let subnet_emission: u64 = SubspaceModule::get_subnet_emission( netuid );
    let incentives : Vec<u16> = SubspaceModule::get_incentive(netuid);
    let dividends : Vec<u16>= SubspaceModule::get_dividends(netuid);
    let emissions : Vec<u64> = SubspaceModule::get_emissions(netuid);
    let total_incentives : u16 = incentives.iter().sum();
    let total_dividends : u16 = dividends.iter().sum();
    let total_emissions : u64 = emissions.iter().sum();

    println!("total_emissions: {}", total_emissions);
    println!("total_incentives: {}", total_incentives);
    println!("total_dividends: {}", total_dividends);
    

    println!("emission: {:?}", emissions);
    println!("incentives: {:?}", incentives);
    println!("incentives: {:?}", incentives);
    println!("dividends: {:?}", dividends);

    assert!( total_emissions >= subnet_emission- emission_buffer || total_emissions <= subnet_emission + emission_buffer );
}


#[test]
fn test_no_weights() {
	new_test_ext().execute_with(|| {
        let netuid: u16 = 0;
        register_n_modules( 0, 10, 1000 );
        SubspaceModule::set_tempo( netuid, 1 );
        let keys = SubspaceModule::get_keys( netuid );
        let uids = SubspaceModule::get_uids( netuid );
        
        let incentives : Vec<u16> = SubspaceModule::get_incentive(netuid);
        let dividends : Vec<u16>= SubspaceModule::get_dividends(netuid);
        let emissions : Vec<u64> = SubspaceModule::get_emissions(netuid);
        let total_incentives : u16 = incentives.iter().sum();
        let total_dividends : u16 = dividends.iter().sum();
        let total_emissions : u64 = emissions.iter().sum();
        





	});
}


#[test]
fn test_dividends() {
	new_test_ext().execute_with(|| {
    // CONSSTANTS
    let netuid: u16 = 0;
    let n : u16 = 100;
    let n_list : Vec<u16> = vec![10, 50, 100, 1000];
    let blocks_per_epoch_list : u64 = 1;
    let stake_per_module : u64 = 10_000;
    
    // SETUP NETWORK
    register_n_modules( netuid, n, stake_per_module );
    SubspaceModule::set_tempo( netuid, 1 );
    SubspaceModule::set_max_allowed_weights(netuid, n );

    // for i in 0..n {

    //     let key: U256 = U256::from(i);
    //     register_module( netuid, key, stake_per_module );

    // }
    let keys = SubspaceModule::get_keys( netuid );
    let uids = SubspaceModule::get_uids( netuid );

    
    // do a list of ones for weights
    let weight_uids : Vec<u16> = [1,2].to_vec();

    // do a list of ones for weights
    let weight_values : Vec<u16> = [1,1].to_vec();

    
    set_weights(netuid, keys[0], weight_uids.clone() , weight_values.clone() );

    stake_k1 = SubspaceModule::get_stake_for_uid(netuid, 1);
    stake_k2 = SubspaceModule::get_stake_for_uid(netuid, 2);

    incentive_k1 = SubspaceModule::get_incentive_for_uid(netuid, 1);
    incentive_k2 = SubspaceModule::get_incentive_for_uid(netuid, 2);

    dividens = SubspaceModule::get_dividends_for_uid(netuid, 1);
    dividens_k2 = SubspaceModule::get_dividends_for_uid(netuid, 2);
    assert !( stake_k1 == stake_k2 );
    step_block( 1 );
    check_network_stats(netuid);

    
    });


}


#[test]
fn test_with_weights() {
	new_test_ext().execute_with(|| {

        let n_list : Vec<u16> = vec![10, 50, 100, 1000];
        let blocks_per_epoch_list : u64 = 1;
        let stake_per_module : u64 = 10_000;

        for (netuid, n) in n_list.iter().enumerate() {
            println!("netuid: {}", netuid);
            let netuid: u16 = netuid as u16;
            let n : u16 = *n;

            for i in 0..n {

                println!("i: {}", i);
                println!("keys: {:?}", SubspaceModule::get_keys( netuid ));
                println!("uids: {:?}", SubspaceModule::get_uids( netuid ));
                let key: U256 = U256::from(i);
                println!("Before Registered: {:?} -> {:?}",key, SubspaceModule::is_key_registered( netuid, &key ));
                register_module( netuid, key, stake_per_module );
                println!("After Registered: {:?} -> {:?}",key, SubspaceModule::is_key_registered( netuid, &key ));

            }
            SubspaceModule::set_tempo( netuid, 1 );
            SubspaceModule::set_max_allowed_weights(netuid, n );
            let keys = SubspaceModule::get_keys( netuid );
            let uids = SubspaceModule::get_uids( netuid );
    
            
            let weight_values : Vec<u16> = (0..n).collect();
            let weight_uids : Vec<u16> = (0..n).collect();

            for i in 0..n {
    
                SubspaceModule::set_weights( get_origin(keys[i as usize]), netuid, weight_values.clone(), weight_uids.clone() ).unwrap();
            }
            step_block( 1 );
            check_network_stats(netuid);
        }



	});
}

#[test]
fn test_blocks_until_epoch(){
    new_test_ext().execute_with(|| { 

        // Check tempo = 0 block = * netuid = *
        assert_eq!( SubspaceModule::blocks_until_next_epoch( 0, 0, 0 ), 0 ); 

        // Check tempo = 1 block = * netuid = *
        assert_eq!( SubspaceModule::blocks_until_next_epoch( 0, 1, 0 ),  0 ); 
        assert_eq!( SubspaceModule::blocks_until_next_epoch( 1, 1, 0 ),  0 ); 
        assert_eq!( SubspaceModule::blocks_until_next_epoch( 0, 1, 1 ),  0 ); 
        assert_eq!( SubspaceModule::blocks_until_next_epoch( 1, 2, 1 ),  0 ); 
        assert_eq!( SubspaceModule::blocks_until_next_epoch( 0, 4, 3 ),  3 ); 
        assert_eq!( SubspaceModule::blocks_until_next_epoch( 10, 5, 2 ),  2 ); 
        // Check general case.
        for netuid in 0..30 as u16 { 
            for block in 0..30 as u64 {
                for tempo in 1..30 as u16 {
                    assert_eq!( SubspaceModule::blocks_until_next_epoch( netuid, tempo, block ),  ( block + netuid as u64 ) % ( tempo as u64  ) ); 
                }
            }
        } 


    });
}
