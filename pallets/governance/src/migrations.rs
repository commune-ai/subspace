use core::marker::PhantomData;

use frame_support::{
    migrations::VersionedMigration,
    traits::{OnRuntimeUpgrade, StorageVersion, UncheckedOnRuntimeUpgrade},
    BoundedVec,
};

use crate::{
    proposal::{ProposalData, ProposalStatus},
    *,
};

use pallet_subspace::Pallet as PalletSubspace;

#[derive(Default)]
pub struct InitialMigration<T>(PhantomData<T>);

impl<T: Config + pallet_subspace::Config> OnRuntimeUpgrade for InitialMigration<T> {
    fn on_runtime_upgrade() -> frame_support::weights::Weight {
        use pallet_subspace::migrations::v11::old_storage as old;

        if StorageVersion::get::<Pallet<T>>() != 0 {
            return frame_support::weights::Weight::zero();
        }

        log::info!("Initializing governance storage, importing proposals...");

        for (id, proposal) in old::Proposals::<T>::iter() {
            let metadata = match &proposal.data {
                old::ProposalData::Custom(data)
                | old::ProposalData::SubnetCustom { data, .. }
                | old::ProposalData::TransferDaoTreasury { data, .. } => {
                    BoundedVec::truncate_from(data.clone())
                }
                _ => Default::default(),
            };

            let data = match proposal.data {
                old::ProposalData::Custom(_) => ProposalData::GlobalCustom,
                old::ProposalData::GlobalParams(params) => ProposalData::GlobalParams(params),
                old::ProposalData::SubnetParams { netuid, params } => ProposalData::SubnetParams {
                    subnet_id: netuid,
                    params,
                },
                old::ProposalData::SubnetCustom { netuid, .. } => {
                    ProposalData::SubnetCustom { subnet_id: netuid }
                }
                old::ProposalData::TransferDaoTreasury { value, dest, .. } => {
                    ProposalData::TransferDaoTreasury {
                        account: dest,
                        amount: value,
                    }
                }
                old::ProposalData::Expired => {
                    log::trace!("proposal {id} is expired, defaulting to GlobalCustom data");
                    ProposalData::GlobalCustom
                }
            };

            let proposal = Proposal {
                id,
                proposer: proposal.proposer,
                expiration_block: proposal.expiration_block,
                data,
                metadata,
                status: match proposal.status {
                    old::ProposalStatus::Pending => ProposalStatus::Open {
                        votes_for: proposal.votes_for.try_into().unwrap_or_default(),
                        votes_against: proposal.votes_against.try_into().unwrap_or_default(),
                    },
                    old::ProposalStatus::Accepted => ProposalStatus::Accepted {
                        block: proposal.finalization_block.unwrap_or_default(),
                        stake_for: 0,
                        stake_against: 0,
                    },
                    old::ProposalStatus::Refused => ProposalStatus::Refused {
                        block: proposal.finalization_block.unwrap_or_default(),
                        stake_for: 0,
                        stake_against: 0,
                    },
                    old::ProposalStatus::Expired => ProposalStatus::Expired,
                },
                proposal_cost: proposal.proposal_cost,
                creation_block: proposal.creation_block,
            };

            Proposals::<T>::set(id, Some(proposal));

            log::debug!("migrated proposal {id}");
        }
        log::info!("Imported {} proposals", Proposals::<T>::iter().count());

        // Print out the proposals
        for (id, proposal) in Proposals::<T>::iter() {
            log::info!(
                "Proposal {{
            id: {},
            proposal: {:?}
        }}",
                id,
                proposal
            );
        }

        log::info!("Importing treasury balance...");
        let treasury_account = DaoTreasuryAddress::<T>::get();
        let old_treasury_balance = old::GlobalDaoTreasury::<T>::get();

        let treasury_account_balance = PalletSubspace::<T>::get_balance_u64(&treasury_account);
        if treasury_account_balance != old_treasury_balance {
            log::info!("Treasury balance: {old_treasury_balance}");

            PalletSubspace::<T>::add_balance_to_account(
                &treasury_account,
                PalletSubspace::<T>::u64_to_balance(old_treasury_balance).unwrap_or_default(),
            );
        }

        let account_balance = PalletSubspace::<T>::get_balance_u64(&treasury_account);
        log::info!(
            "Treasury transferred to account ({treasury_account:?}), tokens: {account_balance}"
        );

        log::info!("Migrating curator...");
        let curator = old::Curator::<T>::get();
        match curator {
            Some(curator) => {
                log::info!("current curator: {curator:?}");
                Curator::<T>::set(curator);
            }
            None => {
                log::error!("no curator found");
            }
        }

        log::info!("Migrating whitelist...");
        for (id, account) in old::LegitWhitelist::<T>::iter() {
            LegitWhitelist::<T>::insert(id, account);
        }

        log::info!("LegitWhitelist:");
        for (key, value) in LegitWhitelist::<T>::iter() {
            log::info!("{key:?} -> {value}");
        }
        log::info!(" ");

        log::info!("Migrating general subnet application cost...");
        let cost = old::GeneralSubnetApplicationCost::<T>::get();
        GeneralSubnetApplicationCost::<T>::set(cost);
        log::info!(
            "GeneralSubnetApplicationCost -> {}",
            GeneralSubnetApplicationCost::<T>::get()
        );

        log::info!("Migrating curator applications...");
        for (id, application) in old::CuratorApplications::<T>::iter() {
            CuratorApplications::<T>::insert(
                id,
                dao::CuratorApplication {
                    id,
                    user_id: application.user_id,
                    paying_for: application.paying_for,
                    data: BoundedVec::truncate_from(application.data),
                    status: match application.status {
                        old::ApplicationStatus::Pending => dao::ApplicationStatus::Pending,
                        old::ApplicationStatus::Accepted => dao::ApplicationStatus::Accepted,
                        old::ApplicationStatus::Refused => dao::ApplicationStatus::Refused,
                    },
                    application_cost: application.application_cost,
                },
            );
        }

        log::info!("CuratorApplications:");
        for (key, value) in CuratorApplications::<T>::iter() {
            log::info!("  {key} -> {value:?}");
        }

        for subnet_id in pallet_subspace::N::<T>::iter_keys() {
            SubnetGovernanceConfig::<T>::set(
                subnet_id,
                GovernanceConfiguration {
                    vote_mode: old::VoteModeSubnet::<T>::get(subnet_id)
                        .unwrap_or(VoteMode::Authority),
                    proposal_cost: old::ProposalCost::<T>::get().unwrap_or(10_000_000_000_000),
                    proposal_expiration: old::ProposalExpiration::<T>::get().unwrap_or(130_000),
                    ..Default::default()
                },
            )
        }

        log::info!("Migrated subnet governance config");

        GlobalGovernanceConfig::<T>::set(GovernanceConfiguration {
            proposal_cost: old::ProposalCost::<T>::get().unwrap_or(10_000_000_000_000),
            proposal_expiration: old::ProposalExpiration::<T>::get().unwrap_or(130_000),
            ..Default::default()
        });

        log::info!("Migrated global governance config");

        frame_support::weights::Weight::zero()
    }
}

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
