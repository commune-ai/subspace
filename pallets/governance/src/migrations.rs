use crate::*;
use core::marker::PhantomData;
use frame_support::{migrations::VersionedMigration, traits::UncheckedOnRuntimeUpgrade};

pub type MigrationV1<T> =
    VersionedMigration<0, 1, _MigrationV1<T>, Pallet<T>, <T as frame_system::Config>::DbWeight>;

#[derive(Default)]
#[doc(hidden)]
pub struct _MigrationV1<T>(PhantomData<T>);

impl<T: Config + pallet_subspace::Config> UncheckedOnRuntimeUpgrade for _MigrationV1<T> {
    fn on_runtime_upgrade() -> frame_support::weights::Weight {
        // TODO: add migrations
        frame_support::weights::Weight::zero()
    }
}
