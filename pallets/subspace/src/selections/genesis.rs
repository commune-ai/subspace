use crate::*;
use frame_support::pallet_macros::pallet_section;

#[pallet_section]
pub mod genesis {
    #[pallet::genesis_config]
    pub struct GenesisConfig<T: Config> {
        pub block: u32,
        _phantom: PhantomData<T>,
    }

    impl<T: Config> Default for GenesisConfig<T> {
        fn default() -> Self {
            Self {
                block: 0,
                _phantom: PhantomData,
            }
        }
    }

    // TODO: move it completelly out of the subspace pallet to a dedicated one
    #[pallet::genesis_build]
    impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
        fn build(&self) {
        }
    }
}
