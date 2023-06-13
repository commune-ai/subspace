
use super::*;
use frame_support::inherent::Vec;
use sp_core::U256;
use frame_support::pallet_prelude::DispatchResult;
use crate::system::ensure_root;

impl<T: Config> Pallet<T> {

    // ========================
	// ==== Global Setters ====
	// ========================
    pub fn set_tempo( netuid: u16, tempo: u16 ) { Tempo::<T>::insert( netuid, tempo ); }
    pub fn set_last_adjustment_block( netuid: u16, last_adjustment_block: u64 ) { LastAdjustmentBlock::<T>::insert( netuid, last_adjustment_block ); }
    pub fn set_blocks_since_last_step( netuid: u16, blocks_since_last_step: u64 ) { BlocksSinceLastStep::<T>::insert( netuid, blocks_since_last_step ); }
    pub fn set_registrations_this_block( netuid: u16, registrations_this_block: u16 ) { RegistrationsThisBlock::<T>::insert(netuid, registrations_this_block); }
    pub fn set_last_mechanism_step_block( netuid: u16, last_mechanism_step_block: u64 ) { LastMechansimStepBlock::<T>::insert(netuid, last_mechanism_step_block); }
    pub fn set_registrations_this_interval( netuid: u16, registrations_this_interval: u16 ) { RegistrationsThisInterval::<T>::insert(netuid, registrations_this_interval); }

    // ========================
	// ==== Global Getters ====
	// ========================
    pub fn get_block_emission() -> u64 { BlockEmission::<T>::get() }
    pub fn get_current_block_as_u64( ) -> u64 { TryInto::try_into( <frame_system::Pallet<T>>::block_number() ).ok().expect("blockchain will not exceed 2^64 blocks; QED.") }

    // ==============================
	// ==== Yuma params ====
	// ==============================
    pub fn get_emission( netuid:u16 ) -> Vec<u64> { Emission::<T>::get( netuid ) }
    pub fn get_incentive( netuid:u16 ) -> Vec<u16> { Incentive::<T>::get( netuid ) }
    pub fn get_dividends( netuid:u16 ) -> Vec<u16> { Dividends::<T>::get( netuid ) }
    pub fn get_last_update( netuid:u16 ) -> Vec<u64> { LastUpdate::<T>::get( netuid ) }
    
    // Emmision is the same as the Yuma params 

    
    pub fn set_last_update_for_uid( netuid:u16, uid: u16, last_update: u64 ) { 
        let mut updated_last_update_vec = Self::get_last_update( netuid ); 
        if (uid as usize) < updated_last_update_vec.len() { 
            updated_last_update_vec[uid as usize] = last_update;
            LastUpdate::<T>::insert( netuid, updated_last_update_vec );
        }  
    }

    pub fn get_emission_for_uid( netuid:u16, uid: u16) -> u64 {let vec =  Emission::<T>::get( netuid ); if (uid as usize) < vec.len() { return vec[uid as usize] } else{ return 0 } }
    pub fn get_incentive_for_uid( netuid:u16, uid: u16) -> u16 { let vec = Incentive::<T>::get( netuid ); if (uid as usize) < vec.len() { return vec[uid as usize] } else{ return 0 } }
    pub fn get_dividends_for_uid( netuid:u16, uid: u16) -> u16 { let vec = Dividends::<T>::get( netuid ); if (uid as usize) < vec.len() { return vec[uid as usize] } else{ return 0 } }
    pub fn get_last_update_for_uid( netuid:u16, uid: u16) -> u64 { let vec = LastUpdate::<T>::get( netuid ); if (uid as usize) < vec.len() { return vec[uid as usize] } else{ return 0 } }
    pub fn get_pruning_score_for_uid( netuid:u16, uid: u16) -> u16 { let vec = Emission::<T>::get( netuid ); if (uid as usize) < vec.len() { return vec[uid as usize] as u16 } else{ return u16::MAX } }

    pub fn get_name_for_uid( netuid:u16, uid: u16) -> Vec<u8> { 
        let module= Modules::<T>::get( netuid, uid ).unwrap();
        return module.name.clone();
    
    }

    // ============================
	// ==== Subnetwork Getters ====
	// ============================
    pub fn get_tempo( netuid:u16 ) -> u16{ Tempo::<T>::get( netuid ) }
    pub fn get_emission_value( netuid: u16 ) -> u64 { EmissionValues::<T>::get( netuid ) }
    pub fn get_pending_emission( netuid:u16 ) -> u64{ PendingEmission::<T>::get( netuid ) }
    pub fn get_last_adjustment_block( netuid: u16) -> u64 { LastAdjustmentBlock::<T>::get( netuid ) }
    pub fn get_blocks_since_last_step(netuid:u16 ) -> u64 { BlocksSinceLastStep::<T>::get( netuid ) }
    pub fn get_registrations_this_block( netuid:u16 ) -> u16 { RegistrationsThisBlock::<T>::get( netuid ) }
    pub fn get_last_mechanism_step_block( netuid: u16 ) -> u64 { LastMechansimStepBlock::<T>::get( netuid ) }
    pub fn get_registrations_this_interval( netuid: u16 ) -> u16 { RegistrationsThisInterval::<T>::get( netuid ) } 
    pub fn get_module_block_at_registration( netuid: u16, module_uid: u16 ) -> u64 { BlockAtRegistration::<T>::get( netuid, module_uid )}

    // ========================
	// ==== Rate Limiting =====
	// ========================
	pub fn get_last_tx_block( key: &T::AccountId ) -> u64 { LastTxBlock::<T>::get( key ) }
	pub fn exceeds_tx_rate_limit( prev_tx_block: u64, current_block: u64 ) -> bool {
        let rate_limit: u64 = Self::get_tx_rate_limit();
		if rate_limit == 0 || prev_tx_block == 0 {
			return false;
		}
        return current_block - prev_tx_block <= rate_limit;
    }



	// Configure tx rate limiting
	pub fn get_tx_rate_limit() -> u64 { TxRateLimit::<T>::get() }
    pub fn set_tx_rate_limit( tx_rate_limit: u64 ) { TxRateLimit::<T>::put( tx_rate_limit ) }
    pub fn do_sudo_set_tx_rate_limit( origin: T::RuntimeOrigin, tx_rate_limit: u64 ) -> DispatchResult { 
        ensure_root( origin )?;
        Self::set_tx_rate_limit( tx_rate_limit );
        log::info!("TxRateLimitSet( tx_rate_limit: {:?} ) ", tx_rate_limit );
        Self::deposit_event( Event::TxRateLimitSet( tx_rate_limit ) );
        Ok(()) 
    }

    pub fn get_serving_rate_limit( netuid: u16 ) -> u64 { ServingRateLimit::<T>::get(netuid) }
    pub fn set_serving_rate_limit( netuid: u16, serving_rate_limit: u64 ) { ServingRateLimit::<T>::insert( netuid, serving_rate_limit ) }
    pub fn do_sudo_set_serving_rate_limit( origin: T::RuntimeOrigin, netuid: u16, serving_rate_limit: u64 ) -> DispatchResult { 
        let key = ensure_signed( origin )?;
        Self::set_serving_rate_limit( netuid, serving_rate_limit );
        log::info!("ServingRateLimitSet( serving_rate_limit: {:?} ) ", serving_rate_limit );
        Self::deposit_event( Event::ServingRateLimitSet( netuid, serving_rate_limit ) );
        Ok(()) 
    }


    pub fn get_weights_set_rate_limit( netuid: u16) -> u64 { WeightsSetRateLimit::<T>::get( netuid ) }
    pub fn set_weights_set_rate_limit( netuid: u16, weights_set_rate_limit: u64 ) { WeightsSetRateLimit::<T>::insert( netuid, weights_set_rate_limit ); }
    pub fn do_sudo_set_weights_set_rate_limit( origin: T::RuntimeOrigin, netuid: u16, weights_set_rate_limit: u64 ) -> DispatchResult { 
        ensure_root( origin )?;
        ensure!(Self::if_subnet_exist(netuid), Error::<T>::NetworkDoesNotExist);
        Self::set_weights_set_rate_limit( netuid, weights_set_rate_limit );
        log::info!("WeightsSetRateLimitSet( netuid: {:?} weights_set_rate_limit: {:?} ) ", netuid, weights_set_rate_limit);
        Self::deposit_event( Event::WeightsSetRateLimitSet( netuid, weights_set_rate_limit) );
        Ok(()) 
    }

    pub fn get_adjustment_interval( netuid: u16) -> u16 { AdjustmentInterval::<T>::get( netuid ) }
    pub fn set_adjustment_interval( netuid: u16, adjustment_interval: u16 ) { AdjustmentInterval::<T>::insert( netuid, adjustment_interval ); }
    pub fn do_set_adjustment_interval( origin: T::RuntimeOrigin, netuid: u16, adjustment_interval: u16 ) -> DispatchResult { 
        ensure_root( origin )?;
        ensure!(Self::if_subnet_exist(netuid), Error::<T>::NetworkDoesNotExist);
        Self::set_adjustment_interval( netuid, adjustment_interval );
        log::info!("AdjustmentIntervalSet( netuid: {:?} adjustment_interval: {:?} ) ", netuid, adjustment_interval);
        Self::deposit_event( Event::AdjustmentIntervalSet( netuid, adjustment_interval) );
        Ok(()) 
    }

    pub fn get_max_weight_limit( netuid: u16) -> u16 { MaxWeightsLimit::<T>::get( netuid ) }    
    pub fn set_max_weight_limit( netuid: u16, max_weight_limit: u16 ) { MaxWeightsLimit::<T>::insert( netuid, max_weight_limit ); }
    pub fn do_sudo_set_max_weight_limit( origin:T::RuntimeOrigin, netuid: u16, max_weight_limit: u16 ) -> DispatchResult {
        ensure_root( origin )?;
        ensure!( Self::if_subnet_exist(netuid), Error::<T>::NetworkDoesNotExist );
        Self::set_max_weight_limit( netuid, max_weight_limit );
        log::info!("MaxWeightLimitSet( netuid: {:?} max_weight_limit: {:?} ) ", netuid, max_weight_limit);
        Self::deposit_event( Event::MaxWeightLimitSet( netuid, max_weight_limit ) );
        Ok(())
    }

    pub fn get_immunity_period(netuid: u16 ) -> u16 { ImmunityPeriod::<T>::get( netuid ) }
    pub fn set_immunity_period( netuid: u16, immunity_period: u16 ) { ImmunityPeriod::<T>::insert( netuid, immunity_period ); }
    pub fn do_sudo_set_immunity_period( origin:T::RuntimeOrigin, netuid: u16, immunity_period: u16 ) -> DispatchResult {
        ensure_root( origin )?;
        ensure!(Self::if_subnet_exist(netuid), Error::<T>::NetworkDoesNotExist);
        Self::set_immunity_period( netuid, immunity_period );
        log::info!("ImmunityPeriodSet( netuid: {:?} immunity_period: {:?} ) ", netuid, immunity_period);
        Self::deposit_event(Event::ImmunityPeriodSet(netuid, immunity_period));
        Ok(())
    }

    pub fn get_min_allowed_weights( netuid:u16 ) -> u16 { MinAllowedWeights::<T>::get( netuid ) }
    pub fn set_min_allowed_weights( netuid: u16, min_allowed_weights: u16 ) { MinAllowedWeights::<T>::insert( netuid, min_allowed_weights ); }
    pub fn do_sudo_set_min_allowed_weights( origin:T::RuntimeOrigin, netuid: u16, min_allowed_weights: u16 ) -> DispatchResult {
        ensure_root( origin )?;
        ensure!(Self::if_subnet_exist(netuid), Error::<T>::NetworkDoesNotExist);
        Self::set_min_allowed_weights( netuid, min_allowed_weights );
        log::info!("MinAllowedWeightSet( netuid: {:?} min_allowed_weights: {:?} ) ", netuid, min_allowed_weights);
        Self::deposit_event( Event::MinAllowedWeightSet( netuid, min_allowed_weights) );
        Ok(())
    }

    pub fn get_max_allowed_uids( netuid: u16 ) -> u16  { MaxAllowedUids::<T>::get( netuid ) }
    pub fn set_max_allowed_uids(netuid: u16, max_allowed: u16) { MaxAllowedUids::<T>::insert( netuid, max_allowed ); }
    pub fn do_sudo_set_max_allowed_uids( origin:T::RuntimeOrigin, netuid: u16, max_allowed_uids: u16 ) -> DispatchResult {
        ensure_root( origin )?;
        ensure!( Self::if_subnet_exist(netuid), Error::<T>::NetworkDoesNotExist );
        ensure!(Self::get_max_allowed_uids(netuid)< max_allowed_uids, Error::<T>::MaxAllowedUIdsNotAllowed);
        Self::set_max_allowed_uids( netuid, max_allowed_uids );
        log::info!("MaxAllowedUidsSet( netuid: {:?} max_allowed_uids: {:?} ) ", netuid, max_allowed_uids);
        Self::deposit_event( Event::MaxAllowedUidsSet( netuid, max_allowed_uids) );
        Ok(())
    }

        
            
    pub fn get_activity_cutoff( netuid: u16 ) -> u16  { ActivityCutoff::<T>::get( netuid ) }
    pub fn do_sudo_set_activity_cutoff( origin:T::RuntimeOrigin, netuid: u16, activity_cutoff: u16 ) -> DispatchResult {
        let key = ensure_signed( origin )?;
        ensure!(Self::if_subnet_exist(netuid), Error::<T>::NetworkDoesNotExist);
        ActivityCutoff::<T>::insert( netuid, activity_cutoff ); 
        log::info!("ActivityCutoffSet( netuid: {:?} activity_cutoff: {:?} ) ", netuid, activity_cutoff);
        Self::deposit_event( Event::ActivityCutoffSet( netuid, activity_cutoff) );
        Ok(())
    }
            
    pub fn get_target_registrations_per_interval( netuid: u16 ) -> u16 { TargetRegistrationsPerInterval::<T>::get( netuid ) }
    pub fn set_target_registrations_per_interval( netuid: u16, target_registrations_per_interval: u16 ) { TargetRegistrationsPerInterval::<T>::insert( netuid, target_registrations_per_interval ); }
    pub fn do_sudo_set_target_registrations_per_interval( origin:T::RuntimeOrigin, netuid: u16, target_registrations_per_interval: u16 ) -> DispatchResult {
        ensure_root( origin )?;
        ensure!(Self::if_subnet_exist(netuid), Error::<T>::NetworkDoesNotExist);
        Self::set_target_registrations_per_interval( netuid, target_registrations_per_interval );
        log::info!("RegistrationPerIntervalSet( netuid: {:?} target_registrations_per_interval: {:?} ) ", netuid, target_registrations_per_interval );
        Self::deposit_event( Event::RegistrationPerIntervalSet( netuid, target_registrations_per_interval) );
        Ok(())
    }

    pub fn get_max_registrations_per_block( netuid: u16 ) -> u16 { MaxRegistrationsPerBlock::<T>::get( netuid ) }
    pub fn set_max_registrations_per_block( netuid: u16, max_registrations_per_block: u16 ) { MaxRegistrationsPerBlock::<T>::insert( netuid, max_registrations_per_block ); }
    pub fn do_sudo_set_max_registrations_per_block(
        origin: T::RuntimeOrigin, 
        netuid: u16, 
        max_registrations_per_block: u16
    ) -> DispatchResult {
        ensure_root( origin )?;
        ensure!(Self::if_subnet_exist(netuid), Error::<T>::NetworkDoesNotExist);
        Self::set_max_registrations_per_block( netuid, max_registrations_per_block );
        log::info!("MaxRegistrationsPerBlock( netuid: {:?} max_registrations_per_block: {:?} ) ", netuid, max_registrations_per_block );
        Self::deposit_event( Event::MaxRegistrationsPerBlockSet( netuid, max_registrations_per_block) );
        Ok(())
    }

}


