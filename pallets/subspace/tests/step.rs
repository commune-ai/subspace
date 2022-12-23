mod mock;
use mock::*;

/***********************************************************
	staking::add_stake() tests
************************************************************/

// Tests the step with a single neuron with stake.
#[test]
fn test_step_with_many() {
    new_test_ext().execute_with( || {
        Subspace::set_max_registratations_per_block( 100 );
        let initial_stake:u64 = 1000000000;
        for i in 0..4 { let nonce:u64 = 1000000000*i; register_ok_neuron_with_nonce(i as u64, i as u64, nonce); }
        // Set weights.
        let weights_matrix: Vec<Vec<u32>> = vec! [
            vec! [u32::max_value(), 0, 0, 0],
            vec! [0, u32::max_value(), 0, 0],
            vec! [0, 0, u32::max_value(), 0], 
            vec! [0, 0, 0, u32::max_value()],
        ];
        Subspace::set_stake_from_vector( vec![ initial_stake; 4 ] );
        assert_eq!( Subspace::get_neuron_count(), 4 );
        assert_eq!( Subspace::get_total_issuance(), Subspace::get_initial_total_issuance() + 0 );
        assert_eq!( Subspace::get_stake(), vec![ initial_stake; 4 ] );
        assert_eq!( Subspace::get_ranks(), vec![0; 4] );
        assert_eq!( Subspace::get_trust(), vec![0; 4] );
        assert_eq!( Subspace::get_active(), vec![1; 4] );
        assert_eq!( Subspace::get_consensus(), vec![0; 4] );
        assert_eq!( Subspace::get_incentive(), vec![0; 4] );
        assert_eq!( Subspace::get_emission(), vec![0; 4] );
        assert_eq!( Subspace::get_dividends(), vec![0; 4] );
        assert_eq!( Subspace::get_bonds(), vec![ [ 0; 4]; 4]);
        assert_eq!( Subspace::get_weights(), weights_matrix );
        run_to_block( 1 );
        assert_eq!( Subspace::get_neuron_count(), 4 );
        assert_eq!( Subspace::get_total_issuance(), Subspace::get_initial_total_issuance() + 0 );
        assert_eq!( Subspace::get_stake(), vec![ initial_stake; 4 ] );
        assert_eq!( Subspace::get_ranks(), vec![0; 4] );
        assert_eq!( Subspace::get_trust(), vec![0; 4] );
        assert_eq!( Subspace::get_active(), vec![1; 4] );
        assert_eq!( Subspace::get_consensus(), vec![0; 4] );
        assert_eq!( Subspace::get_incentive(), vec![0; 4] );
        assert_eq!( Subspace::get_emission(), vec![0; 4] );
        assert_eq!( Subspace::get_dividends(), vec![0; 4] );
        assert_eq!( Subspace::get_bonds(), vec![ [ 0; 4]; 4]);
        assert_eq!( Subspace::get_weights(), weights_matrix );
    });
}




// Tests the step with a single neuron with stake.
#[test]
fn test_step_with_many_zero_weights() {
    new_test_ext().execute_with( || {
        Subspace::set_max_registratations_per_block( 100 );
        let initial_stake:u64 = 1000000000;
        for i in 0..4 { let nonce:u64 = 1000000000*i; register_ok_neuron_with_nonce(i as u64, i as u64, nonce); }
        // Set stake.
        Subspace::set_stake_from_vector( vec![ initial_stake; 4 ] );
        // Set weights.
        let weights_matrix: Vec<Vec<u32>> = vec! [
            vec! [u32::max_value(), 0, 0, 0],
            vec! [0, u32::max_value(), 0, 0],
            vec! [0, 0, u32::max_value(), 0], 
            vec! [0, 0, 0, u32::max_value()],
        ];
        Subspace::set_weights_from_matrix( weights_matrix.clone() );

        assert_eq!( Subspace::get_neuron_count(), 4 );
        assert_eq!( Subspace::get_total_issuance(), Subspace::get_initial_total_issuance() + 0 );
        assert_eq!( Subspace::get_stake(), vec![ initial_stake; 4 ] );
        assert_eq!( Subspace::get_ranks(), vec![0; 4] );
        assert_eq!( Subspace::get_trust(), vec![0; 4] );
        assert_eq!( Subspace::get_active(), vec![1; 4] );
        assert_eq!( Subspace::get_consensus(), vec![0; 4] );
        assert_eq!( Subspace::get_incentive(), vec![0; 4] );
        assert_eq!( Subspace::get_emission(), vec![0; 4] );
        assert_eq!( Subspace::get_dividends(), vec![0; 4] );
        assert_eq!( Subspace::get_bonds(), vec![ [ 0; 4]; 4]);
        assert_eq!( Subspace::get_weights(), weights_matrix );
        run_to_block( 1 );
        assert_eq!( Subspace::get_neuron_count(), 4 );
        assert_eq!( Subspace::get_total_issuance(), Subspace::get_initial_total_issuance() + 0 );
        assert_eq!( Subspace::get_stake(), vec![ initial_stake; 4 ] );
        assert_eq!( Subspace::get_ranks(), vec![0; 4] );
        assert_eq!( Subspace::get_trust(), vec![0; 4] );
        assert_eq!( Subspace::get_active(), vec![1; 4] );
        assert_eq!( Subspace::get_consensus(), vec![0; 4] );
        assert_eq!( Subspace::get_incentive(), vec![0; 4] );
        assert_eq!( Subspace::get_emission(), vec![0; 4] );
        assert_eq!( Subspace::get_dividends(), vec![0; 4] );
        assert_eq!( Subspace::get_bonds(), vec![ [ 0; 4]; 4]);
        assert_eq!( Subspace::get_weights(), weights_matrix );
    });
}

// Tests the step with a single neuron with stake.
#[test]
fn test_step_with_many_self_weights() {
    new_test_ext().execute_with( || {
        Subspace::set_max_registratations_per_block( 100 );
        let initial_stake:u64 = 1000000000;
        for i in 0..4 { let nonce:u64 = 1000000000*i; register_ok_neuron_with_nonce(i as u64, i as u64, nonce); }
        // Set stake.
        Subspace::set_stake_from_vector( vec![ initial_stake; 4 ] );
        // Set weights.
        let weights_matrix: Vec<Vec<u32>> = vec! [
            vec! [u32::max_value(), 0, 0, 0 ],
            vec! [0, u32::max_value(), 0, 0 ],
            vec! [0, 0, u32::max_value(), 0 ], 
            vec! [0, 0, 0, u32::max_value() ],
        ];
        Subspace::set_weights_from_matrix( weights_matrix.clone() );

        assert_eq!( Subspace::get_neuron_count(), 4 );
        assert_eq!( Subspace::get_stake(), vec![ initial_stake; 4 ] );
        assert_eq!( Subspace::get_weights(), weights_matrix );
        run_to_block( 1 );
        assert_eq!( Subspace::get_neuron_count(), 4 );
        assert_eq!( Subspace::get_total_issuance(), Subspace::get_initial_total_issuance() + 0 );
        assert_eq!( Subspace::get_stake(), vec![ initial_stake; 4 ] );
        assert_eq!( Subspace::get_ranks(), vec![0; 4] );
        assert_eq!( Subspace::get_trust(), vec![0; 4] );
        assert_eq!( Subspace::get_active(), vec![1; 4] );
        assert_eq!( Subspace::get_consensus(), vec![0; 4] );
        assert_eq!( Subspace::get_incentive(), vec![0; 4] );
        assert_eq!( Subspace::get_emission(), vec![0; 4] );
        assert_eq!( Subspace::get_dividends(), vec![0; 4] );
        assert_eq!( Subspace::get_bonds(), vec![ [ 0; 4]; 4]);
        assert_eq!( Subspace::get_weights(), weights_matrix );
    });
}

pub fn approx_equals( a:u64, b: u64, eps: u64 ) -> bool {
    if a > b {
        if a - b > eps {
            println!("a({:?}) - b({:?}) > {:?}", a, b, eps);
            return false;
        }
    }
    if b > a {
        if b - a > eps {
            println!("b({:?}) - a({:?}) > {:?}", b, a, eps);
            return false;
        }
    }
    return true;
}

pub fn vec_approx_equals( a_vec: &Vec<u64>, b_vec: &Vec<u64>, eps: u64 ) -> bool {
    for (a, b) in a_vec.iter().zip(b_vec.iter()) {
        if !approx_equals( *a, *b, eps ){
            return false;
        }
    }
    return true;
}

pub fn mat_approx_equals( a_vec: &Vec<Vec<u64>>, b_vec: &Vec<Vec<u64>>, eps: u64 ) -> bool {
    for (a, b) in a_vec.iter().zip(b_vec.iter()) {
        if !vec_approx_equals( a, b, eps ){
            return false;
        }
    }
    return true;
}

#[test]
fn test_two_steps_with_many_outward_weights() {
    new_test_ext().execute_with( || {
        Subspace::set_max_registratations_per_block( 100 );
        let initial_stake:u64 = 1000000000;
        let u64m: u64 = 18446744073709551615;
        for i in 0..4 { let nonce:u64 = 1000000000*i; register_ok_neuron_with_nonce(i as u64, i as u64, nonce); }
        // Set stake.
        Subspace::set_stake_from_vector( vec![ initial_stake; 4 ] );
        // Shifted weights.
        let weights_matrix: Vec<Vec<u32>> = vec! [
            vec! [0, u32::max_value(), 0, 0 ],
            vec! [0, 0, u32::max_value(), 0 ],
            vec! [0, 0, 0, u32::max_value() ], 
            vec! [u32::max_value(), 0, 0, 0 ],
        ];
        Subspace::set_weights_from_matrix( weights_matrix.clone() );
        assert_eq!( Subspace::get_neuron_count(), 4 );
        assert_eq!( Subspace::get_stake(), vec![ initial_stake; 4 ] );
        assert_eq!( Subspace::get_weights(), weights_matrix );

        step_block (1);

        assert_eq!( Subspace::get_neuron_count(), 4 );
        assert!( approx_equals( Subspace::get_total_issuance(), Subspace::get_initial_total_issuance() + 1000000000, 10)); // approx
        assert!( vec_approx_equals ( &Subspace::get_stake(), &vec![1250000000, 1250000000, 1250000000, 1250000000], 10) );
        assert!( vec_approx_equals ( &Subspace::get_ranks(), &vec![u64m/4, u64m/4, u64m/4, u64m/4], 100) );
        assert!( vec_approx_equals ( &Subspace::get_trust(), &vec![u64m/4, u64m/4, u64m/4, u64m/4], 100) );
        assert_eq!( Subspace::get_active(), vec![1; 4] );
        assert!( vec_approx_equals ( &Subspace::get_consensus(), &vec![1399336432749266785, 1399336432749266785, 1399336432749266785, 1399336432749266785], 10) );
        assert!( vec_approx_equals ( &Subspace::get_incentive(), &vec![u64m/4, u64m/4, u64m/4, u64m/4], 100) );
        assert!( vec_approx_equals ( &Subspace::get_dividends(), &vec![u64m/4, u64m/4, u64m/4, u64m/4], 100) );
        assert!( vec_approx_equals ( &Subspace::get_emission(), &vec![250000000, 250000000, 250000000, 250000000], 10) );
        let expected_bonds: Vec<Vec<u64>> = vec! [
            vec! [0, 125000000, 0, 0 ],
            vec! [0, 0, 125000000, 0 ],
            vec! [0, 0, 0, 125000000 ], 
            vec! [125000000, 0, 0, 0 ],
        ]; // 250,000,000 * 1/2
        println!(  "{:?} {:?}", expected_bonds, Subspace::get_bonds() );
        assert!( mat_approx_equals ( &Subspace::get_bonds(), &expected_bonds, 10) );
        assert_eq!( Subspace::get_last_mechanism_step_block(), 1 );

        step_block (1);

        assert_eq!( Subspace::get_neuron_count(), 4 );
        assert!( approx_equals( Subspace::get_total_issuance(), Subspace::get_initial_total_issuance() + 2000000000, 100)); // approx
        assert!( vec_approx_equals ( &Subspace::get_stake(), &vec![1500000000, 1500000000, 1500000000, 1500000000], 100) );
        assert!( vec_approx_equals ( &Subspace::get_ranks(), &vec![u64m/4, u64m/4, u64m/4, u64m/4], 100) );
        assert!( vec_approx_equals ( &Subspace::get_trust(), &vec![u64m/4, u64m/4, u64m/4, u64m/4], 100) );
        assert_eq!( Subspace::get_active(), vec![1; 4] );
        assert!( vec_approx_equals ( &Subspace::get_consensus(), &vec![1399336432749266785, 1399336432749266785, 1399336432749266785, 1399336432749266785], 10) );
        assert!( vec_approx_equals ( &Subspace::get_incentive(), &vec![u64m/4, u64m/4, u64m/4, u64m/4], 100) );
        assert!( vec_approx_equals ( &Subspace::get_dividends(), &vec![u64m/4, u64m/4, u64m/4, u64m/4], 100) );
        assert!( vec_approx_equals ( &Subspace::get_emission(), &vec![250000000, 250000000, 250000000, 250000000], 10) );
        let expected_bonds: Vec<Vec<u64>> = vec! [
            vec! [0, 187_500_000, 0, 0 ],
            vec! [0, 0, 187_500_000, 0 ],
            vec! [0, 0, 0, 187_500_000 ], 
            vec! [187_500_000, 0, 0, 0 ],
        ]; // 125000000 * 1/2 + 250,000,000 * 1/2
        assert!( mat_approx_equals ( &Subspace::get_bonds(), &expected_bonds, 100) );
        assert_eq!( Subspace::get_last_mechanism_step_block(), 2 );

        step_block ( 8 );

        assert_eq!( Subspace::get_neuron_count(), 4 );
        assert!( approx_equals( Subspace::get_total_issuance(), Subspace::get_initial_total_issuance() + 1000000000 * 10, 100)); // approx
        assert!( vec_approx_equals ( &Subspace::get_stake(), &vec![1000000000 + 250000000 * 10, 1000000000 + 250000000 * 10, 1000000000 + 250000000 * 10, 1000000000 + 250000000 * 10], 100) );
        assert!( vec_approx_equals ( &Subspace::get_ranks(), &vec![u64m/4, u64m/4, u64m/4, u64m/4], 100) );
        assert!( vec_approx_equals ( &Subspace::get_trust(), &vec![u64m/4, u64m/4, u64m/4, u64m/4], 100) );
        assert_eq!( Subspace::get_active(), vec![1; 4] );
        assert!( vec_approx_equals ( &Subspace::get_consensus(), &vec![1399336432749266785, 1399336432749266785, 1399336432749266785, 1399336432749266785], 10) );
        assert!( vec_approx_equals ( &Subspace::get_incentive(), &vec![u64m/4, u64m/4, u64m/4, u64m/4], 100) );
        assert!( vec_approx_equals ( &Subspace::get_dividends(), &vec![u64m/4, u64m/4, u64m/4, u64m/4], 100) );
        assert!( vec_approx_equals (  &Subspace::get_emission(), &vec![250000000, 250000000, 250000000, 250000000], 10) );
        let expected_bonds: Vec<Vec<u64>> = vec! [
            vec! [0, 249_755_859, 0, 0 ],
            vec! [0, 0, 249_755_859, 0 ],
            vec! [0, 0, 0, 249_755_859 ], 
            vec! [ 249_755_859, 0, 0, 0],
        ];
        assert!( mat_approx_equals ( &Subspace::get_bonds(), &expected_bonds, 100) );

    });
}

#[test]
fn test_two_steps_with_reset_bonds() {
    new_test_ext().execute_with( || {
        Subspace::set_max_registratations_per_block( 100 );
        let initial_stake:u64 = 1000000000;
        for i in 0..4 { let nonce:u64 = 1000000000*i; register_ok_neuron_with_nonce(i as u64, i as u64, nonce); }
        // Set stake.
        Subspace::set_stake_from_vector( vec![ initial_stake; 4 ] );
        // Shifted weights.
        let weights_matrix: Vec<Vec<u32>> = vec! [
            vec! [0, u32::max_value(), 0, 0 ],
            vec! [0, 0, u32::max_value(), 0 ],
            vec! [0, 0, 0, u32::max_value() ], 
            vec! [u32::max_value(), 0, 0, 0 ],
        ];
        Subspace::set_weights_from_matrix( weights_matrix.clone() );
        assert_eq!( Subspace::get_neuron_count(), 4 );
        assert_eq!( Subspace::get_stake(), vec![ initial_stake; 4 ] );
        assert_eq!( Subspace::get_weights(), weights_matrix );
        step_block (1);
        let expected_bonds: Vec<Vec<u64>> = vec! [
            vec! [0, 125000000, 0, 0 ],
            vec! [0, 0, 125000000, 0 ],
            vec! [0, 0, 0, 125000000 ], 
            vec! [125000000, 0, 0, 0 ],
        ]; // 250,000,000 * 1/2
        println!(  "{:?} {:?}", expected_bonds, Subspace::get_bonds() );
        assert!( mat_approx_equals ( &Subspace::get_bonds(), &expected_bonds, 10) );
        Subspace::reset_bonds();
        let expected_bonds: Vec<Vec<u64>> = vec! [
            vec! [0, 0, 0, 0 ],
            vec! [0, 0, 0, 0 ],
            vec! [0, 0, 0, 0 ], 
            vec! [0, 0, 0, 0 ],
        ];
        assert!( mat_approx_equals ( &Subspace::get_bonds(), &expected_bonds, 0) );
        step_block (1);
        let expected_bonds: Vec<Vec<u64>> = vec! [
            vec! [0, 125000000, 0, 0 ],
            vec! [0, 0, 125000000, 0 ],
            vec! [0, 0, 0, 125000000 ], 
            vec! [125000000, 0, 0, 0 ],
        ]; // 250,000,000 * 1/2
        println!(  "{:?} {:?}", expected_bonds, Subspace::get_bonds() );
        assert!( mat_approx_equals ( &Subspace::get_bonds(), &expected_bonds, 10) );
    });
}



// #[test]
// fn test_steps_with_foundation_distribution() {
//     new_test_ext().execute_with( || {
//         Subspace::set_max_registratations_per_block( 100 );
//         let initial_stake:u64 = 1000000000;
//         for i in 0..4 {
//             register_ok_neuron(i as u64, i as u64 );
//         }
//         let weights_matrix: Vec<Vec<u32>> = vec! [
//             vec! [0, u32::max_value(), 0, 0 ],
//             vec! [0, 0, u32::max_value(), 0 ],
//             vec! [0, 0, 0, u32::max_value() ], 
//             vec! [u32::max_value(), 0, 0, 0 ],
//         ];
//         Subspace::set_weights_from_matrix( weights_matrix.clone() );
//         Subspace::set_stake_from_vector( vec![ initial_stake; 4 ] );
//         step_block (1);
//         assert!( approx_equals( Subspace::get_total_issuance(), Subspace::get_initial_total_issuance() + 1000000000, 10)); // approx
//         assert!( vec_approx_equals ( &Subspace::get_stake(), &vec![1250000000, 1250000000, 1250000000, 1250000000], 10) );
//         assert!( vec_approx_equals ( &Subspace::get_emission(), &vec![250000000, 250000000, 250000000, 250000000], 10) );
//         assert_eq!( Subspace::get_coldkey_balance( &Subspace::get_foundation_account() ), 0);

//         Subspace::set_foundation_distribution( 50 );
//         step_block (1);
//         assert!( approx_equals( Subspace::get_total_issuance(), Subspace::get_initial_total_issuance() + 2000000000, 10)); // approx
//         assert!( vec_approx_equals ( &Subspace::get_stake(), &vec![1375000000, 1375000000, 1375000000, 1375000000], 10) );
//         assert!( vec_approx_equals ( &Subspace::get_emission(), &vec![125000000, 125000000, 125000000, 125000000], 10) );
//         assert_eq!( Subspace::get_coldkey_balance( &Subspace::get_foundation_account() ), 500000000);

//         Subspace::set_foundation_distribution( 0 );
//         step_block (1);
//         assert!( approx_equals( Subspace::get_total_issuance(), Subspace::get_initial_total_issuance() + 3000000000, 10)); // approx
//         assert!( vec_approx_equals ( &Subspace::get_stake(), &vec![1625000000, 1625000000, 1625000000, 1625000000], 10) );
//         assert!( vec_approx_equals ( &Subspace::get_emission(), &vec![250000000, 250000000, 250000000, 250000000], 10) );
//         assert_eq!( Subspace::get_coldkey_balance( &Subspace::get_foundation_account() ), 500000000);

//         // Test set foundation account.
//         Subspace::set_foundation_distribution( 50 );
//         let prev_foundation_account: u64 = Subspace::get_foundation_account();
//         Subspace::set_foundation_account( 1 ); 
//         assert_eq!( Subspace::get_foundation_account(), 1 );
//         step_block (1);
//         assert!( approx_equals( Subspace::get_total_issuance(), Subspace::get_initial_total_issuance() + 4000000000, 10)); // approx
//         assert!( vec_approx_equals ( &Subspace::get_stake(), &vec![1750000000, 1750000000, 1750000000, 1750000000], 10) );
//         assert!( vec_approx_equals ( &Subspace::get_emission(), &vec![125000000, 125000000, 125000000, 125000000], 10) );
//         assert_eq!( Subspace::get_coldkey_balance( &prev_foundation_account ), 500000000);
//         assert_eq!( Subspace::get_coldkey_balance( &Subspace::get_foundation_account() ), 500000000);
//     });
// }


#[test]
fn test_step_only_every_3_with_many_outward_weights() {
    new_test_ext().execute_with( || {
        Subspace::set_max_registratations_per_block( 100 );
        let initial_stake:u64 = 1000000000;
        for i in 0..4 { let nonce:u64 = 1000000000*i; register_ok_neuron_with_nonce(i as u64, i as u64, nonce); }
        let weights_matrix: Vec<Vec<u32>> = vec! [
            vec! [0, u32::max_value(), 0, 0 ],
            vec! [0, 0, u32::max_value(), 0 ],
            vec! [0, 0, 0, u32::max_value() ], 
            vec! [u32::max_value(), 0, 0, 0 ],
        ];
        Subspace::set_stake_from_vector( vec![ initial_stake; 4 ] );
        Subspace::set_weights_from_matrix( weights_matrix.clone() );
        // Check 3.
        Subspace::set_blocks_per_step(3);
        assert!( approx_equals( Subspace::get_total_issuance(), Subspace::get_initial_total_issuance(), 1)); // approx
        step_block (1);
        assert!( approx_equals( Subspace::get_total_issuance(), Subspace::get_initial_total_issuance(), 1)); // approx
        step_block (1);
        assert!( approx_equals( Subspace::get_total_issuance(), Subspace::get_initial_total_issuance(), 1)); // approx
        step_block (1);
        assert!( approx_equals( Subspace::get_total_issuance(), Subspace::get_initial_total_issuance() + 1000000000 * 3, 1000)); // approx
        // Check 1
        Subspace::set_blocks_per_step(1);
        step_block (1);
        assert!( approx_equals( Subspace::get_total_issuance(), Subspace::get_initial_total_issuance() + 1000000000 * 4, 1000)); // approx
        // Check 1 again.
        step_block (1);
        assert!( approx_equals( Subspace::get_total_issuance(), Subspace::get_initial_total_issuance() + 1000000000 * 5, 1000)); // approx
        // Check 5.
        Subspace::set_blocks_per_step(5);
        step_block (1);
        step_block (1);
        step_block (1);
        step_block (1);
        step_block (1);
        assert!( approx_equals( Subspace::get_total_issuance(), Subspace::get_initial_total_issuance() + 1000000000 * 10, 1000)); // approx
        // Check 5 again.
        step_block (1);
        step_block (1);
        step_block (1);
        step_block (1);
        step_block (1);
        assert!( approx_equals( Subspace::get_total_issuance(), Subspace::get_initial_total_issuance() + 1000000000 * 15, 1000)); // approx
        // Check 0 values.
        Subspace::set_blocks_per_step(0);
        step_block (1);
        assert!( approx_equals( Subspace::get_total_issuance(), Subspace::get_initial_total_issuance() + 1000000000 * 16, 1000)); // approx
        Subspace::set_blocks_per_step(10);
        step_block (1);
        step_block (1);
        step_block (1);
        step_block (1);
        step_block (1);
        step_block (1);
        step_block (1);
        step_block (1);
        // Check Lower step prematurely.
        Subspace::set_blocks_per_step(9);
        step_block (1);
        assert!( approx_equals( Subspace::get_total_issuance(), Subspace::get_initial_total_issuance() + 1000000000 * 25, 1000)); // approx
        // Check 100
        Subspace::set_blocks_per_step(100);
        step_block (100);
        assert!( approx_equals( Subspace::get_total_issuance(), Subspace::get_initial_total_issuance() + 1000000000 * 125, 1000)); // approx
    });
}



#[test]
fn test_two_steps_with_activity_cuttoff() {
    new_test_ext().execute_with( || {
        Subspace::set_max_registratations_per_block( 100 );
        let initial_stake:u64 = 1000000000;
        let u64m: u64 = 18446744073709551615;
        for i in 0..4 { let nonce:u64 = 1000000000*i; register_ok_neuron_with_nonce(i as u64, i as u64, nonce); }
        // Set stake.
        Subspace::set_stake_from_vector( vec![ initial_stake; 4 ] );
        Subspace::set_activity_cutoff( 2 );

        // Shifted weights.
        let weights_matrix: Vec<Vec<u32>> = vec! [
            vec! [0, u32::max_value(), 0, 0 ],
            vec! [0, 0, u32::max_value(), 0 ],
            vec! [0, 0, 0, u32::max_value() ], 
            vec! [u32::max_value(), 0, 0, 0 ],
        ];
        Subspace::set_weights_from_matrix( weights_matrix.clone() );
        assert_eq!( Subspace::get_neuron_count(), 4 );
        assert_eq!( Subspace::get_stake(), vec![ initial_stake; 4 ] );
        assert_eq!( Subspace::get_weights(), weights_matrix );

        step_block (1);

        assert_eq!( Subspace::get_neuron_count(), 4 );
        assert!( approx_equals( Subspace::get_total_issuance(), Subspace::get_initial_total_issuance() + 1000000000, 10)); // approx
        assert!( vec_approx_equals ( &Subspace::get_stake(), &vec![1250000000, 1250000000, 1250000000, 1250000000], 10) );
        assert!( vec_approx_equals ( &Subspace::get_ranks(), &vec![u64m/4, u64m/4, u64m/4, u64m/4], 100) );
        assert!( vec_approx_equals ( &Subspace::get_trust(), &vec![u64m/4, u64m/4, u64m/4, u64m/4], 100) );
        assert_eq!( Subspace::get_active(), vec![1; 4] );
        assert!( vec_approx_equals ( &Subspace::get_consensus(), &vec![1399336432749266785, 1399336432749266785, 1399336432749266785, 1399336432749266785], 10) );
        assert!( vec_approx_equals ( &Subspace::get_incentive(), &vec![u64m/4, u64m/4, u64m/4, u64m/4], 100) );
        assert!( vec_approx_equals ( &Subspace::get_dividends(), &vec![u64m/4, u64m/4, u64m/4, u64m/4], 100) );
        assert!( vec_approx_equals ( &Subspace::get_emission(), &vec![250000000, 250000000, 250000000, 250000000], 10) );
        let expected_bonds: Vec<Vec<u64>> = vec! [
            vec! [0, 125000000, 0, 0 ],
            vec! [0, 0, 125000000, 0 ],
            vec! [0, 0, 0, 125000000 ], 
            vec! [125000000, 0, 0, 0 ],
        ];
        assert!( mat_approx_equals ( &Subspace::get_bonds(), &expected_bonds, 10) );

        step_block (1);

        assert_eq!( Subspace::get_neuron_count(), 4 );
        assert!( approx_equals( Subspace::get_total_issuance(), Subspace::get_initial_total_issuance() + 1000000000, 100)); // approx
        assert!( vec_approx_equals ( &Subspace::get_stake(), &vec![1250000000, 1250000000, 1250000000, 1250000000], 100) );
        assert!( vec_approx_equals ( &Subspace::get_ranks(), &vec![0, 0, 0, 0], 100) );
        assert!( vec_approx_equals ( &Subspace::get_trust(), &vec![0, 0, 0, 0], 100) );
        assert_eq!( Subspace::get_active(), vec![0; 4] );
        assert!( vec_approx_equals ( &Subspace::get_consensus(), &vec![0, 0, 0, 0], 10) );
        assert!( vec_approx_equals ( &Subspace::get_incentive(), &vec![0, 0, 0, 0], 100) );
        assert!( vec_approx_equals ( &Subspace::get_dividends(), &vec![0, 0, 0, 0], 100) );
        assert!( vec_approx_equals ( &Subspace::get_emission(), &vec![0, 0, 0, 0], 10) );
        let expected_bonds: Vec<Vec<u64>> = vec! [
            vec! [0, 62500000, 0, 0 ],
            vec! [0, 0, 62500000, 0 ],
            vec! [0, 0, 0, 62500000 ], 
            vec! [62500000, 0, 0, 0 ],
        ];
        assert!( mat_approx_equals ( &Subspace::get_bonds(), &expected_bonds, 100) );
        
        step_block ( 8 );

        assert_eq!( Subspace::get_neuron_count(), 4 );
        assert!( approx_equals( Subspace::get_total_issuance(), Subspace::get_initial_total_issuance() + 1000000000, 100)); // approx
        assert!( vec_approx_equals ( &Subspace::get_stake(), &vec![1250000000, 1250000000, 1250000000, 1250000000], 100) );
        assert!( vec_approx_equals ( &Subspace::get_ranks(), &vec![0, 0, 0, 0], 100) );
        assert!( vec_approx_equals ( &Subspace::get_trust(), &vec![0, 0, 0, 0], 100) );
        assert_eq!( Subspace::get_active(), vec![0; 4] );
        assert!( vec_approx_equals ( &Subspace::get_consensus(), &vec![0, 0, 0, 0], 10) );
        assert!( vec_approx_equals ( &Subspace::get_incentive(), &vec![0, 0, 0, 0], 100) );
        assert!( vec_approx_equals ( &Subspace::get_dividends(), &vec![0, 0, 0, 0], 100) );
        assert!( vec_approx_equals ( &Subspace::get_emission(), &vec![0, 0, 0, 0], 10) );
        let expected_bonds: Vec<Vec<u64>> = vec! [
            vec! [0, 244140, 0, 0 ],
            vec! [0, 0, 244140, 0 ],
            vec! [0, 0, 0, 244140 ], 
            vec! [244140, 0, 0, 0 ],
        ];
        assert!( mat_approx_equals ( &Subspace::get_bonds(), &expected_bonds, 100) );

    });
}


#[test]
fn test_two_steps_with_partial_activity() {
    new_test_ext().execute_with( || {
        Subspace::set_max_registratations_per_block( 100 );
        let initial_stake:u64 = 1000000000;
        let u64m: u64 = 18446744073709551615;
        for i in 0..4 { let nonce:u64 = 1000000000*i; register_ok_neuron_with_nonce(i as u64, i as u64, nonce); }
        // Set stake.
        Subspace::set_stake_from_vector( vec![ initial_stake; 4 ] );
        Subspace::set_activity_cutoff( 1 );

        // Shifted weights.
        let weights_matrix: Vec<Vec<u32>> = vec! [
            vec! [0, u32::max_value(), 0, 0 ],
            vec! [0, 0, u32::max_value(), 0 ],
            vec! [0, 0, 0, u32::max_value() ], 
            vec! [u32::max_value(), 0, 0, 0 ],
        ];
        Subspace::set_weights_from_matrix( weights_matrix.clone() );
        assert_eq!( Subspace::get_neuron_count(), 4 );
        assert_eq!( Subspace::get_stake(), vec![ initial_stake; 4 ] );
        assert_eq!( Subspace::get_weights(), weights_matrix );
        assert_eq!( Subspace::get_lastupdate(), vec![0,0,0,0] );

        step_block (1);
        assert_eq!( Subspace::get_neuron_count(), 4 );
        assert!( approx_equals( Subspace::get_total_issuance(), Subspace::get_initial_total_issuance(), 100)); // approx
        assert!( vec_approx_equals ( &Subspace::get_stake(), &vec![initial_stake, initial_stake, initial_stake, initial_stake], 100) );
        assert!( vec_approx_equals ( &Subspace::get_ranks(), &vec![0, 0, 0, 0], 100) );
        assert!( vec_approx_equals ( &Subspace::get_trust(), &vec![0, 0, 0, 0], 100) );
        assert_eq!( Subspace::get_active(), vec![0; 4] );
        assert!( vec_approx_equals ( &Subspace::get_consensus(), &vec![0, 0, 0, 0], 10) );
        assert!( vec_approx_equals ( &Subspace::get_incentive(), &vec![0, 0, 0, 0], 100) );
        assert!( vec_approx_equals ( &Subspace::get_dividends(), &vec![0, 0, 0, 0], 100) );
        assert!( vec_approx_equals ( &Subspace::get_emission(), &vec![0, 0, 0, 0], 10) );
        let expected_bonds: Vec<Vec<u64>> = vec! [
            vec! [0, 0, 0, 0 ],
            vec! [0, 0, 0, 0 ],
            vec! [0, 0, 0, 0 ], 
            vec! [0, 0, 0, 0 ],
        ];
        assert!( mat_approx_equals ( &Subspace::get_bonds(), &expected_bonds, 100) );

        Subspace::set_activity_cutoff( 2 );
        Subspace::set_last_update_from_vector( vec![1,0,0,0] );
        assert_eq!( Subspace::get_lastupdate(), vec![1,0,0,0] );
        step_block (1);

        assert_eq!( Subspace::get_neuron_count(), 4 );
        assert!( approx_equals( Subspace::get_total_issuance(), Subspace::get_initial_total_issuance() + 1000000000, 100)); // approx
        assert!( vec_approx_equals ( &Subspace::get_stake(), &vec![initial_stake + 500000000, initial_stake + 500000000, initial_stake, initial_stake], 100) );
        assert!( vec_approx_equals ( &Subspace::get_ranks(), &vec![0, u64m, 0, 0], 100) );
        assert!( vec_approx_equals ( &Subspace::get_trust(), &vec![0, u64m, 0, 0], 100) );
        assert_eq!( Subspace::get_active(), vec![1, 0, 0, 0] );
        assert!( vec_approx_equals ( &Subspace::get_incentive(), &vec![0, u64m, 0, 0], 100) );
        assert!( vec_approx_equals ( &Subspace::get_dividends(), &vec![u64m/2, u64m/2, 0, 0], 100) );
        assert!( vec_approx_equals ( &Subspace::get_emission(), &vec![500000000, 500000000, 0, 0], 10) );
        let expected_bonds: Vec<Vec<u64>> = vec! [
            vec! [0, 500000000, 0, 0 ],
            vec! [0, 0, 0, 0 ],
            vec! [0, 0, 0, 0 ], 
            vec! [0, 0, 0, 0 ],
        ];
        assert!( mat_approx_equals ( &Subspace::get_bonds(), &expected_bonds, 100) );


    });
}



// Tests the step without a neuron in the graph.
#[test]
fn test_run_step_ok() {
	new_test_ext().execute_with(|| {
        assert_eq!( Subspace::get_neuron_count(), 0 );
        assert_eq!( Subspace::get_total_stake(), 0 );
        assert_eq!( Subspace::get_total_issuance(), Subspace::get_initial_total_issuance() + 0 );
        run_to_block( 1 );
        assert_eq!( Subspace::get_neuron_count(), 0 );
        assert_eq!( Subspace::get_total_stake(), 0 );
        assert_eq!( Subspace::get_total_issuance(), Subspace::get_initial_total_issuance() + 0 );
	});
}

// Tests the step with a single neuron no stake.
#[test]
fn test_step_with_neuron_no_balances() {
    let coldkey:u64 = 1;
    let hotkey:u64 = 2;
    new_test_ext().execute_with( || {
        Subspace::set_max_registratations_per_block( 100 );
        let neuron = register_ok_neuron( hotkey, coldkey );
        assert_eq!( Subspace::get_neuron_count(), 1 );
        assert_eq!( Subspace::get_total_stake(), 0 );
        assert_eq!( Subspace::get_total_issuance(), Subspace::get_initial_total_issuance() + 0 );
        assert_eq!( Subspace::get_stake(), vec![0] );
        assert_eq!( Subspace::get_ranks(), vec![0] );
        assert_eq!( Subspace::get_trust(), vec![0] );
        assert_eq!( Subspace::get_active(), vec![1] );
        assert_eq!( Subspace::get_consensus(), vec![0] );
        assert_eq!( Subspace::get_incentive(), vec![0] );
        assert_eq!( Subspace::get_dividends(), vec![0] );
        assert_eq!( Subspace::get_bonds_for_neuron(&neuron), vec![0] );
        assert_eq!( Subspace::get_weights_for_neuron(&neuron), vec![u32::max_value()] );
        run_to_block( 1 );
        assert_eq!( Subspace::get_neuron_count(), 1 );
        assert_eq!( Subspace::get_total_stake(), 0);
        assert_eq!( Subspace::get_total_issuance(), Subspace::get_initial_total_issuance() + 0 );
        assert_eq!( Subspace::get_stake(), vec![0] );
        assert_eq!( Subspace::get_ranks(), vec![0] );
        assert_eq!( Subspace::get_trust(), vec![0] );
        assert_eq!( Subspace::get_active(), vec![1] );
        assert_eq!( Subspace::get_consensus(), vec![0] );
        assert_eq!( Subspace::get_incentive(), vec![0] );
        assert_eq!( Subspace::get_dividends(), vec![0] );
        assert_eq!( Subspace::get_bonds_for_neuron(&neuron), vec![0] );
        assert_eq!( Subspace::get_weights_for_neuron(&neuron), vec![u32::max_value()] );
    });
}

// Tests the step with a single neuron with stake.
#[test]
fn test_step_with_neuron_with_balances() {
    let coldkey:u64 = 1;
    let hotkey:u64= 2;
    let initial_stake:u64 = 1000000000;
    new_test_ext().execute_with( || {
        Subspace::set_max_registratations_per_block( 100 );
        let neuron = register_ok_neuron( hotkey, coldkey );
        Subspace::add_stake_to_neuron_hotkey_account(neuron.uid, initial_stake);
        assert_eq!( Subspace::get_total_stake(), initial_stake );
        assert_eq!( Subspace::get_total_issuance(), Subspace::get_initial_total_issuance() + 0 );
        assert_eq!( Subspace::get_neuron_count(), 1 );
        assert_eq!( Subspace::get_stake(), vec![initial_stake] );
        assert_eq!( Subspace::get_ranks(), vec![0] );
        assert_eq!( Subspace::get_trust(), vec![0] );
        assert_eq!( Subspace::get_active(), vec![1] );
        assert_eq!( Subspace::get_consensus(), vec![0] );
        assert_eq!( Subspace::get_incentive(), vec![0] );
        assert_eq!( Subspace::get_dividends(), vec![0] );
        assert_eq!( Subspace::get_bonds_for_neuron(&neuron), vec![0] );
        assert_eq!( Subspace::get_weights_for_neuron(&neuron), vec![u32::max_value()] );
        run_to_block( 1 );
        assert_eq!( Subspace::get_total_stake(), initial_stake );
        assert_eq!( Subspace::get_total_issuance(), Subspace::get_initial_total_issuance() + 0 );
        assert_eq!( Subspace::get_neuron_count(), 1 );
        assert_eq!( Subspace::get_stake(), vec![ initial_stake ] );
        assert_eq!( Subspace::get_ranks(), vec![0] );
        assert_eq!( Subspace::get_trust(), vec![0] );
        assert_eq!( Subspace::get_active(), vec![1] );
        assert_eq!( Subspace::get_consensus(), vec![0] );
        assert_eq!( Subspace::get_incentive(), vec![0] );
        assert_eq!( Subspace::get_dividends(), vec![0] );
    });
}
