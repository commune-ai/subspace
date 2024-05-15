//! The Governance pallet.
#![cfg_attr(not(feature = "std"), no_std)]

pub mod migrations;

use frame_support::{
    dispatch::DispatchResult,
    ensure,
    sp_runtime::{DispatchError, Percent},
    storage::with_storage_layer,
    traits::ConstU32,
    BoundedBTreeSet, BoundedVec, DebugNoBound,
};
use parity_scale_codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;

pub use pallet::*;

type SubnetId = u16;
type ProposalId = u64;
type Nanos = u64;

#[frame_support::pallet]
pub mod pallet {
    use frame_support::{
        pallet_prelude::{ValueQuery, *},
        traits::Currency,
    };
    use frame_system::pallet_prelude::BlockNumberFor;

    use crate::*;

    const STORAGE_VERSION: StorageVersion = StorageVersion::new(0);

    #[pallet::pallet]
    #[pallet::storage_version(STORAGE_VERSION)]
    pub struct Pallet<T>(_);

    #[pallet::config(with_default)]
    pub trait Config: frame_system::Config + pallet_subspace::Config {
        /// The events emitted on proposal changes.
        #[pallet::no_default_bounds]
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// Currency type that will be used to place deposits on modules
        type Currency: Currency<Self::AccountId> + Send + Sync;

        type DefaultProposalCost: Get<u64>;
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_initialize(block_number: BlockNumberFor<T>) -> Weight {
            let block_number: u64 =
                block_number.try_into().ok().expect("blockchain won't pass 2 ^ 64 blocks");

            if block_number % 100 == 0 {
                tick_proposals::<T>(block_number);
            }

            Weight::zero()
        }
    }

    /// A map of all proposals, indexed by their IDs.
    #[pallet::storage]
    pub(crate) type Proposals<T: Config> = StorageMap<_, Identity, ProposalId, Proposal<T>>;

    #[pallet::storage]
    pub(crate) type GlobalProposalCost<T: Config> =
        StorageValue<_, Nanos, ValueQuery, T::DefaultProposalCost>;

    #[pallet::storage]
    pub(crate) type SubnetsProposalCosts<T: Config> =
        StorageMap<_, Identity, SubnetId, Nanos, ValueQuery, T::DefaultProposalCost>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(crate) fn deposit_event)]
    pub enum Event<T: Config> {
        ProposalCreated(ProposalId),

        ProposalAccepted(ProposalId),
        ProposalRefused(ProposalId),
        ProposalExpired(ProposalId),
    }

    #[pallet::error]
    pub enum Error<T> {
        /// The proposal is already finished. Do not retry.
        ProposalIsFinished,
        /// Invalid parameters were provided to the finalization process.
        InvalidProposalFinalizationParameters,
    }
}

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

impl<T: Config> Proposal<T> {
    /// Whether the proposal is still active.
    #[must_use]
    pub fn is_active(&self) -> bool {
        matches!(self.status, ProposalStatus::Open { .. })
    }

    /// Returns the subnet ID that this proposal impact.s
    #[must_use]
    pub fn subnet_id(&self) -> Option<u16> {
        match &self.data {
            ProposalData::SubnetParams { subnet_id, .. }
            | ProposalData::SubnetCustom { subnet_id, .. } => Some(*subnet_id),
            _ => None,
        }
    }

    /// Marks a proposal as accepted and overrides the storage value.
    pub fn accept(mut self, block_number: u64) -> DispatchResult {
        ensure!(self.is_active(), Error::<T>::ProposalIsFinished);

        self.status = ProposalStatus::Accepted {
            block: block_number,
            stake_for: 0,
            stake_against: 0,
        };

        Proposals::<T>::insert(self.id, &self);
        Pallet::<T>::deposit_event(Event::ProposalAccepted(self.id));

        execute_proposal(self)?;

        Ok(())
    }

    /// Marks a proposal as refused and overrides the storage value.
    pub fn refuse(mut self, block_number: u64) -> DispatchResult {
        ensure!(self.is_active(), Error::<T>::ProposalIsFinished);

        self.status = ProposalStatus::Refused {
            block: block_number,
            stake_for: 0,
            stake_against: 0,
        };

        Proposals::<T>::insert(self.id, &self);
        Pallet::<T>::deposit_event(Event::ProposalRefused(self.id));

        Ok(())
    }

    /// Marks a proposal as expired and overrides the storage value.
    pub fn expire(mut self, block_number: u64) -> DispatchResult {
        ensure!(self.is_active(), Error::<T>::ProposalIsFinished);
        ensure!(
            block_number >= self.expiration_block,
            Error::<T>::InvalidProposalFinalizationParameters
        );

        self.status = ProposalStatus::Expired;

        Proposals::<T>::insert(self.id, &self);
        Pallet::<T>::deposit_event(Event::ProposalExpired(self.id));

        Ok(())
    }
}

#[derive(Clone, DebugNoBound, TypeInfo, Decode, Encode, MaxEncodedLen)]
#[scale_info(skip_type_params(T))]
pub enum ProposalStatus<T: Config> {
    Open {
        votes_for: BoundedBTreeSet<T::AccountId, ConstU32<{ u32::MAX }>>,
        votes_against: BoundedBTreeSet<T::AccountId, ConstU32<{ u32::MAX }>>,
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

#[derive(DebugNoBound, TypeInfo, Decode, Encode, MaxEncodedLen)]
#[scale_info(skip_type_params(T))]
pub enum ProposalData<T: Config> {
    GlobalCustom,
    GlobalParams(pallet_subspace::GlobalParams<T>),
    SubnetCustom {
        subnet_id: SubnetId,
    },
    SubnetParams {
        subnet_id: SubnetId,
        params: pallet_subspace::SubnetParams<T>,
    },
    TransferDaoTreasury {
        account: T::AccountId,
        amount: u64,
    },
}

impl<T: Config> ProposalData<T> {
    /// The required amount of stake each of the proposal types requires in order to pass.
    #[must_use]
    pub fn required_stake(&self) -> Percent {
        match self {
            Self::GlobalCustom | Self::SubnetCustom { .. } | Self::TransferDaoTreasury { .. } => {
                Percent::from_parts(50)
            }
            Self::GlobalParams(_) | Self::SubnetParams { .. } => Percent::from_parts(40),
        }
    }
}

fn tick_proposals<T: Config>(block_number: u64) {
    for (id, proposal) in Proposals::<T>::iter().filter(|(_, p)| p.is_active()) {
        let res = with_storage_layer(|| tick_proposal(block_number, proposal));
        if let Err(err) = res {
            log::error!("failed to tick proposal {id}: {err:?}, skipping...");
        }
    }
}

fn tick_proposal<T: Config>(block_number: u64, proposal: Proposal<T>) -> DispatchResult {
    use pallet_subspace::Pallet as SubspacePallet;

    let subnet_id = proposal.subnet_id();

    let ProposalStatus::Open {
        votes_for,
        votes_against,
    } = &proposal.status
    else {
        return Err(Error::<T>::ProposalIsFinished.into());
    };

    let votes_for: u64 = votes_for
        .iter()
        .map(|id| SubspacePallet::<T>::get_account_stake(id, subnet_id))
        .sum();
    let votes_against: u64 = votes_against
        .iter()
        .map(|id| SubspacePallet::<T>::get_account_stake(id, subnet_id))
        .sum();

    let total_stake = votes_for + votes_against;
    let minimal_stake_to_execute =
        SubspacePallet::<T>::get_minimal_stake_to_execute_with_percentage(
            proposal.data.required_stake(),
            subnet_id,
        );

    if total_stake >= minimal_stake_to_execute {
        if votes_against > votes_for {
            proposal.refuse(block_number)
        } else {
            proposal.accept(block_number)
        }
    } else if block_number >= proposal.expiration_block {
        proposal.expire(block_number)
    } else {
        Ok(())
    }
}

fn execute_proposal<T: Config>(proposal: Proposal<T>) -> DispatchResult {
    use pallet_subspace::{
        subnet::SubnetChangeset, Error as SubspaceError, Event as SubspaceEvent, GlobalDaoTreasury,
        Pallet as SubspacePallet,
    };

    match &proposal.data {
        ProposalData::GlobalCustom | ProposalData::SubnetCustom { .. } => {
            // No specific action needed for custom proposals
            // The owners will handle the off-chain logic
        }
        ProposalData::GlobalParams(params) => {
            SubspacePallet::<T>::set_global_params(params.clone());
            SubspacePallet::<T>::deposit_event(SubspaceEvent::GlobalParamsUpdated(params.clone()));
        }
        ProposalData::SubnetParams { subnet_id, params } => {
            let changeset = SubnetChangeset::<T>::update(*subnet_id, params.clone())?;
            changeset.apply(*subnet_id)?;
            SubspacePallet::<T>::deposit_event(SubspaceEvent::SubnetParamsUpdated(*subnet_id));
        }
        ProposalData::TransferDaoTreasury { account, amount } => {
            GlobalDaoTreasury::<T>::try_mutate::<(), DispatchError, _>(|treasury| {
                *treasury = treasury
                    .checked_sub(*amount)
                    .ok_or(SubspaceError::<T>::BalanceCouldNotBeRemoved)?;
                Ok(())
            })?;

            let amount = SubspacePallet::<T>::u64_to_balance(*amount)
                .ok_or(SubspaceError::<T>::CouldNotConvertToBalance)?;
            SubspacePallet::<T>::add_balance_to_account(account, amount);
        }
    }

    // Give the proposer back his tokens, if the proposal passed
    SubspacePallet::<T>::add_balance_to_account(
        &proposal.proposer,
        SubspacePallet::<T>::u64_to_balance(proposal.proposal_cost).unwrap(),
    );

    Ok(())
}
