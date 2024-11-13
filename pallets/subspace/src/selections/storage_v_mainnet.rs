use frame_support::{pallet_macros::pallet_section, traits::StorageVersion};

pub const STORAGE_VERSION: StorageVersion = StorageVersion::new(14);

#[pallet_section]
pub mod storage_version {
    use crate::storage_v::STORAGE_VERSION;

    #[pallet::pallet]
    #[pallet::storage_version(STORAGE_VERSION)]
    #[pallet::without_storage_info]
    pub struct Pallet<T>(_);
}
