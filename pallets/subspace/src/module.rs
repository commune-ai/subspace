use super::*;
use frame_support::storage::IterableStorageMap;
use frame_support::pallet_prelude::{Decode, Encode};
extern crate alloc;
use alloc::vec::Vec;
use frame_support::sp_std::vec;
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


        pub fn replace_module_with_uid( netuid: u16, uid: u16, replace_uid: u16 ) {
            Self::replace_module( netuid, uid, &Keys::<T>::get( netuid, replace_uid ), Names::<T>::get( netuid, replace_uid ), Address::<T>::get( netuid, replace_uid ), Self::get_stake( netuid, &Keys::<T>::get( netuid, replace_uid ) ) );
        }
        // Replace the module under this uid.
        pub fn replace_module( netuid: u16, uid: u16, new_key: &T::AccountId, name: Vec<u8>, address: Vec<u8>, stake: u64 ) {

            log::debug!("remove_network_for_netuid( netuid: {:?} | uid : {:?} | new_key: {:?} ) ", netuid, uid, new_key );
            
            let block_number:u64 = Self::get_current_block_as_u64();
            let old_key: T::AccountId = Keys::<T>::get( netuid, uid );
            // 2. Remove previous set memberships.
            Uids::<T>::remove( netuid, old_key.clone() );  // Remove old key - uid association.
            Uids::<T>::insert( netuid, new_key.clone(), uid ); // Make uid - key association.
            Keys::<T>::insert( netuid, uid, new_key.clone() ); // Make key - uid association.
            
            // pop frm incentive vector and push to new key
            let mut incentive: Vec<u16> = Incentive::<T>::get( netuid ); 
            let mut dividends: Vec<u16> = Dividends::<T>::get( netuid ); 
            let mut last_update: Vec<u64> = LastUpdate::<T>::get( netuid );
            let mut emission: Vec<u64> = Emission::<T>::get( netuid ); 

            
            incentive[uid as usize] = 0 as u16;
            dividends[uid as usize] = 0 as u16;
            emission[uid as usize] = 0 as u64;
            last_update[uid as usize] = block_number as u64;
            
            Incentive::<T>::insert( netuid, incentive ); // Make uid - key association.
            Emission::<T>::insert( netuid, emission ); // Make uid - key association.
            Dividends::<T>::insert( netuid, dividends ); // Make uid - key association.
            LastUpdate::<T>::insert( netuid, last_update ); // Make uid - key association.
            RegistrationBlock::<T>::insert( netuid, uid, block_number ); // Fill block at registration.
            Address::<T>::insert( netuid, uid, address ); // Fill module info.

            let old_name = Names::<T>::get( netuid, uid );
            Namespace::<T>::remove( netuid, old_name.clone() ); // Fill module namespace.
            Namespace::<T>::insert( netuid, name.clone(), uid ); // Fill module namespace.
            Names::<T>::insert( netuid, uid, name.clone() ); // Fill module namespace.

            // 3. Remove the network if it is empty.
            // Weights::<T>::insert( netuid, uid, vec![] as Vec<(u16, u16)> ); // Make uid - key association.
            Weights::<T>::insert( netuid, uid, vec![] as Vec<(u16, u16)> ); // Make uid - key association.
            // 3. Remove the stake from the old account and add to the new
            Self::remove_stake_from_storage( netuid, &old_key );

            // add stake to new key
            Self::add_stake_on_account( netuid, &new_key, stake );
            
        }




    

        // Replace the module under this uid.
        pub fn remove_module( netuid: u16, uid: u16 ) {
            // 1. Get the old key under this position.

            let n = Self::get_subnet_n( netuid );

            assert!( n > 0, "There are no modules in this network." );
            assert!( uid < n, "The uid is out of bounds." );

            let replace_uid = Self::get_subnet_n( netuid ) - 1;

            
            Self::replace_module_with_uid( netuid, uid, replace_uid );

            let replace_key: T::AccountId = Keys::<T>::get( netuid, replace_uid );
            // 2. Remove previous set memberships.
            Uids::<T>::remove( netuid, &replace_key.clone() ); 
            Keys::<T>::remove( netuid, replace_uid ); // Make key - uid association.
            Address::<T>::remove(netuid, replace_uid ); // Make uid - key association.
            RegistrationBlock::<T>::remove( netuid, replace_uid ); // Fill block at registration.
            Weights::<T>::remove( netuid, replace_uid ); // Make uid - key association.
            Names::<T>::remove( netuid, replace_uid ); // Make uid - key association.
            N::<T>::mutate( netuid, |v| *v -= 1 ); // Decrease the number of modules in the network.
            
            // pop frm incentive vector and push to new key
            Incentive::<T>::mutate( netuid, |v| v.pop() );
            Dividends::<T>::mutate( netuid, |v| v.pop() );
            Emission::<T>::mutate( netuid, |v| v.pop() );
            LastUpdate::<T>::mutate( netuid, |v| v.pop() );


            // 3. Remove the network if it is empty.
            if N::<T>::get( netuid ) == 0 {
                Self::remove_network_for_netuid( netuid );
            }


            Self::remove_stake_from_storage( netuid, &replace_key );
            
            // 4. Emit the event.
            
        }
    

        // Appends the uid to the network.
        pub fn append_module( netuid: u16, key: &T::AccountId , name: Vec<u8>, address: Vec<u8>, stake: u64) -> u16{
    
            // 1. Get the next uid. This is always equal to subnetwork_n.
            let uid: u16 = Self::get_subnet_n( netuid );
            let block_number = Self::get_current_block_as_u64();
            log::debug!("append_module( netuid: {:?} | uid: {:?} | new_key: {:?} ) ", netuid, key, uid );
    
            // 3. Expand Yuma with new position.
            Emission::<T>::mutate(netuid, |v| v.push(0) );
            Incentive::<T>::mutate(netuid, |v| v.push(0) );
            Dividends::<T>::mutate(netuid, |v| v.push(0) );
            LastUpdate::<T>::mutate(netuid, |v| v.push( block_number ) );
        
            // 4. Insert new account information.
            Keys::<T>::insert( netuid, uid, key.clone() ); // Make key - uid association.
            Uids::<T>::insert( netuid, key.clone(), uid ); // Make uid - key association.
            RegistrationBlock::<T>::insert( netuid, uid, block_number ); // Fill block at registration.
            Namespace::<T>::insert( netuid, name.clone(), uid ); // Fill module namespace.
            Names::<T>::insert( netuid, uid, name.clone() ); // Fill module namespace.
            Address::<T>::insert( netuid, uid, address.clone() ); // Fill module info.
            Self::add_stake_on_account( netuid, &key, stake );
            
            // 3. Get and increase the uid count.
            N::<T>::insert( netuid, uid + 1 );
    
            return uid;
    
        }   
    
	pub fn get_modules(netuid: u16) -> Vec<ModuleSubnetInfo<T>> {
        if !Self::if_subnet_exist(netuid) {
            return Vec::new();
        }

        let mut modules = Vec::new();
        let n = Self::get_subnet_n(netuid);
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

