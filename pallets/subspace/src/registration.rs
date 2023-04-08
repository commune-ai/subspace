use super::*;
use frame_support::{ pallet_prelude::DispatchResult};
use sp_std::convert::TryInto;
use sp_core::{H256, U256};
use crate::system::ensure_root;
use sp_io::hashing::sha2_256;
use sp_io::hashing::keccak_256;
use frame_system::{ensure_signed};
use sp_std::vec::Vec;
use substrate_fixed::types::I32F32;
use frame_support::sp_std::vec;

const LOG_TARGET: &'static str = "runtime::subspace::registration";

impl<T: Config> Pallet<T> {


    // ---- The implementation for the extrinsic do_registration.
    //
    // # Args:
    // 	* 'origin': (<T as frame_system::Config>RuntimeOrigin):
    // 		- The signature of the calling key.
    //
    // 	* 'netuid' (u16):
    // 		- The u16 network identifier.
    //

    // 	* 'nonce' ( u64 ):
    // 		- Positive integer nonce used in POW.

    // 	* 'key' ( T::AccountId ):
    // 		- Key to be registered to the network.
    //
    // # Event:
    // 	* NeuronRegistered;
    // 		- On successfully registereing a uid to a neuron slot on a subnetwork.
    //
    // # Raises:
    // 	* 'NetworkDoesNotExist':
    // 		- Attempting to registed to a non existent network.
    //
    // 	* 'TooManyRegistrationsThisBlock':
    // 		- This registration exceeds the total allowed on this network this block.
    //
    // 	* 'AlreadyRegistered':
    // 		- The key is already registered on this network.
    //

    pub fn do_registration( 
        origin: T::RuntimeOrigin,
        netuid: u16,
        ip: u128, 
        port: u16, 
        name: Vec<u8>,
        context: Vec<u8>,
    ) -> DispatchResult {

        // --- 1. Check that the caller has signed the transaction. 
        // TODO( const ): This not be the key signature or else an exterior actor can register the key and potentially control it?
        let key = ensure_signed( origin.clone() )?;        
        log::info!("do_registration( key:{:?} netuid:{:?} )", key, netuid );


        // --- 2. Ensure the passed network is valid.
        ensure!( Self::if_subnet_exist( netuid ), Error::<T>::NetworkDoesNotExist ); 

        // --- 3. Ensure we are not exceeding the max allowed registrations per block.
        ensure!( Self::get_registrations_this_block( netuid ) < Self::get_max_registrations_per_block( netuid ), Error::<T>::TooManyRegistrationsThisBlock );

        // --- 4. Ensure that the key is not already registered.
        ensure!( !Uids::<T>::contains_key( netuid, &key ), Error::<T>::AlreadyRegistered );

        // --- 5. Ensure the passed block number is valid, not in the future or too old.
        // Work must have been done within 3 blocks (stops long range attacks).
        let current_block_number: u64 = Self::get_current_block_as_u64();
        // --- 10. If the network account does not exist we will create it here.
        Self::create_account_if_non_existent( &key);         


        // --- 12. Append neuron or prune it.
        let uid: u16;
        let current_subnetwork_n: u16 = Self::get_subnetwork_n( netuid );

        // Possibly there is no neuron slots at all.
        ensure!( Self::get_max_allowed_uids( netuid ) != 0, Error::<T>::NetworkDoesNotExist );
        
        if current_subnetwork_n < Self::get_max_allowed_uids( netuid ) {

            // --- 12.1.1 No replacement required, the uid appends the subnetwork.
            // We increment the subnetwork count here but not below.
            uid = current_subnetwork_n;

            // --- 12.1.2 Expand subnetwork with new account.
            Self::append_neuron( netuid, &key );
            log::info!("add new neuron account");
        } else {
            // --- 12.1.1 Replacement required.
            // We take the neuron with the lowest pruning score here.
            uid = Self::get_neuron_to_prune( netuid );

            // --- 12.1.1 Replace the neuron account with the new info.
            Self::replace_neuron( netuid, uid, &key );
            log::info!("prune neuron");
        }

        // --- 14. Record the registration and increment block and interval counters.
        RegistrationsThisInterval::<T>::mutate( netuid, |val| *val += 1 );
        RegistrationsThisBlock::<T>::mutate( netuid, |val| *val += 1 );
    
        // --- 15. Deposit successful event.
        log::info!("NeuronRegistered( netuid:{:?} uid:{:?} key:{:?}  ) ", netuid, uid, key );
        Self::deposit_event( Event::NeuronRegistered( netuid, uid, key.clone() ) );


        // --- 2. Ensure the key is registered somewhere.
        ensure!( Self::is_key_registered_on_any_network( &key ), Error::<T>::NotRegistered );  

        // --- 4. Get the previous neuron information.
        let mut prev_neuron = Self::get_neuron_info( netuid, &key );  
        let current_block: u64 = Self::get_current_block_as_u64(); 
        ensure!( Self::neuron_passes_rate_limit( netuid, &prev_neuron, current_block ), Error::<T>::ServingRateLimitExceeded );  
      

        if prev_neuron.name.len() > 0 {
            let old_name = prev_neuron.name.clone();
            NeuronNamespace::<T>::remove( netuid, old_name.clone() );
        } 
        ensure!(!Self::name_exists(netuid, name.clone()) , Error::<T>::NeuronNameAlreadyExists); 
        NeuronNamespace::<T>::insert( netuid, name.clone(), key.clone() );

        ensure!( Self::is_valid_ip_address(ip), Error::<T>::InvalidIpType );
        prev_neuron.name = name.clone();
        prev_neuron.ip = ip;
        prev_neuron.port = port;
        prev_neuron.context = context.clone();
        prev_neuron.block = current_block;

        Neurons::<T>::insert( netuid, key.clone(), prev_neuron.clone() );

        // --- 7. We deposit neuron served event.
        log::info!("NeuronServed( key:{:?} ) ", key.clone() );
        Self::deposit_event(Event::NeuronServed( netuid, key.clone() ));



        // --- 16. Ok and done.
        Ok(())
    }


    pub fn do_transfer_registration(  origin: T::RuntimeOrigin, netuid: u16, uid: u16, new_key: T::AccountId ) -> DispatchResult {
        // --- 1. Check that the caller has signed the transaction. 
        // TODO( const ): This not be the key signature or else an exterior actor can register the key and potentially control it?
        let key = ensure_signed( origin.clone() )?;        
        log::info!("do_transfer_registration( key:{:?} netuid:{:?} uid:{:?} new_key:{:?} )", key, netuid, uid, new_key );

        // --- 2. Ensure the passed network is valid.
        ensure!( Self::if_subnet_exist( netuid ), Error::<T>::NetworkDoesNotExist ); 

        // --- 3. Ensure the key is already registered.
        ensure!( Uids::<T>::contains_key( netuid, &key ), Error::<T>::NotRegistered );

        // --- 5. Ensure the passed block number is valid, not in the future or too old.
        // Work must have been done within 3 blocks (stops long range attacks).
        let current_block_number: u64 = Self::get_current_block_as_u64();
        // --- 10. If the network account does not exist we will create it here.
        Self::replace_neuron( netuid, netuid, &key );

        Ok(())
    }



    pub fn vec_to_hash( vec_hash: Vec<u8> ) -> H256 {
        let de_ref_hash = &vec_hash; // b: &Vec<u8>
        let de_de_ref_hash: &[u8] = &de_ref_hash; // c: &[u8]
        let real_hash: H256 = H256::from_slice( de_de_ref_hash );
        return real_hash
    }

    // Determine which peer to prune from the network by finding the element with the lowest pruning score out of
    // immunity period. If all neurons are in immunity period, return node with lowest prunning score.
    // This function will always return an element to prune.
    pub fn get_neuron_to_prune(netuid: u16) -> u16 {
        let mut min_score : u16 = u16::MAX;
        let mut min_score_in_immunity_period = u16::MAX;
        let mut uid_with_min_score = 0;
        let mut uid_with_min_score_in_immunity_period: u16 =  0;
        if Self::get_subnetwork_n( netuid ) == 0 { return 0 } // If there are no neurons in this network.
        for neuron_uid_i in 0..Self::get_subnetwork_n( netuid ) {
            let pruning_score:u16 = Self::get_pruning_score_for_uid( netuid, neuron_uid_i );
            let block_at_registration: u64 = Self::get_neuron_block_at_registration( netuid, neuron_uid_i );
            let current_block :u64 = Self::get_current_block_as_u64();
            let immunity_period: u64 = Self::get_immunity_period(netuid) as u64;
            if min_score == pruning_score {
                if current_block - block_at_registration <  immunity_period { //neuron is in immunity period
                    if min_score_in_immunity_period > pruning_score {
                        min_score_in_immunity_period = pruning_score; 
                        uid_with_min_score_in_immunity_period = neuron_uid_i;
                    }
                }
                else {
                    min_score = pruning_score; 
                    uid_with_min_score = neuron_uid_i;
                }
            }
            // Find min pruning score.
            else if min_score > pruning_score { 
                if current_block - block_at_registration <  immunity_period { //neuron is in immunity period
                    if min_score_in_immunity_period > pruning_score {
                         min_score_in_immunity_period = pruning_score; 
                        uid_with_min_score_in_immunity_period = neuron_uid_i;
                    }
                }
                else {
                    min_score = pruning_score; 
                    uid_with_min_score = neuron_uid_i;
                }
            }
        }
        if min_score == u16::MAX { //all neuorns are in immunity period
            Self::set_pruning_score_for_uid( netuid, uid_with_min_score_in_immunity_period, u16::MAX );
            return uid_with_min_score_in_immunity_period;
        }
        else {
            // We replace the pruning score here with u16 max to ensure that all peers always have a 
            // pruning score. In the event that every peer has been pruned this function will prune
            // the last element in the network continually.
            Self::set_pruning_score_for_uid( netuid, uid_with_min_score, u16::MAX );
            return uid_with_min_score;
        }
    } 


    pub fn get_block_hash_from_u64 ( block_number: u64 ) -> H256 {
        let block_number: T::BlockNumber = TryInto::<T::BlockNumber>::try_into( block_number ).ok().expect("convert u64 to block number.");
        let block_hash_at_number: <T as frame_system::Config>::Hash = system::Pallet::<T>::block_hash( block_number );
        let vec_hash: Vec<u8> = block_hash_at_number.as_ref().into_iter().cloned().collect();
        let deref_vec_hash: &[u8] = &vec_hash; // c: &[u8]
        let real_hash: H256 = H256::from_slice( deref_vec_hash );

        log::trace!(
			target: LOG_TARGET,
			"block_number: {:?}, vec_hash: {:?}, real_hash: {:?}",
			block_number,
			vec_hash,
			real_hash
		);

        return real_hash;
    }

    pub fn hash_to_vec( hash: H256 ) -> Vec<u8> {
        let hash_as_bytes: &[u8] = hash.as_bytes();
        let hash_as_vec: Vec<u8> = hash_as_bytes.iter().cloned().collect();
        return hash_as_vec
    }


    // ---- The implementation for the extrinsic serve_neuron which sets the ip endpoint information for a uid on a network.
    //
    // # Args:
    // 	* 'origin': (<T as frame_system::Config>RuntimeOrigin):
    // 		- The signature of the caller.
    //
    // 	* 'netuid' (u16):
    // 		- The u16 network identifier.
    //
    // 	* 'version' (u64):
    // 		- The commune version identifier.
    //
    // 	* 'ip' (u64):
    // 		- The endpoint ip information as a u128 encoded integer.
    //
    // 	* 'port' (u16):
    // 		- The endpoint port information as a u16 encoded integer.
    // 
    // 	* 'ip_type' (u8):
    // 		- The endpoint ip version as a u8, 4 or 6.
    //
    // 	* 'protocol' (u8):
    // 		- UDP:1 or TCP:0 
    //
    // 	* 'placeholder1' (u8):
    // 		- Placeholder for further extra params.
    //
    // 	* 'placeholder2' (u8):
    // 		- Placeholder for further extra params.
    //
    // # Event:
    // 	* NeuronServed;
    // 		- On successfully serving the neuron info.
    //
    // # Raises:
    // 	* 'NetworkDoesNotExist':
    // 		- Attempting to set weights on a non-existent network.
    //
    // 	* 'NotRegistered':
    // 		- Attempting to set weights from a non registered account.
    //
    // 	* 'InvalidIpType':
    // 		- The ip type is not 4 or 6.
    //
    // 	* 'InvalidIpAddress':
    // 		- The numerically encoded ip address does not resolve to a proper ip.
    //
    // 	* 'ServingRateLimitExceeded':
    // 		- Attempting to set prometheus information withing the rate limit min.
    //
    pub fn do_serve_neuron( 
        origin: T::RuntimeOrigin, 
		netuid: u16,
        ip: u128, 
        port: u16, 
        name: Vec<u8>,
        context: Vec<u8>,
    ) -> dispatch::DispatchResult {
        // --- 1. We check the callers (key) signature.
        let key = ensure_signed(origin)?;

        // --- 2. Ensure the key is registered somewhere.
        ensure!( Self::is_key_registered_on_any_network( &key), Error::<T>::NotRegistered );  

        // --- 4. Get the previous neuron information.
        let mut prev_neuron = Self::get_neuron_info( netuid, &key );  
        let current_block: u64 = Self::get_current_block_as_u64(); 
        ensure!( Self::neuron_passes_rate_limit( netuid, &prev_neuron, current_block ), Error::<T>::ServingRateLimitExceeded );  
      



        // --- 6. We insert the neuron meta.
        // remove the old neuron name from the namespace.
        let mut did_neuron_change : bool = false;


        if prev_neuron.name.len() > 0 {
            let old_name = prev_neuron.name.clone();
            NeuronNamespace::<T>::remove( netuid, old_name.clone() );
        } 
        ensure!(Self::name_exists(netuid, name.clone()) , Error::<T>::NeuronNameAlreadyExists); 
        NeuronNamespace::<T>::insert( netuid, name.clone(), key.clone() );

        ensure!( Self::is_valid_ip_address(ip), Error::<T>::InvalidIpType );
        prev_neuron.name = name.clone();
        prev_neuron.ip = ip;
        prev_neuron.port = port;
        prev_neuron.context = context.clone();
        prev_neuron.block = current_block;
        Neurons::<T>::insert( netuid, key.clone(), prev_neuron.clone() );

        // --- 7. We deposit neuron served event.
        log::info!("NeuronServed( key:{:?} ) ", key.clone() );
        Self::deposit_event(Event::NeuronServed( netuid, key ));

        // --- 8. Return is successful dispatch. 
        Ok(())
    }

    pub fn name_exists( netuid: u16, name: Vec<u8> ) -> bool {
        return NeuronNamespace::<T>::contains_key( netuid, name.clone());
        
    }

    // ---- The implementation for the extrinsic serve_prometheus.
    //
    // # Args:
    // 	* 'origin': (<T as frame_system::Config>RuntimeOrigin):
    // 		- The signature of the caller.
    //
    // 	* 'netuid' (u16):
    // 		- The u16 network identifier.
    //
    // 	* 'version' (u64):
    // 		- The commune version identifier.
    //
    // 	* 'ip' (u64):
    // 		- The prometheus ip information as a u128 encoded integer.
    //
    // 	* 'port' (u16):
    // 		- The prometheus port information as a u16 encoded integer.
    // 
    // 	* 'ip_type' (u8):
    // 		- The prometheus ip version as a u8, 4 or 6.
    //

    // # Raises:
    // 	* 'NetworkDoesNotExist':
    // 		- Attempting to set weights on a non-existent network.
    //
    // 	* 'NotRegistered':
    // 		- Attempting to set weights from a non registered account.
    //
    // 	* 'InvalidIpType':
    // 		- The ip type is not 4 or 6.
    //
    // 	* 'InvalidIpAddress':
    // 		- The numerically encoded ip address does not resolve to a proper ip.
    //
    // 	* 'ServingRateLimitExceeded':
    // 		- Attempting to set prometheus information withing the rate limit min.
    //

    /********************************
     --==[[  Helper functions   ]]==--
    *********************************/

    pub fn neuron_passes_rate_limit( netuid: u16, prev_neuron_info: &NeuronInfo, current_block: u64 ) -> bool {
        let rate_limit: u64 = Self::get_serving_rate_limit(netuid);
        let last_serve = prev_neuron_info.block;
        return rate_limit == 0 || last_serve == 0 || current_block - last_serve >= rate_limit;
    }



    pub fn has_neuron_info( netuid: u16, key: &T::AccountId ) -> bool {
        return Neurons::<T>::contains_key( netuid, key );
    }


    pub fn get_neuron_info( netuid: u16, key: &T::AccountId ) -> NeuronInfo {
        if Self::has_neuron_info( netuid, key ) {
            return Neurons::<T>::get( netuid, key ).unwrap();
        } else{
            return NeuronInfo { 
                block: 0,
                ip: 0,
                port: 0,
                name: vec![],
                context: vec![],
            }

        }
    }


    pub fn is_valid_ip_type(ip_type: u8) -> bool {
        let allowed_values: Vec<u8> = vec![4, 6];
        return allowed_values.contains(&ip_type);
    }


    // @todo (Parallax 2-1-2021) : Implement exclusion of private IP ranges
    pub fn is_valid_ip_address(ip: u128) -> bool {
        let ip_type = Self::get_ip_type(ip);
        if ip == 0 {
            return false;
        }
        if ip_type == 4 {
            if ip == 0 { return false; }
            if ip >= u32::MAX as u128 { return false; }
            if ip == 0x7f000001 { return false; } // Localhost
        }
        if ip_type == 6 {
            if ip == 0x0 { return false; }
            if ip == u128::MAX { return false; }
            if ip == 1 { return false; } // IPv6 localhost
        }
        return true;
    }

    fn get_ip_type(ip: u128) -> u8 {
        // Return the IP type (4 or 6) based on the IP address
        if ip <= u32::MAX as u128 {
            return 4;
        } else if ip <= u128::MAX {
            return 6;
        } 

        // If the IP address is not IPv4 or IPv6 and not private, raise an error
        return 0;
    } 

}