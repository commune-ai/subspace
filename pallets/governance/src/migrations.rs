use core::marker::PhantomData;

use frame_support::{
    migrations::VersionedMigration,
    traits::{OnRuntimeUpgrade, StorageVersion, UncheckedOnRuntimeUpgrade},
    BoundedVec,
};

use crate::{
    Config, GlobalProposalCost, Pallet, Proposal, ProposalData, ProposalStatus, Proposals,
};

#[derive(Default)]
pub struct InitialMigration<T>(PhantomData<T>);

impl<T: Config + pallet_subspace::Config> OnRuntimeUpgrade for InitialMigration<T> {
    fn on_runtime_upgrade() -> frame_support::weights::Weight {
        if StorageVersion::get::<Pallet<T>>() != 0 {
            return frame_support::weights::Weight::zero();
        }

        log::info!("Initializing governance storage, importing proposals...");

        let old_proposal_cost = pallet_subspace::ProposalCost::<T>::get();
        GlobalProposalCost::<T>::set(old_proposal_cost);

        for (id, proposal) in pallet_subspace::Proposals::<T>::iter() {
            let metadata = match &proposal.data {
                pallet_subspace::voting::ProposalData::Custom(data)
                | pallet_subspace::voting::ProposalData::SubnetCustom { data, .. }
                | pallet_subspace::voting::ProposalData::TransferDaoTreasury { data, .. } => {
                    BoundedVec::truncate_from(data.clone())
                }
                _ => Default::default(),
            };

            let data = match proposal.data {
                pallet_subspace::voting::ProposalData::Custom(_) => ProposalData::GlobalCustom,
                pallet_subspace::voting::ProposalData::GlobalParams(params) => {
                    ProposalData::GlobalParams(params)
                }
                pallet_subspace::voting::ProposalData::SubnetParams { netuid, params } => {
                    ProposalData::SubnetParams {
                        subnet_id: netuid,
                        params,
                    }
                }
                pallet_subspace::voting::ProposalData::SubnetCustom { netuid, .. } => {
                    ProposalData::SubnetCustom { subnet_id: netuid }
                }
                pallet_subspace::voting::ProposalData::TransferDaoTreasury {
                    value, dest, ..
                } => ProposalData::TransferDaoTreasury {
                    account: dest,
                    amount: value,
                },
                pallet_subspace::voting::ProposalData::Expired => {
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
                    pallet_subspace::voting::ProposalStatus::Pending => ProposalStatus::Open {
                        votes_for: proposal.votes_for.try_into().unwrap_or_default(),
                        votes_against: proposal.votes_against.try_into().unwrap_or_default(),
                    },
                    pallet_subspace::voting::ProposalStatus::Accepted => ProposalStatus::Accepted {
                        block: proposal.finalization_block.unwrap_or_default(),
                        stake_for: 0,
                        stake_against: 0,
                    },
                    pallet_subspace::voting::ProposalStatus::Refused => ProposalStatus::Refused {
                        block: proposal.finalization_block.unwrap_or_default(),
                        stake_for: 0,
                        stake_against: 0,
                    },
                    pallet_subspace::voting::ProposalStatus::Expired => ProposalStatus::Expired,
                },
                proposal_cost: proposal.proposal_cost,
                creation_block: proposal.creation_block,
            };

            Proposals::<T>::set(id, Some(proposal));

            log::debug!("migrated proposal {id}");
        }

        log::info!("Imported {} proposals", Proposals::<T>::iter().count());

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
