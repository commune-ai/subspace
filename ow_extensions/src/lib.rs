#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "std")]
use sp_externalities::ExternalitiesExt;

#[cfg(feature = "std")]
sp_externalities::decl_extension! {
    pub struct OffworkerExt(Box<dyn OffworkerExtension>);
}

#[cfg(feature = "std")]
impl OffworkerExt {
    pub fn new<T: OffworkerExtension>(t: T) -> Self {
        Self(Box::new(t))
    }
}

#[cfg(feature = "std")]
pub trait OffworkerExtension: Send + 'static {
    fn hash_weight(&self, weights: Vec<(u16, u16)>) -> Option<Vec<u8>>;

    fn decrypt_weight(&self, encrypted: Vec<u8>) -> Option<Vec<(u16, u16)>>;

    fn is_decryption_node(&self) -> bool;

    fn get_encryption_key(&self) -> Option<(Vec<u8>, Vec<u8>)>;
}

#[sp_runtime_interface::runtime_interface]
pub trait Offworker {
    fn hash_weight(
        &mut self,
        weights: sp_std::vec::Vec<(u16, u16)>,
    ) -> Option<sp_std::vec::Vec<u8>> {
        self.extension::<OffworkerExt>()
            .expect("missing offworker ext")
            .hash_weight(weights)
    }

    fn decrypt_weight(
        &mut self,
        encrypted: sp_std::vec::Vec<u8>,
    ) -> Option<sp_std::vec::Vec<(u16, u16)>> {
        self.extension::<OffworkerExt>()
            .expect("missing offworker ext")
            .decrypt_weight(encrypted)
    }

    fn is_decryption_node(&mut self) -> bool {
        self.extension::<OffworkerExt>()
            .expect("missing offworker ext")
            .is_decryption_node()
    }

    fn get_encryption_key(&mut self) -> Option<(sp_std::vec::Vec<u8>, sp_std::vec::Vec<u8>)> {
        self.extension::<OffworkerExt>()
            .expect("missing offworker ext")
            .get_encryption_key()
    }
}
