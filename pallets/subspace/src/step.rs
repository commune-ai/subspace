use super::*;
use crate::math::*;
use frame_support::sp_std::vec;
use frame_support::inherent::Vec;
use substrate_fixed::types::{I32F32, I64F64, I96F32, I110F18};
use frame_support::storage::{IterableStorageMap, IterableStorageDoubleMap};

impl<T: Config> Pallet<T> { 

    pub fn block_step( ) {
        let block_number: u64 = Self::get_current_block_as_u64();
        log::debug!("block_step for block: {:?} ", block_number );
        for ( netuid, tempo )  in <Tempo<T> as IterableStorageMap<u16, u16>>::iter() {

            RegistrationsThisBlock::<T>::mutate(netuid,  |val| *val = 0 );
            let new_queued_emission : u64 = Self::calculate_network_emission( netuid );
            PendingEmission::<T>::mutate( netuid, | queued | *queued += new_queued_emission );
            log::debug!("netuid_i: {:?} queued_emission: +{:?} ", netuid, new_queued_emission );  
            if  (block_number + netuid as u64) % (tempo as u64) > 0 {
                continue;
            }
            let emission_to_drain:u64 = PendingEmission::<T>::get( netuid ).clone(); 
            Self::epoch( netuid, emission_to_drain );
            PendingEmission::<T>::insert( netuid, 0 );

        }
    }


    pub fn epoch( netuid: u16, token_emission: u64 ) {
        // Get subnetwork size.
        let n: u16 = Self::get_subnet_n( netuid );
        log::trace!( "n: {:?}", n );


        if n == 0 {
            return;
        }

        // Get current block.
        let current_block: u64 = Self::get_current_block_as_u64();
        log::trace!( "current_block: {:?}", current_block );


        // Block at registration vector (block when each module was most recently registered).
        let block_at_registration: Vec<u64> = Self::get_block_at_registration( netuid );
        log::trace!( "Block at registration: {:?}", &block_at_registration );

        // ===========
        // == Stake ==
        // ===========

        let mut keys: Vec<(u16, T::AccountId)> = Self::get_uid_key_tuples( netuid );
        log::trace!( "keys: {:?}", &keys );
        // Access network stake as normalized vector.
        let mut stake_64: Vec<I64F64> = vec![ I64F64::from_num(0.0); n as usize ];
        let mut total_stake : I64F64 = I64F64::from_num(Self::get_total_subnet_stake( netuid ).clone());
        if total_stake == I64F64::from_num(0.0) {
            total_stake = I64F64::from_num(1.0);
        }
        for (uid_i, key) in keys.iter() {

            stake_64[ *uid_i as usize ] = I64F64::from_num( Self::get_stake_for_key(netuid, key ).clone()) /  total_stake ;
        }

        let mut stake: Vec<I32F32> = stake_64.iter().map( |x| I32F32::from_num(x.clone()) ).collect();


        // range: I32F32(0, 1)
        log::trace!( "S: {:?}", &stake );

        // Normalize active stake.
        inplace_normalize( &mut stake );
        log::trace!( "S (mask+norm): {:?}", &stake );

        // =============
        // == Weights ==
        // =============

        // Access network weights row normalized.
        let mut weights: Vec<Vec<(u16, I32F32)>> = Self::get_weights_sparse( netuid );
        log::trace!( "W (permit): {:?}", &weights );

        log::trace!( "W (permit+diag): {:?}", &weights );

        // Normalize remaining weights.
        inplace_row_normalize_sparse( &mut weights );
        log::trace!( "W (mask+norm): {:?}", &weights );

        // =============================
        // ==  Incentive ==
        // =============================

        // Compute incentive: r_j = SUM(i) w_ij * s_i.
        let mut incentive: Vec<I32F32> = matmul_sparse( &weights, &stake, n );
        log::trace!( "Incentive: {:?}", &incentive );
        // If emission is zero, do an even split.
        if is_zero( &incentive ) { // no weights set
            for (uid_i, key) in keys.iter() {
                incentive[ *uid_i as usize ] = I32F32::from_num(1.0);
            }
        }
        inplace_normalize( &mut incentive );  // range: I32F32(0, 1)

    
        // Compute bonds delta column normalized.
        let mut bonds: Vec<Vec<(u16, I32F32)>> = row_hadamard_sparse( &weights, &stake ); // ΔB = W◦S (outdated W masked)
        log::trace!( "ΔB: {:?}", &bonds );

        // Normalize bonds delta.
        inplace_col_normalize_sparse( &mut bonds, n ); // sum_i b_ij = 1
        log::trace!( "ΔB (norm): {:?}", &bonds );
        
        // Compute dividends: d_i = SUM(j) b_ij * inc_j.
        // range: I32F32(0, 1)
        let mut dividends: Vec<I32F32> = matmul_transpose_sparse( &bonds, &incentive ).clone();
        // If emission is zero, do an even split.
        if is_zero( &dividends ) { // no weights set
            for (uid_i, key) in keys.iter() {
                dividends[ *uid_i as usize ] = I32F32::from_num(1.0);
            }
        }
    
        inplace_normalize( &mut dividends );
        log::trace!( "D: {:?}", &dividends );

        let cloned_incentive: Vec<u16> = incentive.iter().map(|xi| fixed_proportion_to_u16(*xi)).collect::<Vec<u16>>();
        Incentive::<T>::insert( netuid, cloned_incentive );
        let cloned_dividends: Vec<u16> = dividends.iter().map(|xi| fixed_proportion_to_u16(*xi)).collect::<Vec<u16>>();
        Dividends::<T>::insert( netuid, cloned_dividends );

        // =================================
        // == Emission==
        // =================================

        let incentive_emission: Vec<I64F64> = incentive.clone().iter().map( |x| I64F64::from_num(x.clone()) * I64F64::from_num(token_emission/2) ).collect();
        let dividends_emission: Vec<I64F64> = dividends.clone().iter().map( |x| I64F64::from_num(x.clone()) * I64F64::from_num(token_emission/2) ).collect();

        let incentive_emission: Vec<u64> = incentive_emission.iter().map( |e: &I64F64| e.to_num::<u64>() ).collect();
        let dividends_emission: Vec<u64> = dividends_emission.iter().map( |e: &I64F64| e.to_num::<u64>() ).collect();


        // Emission tuples ( keys, u64 emission)
        for ( uid_i, key ) in keys.iter() {
            Self::increase_stake_on_account(netuid, key, incentive_emission[ *uid_i as usize ] );
        }

        // Dividends tuples ( keys, u64 dividends)
        for ( uid_i, key ) in keys.iter() {
            if dividends_emission[ *uid_i as usize ] > 0 {
                // get the ownership emission for this key
                let ownership_emission_for_key: Vec<(T::AccountId, u64)>  = Self::get_ownership_ratios_emission( netuid, key, dividends_emission[ *uid_i as usize ] );
                
                // add the ownership
                for (owner_key, amount) in ownership_emission_for_key.iter() {                 
                    Self::add_stake_to_module( netuid, owner_key, key, *amount );
                }
            }
        }

        let emission: Vec<u64> = incentive_emission.iter().zip( dividends_emission.iter() ).map( |(inc, div)| inc + div ).collect();
        Emission::<T>::insert( netuid, emission.clone() );

    }


    pub fn get_block_at_registration( netuid:u16 ) -> Vec<u64> { 
        let n: usize = Self::get_subnet_n( netuid ) as usize;
        let mut block_at_registration: Vec<u64> = vec![ 0; n ];
        for module_uid in 0..n {
            if Keys::<T>::contains_key( netuid, module_uid as u16 ){
                block_at_registration[ module_uid ] = Self::get_module_block_at_registration( netuid, module_uid as u16 );
            }
        }
        block_at_registration
    }

    pub fn get_weights_sparse( netuid:u16 ) -> Vec<Vec<(u16, I32F32)>> { 
        let n: usize = Self::get_subnet_n( netuid ) as usize; 
        let mut weights: Vec<Vec<(u16, I32F32)>> = vec![ vec![]; n ]; 
        for ( uid_i, weights_i ) in < Weights<T> as IterableStorageDoubleMap<u16 ,u16, Vec<(u16, u16)> >>::iter_prefix( netuid ) {
            for (uid_j, weight_ij) in weights_i.iter() { 
                weights [ uid_i as usize ].push( ( *uid_j, u16_proportion_to_fixed( *weight_ij ) ));
            }
        }
        // Remove self-weight by masking diagonal.
        weights = mask_diag_sparse( &weights );
        weights
    } 


    pub fn blocks_until_next_epoch( netuid: u16, tempo: u16, block_number: u64 ) -> u64 { 
        if tempo == 0 { return 10 } // Special case: epoch = 0, the network never runs.
        return tempo as u64 - ( block_number + netuid as u64 + 1 ) % ( tempo as u64 + 1 )
    }


    pub fn get_ownership_ratios(netuid:u16, module_key: &T::AccountId ) -> Vec<(T::AccountId, I64F64)> { 

        let stake_from_vector: Vec<(T::AccountId, u64)> = Self::get_stake_from_vector(netuid, module_key);
        let uid = Self::get_uid_for_key(netuid, module_key);
        let mut total_stake_from: I64F64 = I64F64::from_num(0);

        let mut ownership_vector: Vec<(T::AccountId, I64F64)> = Vec::new();

        for (k, v) in stake_from_vector.clone().into_iter() {
            let ownership = I64F64::from_num(v) ;
            ownership_vector.push( (k.clone(), ownership) );
            total_stake_from += ownership;
        }
        if total_stake_from == I64F64::from_num(0) {
            ownership_vector = Vec::new();

        } else {
            ownership_vector = ownership_vector.into_iter().map( |(k, v)| (k, v / total_stake_from) ).collect();

        }


        return ownership_vector;
    }


    pub fn get_ownership_ratios_emission(netuid:u16, module_key: &T::AccountId, emission:u64 ) -> Vec<(T::AccountId, u64)> { 
            
        let ownership_vector: Vec<(T::AccountId, I64F64)> = Self::get_ownership_ratios(netuid, module_key );
        let mut emission_vector: Vec<(T::AccountId, u64)> = Vec::new();

        for (k, v) in ownership_vector {
            let emission_for_delegate = (v * I64F64::from_num(emission)).floor().to_num::<u64>();
            emission_vector.push( (k, emission_for_delegate) );
        }

        return emission_vector;
    }



}