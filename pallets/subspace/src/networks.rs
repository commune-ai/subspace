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


    // ---- The implementation for the extrinsic add_network.
    //
    // # Args:
    // 	* 'origin': (<T as frame_system::Config>RuntimeOrigin):
    // 		- Must be sudo.
    //
    // 	* 'netuid' (u16):
    // 		- The u16 network identifier.
    //
    // 	* 'tempo' ( u16 ):
    // 		- Number of blocks between epoch step.
    //
    // 	* 'modality' ( u16 ):
    // 		- Network modality specifier.
    //
    // # Event:
    // 	* NetworkAdded;
    // 		- On successfully creation of a network.
    //
    // # Raises:
    // 	* 'NetworkExist':
    // 		- Attempting to register an already existing.
    //
    // 	* 'InvalidTempo':
    // 		- Attempting to register a network with an invalid tempo.
    //
    pub fn get_netuid_for_name( name: Vec<u8> ) -> u16 {
        let netuid: u16 = SubnetNamespace::<T>::get(name.clone());
        return netuid;
    }
    
    pub fn do_add_network( 
        origin: T::RuntimeOrigin, 
        name: Vec<u8>,
        context: Vec<u8>,
        tempo: u16, 
        n: u16,
    ) -> dispatch::DispatchResultWithPostInfo {

        ensure!( n > 0, Error::<T>::InvalidMaxAllowedUids);
        // ensure!( max_allowed_uids < InitialMaxAllowedUids, Error::<T>::InvalidMaxAllowedUids);
        ensure!( !Self::if_subnet_name_exists( name.clone() ), Error::<T>::NetworkExist );


        // --- 1. Ensure this is a sudo caller.
        let key = ensure_signed( origin )?;

        // --- 2. Ensure this subnetwork does not already exist.

        // --- 4. Ensure the tempo is valid.
        ensure!( Self::if_tempo_is_valid( tempo ), Error::<T>::InvalidTempo );

        // --- 5. Initialize the network and all its parameters.
        let netuid = Self::init_new_network( name.clone(), tempo , n);
        
        // --- 6. Emit the new network event.
        log::info!("NetworkAdded( netuid:{:?}, name:{:?} )", netuid, name.clone());
        Self::deposit_event( Event::NetworkAdded( netuid, name.clone()) );

        // --- 7. Ok and return.
        Ok(().into())
    }

    // ---- The implementation for the extrinsic remove_network.
    //
    // # Args:
    // 	* 'origin': (<T as frame_system::Config>RuntimeOrigin):
    // 		- Must be sudo.
    //
    // 	* 'netuid' (u16):
    // 		- The u16 network identifier.
    //
    // # Event:
    // 	* NetworkRemoved;
    // 		- On the successfull removing of this network.
    //
    // # Raises:
    // 	* 'NetworkDoesNotExist':
    // 		- Attempting to remove a non existent network.
    //
    pub fn do_remove_network( origin: T::RuntimeOrigin, netuid: u16 ) -> dispatch::DispatchResult {

        // --- 1. Ensure the function caller it Sudo.
        let key = ensure_signed( origin )?;

        // --- 2. Ensure the network to be removed exists.
        ensure!( Self::if_subnet_exist( netuid ), Error::<T>::NetworkDoesNotExist );

        // --- 3. Explicitly erase the network and all its parameters.
        Self::remove_network_by_netuid( netuid );
    
        // --- 4. Emit the event.
        log::info!("NetworkRemoved( netuid:{:?} )", netuid);
        Self::deposit_event( Event::NetworkRemoved( netuid ) );

        // --- 5. Ok and return.
        Ok(())
    }



    // ---- The implementation for the extrinsic set_emission_values.
    //
    // # Args:
    // 	* 'origin': (<T as frame_system::Config>RuntimeOrigin):
    // 		- Must be sudo.
    //
   	// 	* `netuids` (Vec<u16>):
	// 		- A vector of network uids values. This must include all netuids.
	//
	// 	* `emission` (Vec<u64>):
	// 		- The emission values associated with passed netuids in order.
    //
    // # Event:
    // 	* NetworkRemoved;
    // 		- On the successfull removing of this network.
    //
    // # Raises:
    // 	* 'EmissionValuesDoesNotMatchNetworks':
    // 		- Attempting to remove a non existent network.
    //
    pub fn do_set_emission_values( 
        origin: T::RuntimeOrigin, 
        netuids: Vec<u16>,
        emission: Vec<u64>
    ) -> dispatch::DispatchResult {

        // --- 1. Ensure caller is sudo.
        let key = ensure_signed( origin )?;

        // --- 2. Ensure emission values match up to network uids.
        ensure!( netuids.len() == emission.len(), Error::<T>::WeightVecNotEqualSize );

        // --- 3. Ensure we are setting emission for all networks. 
        ensure!( netuids.len() as u16 == TotalNetworks::<T>::get(), Error::<T>::NotSettingEnoughWeights );

        // --- 4. Ensure the passed uids contain no duplicates.
        ensure!( !Self::has_duplicate_netuids( &netuids ), Error::<T>::DuplicateUids );

        // --- 5. Ensure that the passed uids are valid for the network.
        ensure!( !Self::contains_invalid_netuids( &netuids ), Error::<T>::InvalidUid );

        // --- 6. check if sum of emission rates is equal to 1.
        ensure!( emission.iter().sum::<u64>() as u64 == Self::get_block_emission(), Error::<T>::InvalidEmissionValues);

        // --- 7. Add emission values for each network
        Self::set_emission_values( &netuids, &emission );

        // --- 8. Add emission values for each network
        log::info!("EmissionValuesSet()");
        Self::deposit_event( Event::EmissionValuesSet() );

        // --- 9. Ok and return.
        Ok(())
    }

    // Initializes a new subnetwork under netuid with parameters.
    //
    pub fn if_subnet_name_exists(name: Vec<u8>) -> bool {
       
   
        return  SubnetNamespace::<T>::contains_key(name.clone());
    }


    pub fn init_new_network(  name: Vec<u8>, tempo:u16, n:u16) -> u16{
        // --- 1. Get the next network uid.
        let netuid = TotalNetworks::<T>::get();
        TotalNetworks::<T>::mutate( |n| *n += 1 );

        // --- 2. Set network neuron count to 0 size.
        SubnetworkN::<T>::insert( netuid, 0 );

        // --- 3. Set this network uid to alive.
        NetworksAdded::<T>::insert( netuid, true );
        
        // --- 4. Fill tempo memory item.
        Tempo::<T>::insert( netuid, tempo );

    
        MaxAllowedUids::<T>::insert( netuid, n );

        // --- 5. Increase total network count.
        TotalNetworks::<T>::mutate( |n| *n += 1 );

        SubnetNamespace::<T>::insert(name.clone(),  netuid );



        

        // --- 6. Set all default values **explicitly**.
        Self::set_default_values_for_all_parameters( netuid );

        return netuid;
    }

    // Removes the network (netuid) and all of its parameters.
    //
    pub fn remove_network_by_netuid( netuid:u16 ) {

        // --- 1. Remove network count.
        SubnetworkN::<T>::remove( netuid );

        // --- 3. Remove netuid from added networks.
        NetworksAdded::<T>::remove( netuid );

        // --- 4. Erase all memory associated with the network.
        Self::erase_all_network_data( netuid );

        // --- 5. Decrement the network counter.
        TotalNetworks::<T>::mutate(|val| *val -= 1);
    }


    // Removes the network (netuid) and all of its parameters.
    //
    pub fn remove_network_by_name( name:Vec<u8> ) {

        let netuid  = SubnetNamespace::<T>::get(name.clone());
        // --- 1. Remove network count.
        SubnetworkN::<T>::remove( netuid );

        // --- 3. Remove netuid from added networks.
        NetworksAdded::<T>::remove( netuid );

        // --- 4. Erase all memory associated with the network.
        Self::erase_all_network_data( netuid );

        // --- 5. Decrement the network counter.
        TotalNetworks::<T>::mutate(|val| *val -= 1);
    }


    // Explicitly sets all network parameters to their default values.
    // Note: this is required because, although there are defaults, they are not explicitly set until this call.
    //
    pub fn set_default_values_for_all_parameters(netuid: u16){
        // Make network parameters explicit.
        if !Tempo::<T>::contains_key( netuid ) { Tempo::<T>::insert( netuid, Tempo::<T>::get( netuid ));}
        if !MaxAllowedUids::<T>::contains_key( netuid ) { MaxAllowedUids::<T>::insert( netuid, MaxAllowedUids::<T>::get( netuid ));}
        if !ImmunityPeriod::<T>::contains_key( netuid ) { ImmunityPeriod::<T>::insert( netuid, ImmunityPeriod::<T>::get( netuid ));}
        if !ActivityCutoff::<T>::contains_key( netuid ) { ActivityCutoff::<T>::insert( netuid, ActivityCutoff::<T>::get( netuid ));}
        if !EmissionValues::<T>::contains_key( netuid ) { EmissionValues::<T>::insert( netuid, EmissionValues::<T>::get( netuid ));}   
        if !MaxWeightsLimit::<T>::contains_key( netuid ) { MaxWeightsLimit::<T>::insert( netuid, MaxWeightsLimit::<T>::get( netuid ));}
        if !MinAllowedWeights::<T>::contains_key( netuid ) { MinAllowedWeights::<T>::insert( netuid, MinAllowedWeights::<T>::get( netuid )); }
        if !RegistrationsThisInterval::<T>::contains_key( netuid ) { RegistrationsThisInterval::<T>::insert( netuid, RegistrationsThisInterval::<T>::get( netuid ));}
    }

    // Explicitly erases all data associated with this network.
    //
    pub fn erase_all_network_data(netuid: u16){

        // --- 1. Remove incentive mechanism memory.
        let _ = Uids::<T>::clear_prefix( netuid, u32::max_value(), None );
        let _ = Keys::<T>::clear_prefix( netuid, u32::max_value(), None );
        let _ = Bonds::<T>::clear_prefix( netuid, u32::max_value(), None );
        let _ = Weights::<T>::clear_prefix( netuid, u32::max_value(), None );

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
    }



    // Returns true if the items contain duplicates.
    //
    fn has_duplicate_netuids( netuids: &Vec<u16> ) -> bool {
        let mut parsed: Vec<u16> = Vec::new();
        for item in netuids {
            if parsed.contains(&item) { return true; }
            parsed.push(item.clone());
        }
        return false;
    }

    // Checks for any invalid netuids on this network.
    //
    pub fn contains_invalid_netuids( netuids: &Vec<u16> ) -> bool {
        for netuid in netuids {
            if !Self::if_subnet_exist( *netuid ) {
                return true;
            }
        }
        return false;
    }

    // Set emission values for the passed networks. 
    //
    pub fn set_emission_values( netuids: &Vec<u16>, emission: &Vec<u64> ){
        for (i, netuid_i) in netuids.iter().enumerate() {
            Self::set_emission_for_network( *netuid_i, emission[i] ); 
        }
    }

    // Set the emission on a single network.
    //
    pub fn set_emission_for_network( netuid: u16, emission: u64 ){
        EmissionValues::<T>::insert( netuid, emission );
    }

    // Returns true if the subnetwork exists.
    //
    pub fn if_subnet_exist( netuid: u16 ) -> bool{
        return NetworksAdded::<T>::get( netuid );
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
        let mut subnet_netuids = Vec::<u16>::new();
        let mut max_netuid: u16 = 0;
        for ( netuid, added ) in < NetworksAdded<T> as IterableStorageMap<u16, bool> >::iter() {
            if added {
                subnet_netuids.push(netuid);
                if netuid > max_netuid {
                    max_netuid = netuid;
                }
            }
        }

        let mut subnets_info = Vec::<Option<SubnetInfo>>::new();
        for netuid_ in 0..(max_netuid + 1) {
            if subnet_netuids.contains(&netuid_) {
                subnets_info.push(Self::get_subnet_info(netuid_));
            }
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
    
