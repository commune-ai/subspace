use super::*;

impl<T: Config> Pallet<T> {
    pub fn do_serve_module( origin: T::Origin,  name: Vec<u8>,  ip: u128, port: u16, ip_type: u8, self_ownership: u8 ) -> dispatch::DispatchResult {

        // --- We check the callers (key) signature.
        let key_id = ensure_signed(origin)?;

        // --- We make validy checks on the passed data.
        ensure!( Keys::<T>::contains_key(&key_id), Error::<T>::NotRegistered );        
        ensure!( is_valid_ip_type(ip_type), Error::<T>::InvalidIpType );
        ensure!( is_valid_ip_address(ip_type, ip), Error::<T>::InvalidIpAddress );
  
        // --- We get the uid associated with this key account.
        let uid = Self::get_uid_for_key(&key_id);

        // --- We get the module assoicated with this key.
        let mut module = Self::get_module_for_uid(uid);
        module.ip = ip;
        module.port = port;
        module.ip_type = ip_type;
        module.active = 1;
        module.ownership = self_ownership;
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


fn is_valid_ip_type(ip_type: u8) -> bool {
    let allowed_values: Vec<u8> = vec![4, 6];
    return allowed_values.contains(&ip_type);
}

// @todo (Parallax 2-1-2021) : Implement exclusion of private IP ranges
fn is_valid_ip_address(ip_type: u8, addr: u128) -> bool {
    if !is_valid_ip_type(ip_type) {
        return false;
    }

    if addr == 0 {
        return false;
    }

    if ip_type == 4 {
        if addr == 0 { return false; }
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

#[cfg(test)]
mod test {
    use crate::serving::{is_valid_ip_type, is_valid_ip_address};
    use std::net::{Ipv6Addr, Ipv4Addr};

    // Generates an ipv6 address based on 8 ipv6 words and returns it as u128
    pub fn ipv6(a: u16, b: u16, c: u16, d: u16, e: u16, f: u16, g: u16, h: u16) -> u128 {
        return Ipv6Addr::new(a, b, c, d, e, f, g, h).into();
    }

    // Generate an ipv4 address based on 4 bytes and returns the corresponding u128, so it can be fed
    // to the module::subscribe() function
    pub fn ipv4(a: u8, b: u8, c: u8, d: u8) -> u128 {
        let ipv4: Ipv4Addr = Ipv4Addr::new(a, b, c, d);
        let integer: u32 = ipv4.into();
        return u128::from(integer);
    }

    #[test]
    fn test_is_valid_ip_type_ok_ipv4() {
        assert_eq!(is_valid_ip_type(4), true);
    }

    #[test]
    fn test_is_valid_ip_type_ok_ipv6() {
        assert_eq!(is_valid_ip_type(6), true);
    }

    #[test]
    fn test_is_valid_ip_type_nok() {
        assert_eq!(is_valid_ip_type(10), false);
    }

    #[test]
    fn test_is_valid_ip_address_ipv4() {
        assert_eq!(is_valid_ip_address(4, ipv4(8, 8, 8, 8)), true);
    }

    #[test]
    fn test_is_valid_ip_address_ipv6() {
        assert_eq!(is_valid_ip_address(6, ipv6(1, 2, 3, 4, 5, 6, 7, 8)), true);
        assert_eq!(is_valid_ip_address(6, ipv6(1, 2, 3, 4, 5, 6, 7, 8)), true);
    }

    #[test]
    fn test_is_invalid_ipv4_address() {
        assert_eq!(is_valid_ip_address(4, ipv4(0, 0, 0, 0)), false);
        assert_eq!(is_valid_ip_address(4, ipv4(255, 255, 255, 255)), false);
        assert_eq!(is_valid_ip_address(4, ipv4(127, 0, 0, 1)), false);
        assert_eq!(is_valid_ip_address(4, ipv6(0xffff, 2, 3, 4, 5, 6, 7, 8)), false);
    }

    #[test]
    fn test_is_invalid_ipv6_addres() {
        assert_eq!(is_valid_ip_address(6, ipv6(0, 0, 0, 0, 0, 0, 0, 0)), false);
        assert_eq!(is_valid_ip_address(4, ipv6(0xffff, 0xffff, 0xffff, 0xffff, 0xffff, 0xffff, 0xffff, 0xffff)), false);
    }
}
