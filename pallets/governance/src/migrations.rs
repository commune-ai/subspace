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
    use parity_scale_codec::Decode;
    
    use super::*;

    pub mod old_storage {
        use super::*;
        use frame_support::{pallet_prelude::Identity, storage_alias, DebugNoBound, BoundedBTreeSet};
        use parity_scale_codec::{Encode, MaxEncodedLen};
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

            // senate_keys = [
            //     "5H47pSknyzk4NM5LyE6Z3YiRKb3JjhYbea2pAUdocb95HrQL",
            //     "5CwXN5zQFQNoFRaycsiE29ibDDp2mXwnof228y76fMbs2jHd",
            //     "5CMNEDouxNdMUEM6NE9HRYaJwCSBarwr765jeLdHvWEE15NH",
            //     "5FZsiAJS5WMzsrisfLWosyzaCEQ141rncjv55VFLHcUER99c",
            //     "5DyPNNRLbrLWgPZPVES45LfEgFKyfmPbrtJkFLiSbmWLumYj",
            //     "5DPSqGAAy5ze1JGuSJb68fFPKbDmXhfMqoNSHLFnJgUNTPaU",
            //     "5HmjuwYGRXhxxbFz6EJBXpAyPKwRsQxFKdZQeLdTtg5UEudA"
            // ]
            // Above array may not be in the same order as below array

            let senate_keys: [[u8; 32]; 7] = [
                [
                    0xdc, 0xba, 0x95, 0x80, 0x4d, 0x03, 0x39, 0x37, 0x0f, 0x8c, 0xb3, 0xfd, 0xa8, 0xa6, 0x41, 0xc4,
                    0xc5, 0x32, 0x4b, 0x89, 0xf9, 0xa1, 0x22, 0xf6, 0x61, 0x59, 0xd6, 0x88, 0xa3, 0x6e, 0xd5, 0x74,
                ],
                [
                    0x26, 0xc2, 0x31, 0x98, 0xca, 0xd2, 0xcd, 0xb4, 0x55, 0xd8, 0x2d, 0xa5, 0x37, 0x32, 0x43, 0xdd,
                    0xe9, 0xf9, 0x0f, 0x86, 0x3f, 0x88, 0xca, 0x87, 0x34, 0x32, 0x4b, 0xb6, 0x0b, 0x69, 0x65, 0x50,
                ],
                [
                    0x0c, 0xb5, 0x18, 0xc7, 0x9e, 0xe5, 0x70, 0xbc, 0x13, 0x58, 0xda, 0x87, 0xed, 0xc2, 0x4b, 0xe6,
                    0x47, 0x98, 0x90, 0x2b, 0x84, 0x4a, 0x41, 0xf7, 0xe6, 0x0e, 0xa5, 0xb3, 0x70, 0xcd, 0xa2, 0x03,
                ],
                [
                    0x9a, 0xf3, 0xfc, 0x7e, 0xc1, 0x3d, 0xa8, 0x31, 0x19, 0x71, 0x11, 0x64, 0xc3, 0xdb, 0xb5, 0x8d,
                    0x51, 0x22, 0x6a, 0x91, 0x7f, 0x3f, 0xf5, 0x55, 0x6a, 0x0c, 0x4f, 0x7e, 0x6e, 0x26, 0x4f, 0x3b,
                ],
                [
                    0x54, 0x6a, 0x01, 0x08, 0xc3, 0x49, 0xba, 0x47, 0xd2, 0x2e, 0x9a, 0x2f, 0x8c, 0xed, 0x8e, 0x7d,
                    0x15, 0xae, 0x39, 0xc4, 0x5d, 0x06, 0x7c, 0xf4, 0x00, 0xc1, 0xc4, 0x0a, 0x74, 0xda, 0xfb, 0x39,
                ],
                [
                    0x3a, 0x87, 0x51, 0xeb, 0x50, 0x37, 0x25, 0x18, 0x8c, 0xe6, 0x13, 0x67, 0x61, 0x29, 0x9f, 0x6f,
                    0x66, 0x0b, 0x2d, 0x9d, 0xb6, 0xce, 0x8f, 0xdc, 0x12, 0xba, 0xa6, 0x98, 0x2f, 0x00, 0x18, 0x59,
                ],
                [
                    0xfc, 0x79, 0x26, 0xc0, 0x58, 0xe0, 0x40, 0x60, 0x1b, 0xb6, 0x1b, 0x73, 0x78, 0xe1, 0x28, 0x96,
                    0x0e, 0xe5, 0xda, 0x85, 0x11, 0x3c, 0x7a, 0x2a, 0x3b, 0xc7, 0x6f, 0xa0, 0x2b, 0x70, 0xa4, 0x3a,
                ]
            ];
            
            for key in senate_keys {
                let bytes = Vec::from(&key[..]);
                match <T::AccountId as Decode>::decode(&mut &bytes[..]) {
                    Ok(account_id) => {
                        crate::SenateMembers::<T>::insert(account_id, ());
                    },
                    Err(_) => {
                        log::error!("Failed to decode account ID");
                    }
                };
            }
 

            log::info!("Migrated to v3");
            T::DbWeight::get().reads_writes(1, 8)
        }
    }

}