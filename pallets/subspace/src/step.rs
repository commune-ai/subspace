use super::*;
use crate::math::*;
use frame_support::sp_std::vec;
use frame_support::inherent::Vec;
use substrate_fixed::types::{I32F32, I64F64, I96F32, I110F18};
use frame_support::storage::{IterableStorageMap, IterableStorageDoubleMap};

impl<T: Config> Pallet<T> { 
    // Helper function which returns the number of blocks remaining before we will run the epoch on this
    // network. Networks run their epoch when (block_number + netuid + 1 ) % (tempo + 1) = 0
    //


    // Iterates through networks queues more emission onto their pending storage.
    // If a network has no blocks left until tempo, we run the epoch function and generate
    // more token emission tuples for later draining onto accounts.
    //
    pub fn block_step( ) {
        let block_number: u64 = Self::get_current_block_as_u64();
        log::debug!("block_step for block: {:?} ", block_number );
        // --- 1. Adjust difficulties.
		Self::adjust_registration_terms_for_networks( );
        
        // --- 1. Iterate through network ids.
        for ( netuid, tempo )  in <Tempo<T> as IterableStorageMap<u16, u16>>::iter() {

            
            // --- 2. Queue the emission due to this network.
            let new_queued_emission = Self::get_token_emmision( netuid );
            PendingEmission::<T>::mutate( netuid, | queued | *queued += new_queued_emission );
            log::debug!("netuid_i: {:?} queued_emission: +{:?} ", netuid, new_queued_emission );  
            // --- 3. Check to see if this network has reached tempo.
            if Self::blocks_until_next_epoch( netuid, tempo, block_number ) != 0 {
                // --- 3.1 No epoch, increase blocks since last step and continue,
                Self::set_blocks_since_last_step( netuid, Self::get_blocks_since_last_step( netuid ) + 1 );
                continue;
            }

            // --- 4 This network is at tempo and we are running its epoch.
            // First frain the queued emission.
            let emission_to_drain:u64 = PendingEmission::<T>::get( netuid ); 
            PendingEmission::<T>::insert( netuid, 0 );

            // --- 5. Run the epoch mechanism and return emission tuples for keys in the network.
            let emission_tuples_this_block: Vec<(T::AccountId, u64)> = Self::epoch( netuid, emission_to_drain );

        }
    }

    // Calculates reward  values,then updates  incentive, dividend, emission and bonds, and 
    // returns the emissions for uids/keys in a given `netuid`.
    //
    // # Args:
    // 	* 'netuid': ( u16 ):
    //         - The network to distribute the emission onto.
    // 		
    // 	* 'debug' ( bool ):
    // 		- Print debugging outputs.
    //    
    pub fn epoch( netuid: u16, token_emission: u64 ) -> Vec<(T::AccountId, u64)> {
        // Get subnetwork size.
        let n: u16 = Self::get_subnetwork_n( netuid );
        log::trace!( "n: {:?}", n );

        // ======================
        // == Active & updated ==
        // ======================

        // Get current block.
        let current_block: u64 = Self::get_current_block_as_u64();
        log::trace!( "current_block: {:?}", current_block );

        // Get activity cutoff.
        let activity_cutoff: u64 = Self::get_activity_cutoff( netuid ) as u64;
        log::trace!( "activity_cutoff: {:?}", activity_cutoff );

        // Last update vector.
        let last_update: Vec<u64> = Self::get_last_update( netuid );
        log::trace!( "Last update: {:?}", &last_update );

        // Inactive mask.
        let inactive: Vec<bool> = last_update.iter().map(| updated | *updated + activity_cutoff < current_block ).collect();
        log::trace!( "Inactive: {:?}", inactive.clone() );

        // Logical negation of inactive.
        let active: Vec<bool> = inactive.iter().map(|&b| !b).collect();

        // Block at registration vector (block when each module was most recently registered).
        let block_at_registration: Vec<u64> = Self::get_block_at_registration( netuid );
        log::trace!( "Block at registration: {:?}", &block_at_registration );

        // ===========
        // == Stake ==
        // ===========

        let mut keys: Vec<(u16, T::AccountId)> = vec![];
        for ( uid_i, key ) in < Keys<T> as IterableStorageDoubleMap<u16, u16, T::AccountId >>::iter_prefix( netuid ) {
            keys.push( (uid_i, key) ); 
        }
        log::trace!( "keys: {:?}", &keys );

        // Access network stake as normalized vector.
        let mut stake_64: Vec<I64F64> = vec![ I64F64::from_num(0.0); n as usize ];
        for (uid_i, key) in keys.iter() {
            stake_64[ *uid_i as usize ] = I64F64::from_num( Self::get_stake_for_key(netuid, key ) );
        }
        inplace_normalize_64( &mut stake_64 );
        let stake: Vec<I32F32> = vec_fixed64_to_fixed32( stake_64 );
        // range: I32F32(0, 1)
        log::trace!( "S: {:?}", &stake );

        // Remove inactive stake.
        let mut active_stake: Vec<I32F32> = stake.clone();
        inplace_mask_vector( &inactive, &mut active_stake );
        log::trace!( "S (mask): {:?}", &active_stake );

        // Normalize active stake.
        inplace_normalize( &mut active_stake );
        log::trace!( "S (mask+norm): {:?}", &active_stake );

        // =============
        // == Weights ==
        // =============

        // Access network weights row normalized.
        let mut weights: Vec<Vec<(u16, I32F32)>> = Self::get_weights_sparse( netuid );

        // log::trace!( "W (permit): {:?}", &weights );

        // Remove self-weight by masking diagonal.
        weights = mask_diag_sparse( &weights );
        // log::trace!( "W (permit+diag): {:?}", &weights );

        // Remove weights referring to deregistered modules.
        weights = vec_mask_sparse_matrix( &weights, &last_update, &block_at_registration, &| updated, registered | updated <= registered );
        // log::trace!( "W (permit+diag+outdate): {:?}", &weights );

        // Normalize remaining weights.
        inplace_row_normalize_sparse( &mut weights );
        // log::trace!( "W (mask+norm): {:?}", &weights );

        // =============================
        // ==  Incentive ==
        // =============================

        // Compute incentive: r_j = SUM(i) w_ij * s_i.
        let mut incentive: Vec<I32F32> = matmul_sparse( &weights, &active_stake, n );
        inplace_normalize( &mut incentive );  // range: I32F32(0, 1)
        log::trace!( "Incentive: {:?}", &incentive );

        // =========================
        // == Bonds and Dividends ==
        // =========================

        // Access network bonds column normalized.
        let mut bonds: Vec<Vec<(u16, I32F32)>> = Self::get_bonds_sparse( netuid );
        // log::trace!( "B: {:?}", &bonds );
        
        // Remove bonds referring to deregistered modules.
        bonds = vec_mask_sparse_matrix( &bonds, &last_update, &block_at_registration, &| updated, registered | updated <= registered );
        // log::trace!( "B (outdatedmask): {:?}", &bonds );

        // Normalize remaining bonds: sum_i b_ij = 1.
        inplace_col_normalize_sparse( &mut bonds, n );
        // log::trace!( "B (mask+norm): {:?}", &bonds );

        // Compute bonds delta column normalized.
        let mut bonds_delta: Vec<Vec<(u16, I32F32)>> = row_hadamard_sparse( &weights, &active_stake ); // ΔB = W◦S (outdated W masked)
        // log::trace!( "ΔB: {:?}", &bonds_delta );

        // Normalize bonds delta.
        inplace_col_normalize_sparse( &mut bonds_delta, n ); // sum_i b_ij = 1
        // log::trace!( "ΔB (norm): {:?}", &bonds_delta );
    

        // Compute dividends: d_i = SUM(j) b_ij * inc_j.
        // range: I32F32(0, 1)
        let mut dividends: Vec<I32F32> = matmul_transpose_sparse( &bonds_delta, &incentive );
        inplace_normalize( &mut dividends );
        log::trace!( "D: {:?}", &dividends );

        // =================================
        // == Emission and Pruning scores ==
        // =================================

        // Compute normalized emission scores. range: I32F32(0, 1)
        let mut normalized_emission: Vec<I32F32> = incentive.iter().zip( dividends.clone() ).map( |(ii, di)| ii + di ).collect();
        inplace_normalize( &mut normalized_emission );

        // If emission is zero, replace emission with normalized stake.
        if is_zero( &normalized_emission ) { // no weights set | outdated weights | self_weights
            if is_zero( &active_stake ) { // no active stake
                normalized_emission = stake.clone(); // do not mask inactive, assumes stake is normalized
            }
            else {
                normalized_emission = active_stake.clone(); // emission proportional to inactive-masked normalized stake
            }
        }
        
        // Compute rao based emission scores. range: I96F32(0, token_emission)
        let float_token_emission: I96F32 = I96F32::from_num( token_emission );
        let emission: Vec<I96F32> = normalized_emission.iter().map( |e: &I32F32| I96F32::from_num( *e ) * float_token_emission ).collect();
        let emission: Vec<u64> = emission.iter().map( |e: &I96F32| e.to_num::<u64>() ).collect();
        log::trace!( "nE: {:?}", &normalized_emission );
        log::trace!( "E: {:?}", &emission );

        // ===================
        // == Value storage ==
        // ===================
        let cloned_emission: Vec<u64> = emission.clone();
        Emission::<T>::insert( netuid, cloned_emission );
        let cloned_incentive: Vec<u16> = incentive.iter().map(|xi| fixed_proportion_to_u16(*xi)).collect::<Vec<u16>>();
        Incentive::<T>::insert( netuid, cloned_incentive );
        let cloned_dividends: Vec<u16> = dividends.iter().map(|xi| fixed_proportion_to_u16(*xi)).collect::<Vec<u16>>();
        Dividends::<T>::insert( netuid, cloned_dividends );
        Active::<T>::insert( netuid, active.clone() );

        // Emission tuples ( keys, u64 emission)
        let mut result: Vec<(T::AccountId, u64)> = vec![]; 
        for ( uid_i, key ) in keys.iter() {
            result.push( ( key.clone(), emission[ *uid_i as usize ] ) );
        }


            
        // --- 6. emmit
        for (key, amount) in result.iter() {                 
            Self::increase_stake_on_account(netuid, &key, *amount );
        }    
    
        // --- 7 Set counters.
        Self::set_blocks_since_last_step( netuid, 0 );
        Self::set_last_mechanism_step_block( netuid, current_block );    

        result
    }

    pub fn get_normalized_stake( netuid:u16 ) -> Vec<I32F32> {
        let n: usize = Self::get_subnetwork_n( netuid ) as usize; 
        let mut stake_64: Vec<I64F64> = vec![ I64F64::from_num(0.0); n ]; 
        for module_uid in 0..n {
            stake_64[module_uid] = I64F64::from_num( Self::get_stake_for_uid( netuid, module_uid as u16 ) );
        }
        inplace_normalize_64( &mut stake_64 );
        let stake: Vec<I32F32> = vec_fixed64_to_fixed32( stake_64 );
        stake
    }

    pub fn get_block_at_registration( netuid:u16 ) -> Vec<u64> { 
        let n: usize = Self::get_subnetwork_n( netuid ) as usize;
        let mut block_at_registration: Vec<u64> = vec![ 0; n ];
        for module_uid in 0..n {
            if Keys::<T>::contains_key( netuid, module_uid as u16 ){
                block_at_registration[ module_uid ] = Self::get_module_block_at_registration( netuid, module_uid as u16 );
            }
        }
        block_at_registration
    }

    pub fn get_weights_sparse( netuid:u16 ) -> Vec<Vec<(u16, I32F32)>> { 
        let n: usize = Self::get_subnetwork_n( netuid ) as usize; 
        let mut weights: Vec<Vec<(u16, I32F32)>> = vec![ vec![]; n ]; 
        for ( uid_i, weights_i ) in < Weights<T> as IterableStorageDoubleMap<u16 ,u16, Vec<(u16, u16)> >>::iter_prefix( netuid ) {
            for (uid_j, weight_ij) in weights_i.iter() { 
                weights [ uid_i as usize ].push( ( *uid_j, u16_proportion_to_fixed( *weight_ij ) ));
            }
        }
        weights
    } 

    pub fn get_weights( netuid:u16 ) -> Vec<Vec<I32F32>> { 
        let n: usize = Self::get_subnetwork_n( netuid ) as usize; 
        let mut weights: Vec<Vec<I32F32>> = vec![ vec![ I32F32::from_num(0.0); n ]; n ]; 
        for ( uid_i, weights_i ) in < Weights<T> as IterableStorageDoubleMap<u16,u16, Vec<(u16, u16)> >>::iter_prefix( netuid ) {
            for (uid_j, weight_ij) in weights_i.iter() { 
                weights [ uid_i as usize ] [ *uid_j as usize ] = u16_proportion_to_fixed(  *weight_ij );
            }
        }
        weights
    }

    pub fn get_bonds_sparse( netuid:u16 ) -> Vec<Vec<(u16, I32F32)>> { 
        let n: usize = Self::get_subnetwork_n( netuid ) as usize; 
        let mut bonds: Vec<Vec<(u16, I32F32)>> = vec![ vec![]; n ]; 
        for ( uid_i, bonds_i ) in < Bonds<T> as IterableStorageDoubleMap<u16, u16, Vec<(u16, u16)> >>::iter_prefix( netuid ) {
            for (uid_j, bonds_ij) in bonds_i.iter() { 
                bonds [ uid_i as usize ].push( ( *uid_j, u16_proportion_to_fixed( *bonds_ij ) ));
            }
        }
        bonds
    } 

    pub fn get_bonds( netuid:u16 ) -> Vec<Vec<I32F32>> { 
        let n: usize = Self::get_subnetwork_n( netuid ) as usize; 
        let mut bonds: Vec<Vec<I32F32>> = vec![ vec![ I32F32::from_num(0.0); n ]; n ]; 
        for ( uid_i, bonds_i ) in < Bonds<T> as IterableStorageDoubleMap<u16, u16, Vec<(u16, u16)> >>::iter_prefix( netuid ) {
            for (uid_j, bonds_ij) in bonds_i.iter() { 
                bonds [ uid_i as usize ] [ *uid_j as usize ] = u16_proportion_to_fixed( *bonds_ij );
            }
        }
        bonds
    }

    pub fn blocks_until_next_epoch( netuid: u16, tempo: u16, block_number: u64 ) -> u64 { 
        if tempo == 0 { return 10 } // Special case: tempo = 0, the network never runs.
        // tempo | netuid | # first epoch block
        //   1        0               0
        //   1        1               1
        //   2        0               1
        //   2        1               0
        //   100      0              99
        //   100      1              98
        return tempo as u64 - ( block_number + netuid as u64 + 1 ) % ( tempo as u64 + 1 )
    }

 


    // Adjusts the network of every active network. Reseting state parameters.
    //
    pub fn adjust_registration_terms_for_networks( ) {
        
        // --- 1. Iterate through each network.
        for ( netuid, _ )  in <NetworksAdded<T> as IterableStorageMap<u16, bool>>::iter(){

            let last_adjustment_block: u64 = Self::get_last_adjustment_block( netuid );
            let adjustment_interval: u16 = Self::get_adjustment_interval( netuid );
            let current_block: u64 = Self::get_current_block_as_u64( ); 
            log::debug!("netuid: {:?} last_adjustment_block: {:?} adjustment_interval: {:?} current_block: {:?}", 
                netuid,
                last_adjustment_block,
                adjustment_interval,
                current_block
            );

            // --- 3. Check if we are at the adjustment interval for this network.
            // If so, we need to adjust the registration based on target and actual registrations.
            if ( current_block - last_adjustment_block ) >= adjustment_interval as u64 {

                let registrations_this_interval: u16 = Self::get_registrations_this_interval( netuid );
                let target_registrations_this_interval: u16 = Self::get_target_registrations_per_interval( netuid );

                // --- 6. Drain all counters for this network for this interval.
                Self::set_last_adjustment_block( netuid, current_block );
                Self::set_registrations_this_interval( netuid, 0 );
            }

            // --- 7. Drain block registrations for each network. Needed for registration rate limits.
            Self::set_registrations_this_block( netuid, 0 );
        }
    }


}