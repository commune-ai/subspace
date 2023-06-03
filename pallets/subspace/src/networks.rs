use super::*;
use frame_support::{sp_std::vec};
use sp_std::vec::Vec;
use frame_system::ensure_root;
use frame_support::IterableStorageDoubleMap;
use frame_support::storage::IterableStorageMap;
use frame_support::pallet_prelude::{Decode, Encode};
use codec::Compact;
use frame_support::pallet_prelude::DispatchError;
extern crate alloc;


#[derive(Decode, Encode, PartialEq, Eq, Clone, Debug)]
pub struct SubnetInfo {
    netuid: Compact<u16>,
    immunity_period: Compact<u16>,
    min_allowed_weights: Compact<u16>,
    max_weights_limit: Compact<u16>,
    subnetwork_n: Compact<u16>,
    max_allowed_uids: Compact<u16>,
    blocks_since_last_step: Compact<u64>,
    tempo: Compact<u16>,
    emission_values: Compact<u64>,
}




impl<T: Config> Pallet<T> { 

    // Explicitly sets all network parameters to their default values.
    // Note: this is required because, although there are defaults, they are not explicitly set until this call.
    //
    pub fn set_default_values_for_all_parameters(netuid: u16){
        // Make network parameters explicit.
        // --- 1. Remove network count.

        Tempo::<T>::insert( netuid, Tempo::<T>::get( netuid ));
        MaxAllowedUids::<T>::insert( netuid, MaxAllowedUids::<T>::get( netuid ));
        ImmunityPeriod::<T>::insert( netuid, ImmunityPeriod::<T>::get( netuid ));
        ActivityCutoff::<T>::insert( netuid, ActivityCutoff::<T>::get( netuid ));
        EmissionValues::<T>::insert( netuid, EmissionValues::<T>::get( netuid ));
        MaxWeightsLimit::<T>::insert( netuid, MaxWeightsLimit::<T>::get( netuid ));
        MinAllowedWeights::<T>::insert( netuid, MinAllowedWeights::<T>::get( netuid ));
        RegistrationsThisInterval::<T>::insert( netuid, RegistrationsThisInterval::<T>::get( netuid ));
    }

    // Explicitly erases all data associated with this network.
    //
    pub fn erase_all_network_data(netuid: u16){

        // --- 1. Remove incentive mechanism memory.
        Uids::<T>::clear_prefix( netuid, u32::max_value(), None );
        Keys::<T>::clear_prefix( netuid, u32::max_value(), None );
        Bonds::<T>::clear_prefix( netuid, u32::max_value(), None );
        Weights::<T>::clear_prefix( netuid, u32::max_value(), None );

        Active::<T>::remove( netuid );
        Emission::<T>::remove( netuid );
        Incentive::<T>::remove( netuid );
        Dividends::<T>::remove( netuid );
        LastUpdate::<T>::remove( netuid );

        // --- 2. Erase network parameters.
        Tempo::<T>::remove( netuid );
        MaxAllowedUids::<T>::remove( netuid );
        ImmunityPeriod::<T>::remove( netuid );
        ActivityCutoff::<T>::remove( netuid );
        EmissionValues::<T>::remove( netuid );
        MaxWeightsLimit::<T>::remove( netuid );
        MinAllowedWeights::<T>::remove( netuid );
        RegistrationsThisInterval::<T>::remove( netuid );
        SubnetworkN::<T>::remove( netuid );


    }

    // Returns true if the subnetwork exists.
    //
    pub fn if_subnet_exist( netuid: u16 ) -> bool{
        return SubnetworkN::<T>::contains_key( netuid );
    }
    pub fn least_staked_netuid() -> u16 {
        let mut min_stake: u64 = 0;
        let mut min_stake_netuid: u16 = 0;
        for ( netuid, net_stake ) in <TotalSubnetStake<T> as IterableStorageMap<u16, u64> >::iter(){
            
            if net_stake <= min_stake {
                min_stake = net_stake;
                min_stake_netuid = netuid;
            }
        }

        return min_stake_netuid;
    }

    pub fn connect_network(name: Vec<u8>) -> u16 {
        if Self::if_subnet_name_exists( name.clone() ) {
            return Self::get_netuid_for_name( name.clone() );
        } else {
            return Self::add_network( name.clone() );
        }
            
    }

    pub fn get_network_stake( netuid: u16 ) -> u64 {
        return TotalSubnetStake::<T>::get( netuid );
    }

    

    pub fn add_network( name: Vec<u8>) -> u16 {
        // ensure!( max_allowed_uids < InitialMaxAllowedUids, Error::<T>::InvalidMaxAllowedUids);

        let total_networks = TotalNetworks::<T>::get();
        let max_networks = MaxAllowedSubnets::<T>::get();
        if total_networks >= max_networks{
            let netuid = Self::least_staked_netuid();
            Self::remove_network_for_netuid( netuid );
        }
        let netuid = TotalNetworks::<T>::get();
        SubnetworkN::<T>::insert( netuid, 1 );
        Self::set_default_values_for_all_parameters( netuid );
        
        
        TotalNetworks::<T>::mutate( |n| *n += 1 );
    
    
        // --- 6. Emit the new network event.
        log::info!("NetworkAdded( netuid:{:?}, name:{:?} )", netuid, name.clone());
        Self::deposit_event( Event::NetworkAdded( netuid, name.clone()) );
    

        return netuid;

    }



    // Initializes a new subnetwork under netuid with parameters.
    //
    pub fn if_subnet_name_exists(name: Vec<u8>) -> bool {
       
   
        return  SubnetNamespace::<T>::contains_key(name.clone());
    }


    pub fn get_netuid_for_name( name: Vec<u8> ) -> u16 {
        let netuid: u16 = SubnetNamespace::<T>::get(name.clone());
        return netuid;
    }


    pub fn get_name_for_netuid( netuid : u16) -> Vec<u8> {
        for ( name, _netuid ) in <SubnetNamespace<T> as IterableStorageMap<Vec<u8>, u16>>::iter(){
            if _netuid == netuid {
                return name;
            }
        }
        return Vec::new();
    }




    // Removes the network (netuid) and all of its parameters.
    //
    pub fn remove_network_for_netuid( netuid: u16 ) -> u16 {
        let name = Self::get_name_for_netuid( netuid );
        return Self::remove_network( name );
    }
    pub fn remove_network( name: Vec<u8> ) -> u16 {
        // --- 2. Ensure the network to be removed exists.
        if !Self::if_subnet_name_exists( name.clone() ) {
            return 0;
        }
        let netuid = Self::get_netuid_for_name( name.clone() );
        SubnetNamespace::<T>::remove( name.clone() );
        // --- 4. Erase all memory associated with the network.
        Self::erase_all_network_data( netuid );
        TotalNetworks::<T>::mutate(|val| *val -= 1);
        // --- 4. Emit the event.
        log::info!("NetworkRemoved( netuid:{:?} )", netuid);
        Self::deposit_event( Event::NetworkRemoved( netuid ) );

        return netuid;
        

    }

    // Returns true if the passed tempo is allowed.
    //
    pub fn if_tempo_is_valid(tempo: u16) -> bool {
        tempo < u16::MAX
    }


	pub fn get_subnet_info(netuid: u16) -> Option<SubnetInfo> {
        if !Self::if_subnet_exist(netuid) {
            return None;
        }

        let immunity_period = Self::get_immunity_period(netuid);
        let min_allowed_weights = Self::get_min_allowed_weights(netuid);
        let max_weights_limit = Self::get_max_weight_limit(netuid);
        let subnetwork_n = Self::get_subnetwork_n(netuid);
        let max_allowed_uids = Self::get_max_allowed_uids(netuid);
        let blocks_since_last_step = Self::get_blocks_since_last_step(netuid);
        let tempo = Self::get_tempo(netuid);
        let emission_values = Self::get_emission_value(netuid);



        return Some(SubnetInfo {
            immunity_period: immunity_period.into(),
            netuid: netuid.into(),
            min_allowed_weights: min_allowed_weights.into(),
            max_weights_limit: max_weights_limit.into(),
            subnetwork_n: subnetwork_n.into(),
            max_allowed_uids: max_allowed_uids.into(),
            blocks_since_last_step: blocks_since_last_step.into(),
            tempo: tempo.into(),
            emission_values: emission_values.into(),
        })
	}

    pub fn get_subnets_info() -> Vec<Option<SubnetInfo>> {
        let mut subnets_info = Vec::<Option<SubnetInfo>>::new();
        for ( netuid, net_n ) in < SubnetworkN<T> as IterableStorageMap<u16, u16> >::iter() {
            subnets_info.push(Self::get_subnet_info(netuid));
        }
        return subnets_info;
	}


    // Returns the number of filled slots on a network.
    ///
    pub fn get_subnetwork_n( netuid:u16 ) -> u16 { 
        return SubnetworkN::<T>::get( netuid ) 
    }
    
    // Replace the neuron under this uid.
    pub fn replace_neuron( netuid: u16, uid_to_replace: u16, new_key: &T::AccountId ) {

        log::debug!("replace_neuron( netuid: {:?} | uid_to_replace: {:?} | new_key: {:?} ) ", netuid, uid_to_replace, new_key );

        // 1. Get the old key under this position.
        let old_key: T::AccountId = Keys::<T>::get( netuid, uid_to_replace );

        // 2. Remove previous set memberships.
        Uids::<T>::remove( netuid, old_key.clone() ); 
        IsNetworkMember::<T>::remove( old_key.clone(), netuid );
        Keys::<T>::remove( netuid, uid_to_replace ); 
        let block_number:u64 = Self::get_current_block_as_u64();

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

        // 3. Expand Yuma with new position.
        Active::<T>::mutate(netuid, |v| v.push( true ) );
        Emission::<T>::mutate(netuid, |v| v.push(0) );
        Incentive::<T>::mutate(netuid, |v| v.push(0) );
        Dividends::<T>::mutate(netuid, |v| v.push(0) );
        LastUpdate::<T>::mutate(netuid, |v| v.push( block_number ) );
    
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
    pub fn get_stake_for_uid( netuid: u16, neuron_uid: u16) -> u64 { 
        return Self::get_stake_for_key( netuid, &Self::get_key_for_net_and_uid( netuid, neuron_uid) )
    }

    pub fn get_stake_for_key( netuid: u16, key: &T::AccountId) -> u64 { 
        if Self::is_key_registered_on_network( netuid, &key) {
            return Stake::<T>::get( netuid, &key );
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
    
