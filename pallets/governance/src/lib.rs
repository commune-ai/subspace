//! The Governance pallet.
#![cfg_attr(not(feature = "std"), no_std)]

pub mod migrations;
pub mod proposal;
pub mod voting;

use core::marker::PhantomData;

use frame_support::{
    ensure,
    sp_runtime::{DispatchError, Percent},
};
use frame_system::pallet_prelude::OriginFor;
use pallet_subspace::voting::VoteMode;
use parity_scale_codec::{Decode, Encode, MaxEncodedLen};
use proposal::{Proposal, ProposalId};
use scale_info::TypeInfo;
use sp_std::vec::Vec;

pub use pallet::*;

type SubnetId = u16;
type Nanos = u64;

#[frame_support::pallet]
pub mod pallet {
    #![allow(clippy::too_many_arguments)]

    use frame_support::{
        pallet_prelude::{ValueQuery, *},
        traits::{Currency, StorageInstance},
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
                proposal::tick_proposals::<T>(block_number);
            }

            Weight::zero()
        }
    }

    impl<T: Config> StorageInstance for Pallet<T> {
        const STORAGE_PREFIX: &'static str = "Governance";

        fn pallet_prefix() -> &'static str {
            "Governance"
        }
    }

    #[pallet::storage]
    pub type GovernanceConfig<T: Config> = StorageValue<_, GovernanceConfiguration<T>, ValueQuery>;

    #[pallet::storage]
    pub type SubnetGovernanceConfig<T: Config> =
        StorageMap<_, Identity, SubnetId, GovernanceConfiguration<T>, ValueQuery>;

    /// A map of all proposals, indexed by their IDs.
    #[pallet::storage]
    pub type Proposals<T: Config> = StorageMap<_, Identity, ProposalId, Proposal<T>>;

    /// A map relating all modules and the stakers that are currently delegating their voting power.
    ///
    /// Indexed by the **staked** module and the subnet the stake is allocated to, the value is a
    /// set of all modules that are delegating their voting power on that subnet.
    #[pallet::storage]
    pub(crate) type DelegatedVotingPower<T: Config> = StorageDoubleMap<
        _,
        Identity,
        T::AccountId,
        Identity,
        SubnetId,
        BoundedBTreeSet<T::AccountId, ConstU32<{ u32::MAX }>>,
        ValueQuery,
    >;

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::call_index(0)]
        #[pallet::weight((Weight::zero(), DispatchClass::Normal, Pays::No))]
        pub fn add_global_proposal(
            origin: OriginFor<T>,
            data: Vec<u8>,
            max_name_length: u16,
            min_name_length: u16,
            max_allowed_subnets: u16,
            max_allowed_modules: u16,
            max_registrations_per_block: u16,
            max_allowed_weights: u16,
            max_burn: u64,
            min_burn: u64,
            floor_delegation_fee: Percent,
            floor_founder_share: u8,
            min_weight_stake: u64,
            target_registrations_per_interval: u16,
            target_registrations_interval: u16,
            adjustment_alpha: u64,
            curator: T::AccountId,
            subnet_stake_threshold: Percent,
            proposal_cost: u64,
            proposal_expiration: u32,
            proposal_participation_threshold: Percent,
            general_subnet_application_cost: u64,
        ) -> DispatchResult {
            let mut params = pallet_subspace::Pallet::<T>::global_params();
            params.max_name_length = max_name_length;
            params.min_name_length = min_name_length;
            params.max_allowed_subnets = max_allowed_subnets;
            params.max_allowed_modules = max_allowed_modules;
            params.max_registrations_per_block = max_registrations_per_block;
            params.max_allowed_weights = max_allowed_weights;
            params.floor_delegation_fee = floor_delegation_fee;
            params.floor_founder_share = floor_founder_share;
            params.min_weight_stake = min_weight_stake;
            params.curator = curator;
            params.subnet_stake_threshold = subnet_stake_threshold;
            params.proposal_cost = proposal_cost;
            params.proposal_expiration = proposal_expiration;
            params.proposal_participation_threshold = proposal_participation_threshold;
            params.general_subnet_application_cost = general_subnet_application_cost;

            params.burn_config.min_burn = min_burn;
            params.burn_config.max_burn = max_burn;
            params.burn_config.adjustment_alpha = adjustment_alpha;
            params.burn_config.adjustment_interval = target_registrations_interval;
            params.burn_config.expected_registrations = target_registrations_per_interval;

            Self::do_add_global_proposal(origin, data, params)
        }

        #[pallet::call_index(1)]
        #[pallet::weight((Weight::zero(), DispatchClass::Normal, Pays::No))]
        pub fn add_subnet_proposal(
            origin: OriginFor<T>,
            subnet_id: u16,
            data: Vec<u8>,
            founder: T::AccountId,
            name: BoundedVec<u8, ConstU32<256>>,
            founder_share: u16,
            immunity_period: u16,
            incentive_ratio: u16,
            max_allowed_uids: u16,
            max_allowed_weights: u16,
            min_allowed_weights: u16,
            min_stake: u64,
            max_weight_age: u64,
            tempo: u16,
            trust_ratio: u16,
            maximum_set_weight_calls_per_epoch: u16,
            vote_mode: VoteMode,
            bonds_ma: u64,
        ) -> DispatchResult {
            let mut params = pallet_subspace::Pallet::subnet_params(subnet_id);
            params.founder = founder;
            params.name = name;
            params.founder_share = founder_share;
            params.immunity_period = immunity_period;
            params.incentive_ratio = incentive_ratio;
            params.max_allowed_uids = max_allowed_uids;
            params.max_allowed_weights = max_allowed_weights;
            params.min_allowed_weights = min_allowed_weights;
            params.min_stake = min_stake;
            params.max_weight_age = max_weight_age;
            params.tempo = tempo;
            params.trust_ratio = trust_ratio;
            params.maximum_set_weight_calls_per_epoch = maximum_set_weight_calls_per_epoch;
            params.vote_mode = vote_mode;
            params.bonds_ma = bonds_ma;
            Self::do_add_subnet_proposal(origin, subnet_id, data, params)
        }

        #[pallet::call_index(2)]
        #[pallet::weight((Weight::zero(), DispatchClass::Normal, Pays::No))]
        pub fn add_custom_proposal(origin: OriginFor<T>, data: Vec<u8>) -> DispatchResult {
            Self::do_add_custom_proposal(origin, data)
        }

        #[pallet::call_index(3)]
        #[pallet::weight((Weight::zero(), DispatchClass::Normal, Pays::No))]
        pub fn add_custom_subnet_proposal(
            origin: OriginFor<T>,
            netuid: u16,
            data: Vec<u8>,
        ) -> DispatchResult {
            Self::do_add_custom_subnet_proposal(origin, netuid, data)
        }

        #[pallet::call_index(4)]
        #[pallet::weight((Weight::zero(), DispatchClass::Normal, Pays::No))]
        pub fn add_transfer_dao_treasury_proposal(
            origin: OriginFor<T>,
            data: Vec<u8>,
            value: u64,
            dest: T::AccountId,
        ) -> DispatchResult {
            Self::do_add_transfer_dao_treasury_proposal(origin, data, value, dest)
        }

        #[pallet::call_index(5)]
        #[pallet::weight((Weight::zero(), DispatchClass::Normal, Pays::No))]
        pub fn vote_proposal(
            origin: OriginFor<T>,
            proposal_id: u64,
            agree: bool,
        ) -> DispatchResult {
            Self::do_vote_proposal(origin, proposal_id, agree)
        }

        #[pallet::call_index(6)]
        #[pallet::weight((Weight::zero(), DispatchClass::Normal, Pays::No))]
        pub fn remove_vote_proposal(origin: OriginFor<T>, proposal_id: u64) -> DispatchResult {
            Self::do_remove_vote_proposal(origin, proposal_id)
        }
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(crate) fn deposit_event)]
    pub enum Event<T: Config> {
        ProposalCreated(ProposalId),

        ProposalAccepted(ProposalId),
        ProposalRefused(ProposalId),
        ProposalExpired(ProposalId),

        ProposalVoted(u64, T::AccountId, bool),

        ProposalVoteUnregistered(u64, T::AccountId),
    }

    #[pallet::error]
    pub enum Error<T> {
        /// The proposal is already finished. Do not retry.
        ProposalIsFinished,
        /// Invalid parameters were provided to the finalization process.
        InvalidProposalFinalizationParameters,
        /// Invalid parameters were provided to the voting process.
        InvalidProposalVotingParameters,
        /// Negative proposal cost when setting global or subnet governance configuration
        InvalidProposalCost,
        /// Negative expiration when setting global or subnet governance configuration
        InvalidExpiration,
        /// Key doesn't have enough tokens to create a proposal
        NotEnoughBalanceToPropose,
        /// Proposal data is empty
        ProposalDataTooSmall,
        /// Proposal data is bigger than 256 characters
        ProposalDataTooLarge,
        /// The staked module is already delegating for 2 ^ 32 keys.
        ModuleDelegatingForMaxStakers,
        /// Proposal with given id doesn't exist
        ProposalNotFound,
        /// Proposal was either accepted, refused or expired and cannot accept votes
        ProposalClosed,
        /// Proposal data isn't composed by valid UTF-8 characters
        InvalidProposalData,
        /// Invalid value given when transforming a u64 into T::Currency
        InvalidCurrencyConversionValue,
        /// Dao Treasury doesn't have enough funds to be transferred
        InsufficientDaoTreasuryFunds,
        /// Subnet is on Authority Mode
        NotVoteMode,
        /// Key has already voted on given Proposal
        AlreadyVoted,
        /// Key hasn't voted on given Proposal
        NotVoted,
        /// Key doesn't have enough stake to vote
        InsufficientStake,
    }
}

#[derive(
    Clone, TypeInfo, Decode, Encode, PartialEq, Eq, frame_support::DebugNoBound, MaxEncodedLen,
)]
#[scale_info(skip_type_params(T))]
pub struct GovernanceConfiguration<T: Config> {
    pub proposal_cost: Nanos,
    pub expiration: u32,
    pub vote_mode: VoteMode,
    pub _pd: PhantomData<T>,
}

impl<T: Config> Default for GovernanceConfiguration<T> {
    fn default() -> Self {
        Self {
            proposal_cost: 10_000_000_000_000,
            expiration: 130000,
            vote_mode: VoteMode::Vote,
            _pd: PhantomData,
        }
    }
}

impl<T: Config> GovernanceConfiguration<T> {
    pub fn apply_global(self) -> Result<(), DispatchError> {
        ensure!(self.proposal_cost > 0, Error::<T>::InvalidProposalCost);
        ensure!(self.expiration > 0, Error::<T>::InvalidProposalCost);

        GovernanceConfig::<T>::set(self);
        Ok(())
    }

    pub fn apply_subnet(self, subnet_id: SubnetId) -> Result<(), DispatchError> {
        SubnetGovernanceConfig::<T>::set(subnet_id, self);
        Ok(())
    }
}

impl<T: Config> Pallet<T> {
    pub fn set_delegated_voting_power(
        subnet_id: u16,
        staked: T::AccountId,
        staker: T::AccountId,
    ) -> Result<(), DispatchError> {
        DelegatedVotingPower::<T>::mutate(staked, subnet_id, |stakers| stakers.try_insert(staker))
            .map_err(|_| Error::<T>::ModuleDelegatingForMaxStakers.into())
            .map(|_| ())
    }

    pub fn remove_delegated_voting_power(
        subnet_id: u16,
        staked: T::AccountId,
        staker: T::AccountId,
    ) {
        DelegatedVotingPower::<T>::mutate(staked, subnet_id, |stakers| stakers.remove(&staker));
    }

    pub fn deregister_delegated_voting_power_on_module(subnet_id: u16, staked: T::AccountId) {
        DelegatedVotingPower::<T>::remove(staked, subnet_id);
    }

    pub fn deregister_delegated_voting_power_on_subnet(subnet_id: u16) {
        let subnet_modules: sp_std::vec::Vec<_> = DelegatedVotingPower::<T>::iter_keys()
            .filter(|(_, subnet)| *subnet == subnet_id)
            .collect();
        subnet_modules.iter().for_each(|(a, b)| DelegatedVotingPower::<T>::remove(a, b));
    }
}
