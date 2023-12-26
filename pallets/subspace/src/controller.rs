 
use frame_support::{pallet_prelude::DispatchResult};
use substrate_fixed::types::{I110F18, I32F32, I64F64, I96F32};

use super::*;


impl<T: Config> Pallet<T> {

    pub fn get_controlled_keys(
        controller: T::AccountId,
    ) -> Vec<T::AccountId> {
        return Controller2Keys::<T>::iter_prefix_values(controller)
            .collect::<Vec<T::AccountId>>()
    }

    pub fn get_controller(
        key: T::AccountId,
    ) -> T::AccountId {
        return Key2Controller::<T>::get(key)
    }

    pub fn do_add_controller_to_key(
        origin: T::Origin,
        key: T::AccountId,
        controller: T::AccountId,
    ) -> DispatchResult {
        let key = ensure_signed(origin)?;
        ensure!(who == controller, Error::<T>::NotController);
        ensure!(!Self::is_key_controlled(key), Error::<T>::AlreadyControlled);
        ensure!(!Self::check_key_controller(key, controller), Error::<T>::AlreadyController);
        Key2Controller::<T>::insert(&key, controller);
        // insert it into the key vector Vec<T::AccountId>
        // Key2Controller::<T>::insert(&key, controller);
        let controller_keys = Self::get_controlled_keys(controller);
        controler_keys.push(key);
        Controller2Keys::<T>::insert(&controller, key);
        Ok(())

        
    }

    pub fn check_key_controller(
        key: T::AccountId,
        controller: T::AccountId,
    ) -> bool {
        return Key2Controller::<T>::get(key) == controller
    }

    pub fn is_key_controlled(
        key: T::AccountId,
    ) -> bool {
        return Key2Controller::<T>::contains_key(key)
    }


    pub fn remove_controller_key (
        key: T::AccountId,
        controller: T::AccountId,
    ) -> bool {
        let mut changeed = false;
        let controller_keys = Self::get_controlled_keys(controller);
        for (i, k) in controller_keys.iter().enumerate() {
            if *k == key {
                controller_keys.remove(i);
                Controller2Keys::<T>::insert(&controller, controller_keys);
                break;

            }
        }


    }

    pub fn remove_controller_from_key(
        key: T::AccountId,
        controller: T::AccountId,
    ) -> DispatchResult {
        Key2Controller::<T>::remove(key);

        Ok(())
    }

    pub fn get_controlled_keys(
        controller: T::AccountId,
    ) -> Vec<T::AccountId> {

        let mut controlled_keys = Vec::new();
        for (key, _) in 
            if controller == key {
                controlled_keys.push(key);
            }
        }
        controlled_keys
    }


