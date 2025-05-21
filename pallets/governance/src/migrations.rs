use crate::*;
use frame_support::{
    pallet_prelude::ValueQuery,
    traits::{ConstU32, Get, StorageVersion},
};

pub mod v2 {
    use dao::CuratorApplication;
    use frame_support::{traits::OnRuntimeUpgrade, weights::Weight};

    use super::*;

    pub mod old_storage {
        use super::*;
        use dao::ApplicationStatus;
        use frame_support::{pallet_prelude::TypeInfo, storage_alias, Identity};
        use pallet_subspace::AccountIdOf;
        use parity_scale_codec::{Decode, Encode};
        use sp_runtime::BoundedVec;

        #[derive(Encode, Decode, TypeInfo)]
        pub struct CuratorApplication<T: Config> {
            pub id: u64,
            pub user_id: T::AccountId,
            pub paying_for: T::AccountId,
            pub data: BoundedVec<u8, ConstU32<256>>,
            pub status: ApplicationStatus,
            pub application_cost: u64,
        }

        #[storage_alias]
        pub type CuratorApplications<T: Config> =
            StorageMap<Pallet<T>, Identity, u64, CuratorApplication<T>>;

        #[storage_alias]
        pub type LegitWhitelist<T: Config> =
            StorageMap<Pallet<T>, Identity, AccountIdOf<T>, u8, ValueQuery>;
    }

    pub struct MigrateToV2<T>(sp_std::marker::PhantomData<T>);

    impl<T: Config> OnRuntimeUpgrade for MigrateToV2<T> {
        fn on_runtime_upgrade() -> frame_support::weights::Weight {
            let on_chain_version = StorageVersion::get::<Pallet<T>>();
            if on_chain_version != 1 {
                log::info!("Storage v2 already updated");
                return Weight::zero();
            }

            StorageVersion::new(2).put::<Pallet<T>>();

            CuratorApplications::<T>::translate(
                |_key, old_value: v2::old_storage::CuratorApplication<T>| {
                    Some(CuratorApplication {
                        id: old_value.id,
                        user_id: old_value.user_id,
                        paying_for: old_value.paying_for,
                        data: old_value.data,
                        status: old_value.status,
                        application_cost: old_value.application_cost,
                        block_number: 0,
                    })
                },
            );

            let old_whitelist: Vec<_> = old_storage::LegitWhitelist::<T>::iter().collect();
            _ = old_storage::LegitWhitelist::<T>::clear(u32::MAX, None);

            for (account, _) in old_whitelist {
                LegitWhitelist::<T>::insert(account, ());
            }

            log::info!("Migrated to v2");

            T::DbWeight::get().reads_writes(2, 2)
        }
    }
}

pub mod v3 {
    use frame_support::{traits::OnRuntimeUpgrade, weights::Weight};
    use sp_runtime::BoundedVec;
    use proposal::Proposal;
    
    use super::*;

    pub mod old_storage {
        use super::*;
        use frame_support::{pallet_prelude::Identity, storage_alias, DebugNoBound, BoundedBTreeSet};
        use parity_scale_codec::{Decode, Encode, MaxEncodedLen};
        use scale_info::TypeInfo;
        use proposal::{ProposalId, ProposalData};

        #[derive(DebugNoBound, TypeInfo, Decode, Encode, MaxEncodedLen)]
        #[scale_info(skip_type_params(T))]
        pub struct Proposal<T: Config> {
            pub id: ProposalId,
            pub proposer: T::AccountId,
            pub expiration_block: u64,
            pub data: ProposalData<T>,
            pub status: ProposalStatus<T>,
            pub metadata: BoundedVec<u8, ConstU32<256>>,
            pub proposal_cost: u64,
            pub creation_block: u64,
        }

        #[derive(Clone, DebugNoBound, TypeInfo, Decode, Encode, MaxEncodedLen, PartialEq, Eq)]
        #[scale_info(skip_type_params(T))]
        pub enum ProposalStatus<T: Config> {
            Open {
                votes_for: BoundedBTreeSet<T::AccountId, ConstU32<{ u32::MAX }>>,
                votes_against: BoundedBTreeSet<T::AccountId, ConstU32<{ u32::MAX }>>,
                stake_for: u64,
                stake_against: u64,
            },
            Accepted {
                block: u64,
                stake_for: u64,
                stake_against: u64,
            },
            Refused {
                block: u64,
                stake_for: u64,
                stake_against: u64,
            },
            Expired,
        }

        #[storage_alias]
        pub type Proposals<T: Config> = StorageMap<Pallet<T>, Identity, ProposalId, Proposal<T>>;
    }

    pub struct MigrateToV3<T>(sp_std::marker::PhantomData<T>);

    impl<T: Config> OnRuntimeUpgrade for MigrateToV3<T> {
        fn on_runtime_upgrade() -> Weight {
            let on_chain_version = StorageVersion::get::<Pallet<T>>();
            log::info!("Current Storage Version is {:?}", on_chain_version);

            #[cfg(not(feature = "testnet"))]
            if on_chain_version != 2 {
                log::info!("Storage v3 is already updated or previous migration not applied");
                return Weight::zero();
            }

            #[cfg(not(feature = "testnet"))]
            StorageVersion::new(3).put::<Pallet<T>>();

            #[cfg(feature = "testnet")]
            if on_chain_version != 5 {
                log::info!("Storage v3 is already updated or previous migration not applied");
                return Weight::zero();
            }

            #[cfg(feature = "testnet")]
            StorageVersion::new(6).put::<Pallet<T>>();

            Proposals::<T>::translate(
                |_key, old: v3::old_storage::Proposal<T>| {
                    Some(Proposal {
                        id: old.id,
                        proposer: old.proposer,
                        expiration_block: old.expiration_block,
                        data: old.data,
                        status: match old.status {
                            v3::old_storage::ProposalStatus::<T>::Open {
                                votes_for,
                                votes_against,
                                stake_for,
                                stake_against,
                            } => ProposalStatus::Open {
                                votes_for,
                                votes_against,
                                stake_for,
                                stake_against,
                            },
                            v3::old_storage::ProposalStatus::<T>::Accepted {
                                block,
                                stake_for,
                                stake_against,
                            } => ProposalStatus::Accepted {
                                block,
                                stake_for,
                                stake_against,
                            },
                            v3::old_storage::ProposalStatus::<T>::Refused {
                                block,
                                stake_for,
                                stake_against,
                            } => ProposalStatus::Refused {
                                block,
                                stake_for,
                                stake_against,
                            },
                            v3::old_storage::ProposalStatus::<T>::Expired => ProposalStatus::Expired,
                        },
                        metadata: old.metadata,
                        proposal_cost: old.proposal_cost,
                        creation_block: old.creation_block,
                    })
                },
            );


            log::info!("Migrated to v3");
            T::DbWeight::get().reads_writes(1, 1)
        }
    }

}