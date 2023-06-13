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
    name: Vec<u8>,
    immunity_period: Compact<u16>,
    min_allowed_weights: Compact<u16>,
    max_weights_limit: Compact<u16>,
    subnetwork_n: Compact<u16>,
    max_allowed_uids: Compact<u16>,
    blocks_since_last_step: Compact<u64>,
    tempo: Compact<u16>,
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
        Weights::<T>::clear_prefix( netuid, u32::max_value(), None );
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

        // --- 3. Erase network stake, and remove network from list of networks.
        for ( key, stated_amount ) in <Stake<T> as IterableStorageDoubleMap<u16, T::AccountId, u64> >::iter_prefix(netuid){
            Self::decrease_stake_on_account( netuid, &key, stated_amount );
        }
        // --- 4. Remove all stake.
        Stake::<T>::remove_prefix( netuid, None );
        let total_network_stake: u16 = TotalSubnetStake::<T>::get( netuid ).try_into().unwrap();
        TotalSubnetStake::<T>::mutate(netuid, |n| *n -= total_network_stake as u64 );
        TotalSubnetStake::<T>::remove( netuid );



    }

    // Returns true if the subnetwork exists.
    //
    pub fn if_subnet_exist( netuid: u16 ) -> bool{
        return SubnetworkN::<T>::contains_key( netuid );
    }
    pub fn least_staked_netuid(stake: u64) -> u16 {
        let mut min_stake: u64 = 0;
        let mut min_stake_netuid: u16 = 0;
        if stake > min_stake {
            for ( netuid, net_stake ) in <TotalSubnetStake<T> as IterableStorageMap<u16, u64> >::iter(){
                if net_stake <= min_stake {
                    min_stake = net_stake;
                    min_stake_netuid = netuid;
                }
            }
        }
        return min_stake_netuid;
    }


    pub fn get_network_stake( netuid: u16 ) -> u64 {
        return TotalSubnetStake::<T>::get( netuid );
    }
    

    pub fn add_network( name: Vec<u8>, stake: u64) -> u16 {

        // --- 1. Ensure that the network name does not already exist.
        let total_networks = TotalNetworks::<T>::get();
        let max_networks = MaxAllowedSubnets::<T>::get();
        // if networks 
        let netuid : u16 ; 
        if total_networks >= max_networks {
            netuid = Self::least_staked_netuid(stake);
            Self::remove_network_for_netuid( netuid );
        } else {
            netuid = TotalNetworks::<T>::get();

        }
        SubnetworkN::<T>::insert( netuid, 1 );
        Self::set_default_values_for_all_parameters( netuid );
        
        SubnetNamespace::<T>::insert( name.clone(), netuid );
        TotalNetworks::<T>::mutate( |n| *n += 1 );
    
    
        // --- 6. Emit the new network event.
        log::info!("NetworkAdded( netuid:{:?}, name:{:?} )", netuid, name.clone());
        Self::deposit_event( Event::NetworkAdded( netuid, name.clone()) );
    

        return netuid;

    }



    // Initializes a new subnetwork under netuid with parameters.
    //
    pub fn if_subnet_name_exists(name: Vec<u8>) -> bool {
       
   
        return  SubnetNamespace::<T>::contains_key(name.clone()).into();
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
        let name = Self::get_name_for_netuid(netuid);
        let min_allowed_weights = Self::get_min_allowed_weights(netuid);
        let max_weights_limit = Self::get_max_weight_limit(netuid);
        let subnetwork_n = Self::get_subnetwork_n(netuid);
        let max_allowed_uids = Self::get_max_allowed_uids(netuid);
        let blocks_since_last_step = Self::get_blocks_since_last_step(netuid);
        let tempo = Self::get_tempo(netuid);



        return Some(SubnetInfo {
            immunity_period: immunity_period.into(),
            name: name,
            netuid: netuid.into(),
            min_allowed_weights: min_allowed_weights.into(),
            max_weights_limit: max_weights_limit.into(),
            subnetwork_n: subnetwork_n.into(),
            max_allowed_uids: max_allowed_uids.into(),
            blocks_since_last_step: blocks_since_last_step.into(),
            tempo: tempo.into(),
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
    pub fn get_key_for_uid( netuid: u16, module_uid: u16) ->  T::AccountId {
        Keys::<T>::try_get(netuid, module_uid).unwrap() 
    }
    

    // Returns the uid of the key in the network as a Result. Ok if the key has a slot.
    //
    pub fn get_uid_for_key( netuid: u16, key: &T::AccountId) -> Result<u16, DispatchError> { 
        return Uids::<T>::try_get(netuid, &key).map_err(|_err| Error::<T>::NotRegistered.into()) 
    }

    pub fn get_uid_for_name ( netuid: u16, name: Vec<u8> ) -> u16  {
        return ModuleNamespace::<T>::get(netuid, name)
    }

    pub fn get_name_for_uid ( netuid: u16, uid: u16 ) -> Vec<u8>  {
        return Names::<T>::get(netuid, uid);
    }


    pub fn if_module_name_exists( netuid: u16, name: Vec<u8> ) -> bool {
        return ModuleNamespace::<T>::contains_key( netuid, name.clone() );
        
    }

    // Returns the stake of the uid on network or 0 if it doesnt exist.
    //
    pub fn get_stake_for_uid( netuid: u16, module_uid: u16) -> u64 { 
        return Self::get_stake_for_key( netuid, &Self::get_key_for_uid( netuid, module_uid) )
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
    
