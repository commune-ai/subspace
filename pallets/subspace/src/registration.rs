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


    pub fn do_registration( 
        origin: T::RuntimeOrigin,
        network: Vec<u8>,
        name: Vec<u8>,
        address: Vec<u8>,
        stake_amount: u64,
    ) -> DispatchResult {

        // --- 1. Check that the caller has signed the transaction. 
        // TODO( const ): This not be the key signature or else an exterior actor can register the key and potentially control it?
        let key = ensure_signed( origin.clone() )?;  
        // --- 2. Ensure we are not exceeding the max allowed registrations per block.
        ensure!( Self::get_registrations_this_block() <= Self::get_max_registrations_per_block( ), Error::<T>::TooManyRegistrationsThisBlock );
        ensure!(Self::has_enough_balance( &key, stake_amount ) || stake_amount == 0, Error::<T>::NotEnoughBalanceToRegister);

        let mut netuid: u16 = 0;       
        let new_network : bool = !Self::if_subnet_name_exists( network.clone() );

        if new_network {
            // --- 2. Ensure that the network name is not already registered.
            ensure!( !Self::if_subnet_name_exists( network.clone() ), Error::<T>::NetworkAlreadyRegistered );
            ensure!( Self::enough_stake_to_start_network( stake_amount ), Error::<T>::NotEnoughStakeToStartNetwork );
            netuid = Self::add_network_from_registration(network.clone(), stake_amount, &key);
            
        }  else {
            netuid = Self::get_netuid_for_name( network.clone() );
            ensure!( !Self::is_key_registered(netuid, &key), Error::<T>::KeyAlreadyRegistered );
            ensure!( !Self::if_module_name_exists( netuid, name.clone() ), Error::<T>::NameAlreadyRegistered );
            
            RegistrationsThisBlock::<T>::mutate( |val| *val += 1 );
            netuid = Self::get_netuid_for_name( network.clone() );            
            
        }



        let mut uid: u16;

        let n: u16 = Self::get_subnet_n( netuid );

        if n < Self::get_max_allowed_uids( netuid ) {
            uid = Self::append_module( netuid, &key , name.clone(), address.clone());
        } else {
            let lowest_uid: u16 = Self::get_lowest_uid( netuid );
            Self::remove_module( netuid, lowest_uid );
            uid = Self::append_module( netuid, &key , name.clone(), address.clone());
            log::info!("prune module {:?} from network {:?} ", uid, netuid);
        }

        if stake_amount > 0 {
            Self::do_add_stake(origin.clone(),  netuid, key.clone(), stake_amount );
        }
        // ---Deposit successful event.
        log::info!("ModuleRegistered( netuid:{:?} uid:{:?} key:{:?}  ) ", netuid, uid, key );
        Self::deposit_event( Event::ModuleRegistered( netuid, uid, key.clone() ) );

        // --- 5. Ok and done.
        Ok(())
    }


    pub fn vec_to_hash( vec_hash: Vec<u8> ) -> H256 {
        let de_ref_hash = &vec_hash; // b: &Vec<u8>
        let de_de_ref_hash: &[u8] = &de_ref_hash; // c: &[u8]
        let real_hash: H256 = H256::from_slice( de_de_ref_hash );
        return real_hash
    }

    // Determine which peer to prune from the network by finding the element with the lowest pruning score out of
    // immunity period. If all modules are in immunity period, return node with lowest prunning score.
    // This function will always return an element to prune.

    pub fn get_lowest_uid(netuid: u16) -> u16 {
        let mut min_score : u16 = u16::MAX;
        let mut uid_with_min_score = 0;
        let n = Self::get_subnet_n( netuid );
        let current_block :u64 = Self::get_current_block_as_u64();
        let mut uids_in_immunity_period: Vec<u16> = Vec::new();
        let mut uid_found: bool= false;
        let n: u16 = Self::get_subnet_n( netuid );
        let mut  lowest_priority_uid : u16 = u16::MAX;
        for module_uid_i in 0..n {
            let block_at_registration: u64 = Self::get_module_registration_block( netuid, module_uid_i );
            let immunity_period: u64 = Self::get_immunity_period(netuid) as u64;
            let mut pruning_score = Self::get_pruning_score_for_uid( netuid,  module_uid_i);

            // Find min pruning score.
            
            if min_score >= pruning_score { 

                min_score = pruning_score;
                uid_with_min_score = module_uid_i;

                if immunity_period > current_block - block_at_registration  { 
                    uids_in_immunity_period.push(module_uid_i);
                } else {
                    lowest_priority_uid = module_uid_i;
                }
            }
        

        }

        let max_immunity_uids = Self::get_max_immunity_uids(netuid);

        if uids_in_immunity_period.len() as u16 >= max_immunity_uids {
            lowest_priority_uid = uid_with_min_score;
        }

        if lowest_priority_uid == u16::MAX {
            lowest_priority_uid = uid_with_min_score;
        }


        return lowest_priority_uid;
    } 

    // Returns a random index in range 0..n.
    pub fn random_idx( n: u16 ) -> u16 {
        let block_number: u64 = Self::get_current_block_as_u64();
        // take the modulos of the blocknumber
        let idx: u16 = ((block_number % u16::MAX as u64) % (n as u64)) as u16;
        return idx

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


    pub fn do_update_module( 
        origin: T::RuntimeOrigin, 
		netuid: u16,
        name: Vec<u8>,
        address: Vec<u8>, 
    ) -> dispatch::DispatchResult {
        // --- 1. We check the callers (key) signature.
        let key = ensure_signed(origin)?;
        ensure!(Self::if_subnet_netuid_exists(netuid), Error::<T>::NetworkDoesNotExist);

        // --- 2. Ensure the key is registered somewhere.
        ensure!( Self::is_registered( netuid, &key.clone() ), Error::<T>::NotRegistered );  
        let uid : u16 = Self::get_uid_for_key( netuid, &key );

        
        // --- 4. Get the previous module information.
        let current_block: u64 = Self::get_current_block_as_u64(); 
        
    
        // if len(name) > 0, then we update the name.
        if name.len() > 0 {
            ensure!( name.len() <= MaxNameLength::<T>::get() as usize, Error::<T>::ModuleNameTooLong );
            let old_name = Names::<T>::get( netuid, uid ); // Get the old name.
            ensure!(!Self::if_module_name_exists(netuid, name.clone()) , Error::<T>::ModuleNameAlreadyExists); 
            Names::<T>::insert( netuid, uid, name.clone() );
        }
        // if len(address) > 0, then we update the address.
        if address.len() > 0 {
            Address::<T>::insert( netuid, uid, address.clone() );
        }

        // --- 8. Return is successful dispatch. 
        Ok(())
    }







}