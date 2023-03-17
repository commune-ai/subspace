use super::*;

impl<T: Config> Pallet<T> {
    pub fn do_serve_module( origin: T::Origin,  name: Vec<u8>,  ip: u128, port: u16 ) -> dispatch::DispatchResult {

        // --- We check the callers (key) signature.
        let key_id = ensure_signed(origin)?;

        // --- We make validy checks on the passed data.
        ensure!( Keys::<T>::contains_key(&key_id), Error::<T>::NotRegistered );        
        ensure!( is_valid_ip_address( ip), Error::<T>::InvalidIpAddress );
  
        // --- We get the uid associated with this key account.
        let uid = Self::get_uid_for_key(&key_id);

        // --- We get the module assoicated with this key.
        let mut module = Self::get_module_for_uid(uid);
        module.ip = ip;
        module.port = port;
        module.active = 1;
        // set the ownership to 50% of the max value, this means that the module is owned by the key account.
        module.ownership = u8::MAX / 2 ;
        module.last_update = Self::get_current_block_as_u64();

        // --- We deposit the module updated event
        Name2uid::<T>::insert(name.clone(), uid);
        Modules::<T>::insert(uid, module);
        Self::deposit_event(Event::ModuleServed(uid));
        
        Ok(())
    }

    /********************************
     --==[[  Helper functions   ]]==--
    *********************************/

}

// @todo (Parallax 2-1-2021) : Implement exclusion of private IP ranges

fn get_ip_type(ip: u128) -> u8 {
    if ip <= u32::MAX as u128 {
        return 4;
    } else if ip <= u128::MAX {
        return 6;
    } 

    // If the IP address is not IPv4 or IPv6 and not private, raise an error
    return 0;
} 



fn is_valid_ip_address( addr: u128) -> bool {
    let ip_type: u8 = get_ip_type(addr);

    if addr == 0 {
        return false;
    }

    if ip_type == 6{ return false; }  {
        
        if addr >= u32::MAX as u128 { return false; }
        if addr == 0x7f000001 { return false; } // Localhost
    }

    if ip_type == 6 {
        if addr == 0x0 { return false; }
        if addr == u128::MAX { return false; }
        if addr == 1 { return false; } // IPv6 localhost
    }

    return true;
}
