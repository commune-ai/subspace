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
    active: bool,
    name: Vec<u8>,
    last_update: Compact<u64>,
    
    // Subnet Info
    stake: Vec<(T::AccountId, Compact<u64>)>, // map of key to stake on this module/key (includes delegations)
    emission: Compact<u64>,
    incentive: Compact<u16>,
    dividends: Compact<u16>,
    weights: Vec<(Compact<u16>, Compact<u16>)>, // Vec of (uid, weight)
    bonds: Vec<(Compact<u16>, Compact<u16>)>, // Vec of (uid, bond)
}


impl<T: Config> Pallet<T> {
	pub fn get_modules(netuid: u16) -> Vec<ModuleSubnetInfo<T>> {
        if !Self::if_subnet_exist(netuid) {
            return Vec::new();
        }

        let mut modules = Vec::new();
        let n = Self::get_subnetwork_n(netuid);
        for uid in 0..n {
            let uid = uid;
            let netuid = netuid;

            let _module = Self::get_module_subnet_exists(netuid, uid);
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

    fn get_module_subnet_exists(netuid: u16, uid: u16) -> Option<ModuleSubnetInfo<T>> {
        let key = Self::get_key_for_uid(netuid, uid);
        let module_info = Self::get_module_info( netuid, &key.clone() );


                
        let active = Self::get_active_for_uid( netuid, uid as u16 );
        let emission = Self::get_emission_for_uid( netuid, uid as u16 );
        let incentive = Self::get_incentive_for_uid( netuid, uid as u16 );
        let dividends = Self::get_dividends_for_uid( netuid, uid as u16 );
        let last_update = Self::get_last_update_for_uid( netuid, uid as u16 );
        let name = Self::get_name_for_uid( netuid, uid as u16 );

        let weights = <Weights<T>>::get(netuid, uid).iter()
            .filter_map(|(i, w)| if *w > 0 { Some((i.into(), w.into())) } else { None })
            .collect::<Vec<(Compact<u16>, Compact<u16>)>>();
        
        let bonds = <Bonds<T>>::get(netuid, uid).iter()
            .filter_map(|(i, b)| if *b > 0 { Some((i.into(), b.into())) } else { None })
            .collect::<Vec<(Compact<u16>, Compact<u16>)>>();
        
        let stake: Vec<(T::AccountId, Compact<u64>)> = Stake::<T>::iter_prefix(netuid)
            .map(|(key, stake)| (key, stake.into()))
            .collect();

        

        let module = ModuleSubnetInfo {
            key: key.clone(),
            uid: uid.into(),
            netuid: netuid.into(),
            active: active,
            stake: stake,
            emission: emission.into(),
            incentive: incentive.into(),
            dividends: dividends.into(),
            last_update: last_update.into(),
            weights: weights,
            bonds: bonds,
            name: name.clone()
        };
        
        return Some(module);
    }

    pub fn get_module(netuid: u16, uid: u16) -> Option<ModuleSubnetInfo<T>> {
        if !Self::if_subnet_exist(netuid) {
            return None;
        }

        let module = Self::get_module_subnet_exists(netuid, uid);
        return module;
	}



}

