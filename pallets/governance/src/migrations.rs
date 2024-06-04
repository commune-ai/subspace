use core::marker::PhantomData;

use frame_support::{
    migrations::VersionedMigration,
    traits::{OnRuntimeUpgrade, StorageVersion, UncheckedOnRuntimeUpgrade},
    BoundedVec,
};
use sp_std::collections::btree_set::BTreeSet;

use crate::{
    proposal::{ProposalData, ProposalStatus},
    *,
};

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

        let mut delegating = BTreeSet::new();
        for (_, staker, _) in pallet_subspace::StakeTo::<T>::iter() {
            delegating.insert(staker);
        }
        DelegatingVotingPower::<T>::set(delegating.try_into().unwrap_or_default());

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
