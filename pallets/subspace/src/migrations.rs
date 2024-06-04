use super::*;

use frame_support::{
    traits::{Get, OnRuntimeUpgrade, StorageInstance, StorageVersion},
    weights::Weight,
};

impl<T: Config> StorageInstance for Pallet<T> {
    fn pallet_prefix() -> &'static str {
        "Subspace"
    }

    const STORAGE_PREFIX: &'static str = "Subspace";
}

use sp_core::crypto::Ss58Codec;
use sp_runtime::AccountId32;

pub fn ss58_to_account_id<T: Config>(
    ss58_address: &str,
) -> Result<T::AccountId, sp_core::crypto::PublicError> {
    let account_id = AccountId32::from_ss58check(ss58_address)?;
    let account_id_vec = account_id.encode();
    Ok(T::AccountId::decode(&mut &account_id_vec[..]).unwrap())
}

pub mod v11 {
    use self::{
        global::BurnConfiguration,
        old_storage::{GlobalDaoTreasury, MaxBurn, MinBurn},
    };
    use super::*;

    pub mod old_storage {
        use super::*;
        use frame_support::{pallet_prelude::ValueQuery, storage_alias, Identity};
        use pallet_governance_api::VoteMode;

        #[storage_alias]
        pub type MinBurn<T: Config> = StorageValue<Pallet<T>, u64, ValueQuery>;

        #[storage_alias]
        pub type MaxBurn<T: Config> = StorageValue<Pallet<T>, u64, ValueQuery>;

        #[storage_alias]
        pub type AdjustmentAlpha<T: Config> = StorageValue<Pallet<T>, u64, ValueQuery>;

        #[storage_alias]
        pub type GlobalDaoTreasury<T: Config> = StorageValue<Pallet<T>, u64, ValueQuery>;

        #[storage_alias]
        pub type Proposals<T: Config> = StorageMap<Pallet<T>, Identity, u64, Proposal<T>>;

        #[derive(Clone, Debug, TypeInfo, Decode, Encode)]
        #[scale_info(skip_type_params(T))]
        pub struct Proposal<T: Config> {
            pub id: u64,
            pub proposer: T::AccountId,
            pub expiration_block: u64,
            pub data: ProposalData<T>,
            pub status: ProposalStatus,
            pub votes_for: BTreeSet<T::AccountId>, // account addresses
            pub votes_against: BTreeSet<T::AccountId>, // account addresses
            pub proposal_cost: u64,
            pub creation_block: u64,
            pub finalization_block: Option<u64>,
        }

        #[derive(Clone, Debug, PartialEq, Eq, TypeInfo, Decode, Encode)]
        #[scale_info(skip_type_params(T))]
        pub enum ProposalData<T: Config> {
            Custom(Vec<u8>),
            GlobalParams(GlobalParams<T>),
            SubnetParams {
                netuid: u16,
                params: SubnetParams<T>,
            },
            SubnetCustom {
                netuid: u16,
                data: Vec<u8>,
            },
            Expired,
            TransferDaoTreasury {
                data: Vec<u8>,
                value: u64,
                dest: T::AccountId,
            },
        }

        #[derive(Clone, Debug, Default, PartialEq, Eq, TypeInfo, Decode, Encode)]
        pub enum ProposalStatus {
            #[default]
            Pending,
            Accepted,
            Refused,
            Expired,
        }

        #[storage_alias]
        pub type VoteModeSubnet<T: Config> = StorageMap<Pallet<T>, Identity, u16, VoteMode>;

        #[storage_alias]
        pub type ProposalCost<T: Config> = StorageValue<Pallet<T>, u64>;

        #[storage_alias]
        pub type ProposalExpiration<T: Config> = StorageValue<Pallet<T>, u32>;
    }

    pub struct MigrateToV11<T>(sp_std::marker::PhantomData<T>);

    impl<T: Config> OnRuntimeUpgrade for MigrateToV11<T> {
        fn on_runtime_upgrade() -> Weight {
            let on_chain_version = StorageVersion::get::<Pallet<T>>();

            if on_chain_version != 10 {
                log::info!("Storage v11 already updated");
                return Weight::zero();
            }

            let current_adjustment_alpha = old_storage::AdjustmentAlpha::<T>::get();
            // Nuke the old adjustement alpha storage
            for netuid in N::<T>::iter_keys() {
                AdjustmentAlpha::<T>::insert(netuid, current_adjustment_alpha);
            }
            old_storage::AdjustmentAlpha::<T>::kill();
            log::info!("Migrating adjustment alpha to v11");

            let burn_config = BurnConfiguration::<T> {
                min_burn: MinBurn::<T>::get(),
                max_burn: MaxBurn::<T>::get(),
                _pd: PhantomData,
            };

            if let Err(err) = burn_config.apply() {
                log::error!("error migrating burn configurations: {err:?}")
            } else {
                log::info!("Migrated burn-related params to BurnConfig in v11");
            }

            let old_treasury_balance = GlobalDaoTreasury::<T>::get();
            let treasury_account = DaoTreasuryAddress::<T>::get();
            log::info!("Treasury balance: {old_treasury_balance}");
            Pallet::<T>::add_balance_to_account(
                &treasury_account,
                Pallet::<T>::u64_to_balance(old_treasury_balance).unwrap_or_default(),
            );
            GlobalDaoTreasury::<T>::set(0);

            let account_balance = Pallet::<T>::get_balance_u64(&treasury_account);
            log::info!("Treasury transferred, treasury account now has {account_balance}");
            log::info!("Treasury account: {treasury_account:?}");

            /*
                        Subnet floor founder share raise
            Initially the DAO agreed to set the floor founder share to 8% because only one subnet had been launched, which is prepared to be ready right after the incentives v1 update. For fairness, the fee was set low.

            Now more and more subnets are starting to operate and gain traction, and its time to raise it to an appropriate level of 16%.

            The subnet 0 founder share has to be raised proportionally to 20% to maintain intended effects.
                         */

            let new_founder_share: u16 = 16;
            let new_founder_share_general_subnet: u16 = 20;
            let general_subnet_netuid: u16 = 0;

            FounderShare::<T>::iter().for_each(|(netuid, share)| {
                if netuid == general_subnet_netuid {
                    FounderShare::<T>::insert(netuid, new_founder_share_general_subnet);
                    log::info!("Migrated general subnet founder share to v11");
                } else if share < new_founder_share {
                    FounderShare::<T>::insert(netuid, new_founder_share);
                }
            });

            let founder_shares: Vec<_> =
                FounderShare::<T>::iter().map(|(_, share)| share).collect();

            FloorFounderShare::<T>::put(new_founder_share as u8);

            log::info!(
                "Migrated founder share to v11, it now looks like {:?}",
                founder_shares
            );

            StorageVersion::new(11).put::<Pallet<T>>();
            T::DbWeight::get().writes(1)
        }
    }
}
