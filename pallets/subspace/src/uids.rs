use super::*;
use frame_support::{sp_std::vec};
use sp_std::vec::Vec;
use frame_support::storage::IterableStorageMap;
use frame_support::pallet_prelude::DispatchError;
use frame_support::storage::IterableStorageDoubleMap;

impl<T: Config> Pallet<T> { 

    // Returns the number of filled slots on a network.
    ///
    pub fn get_subnetwork_n( netuid:u16 ) -> u16 { 
        return SubnetworkN::<T>::get( netuid ) 
    }

    // Replace the neuron under this uid.
    pub fn replace_neuron( netuid: u16, uid_to_replace: u16, new_key: &T::AccountId, block_number:u64 ) {

        log::debug!("replace_neuron( netuid: {:?} | uid_to_replace: {:?} | new_key: {:?} ) ", netuid, uid_to_replace, new_key );

        // 1. Get the old key under this position.
        let old_key: T::AccountId = Keys::<T>::get( netuid, uid_to_replace );

        // 2. Remove previous set memberships.
        Uids::<T>::remove( netuid, old_key.clone() ); 
        IsNetworkMember::<T>::remove( old_key.clone(), netuid );
        Keys::<T>::remove( netuid, uid_to_replace ); 

        // 3. Create new set memberships.
        Self::set_active_for_uid( netuid, uid_to_replace, true ); // Set to active by default.
        Keys::<T>::insert( netuid, uid_to_replace, new_key.clone() ); // Make key - uid association.
        Uids::<T>::insert( netuid, new_key.clone(), uid_to_replace ); // Make uid - key association.
        BlockAtRegistration::<T>::insert( netuid, uid_to_replace, block_number ); // Fill block at registration.
        IsNetworkMember::<T>::insert( new_key.clone(), netuid, true ); // Fill network is member.
    }

    // Appends the uid to the network.
    pub fn append_neuron( netuid: u16, new_key: &T::AccountId ) {

        // 1. Get the next uid. This is always equal to subnetwork_n.
        let next_uid: u16 = Self::get_subnetwork_n( netuid );
        let block_number = Self::get_current_block_as_u64();
        log::debug!("append_neuron( netuid: {:?} | next_uid: {:?} | new_key: {:?} ) ", netuid, new_key, next_uid );

        // 2. Get and increase the uid count.
        SubnetworkN::<T>::insert( netuid, next_uid + 1 );

        // 3. Expand Yuma Consensus with new position.
        Rank::<T>::mutate(netuid, |v| v.push(0) );
        Trust::<T>::mutate(netuid, |v| v.push(0) );
        Active::<T>::mutate(netuid, |v| v.push( true ) );
        Emission::<T>::mutate(netuid, |v| v.push(0) );
        Consensus::<T>::mutate(netuid, |v| v.push(0) );
        Incentive::<T>::mutate(netuid, |v| v.push(0) );
        Dividends::<T>::mutate(netuid, |v| v.push(0) );
        LastUpdate::<T>::mutate(netuid, |v| v.push( block_number ) );
        PruningScores::<T>::mutate(netuid, |v| v.push(0) );
        ValidatorTrust::<T>::mutate(netuid, |v| v.push(0) );
        ValidatorPermit::<T>::mutate(netuid, |v| v.push(false) );
 
        // 4. Insert new account information.
        Keys::<T>::insert( netuid, next_uid, new_key.clone() ); // Make key - uid association.
        Uids::<T>::insert( netuid, new_key.clone(), next_uid ); // Make uid - key association.
        BlockAtRegistration::<T>::insert( netuid, next_uid, block_number ); // Fill block at registration.
        IsNetworkMember::<T>::insert( new_key.clone(), netuid, true ); // Fill network is member.
    }

    // Returns true if the uid is set on the network.
    //
    pub fn is_uid_exist_on_network(netuid: u16, uid: u16) -> bool {
        return  Keys::<T>::contains_key(netuid, uid);
    }

    // Returns true if the key holds a slot on the network.
    //
    pub fn is_key_registered_on_network( netuid:u16, key: &T::AccountId ) -> bool { 
        return Uids::<T>::contains_key( netuid, key ) 
    }

    // Returs the key under the network uid as a Result. Ok if the uid is taken.
    //
    pub fn get_key_for_net_and_uid( netuid: u16, neuron_uid: u16) ->  T::AccountId {
        Keys::<T>::try_get(netuid, neuron_uid).unwrap() 
    }
    

    // Returns the uid of the key in the network as a Result. Ok if the key has a slot.
    //
    pub fn get_uid_for_net_and_key( netuid: u16, key: &T::AccountId) -> Result<u16, DispatchError> { 
        return Uids::<T>::try_get(netuid, &key).map_err(|_err| Error::<T>::NotRegistered.into()) 
    }

    // Returns the stake of the uid on network or 0 if it doesnt exist.
    //
    pub fn get_stake_for_uid_and_subnetwork( netuid: u16, neuron_uid: u16) -> u64 { 
        if Self::is_uid_exist_on_network( netuid, neuron_uid) {
            return Self::get_total_stake_for_key( &Self::get_key_for_net_and_uid( netuid, neuron_uid ) ) 
        } else {
            return 0;
        }
    }


    // Return the total number of subnetworks available on the chain.
    //
    pub fn get_number_of_subnets()-> u16 {
        let mut number_of_subnets : u16 = 0;
        for (_, _)  in <SubnetworkN<T> as IterableStorageMap<u16, u16>>::iter(){
            number_of_subnets = number_of_subnets + 1;
        }
        return number_of_subnets;
    }

    // Return a list of all networks a key is registered on.
    //
    pub fn get_registered_networks_for_key( key: &T::AccountId )-> Vec<u16> {
        let mut all_networks: Vec<u16> = vec![];
        for ( network, is_registered)  in <IsNetworkMember<T> as IterableStorageDoubleMap< T::AccountId, u16, bool >>::iter_prefix( key ){
            if is_registered { all_networks.push( network ) }
        }
        all_networks
    }

    // Return true if a key is registered on any network.
    //
    pub fn is_key_registered_on_any_network( key: &T::AccountId )-> bool {
        for ( _, is_registered)  in <IsNetworkMember<T> as IterableStorageDoubleMap< T::AccountId, u16, bool >>::iter_prefix( key ){
            if is_registered { return true }
        }
        false
    }
}
