use super::*;
use frame_support::storage::IterableStorageMap;
use frame_support::pallet_prelude::{Decode, Encode};
extern crate alloc;
use alloc::vec::Vec;
use codec::Compact;

#[derive(Decode, Encode, PartialEq, Eq, Clone, Debug)]
pub struct ModuleSubnetInfo<T: Config> {
    key: T::AccountId,
    uid: Compact<u16>,
    netuid: Compact<u16>,
    name: Vec<u8>,
    last_update: Compact<u64>,
    
    // Subnet Info
    stake: Vec<(T::AccountId, Compact<u64>)>, // map of key to stake on this module/key (includes delegations)
    emission: Compact<u64>,
    incentive: Compact<u16>,
    dividends: Compact<u16>,
    weights: Vec<(Compact<u16>, Compact<u16>)>, // Vec of (uid, weight)
}


impl<T: Config> Pallet<T> {


        // Replace the module under this uid.
        pub fn replace_module( netuid: u16, uid_to_replace: u16, new_key: &T::AccountId, name: Vec<u8>, address: Vec<u8>, stake: u64 ) {

            log::debug!("replace_module( netuid: {:?} | uid_to_replace: {:?} | new_key: {:?} ) ", netuid, uid_to_replace, new_key );
    
            // 1. Get the old key under this position.
            let old_key: T::AccountId = Keys::<T>::get( netuid, uid_to_replace );
            let uid = Uids::<T>::get( netuid, old_key.clone()).unwrap();
            // 2. Remove previous set memberships.
            Uids::<T>::remove( netuid, old_key.clone() ); 
            Keys::<T>::remove( netuid, uid_to_replace ); 
            let block_number:u64 = Self::get_current_block_as_u64();
            // 3. Create new set memberships.
            Keys::<T>::insert( netuid, uid_to_replace, new_key.clone() ); // Make key - uid association.
            Uids::<T>::insert( netuid, new_key.clone(), uid_to_replace ); // Make uid - key association.
            BlockAtRegistration::<T>::insert( netuid, uid_to_replace, block_number ); // Fill block at registration.
            Addresses::<T>::insert( netuid, uid, address ); // Fill module info.
            Self::decrease_all_stake_on_account( netuid, &old_key.clone() );
            Stake::<T>::remove( netuid, &old_key.clone() ); // Make uid - key association.
            Self::increase_stake_on_account( netuid, &new_key.clone(), stake );
            // 4. Emit the event.
            
        }
    

        // Replace the module under this uid.
        pub fn remove_module( netuid: u16, uid: u16 ) {
            // 1. Get the old key under this position.
            let key: T::AccountId = Keys::<T>::get( netuid, uid );
            // 2. Remove previous set memberships.
            Uids::<T>::remove( netuid, key.clone() ); 
            Keys::<T>::remove( netuid, uid ); 
            Addresses::<T>::remove(netuid, uid );
            BlockAtRegistration::<T>::remove( netuid, uid );
            Keys::<T>::remove( netuid, uid); // Make key - uid association.
            Uids::<T>::remove( netuid, key.clone() ); // Make uid - key association.
            Weights::<T>::remove( netuid, uid ); // Make uid - key association.
            Self::decrease_all_stake_on_account( netuid, &key.clone() );
            Stake::<T>::remove( netuid, &key.clone() ); // Make uid - key association.
            SubnetN::<T>::insert( netuid, uid - 1 );
            if SubnetN::<T>::get( netuid ) == 0 {
                Self::remove_network_for_netuid( netuid );
            }

    
            
            // 4. Emit the event.
            
        }
    

        // Appends the uid to the network.
        pub fn append_module( netuid: u16, key: &T::AccountId , name: Vec<u8>, address: Vec<u8>, stake: u64) -> u16{
    
            // 1. Get the next uid. This is always equal to subnetwork_n.
            let uid: u16 = Self::get_subnetwork_n( netuid );
            let block_number = Self::get_current_block_as_u64();
            log::debug!("append_module( netuid: {:?} | uid: {:?} | new_key: {:?} ) ", netuid, key, uid );
    
            // 2. Get and increase the uid count.
            SubnetN::<T>::insert( netuid, uid + 1 );
    
            // 3. Expand Yuma with new position.
            Emission::<T>::mutate(netuid, |v| v.push(0) );
            Incentive::<T>::mutate(netuid, |v| v.push(0) );
            Dividends::<T>::mutate(netuid, |v| v.push(0) );
            LastUpdate::<T>::mutate(netuid, |v| v.push( block_number ) );
        
            // 4. Insert new account information.
            Keys::<T>::insert( netuid, uid, key.clone() ); // Make key - uid association.
            Uids::<T>::insert( netuid, key.clone(), uid ); // Make uid - key association.
            BlockAtRegistration::<T>::insert( netuid, uid, block_number ); // Fill block at registration.
            ModuleNamespace::<T>::insert( netuid, name.clone(), uid ); // Fill module namespace.
            Addresses::<T>::insert( netuid, uid, address ); // Fill module info.

            Self::increase_stake_on_account( netuid, &key, stake );

            return uid;
    
        }   
    
	pub fn get_modules(netuid: u16) -> Vec<ModuleSubnetInfo<T>> {
        if !Self::if_subnet_exist(netuid) {
            return Vec::new();
        }

        let mut modules = Vec::new();
        let n = Self::get_subnetwork_n(netuid);
        for uid in 0..n {
            let uid = uid;
            let netuid = netuid;

            let _module = Self::get_module_subnet_info(netuid, uid);
            let module;
            if _module.is_none() {
                break; // No more modules
            } else {
                // No error, key was registered
                module = _module.expect("Module should exist");
            }

            modules.push( module );
        }
        return modules;
	}

    fn get_module_subnet_info(netuid: u16, uid: u16) -> Option<ModuleSubnetInfo<T>> {
        let key = Self::get_key_for_uid(netuid, uid);


        let emission = Self::get_emission_for_uid( netuid, uid as u16 );
        let incentive = Self::get_incentive_for_uid( netuid, uid as u16 );
        let dividends = Self::get_dividends_for_uid( netuid, uid as u16 );
        let last_update = Self::get_last_update_for_uid( netuid, uid as u16 );
        let name = Self::get_name_for_uid( netuid, uid as u16 );

        let weights = <Weights<T>>::get(netuid, uid).iter()
            .filter_map(|(i, w)| if *w > 0 { Some((i.into(), w.into())) } else { None })
            .collect::<Vec<(Compact<u16>, Compact<u16>)>>();
        
        let stake: Vec<(T::AccountId, Compact<u64>)> = Stake::<T>::iter_prefix(netuid)
            .map(|(key, stake)| (key, stake.into()))
            .collect();

        

        let module = ModuleSubnetInfo {
            key: key.clone(),
            uid: uid.into(),
            netuid: netuid.into(),
            stake: stake,
            emission: emission.into(),
            incentive: incentive.into(),
            dividends: dividends.into(),
            last_update: last_update.into(),
            weights: weights,
            name: name.clone()
        };
        
        return Some(module);
    }



    pub fn get_module(netuid: u16, uid: u16) -> Option<ModuleSubnetInfo<T>> {
        if !Self::if_subnet_exist(netuid) {
            return None;
        }

        let module = Self::get_module_subnet_info(netuid, uid);
        return module;
	}




}

