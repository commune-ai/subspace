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
    let incentives : Vec<u16> = SubspaceModule::get_incentives(netuid);
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
        
        let incentives : Vec<u16> = SubspaceModule::get_incentives(netuid);
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
    let n : u16 = 10;
    let n_list : Vec<u16> = vec![10, 50, 100, 1000];
    let blocks_per_epoch_list : u64 = 1;
    let stake_per_module : u64 = 10_000;
    
    // SETUP NETWORK
    register_n_modules( netuid, n, stake_per_module );
    SubspaceModule::set_tempo( netuid, 1 );
    SubspaceModule::set_max_allowed_weights(netuid, n );
    SubspaceModule::set_min_allowed_weights(netuid, 0 );

    // for i in 0..n {

    //     let key: U256 = U256::from(i);
    //     register_module( netuid, key, stake_per_module );

    // }
    let keys = SubspaceModule::get_keys( netuid );
    let uids = SubspaceModule::get_uids( netuid );

    
    // do a list of ones for weights
    let weight_uids : Vec<u16> = [2,3].to_vec();
    // do a list of ones for weights
    let weight_values : Vec<u16> = [1,1].to_vec();
    set_weights(netuid, keys[0], weight_uids.clone() , weight_values.clone() );
    set_weights(netuid, keys[1], weight_uids.clone() , weight_values.clone() );

    step_block( 1 );
    let incentives : Vec<u16> = SubspaceModule::get_incentives(netuid);
    let dividends : Vec<u16>= SubspaceModule::get_dividends(netuid);
    let emissions : Vec<u64> = SubspaceModule::get_emissions(netuid);
    let stakes : Vec<u64> = SubspaceModule::get_stakes(netuid);



    // evaluate votees
    assert !( incentives[2]> 0);
    assert !( dividends[2] == dividends[3] );
    assert !( incentives[2] == incentives[3] );
    assert !( stakes[2] == stakes[3] );
    assert !( emissions[2] == emissions[3] );

    // evaluate voters
    assert !( dividends[0] == dividends[1] );
    assert !( incentives[0] == incentives[1] );
    assert !( stakes[0] == stakes[1] );
    check_network_stats(netuid);

    
    });


}

fn test_pruning() {
	new_test_ext().execute_with(|| {
    // CONSSTANTS
    let netuid: u16 = 0;
    let n : u16 = 100;
    let blocks_per_epoch_list : u64 = 1;
    let stake_per_module : u64 = 10_000;
    let tempo : u16 = 1;
    
    // SETUP NETWORK
    register_n_modules( netuid, n, stake_per_module );

    SubspaceModule::set_tempo( netuid, 1 );
    SubspaceModule::set_max_allowed_weights(netuid, n );
    SubspaceModule::set_min_allowed_weights(netuid, 0 );

    // for i in 0..n {

    //     let key: U256 = U256::from(i);
    //     register_module( netuid, key, stake_per_module );

    // }
    let keys = SubspaceModule::get_keys( netuid );
    let uids = SubspaceModule::get_uids( netuid );

    
    // do a list of ones for weights
    let weight_uids : Vec<u16> = (0..n).collect();
    // do a list of ones for weights
    let mut weight_values : Vec<u16> = weight_uids.iter().map(|x| 1 as u16 ).collect();

    let prune_uid: u16 = n - 1;
    weight_values[prune_uid as usize] = 0;
    set_weights(netuid, keys[0], weight_uids.clone() , weight_values.clone() );
    step_block( tempo );
    let lowest_priority_uid: u16 = SubspaceModule::get_lowest_uid(netuid);
    assert !( lowest_priority_uid == prune_uid );

    let new_key : U256 = U256::from( n + 1 );
    register_module( netuid, new_key, stake_per_module );
    let is_registered: bool = SubspaceModule::is_key_registered( netuid, &new_key);
    assert!( is_registered );
    assert!( SubspaceModule::get_subnet_n( netuid ) == n );
    let is_prune_registered: bool = SubspaceModule::is_key_registered( netuid, &keys[prune_uid as usize]);
    assert!( !is_prune_registered );
    check_network_stats(netuid);

    
    });


}
#[test]
fn test_lowest_priority_mechanism() {
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
    SubspaceModule::set_min_allowed_weights(netuid, 0 );

    // for i in 0..n {

    //     let key: U256 = U256::from(i);
    //     register_module( netuid, key, stake_per_module );

    // }
    let keys = SubspaceModule::get_keys( netuid );
    let uids = SubspaceModule::get_uids( netuid );

    
    // do a list of ones for weights
    let weight_uids : Vec<u16> = (0..n).collect();
    // do a list of ones for weights
    let mut weight_values : Vec<u16> = weight_uids.iter().map(|x| 1 as u16 ).collect();

    let prune_uid: u16 = n - 1;
    weight_values[prune_uid as usize] = 0;
    set_weights(netuid, keys[0], weight_uids.clone() , weight_values.clone() );



    step_block( 1 );
    let incentives : Vec<u16> = SubspaceModule::get_incentives(netuid);
    let dividends : Vec<u16>= SubspaceModule::get_dividends(netuid);
    let emissions : Vec<u64> = SubspaceModule::get_emissions(netuid);
    let stakes : Vec<u64> = SubspaceModule::get_stakes(netuid);

    assert !( emissions[prune_uid as usize] == 0 );
    assert !( incentives[prune_uid as usize] == 0 );
    assert !( dividends[prune_uid as usize] == 0 );

    let lowest_priority_uid: u16 = SubspaceModule::get_lowest_uid(netuid);
    println!("lowest_priority_uid: {}", lowest_priority_uid);
    println!("prune_uid: {}", prune_uid);
    println!("emissions: {:?}", emissions);
    println!("lowest_priority_uid: {:?}", lowest_priority_uid);
    println!("dividends: {:?}", dividends);
    println!("incentives: {:?}", incentives);
    assert !( lowest_priority_uid == prune_uid );

    check_network_stats(netuid);

    
    });


}


// #[test]
// fn test_deregister_zero_emission_uids() {
// 	new_test_ext().execute_with(|| {
//     // CONSSTANTS
//     let netuid: u16 = 0;
//     let n : u16 = 100;
//     let num_zero_uids : u16 = 10;
//     let blocks_per_epoch_list : u64 = 1;
//     let stake_per_module : u64 = 10_000;
    
//     // SETUP NETWORK
//     let tempo: u16 = 1;
//     register_n_modules( netuid, n, stake_per_module );
//     SubspaceModule::set_tempo( netuid, tempo );
//     SubspaceModule::set_max_allowed_weights(netuid, n );
//     SubspaceModule::set_min_allowed_weights(netuid, 0 );
//     SubspaceModule::set_immunity_period(netuid, tempo );

//     let keys = SubspaceModule::get_keys( netuid );
//     let uids = SubspaceModule::get_uids( netuid );
//     // do a list of ones for weights
//     let weight_uids : Vec<u16> = (0..n).collect();
//     // do a list of ones for weights
//     let mut weight_values : Vec<u16> = weight_uids.iter().map(|x| 1 as u16 ).collect();

//     let mut shuffled_uids: Vec<u16> = weight_uids.clone().to_vec();
//     shuffled_uids.shuffle(&mut thread_rng());

//     let mut zero_uids : Vec<u16> = shuffled_uids[0..num_zero_uids as usize].to_vec();

//     for uid in zero_uids.iter() {
//         weight_values[*uid as usize] = 0;
        
//     }
//     let old_n  : u16 = SubspaceModule::get_subnet_n( netuid );
//     set_weights(netuid, keys[0], weight_uids.clone() , weight_values.clone() );
//     step_block( tempo );
//     let n: u16 = SubspaceModule::get_subnet_n( netuid );
//     assert !( old_n - num_zero_uids == n );
    
//     });


// }


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




#[test]
fn simulation_final_boss() {
	new_test_ext().execute_with(|| {
    // CONSSTANTS
    let netuid: u16 = 0;
    let n : u16 = 100;
    let blocks_per_epoch_list : u64 = 1;
    let stake_per_module : u64 = 10_000;
    let tempo : u16 = 1;
    let num_blocks : u64 = 100;
    let min_stake : u64 = 1000;

    // SETUP NETWORK
    for i in 0..n {

        let key: U256 = U256::from(i);
        register_module( netuid, key, stake_per_module );
    }


    let mut keys : Vec<U256> = SubspaceModule::get_keys( netuid );



    for i in 0..n {

        let key: U256 = U256::from(i);
        let mut weight_uids : Vec<u16> = (0..n).collect();
        weight_uids.shuffle(&mut thread_rng());

        // shuffle the stakers
        let stake_ratio : u16 = thread_rng().gen_range(0..n) as u16;
        keys.shuffle(&mut thread_rng());
        let mut staker_keys : Vec<U256> = keys.clone()[0..stake_ratio as usize].to_vec();


        for mut staker_key in staker_keys.iter() {
            let staker_stake : u64 = SubspaceModule::get_self_stake( netuid, staker_key );
            let stake_balance : u64 = SubspaceModule::get_balance_u64( staker_key );

            if staker_stake < min_stake {
                continue;
            }
            println!("staker_stake: {:?}", staker_stake);

            let stake_amount: u64 = thread_rng().gen_range(1..staker_stake) as u64;
            let origin = get_origin(*staker_key);

            println!("staker_key: {:?}", staker_key);
            println!("stake_amount: {:?}", stake_amount);
            println!("staker_stake: {:?}", staker_stake);
            
            SubspaceModule::remove_stake(origin.clone(), netuid, *staker_key, stake_amount ).unwrap();
            let stake_balance : u64 = SubspaceModule::get_balance_u64( staker_key );
            println!("stake_balance: {:?}", stake_balance);

            SubspaceModule::add_stake(origin, netuid, key, stake_amount ).unwrap();
        }
    }

    SubspaceModule::set_tempo( netuid, 1 );
    SubspaceModule::set_max_allowed_weights(netuid, n );
    SubspaceModule::set_min_allowed_weights(netuid, 1 );
    SubspaceModule::set_max_allowed_uids(netuid, n );
    

    // do a list of ones for weights

    let keys: Vec<U256> = SubspaceModule::get_keys( netuid );
    let mut expected_total_stake: u64 = SubspaceModule::get_total_subnet_stake( netuid );

    for i in 0..num_blocks {
        let mut weight_uids : Vec<u16> = (0..n).collect();
        weight_uids.shuffle(&mut thread_rng());
        // do a list of ones for weights
        // normal distribution

        for i in 0..n {
            let mut rng = thread_rng();
            let mut weight_values : Vec<u16> = weight_uids.iter().map(|x| rng.gen_range(0..100) as u16 ).collect();
            weight_values.shuffle(&mut thread_rng());
            let key_stake: u64 = SubspaceModule::get_stake( netuid, &keys[i as usize] );
            if key_stake == 0 {
                continue;
            }

            set_weights(netuid, keys[i as usize], weight_uids.clone() , weight_values.clone() );
        }

        let test_key = keys.choose(&mut thread_rng()).unwrap();
        let test_uid = SubspaceModule::get_uid_for_key( netuid, test_key );
        let test_key_stake_before : u64 = SubspaceModule::get_stake( netuid, test_key );
        let test_key_stake_from_vector_before : Vec<(U256, u64)> = SubspaceModule::get_stake_from_vector( netuid, test_key );


        step_block( tempo );
        let emissions : Vec<u64> = SubspaceModule::get_emissions( netuid );

        let test_key_stake : u64 = SubspaceModule::get_stake( netuid, test_key );
        let test_key_stake_from_vector : Vec<(U256, u64)> = SubspaceModule::get_stake_from_vector( netuid, test_key );
        let test_key_stake_from_vector_sum : u64 = test_key_stake_from_vector.iter().map(|x| x.1 ).sum();
        assert!( test_key_stake == test_key_stake_from_vector_sum, "test_key_stake: {} != test_key_stake_from_vector_sum: {}", test_key_stake, test_key_stake_from_vector_sum );

        let test_key_stake_difference : u64 = test_key_stake - test_key_stake_before;
        let test_key_emission = emissions[test_uid as usize];
        let errror_delta : u64 = (test_key_emission as f64 * 0.001) as u64;
        assert!( test_key_stake_difference > test_key_emission - errror_delta || test_key_stake_difference < test_key_emission + errror_delta, "test_key_stake_difference: {} != test_key_emission: {}", test_key_stake_difference, test_key_emission ); 
        
        println!("test_uid {}", test_uid);
        println!("test_key_stake_from_vector_before: {:?}", test_key_stake_from_vector_before);
        println!("test_key_stake_from_vector: {:?}", test_key_stake_from_vector);
        for (i,(stake_key, stake_amount)) in test_key_stake_from_vector.iter().enumerate() {
            let stake_ratio : f64 = *stake_amount as f64 / test_key_stake as f64;

            let expected_emission : u64 = (test_key_emission as f64 * stake_ratio) as u64;
            println!("expected_emission: {}", expected_emission);
            println!("emissions[i]: {}", emissions[i]);
            println!("stake_amount: {}", stake_amount);
            println!("test_key_stake_from_vector_before[i].1): {}", test_key_stake_from_vector_before[i].1);

            let errror_delta : u64 = (*stake_amount as f64 * 0.001) as u64;

            let test_key_difference : u64 = stake_amount - test_key_stake_from_vector_before[i].1;
            assert!( test_key_difference < expected_emission + errror_delta ||  test_key_difference > expected_emission - errror_delta ,  "test_key_difference: {} != expected_emission: {}", test_key_difference, expected_emission );
            
        }


        // check stake key
        
        let lowest_priority_uid: u16 = SubspaceModule::get_lowest_uid(netuid);
        let lowest_priority_key: U256 = SubspaceModule::get_key_for_uid(netuid, lowest_priority_uid);
        let mut lowest_priority_stake: u64 = SubspaceModule::get_stake( netuid, &lowest_priority_key );
        let mut lowest_priority_balance: u64 = SubspaceModule::get_balance_u64(&lowest_priority_key );
        println!("lowest_priority_stake (BEFORE DEREG): {}", lowest_priority_stake);
        println!("lowest_priority_balance (BEFORE DEREG): {}", lowest_priority_balance);
        
        let new_key : U256 = U256::from( n + i as u16 + 1 );
        register_module( netuid, new_key, stake_per_module );
        assert!( !SubspaceModule::is_key_registered( netuid, &lowest_priority_key) );
        assert!( SubspaceModule::get_subnet_n( netuid ) == n );

        expected_total_stake += SubspaceModule::get_subnet_emission( netuid ) as u64 + stake_per_module;
        expected_total_stake -= lowest_priority_stake;

        lowest_priority_stake = SubspaceModule::get_stake( netuid, &lowest_priority_key );
        lowest_priority_balance = SubspaceModule::get_balance_u64( &lowest_priority_key );
        println!("lowest_priority_stake (AFTER DEREG): {}", lowest_priority_stake);
        println!("lowest_priority_balance (BEFORE DEREG): {}", lowest_priority_balance);

        assert!( lowest_priority_stake == 0 );
        assert!( SubspaceModule::get_stake( netuid, &new_key ) == stake_per_module );
        let emissions: Vec<u64> = SubspaceModule::get_emissions( netuid );

        let sumed_emission  : u64 = emissions.iter().sum();
        let expected_emission : u64 = SubspaceModule::get_subnet_emission( netuid )  as u64;
        println!("sumed_emission: {}", sumed_emission);
        println!("expected_emission: {}", expected_emission);

        let delta : u64 = 10_000_000;
        assert!( sumed_emission > expected_emission - delta || sumed_emission < expected_emission + delta );



        let total_stake = SubspaceModule::get_total_subnet_stake( netuid );
        assert!( total_stake > expected_total_stake - delta  || total_stake < expected_total_stake + delta , "total_stake: {} != expected_total_stake: {}", total_stake, expected_total_stake );
    

    

    }


    
    });


}