use super::*;
use frame_support::sp_std::vec;
use sp_std::vec::Vec;

impl<T: Config> Pallet<T> {


    pub fn do_set_weights( origin: T::RuntimeOrigin, netuid: u16, uids: Vec<u16>, values: Vec<u16> ) -> dispatch::DispatchResult{

        // --- 1. Check the caller's signature. This is the key of a registered account.
        let key = ensure_signed( origin )?;

        let stake: u64 = Self::get_stake_for_key( netuid, &key );

        ensure!( stake > 0, Error::<T>::NotEnoughStaketoSetWeights );

        // --- 2. Check to see if this is a valid network.
        ensure!( Self::if_subnet_exist( netuid ), Error::<T>::NetworkDoesNotExist );
        log::info!("do_set_weights( origin:{:?} netuid:{:?}, uids:{:?}, values:{:?})", key, netuid, uids, values );
        // --- 3. Check that the length of uid list and value list are equal for this network.
        ensure!( Self::uids_match_values( &uids, &values ), Error::<T>::WeightVecNotEqualSize );

        // --- 4. Check to see if the number of uids is within the max allowed uids for this network.
        ensure!( Self::check_len_uids_within_allowed( netuid, &uids ), Error::<T>::TooManyUids);

        // --- 5. Check to see if the key is registered to the passed network.
        ensure!( Self::is_key_registered_on_network( netuid, &key ), Error::<T>::NotRegistered );

        // --- 7. Get the module uid of associated key on network netuid.
        
        let module_uid : u16 =   Self::get_uid_for_key( netuid, &key );

        // --- 8. Ensure the uid is not setting weights faster than the weights_set_rate_limit.
        let current_block: u64 = Self::get_current_block_as_u64();

 
        // --- 10. Ensure the passed uids contain no duplicates.
        ensure!( !Self::has_duplicate_uids( &uids ), Error::<T>::DuplicateUids );

        // --- 11. Ensure that the passed uids are valid for the network.
        ensure!( !Self::contains_invalid_uids( netuid, &uids ), Error::<T>::InvalidUid );

        // --- 12. Ensure that the weights have the required length.
        ensure!( Self::check_length( netuid, module_uid, &uids, &values ), Error::<T>::NotSettingEnoughWeights );

        // --- 13. Normalize the weights.
        let normalized_values = Self::normalize_weights( values );

        // --- 15. Zip weights for sinking to storage map.
        let mut zipped_weights: Vec<( u16, u16 )> = vec![];
        for ( uid, val ) in uids.iter().zip(normalized_values.iter()) { zipped_weights.push((*uid, *val)) }

        // --- 16. Set weights under netuid, uid double map entry.
        Weights::<T>::insert( netuid, module_uid, zipped_weights );

        // --- 17. Set the activity for the weights on this network.
        Self::set_last_update_for_uid( netuid, module_uid, current_block );

        // --- 18. Emit the tracking event.
        log::info!("WeightsSet( netuid:{:?}, module_uid:{:?} )", netuid, module_uid );
        Self::deposit_event( Event::WeightsSet( netuid, module_uid ) );

        // --- 19. Return ok.
        Ok(())
    }



    // Checks for any invalid uids on this network.
    pub fn contains_invalid_uids( netuid: u16, uids: &Vec<u16> ) -> bool {
        for uid in uids {
            if !Self::is_uid_exist_on_network( netuid, *uid ) {
                return true;
            }
        }
        return false;
    }

    // Returns true if the passed uids have the same length of the passed values.
    fn uids_match_values(uids: &Vec<u16>, values: &Vec<u16>) -> bool {
        return uids.len() == values.len();
    }

    // Returns true if the items contain duplicates.
    fn has_duplicate_uids(items: &Vec<u16>) -> bool {
        let mut parsed: Vec<u16> = Vec::new();
        for item in items {
            if parsed.contains(&item) { return true; }
            parsed.push(item.clone());
        }
        return false;
    }

    // Returns True if the uids and weights are have a valid length for uid on network.
    pub fn check_length( netuid: u16, uid: u16, uids: &Vec<u16>, weights: &Vec<u16> ) -> bool {
        let min_allowed_length: usize = Self::get_min_allowed_weights(netuid) as usize;

        // Check self weight. Allowed to set single value for self weight.
        if Self::is_self_weight(uid, uids, weights) {
            return true;
        }
        // Check if number of weights exceeds min.
        if weights.len() >= min_allowed_length {
            return true;
        }
        // To few weights.
        return false;
    }

    // Implace normalizes the passed positive integer weights so that they sum to u16 max value.
    pub fn normalize_weights(mut weights: Vec<u16>) -> Vec<u16> {
        let sum: u64 = weights.iter().map(|x| *x as u64).sum();
        if sum == 0 { return weights; }
        weights.iter_mut().for_each(|x| { *x = (*x as u64 * u16::max_value() as u64 / sum) as u16; });
        return weights;
    }


    // Returns true if the uids and weights correspond to a self weight on the uid.
    pub fn is_self_weight( uid: u16, uids: &Vec<u16>, weights: &Vec<u16> ) -> bool {
        if weights.len() != 1 { return false; }
        if uid != uids[0] { return false; } 
        return true;
    }

    // Returns False is the number of uids exceeds the allowed number of uids for this network.
    pub fn check_len_uids_within_allowed( netuid: u16, uids: &Vec<u16> ) -> bool {
        let subnetwork_n: u16 = Self::get_subnetwork_n( netuid );
        // we should expect at most subnetwork_n uids.
        return uids.len() <= subnetwork_n as usize;
    }
    
}