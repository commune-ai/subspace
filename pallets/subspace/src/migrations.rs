use crate::*;
use frame_support::{
    pallet_prelude::ValueQuery,
    traits::{Get, StorageVersion},
};

pub mod v15 {
    use dao::CuratorApplication;
    use frame_support::{traits::OnRuntimeUpgrade, weights::Weight};

    use super::*;

    pub mod old_storage {
        use super::*;
        use frame_support::{pallet_prelude::TypeInfo, storage_alias, Identity};

        #[storage_alias]
        pub type Weights<T: Config> = StorageMap<Pallet<T>, u16, Identity, u16, Vec<(u16, u16)>>;
    }
}
