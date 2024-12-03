use frame_system::{Config, Pallet};

#[derive(sp_core::Decode, sp_core::Encode, scale_info::TypeInfo)]
pub struct AuthorityNode {
    pub id: u32,
    pub encryption_key: (Vec<u8>, Vec<u8>), // (n, e) from RSA 512
}

#[derive(sp_core::Decode, sp_core::Encode, scale_info::TypeInfo)]
pub struct EncryptedData {
    runs: Vec<()>,
}

const DISTRIBUTION_INTERVAL: usize = 5; // successfull decryptions

impl<T: Config> super::Pallet<T> {
    fn distribute_nodes() {
        
    }

    fn do_authority_keepalive(
        
    ) {

    }
}
