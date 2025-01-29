//! The Governance pallet.
#![allow(non_camel_case_types, non_snake_case)]
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

pub mod dao;
pub mod migrations;
pub mod proposal;
pub mod voting;
pub mod weights; // Weight benchmarks

use frame_support::{
    dispatch::DispatchResult,
    ensure,
    sp_runtime::{DispatchError, Percent},
};
use frame_system::pallet_prelude::OriginFor;
use sp_std::vec::Vec;
use substrate_fixed::types::I64F64;

pub use pallet::*;
pub use pallet_governance_api::*;
use pallet_subspace::{
    self,
    params::{burn::GeneralBurnConfiguration},
    DefaultKey,
};

pub use proposal::{Proposal, ProposalData, ProposalId, ProposalStatus, UnrewardedProposal};


#[frame_support::pallet]
pub mod pallet {
    #![allow(clippy::too_many_arguments)]

    pub use crate::weights::WeightInfo;
    use crate::{dao::CuratorApplication, *};
    use frame_support::{
        pallet_prelude::{ValueQuery, *},
        traits::{Currency, StorageInstance},
        PalletId,
    };
    use frame_system::pallet_prelude::{ensure_signed, BlockNumberFor};
    use sp_runtime::traits::AccountIdConversion;

    #[cfg(feature = "testnet")]
    const STORAGE_VERSION: StorageVersion = StorageVersion::new(4);

    #[cfg(not(feature = "testnet"))]
    const STORAGE_VERSION: StorageVersion = StorageVersion::new(2);

    #[pallet::pallet]
    #[pallet::storage_version(STORAGE_VERSION)]
    pub struct Pallet<T>(_);

    #[pallet::config(with_default)]
    pub trait Config: frame_system::Config + pallet_subspace::Config {
        /// This pallet's ID, used for generating the treasury account ID.
        #[pallet::constant]
        type PalletId: Get<PalletId>;

        /// The events emitted on proposal changes.
        #[pallet::no_default_bounds]
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// Currency type that will be used to place deposits on modules
        type Currency: Currency<Self::AccountId> + Send + Sync;

        /// The weight information of this pallet.
        type WeightInfo: WeightInfo;
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_initialize(block_number: BlockNumberFor<T>) -> Weight {
            let block_number: u64 =
                block_number.try_into().ok().expect("blockchain won't pass 2 ^ 64 blocks");

            proposal::tick_proposals::<T>(block_number);
            proposal::tick_proposal_rewards::<T>(block_number);

            Weight::zero()
        }
    }

    impl<T: Config> StorageInstance for Pallet<T> {
        const STORAGE_PREFIX: &'static str = "Governance";

        fn pallet_prefix() -> &'static str {
            "Governance"
        }
    }

    /// A map of all proposals, indexed by their IDs.
    #[pallet::storage]
    pub type Proposals<T: Config> = StorageMap<_, Identity, ProposalId, Proposal<T>>;

    #[pallet::storage]
    pub type UnrewardedProposals<T: Config> =
        StorageMap<_, Identity, ProposalId, UnrewardedProposal<T>>;

    // --- Storage Items ---

    /// A map relating all modules and the stakers that are currently **NOT** delegating their
    /// voting power.
    ///
    /// Indexed by the **staked** module and the subnet the stake is allocated to, the value is a
    /// set of all modules that are delegating their voting power on that subnet.
    #[pallet::storage]
    pub type NotDelegatingVotingPower<T: Config> =
        StorageValue<_, BoundedBTreeSet<T::AccountId, ConstU32<{ u32::MAX }>>, ValueQuery>;

    #[pallet::storage]
    pub type GlobalGovernanceConfig<T: Config> =
        StorageValue<_, GovernanceConfiguration, ValueQuery>;

    #[pallet::type_value] // This has to be different than DefaultKey, so we are not conflicting in tests.
    pub fn DefaultDaoTreasuryAddress<T: Config>() -> T::AccountId {
        <T as Config>::PalletId::get().into_account_truncating()
    }

    #[pallet::storage]
    pub type DaoTreasuryAddress<T: Config> =
        StorageValue<_, T::AccountId, ValueQuery, DefaultDaoTreasuryAddress<T>>;


    /// Determines whether smart contract can be deployed by everyone or only by the curator
    #[pallet::storage]
    pub type RestrictContractDeploy<T: Config> = StorageValue<_, bool, ValueQuery>;

    // --- Curator Related Storage ---

    #[pallet::storage]
    pub type CuratorApplications<T: Config> = StorageMap<_, Identity, u64, CuratorApplication<T>>;

    #[pallet::storage]
    pub type LegitWhitelist<T: Config> = StorageMap<_, Identity, T::AccountId, (), ValueQuery>;

    #[pallet::storage]
    pub type Curator<T: Config> = StorageValue<_, T::AccountId, ValueQuery, DefaultKey<T>>;

    // --- Extrinsics ---

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        // ---  Adding Proposals ---

        #[pallet::call_index(0)]
        #[pallet::weight((<T as pallet::Config>::WeightInfo::add_global_params_proposal(), DispatchClass::Normal, Pays::No))]
        pub fn add_global_params_proposal(
            origin: OriginFor<T>,
            data: Vec<u8>,
            max_name_length: u16,
            min_name_length: u16,
            max_allowed_modules: u16,
            max_registrations_per_block: u16,
            floor_stake_delegation_fee: Percent,
            floor_validator_weight_fee: Percent,
            floor_founder_share: u8,
            min_weight_stake: u64,
            curator: T::AccountId,
            proposal_cost: u64,
            proposal_expiration: u32,
            general_subnet_application_cost: u64,
            subnet_immunity_period: u64,
        ) -> DispatchResult {
            let mut params = pallet_subspace::Pallet::<T>::global_params();
            params.max_name_length = max_name_length;
            params.min_name_length = min_name_length;
            params.max_allowed_modules = max_allowed_modules;
            params.max_registrations_per_block = max_registrations_per_block;
            params.floor_stake_delegation_fee = floor_stake_delegation_fee;
            params.floor_validator_weight_fee = floor_validator_weight_fee;
            params.floor_founder_share = floor_founder_share;
            params.min_weight_stake = min_weight_stake;
            params.curator = curator;
            params.governance_config.proposal_cost = proposal_cost;
            params.governance_config.proposal_expiration = proposal_expiration;
            params.general_subnet_application_cost = general_subnet_application_cost;
            params.subnet_immunity_period = subnet_immunity_period;
            Self::do_add_global_params_proposal(origin, data, params)
        }

        #[pallet::call_index(2)]
        #[pallet::weight((<T as pallet::Config>::WeightInfo::add_global_custom_proposal(), DispatchClass::Normal, Pays::No))]
        pub fn add_global_custom_proposal(origin: OriginFor<T>, data: Vec<u8>) -> DispatchResult {
            Self::do_add_global_custom_proposal(origin, data)
        }


        #[pallet::call_index(4)]
        #[pallet::weight((<T as pallet::Config>::WeightInfo::add_transfer_dao_treasury_proposal(), DispatchClass::Normal, Pays::No))]
        pub fn add_transfer_dao_treasury_proposal(
            origin: OriginFor<T>,
            data: Vec<u8>,
            value: u64,
            dest: T::AccountId,
        ) -> DispatchResult {
            Self::do_add_transfer_dao_treasury_proposal(origin, data, value, dest)
        }

        // --- Voting / Unvoting proposals ---

        // This has to pay fee, so very low stake keys don't spam the voting system.
        #[pallet::call_index(5)]
        #[pallet::weight((<T as pallet::Config>::WeightInfo::vote_proposal(), DispatchClass::Normal, Pays::Yes))]
        pub fn vote_proposal(
            origin: OriginFor<T>,
            proposal_id: u64,
            agree: bool,
        ) -> DispatchResult {
            Self::do_vote_proposal(origin, proposal_id, agree)
        }

        #[pallet::call_index(6)]
        #[pallet::weight((<T as pallet::Config>::WeightInfo::remove_vote_proposal(), DispatchClass::Normal, Pays::No))]
        pub fn remove_vote_proposal(origin: OriginFor<T>, proposal_id: u64) -> DispatchResult {
            Self::do_remove_vote_proposal(origin, proposal_id)
        }

        #[pallet::call_index(7)]
        #[pallet::weight((<T as pallet::Config>::WeightInfo::enable_vote_power_delegation(), DispatchClass::Normal, Pays::No))]
        pub fn enable_vote_power_delegation(origin: OriginFor<T>) -> DispatchResult {
            let key = ensure_signed(origin)?;
            Self::update_delegating_voting_power(&key, true)
        }

        #[pallet::call_index(8)]
        #[pallet::weight((<T as pallet::Config>::WeightInfo::disable_vote_power_delegation(), DispatchClass::Normal, Pays::No))]
        pub fn disable_vote_power_delegation(origin: OriginFor<T>) -> DispatchResult {
            let key = ensure_signed(origin)?;
            Self::update_delegating_voting_power(&key, false)
        }


        #[pallet::call_index(9)]
        #[pallet::weight((<T as pallet::Config>::WeightInfo::add_dao_application(), DispatchClass::Normal, Pays::No))]
        pub fn add_dao_application(
            origin: OriginFor<T>,
            application_key: T::AccountId,
            data: Vec<u8>,
        ) -> DispatchResult {
            Self::do_add_dao_application(origin, application_key, data)
        }

        #[pallet::call_index(10)]
        #[pallet::weight((<T as pallet::Config>::WeightInfo::refuse_dao_application(), DispatchClass::Normal, Pays::No))]
        pub fn refuse_dao_application(origin: OriginFor<T>, id: u64) -> DispatchResult {
            Self::do_refuse_dao_application(origin, id)
        }

        #[pallet::call_index(11)]
        #[pallet::weight((<T as pallet::Config>::WeightInfo::add_to_whitelist(), DispatchClass::Normal, Pays::No))]
        pub fn add_to_whitelist(origin: OriginFor<T>, module_key: T::AccountId) -> DispatchResult {
            Self::do_add_to_whitelist(origin, module_key)
        }

        #[pallet::call_index(12)]
        #[pallet::weight((<T as pallet::Config>::WeightInfo::remove_from_whitelist(), DispatchClass::Normal, Pays::No))]
        pub fn remove_from_whitelist(
            origin: OriginFor<T>,
            module_key: T::AccountId,
        ) -> DispatchResult {
            Self::do_remove_from_whitelist(origin, module_key)
        }
    }

    // --- Events ---

    #[pallet::event]
    #[pallet::generate_deposit(pub(crate) fn deposit_event)]
    pub enum Event<T: Config> {
        /// A new proposal has been created.
        ProposalCreated(ProposalId),
        /// A proposal has been accepted.
        ProposalAccepted(ProposalId),
        /// A proposal has been refused.
        ProposalRefused(ProposalId),
        /// A proposal has expired.
        ProposalExpired(ProposalId),
        /// A vote has been cast on a proposal.
        ProposalVoted(u64, T::AccountId, bool),
        /// A vote has been unregistered from a proposal.
        ProposalVoteUnregistered(u64, T::AccountId),
        /// A module account has been added to the whitelist.
        WhitelistModuleAdded(T::AccountId),
        /// A module account has been removed from the whitelist.
        WhitelistModuleRemoved(T::AccountId),
        /// A new application has been created.
        ApplicationCreated(u64),
    }

    // ---  Errors ---

    #[pallet::error]
    pub enum Error<T> {
        /// The proposal is already finished. Do not retry.
        ProposalIsFinished,
        /// Invalid parameters were provided to the finalization process.
        InvalidProposalFinalizationParameters,
        /// Invalid parameters were provided to the voting process.
        InvalidProposalVotingParameters,
        /// Negative proposal cost when setting global or subnet governance configuration.
        InvalidProposalCost,
        /// Negative expiration when setting global or subnet governance configuration.
        InvalidProposalExpiration,
        /// Key doesn't have enough tokens to create a proposal.
        NotEnoughBalanceToPropose,
        /// Proposal data is empty.
        ProposalDataTooSmall,
        /// Proposal data is bigger than 256 characters.
        ProposalDataTooLarge,
        /// The staked module is already delegating for 2 ^ 32 keys.
        ModuleDelegatingForMaxStakers,
        /// Proposal with given id doesn't exist.
        ProposalNotFound,
        /// Proposal was either accepted, refused or expired and cannot accept votes.
        ProposalClosed,
        /// Proposal data isn't composed by valid UTF-8 characters.
        InvalidProposalData,
        /// Invalid value given when transforming a u64 into T::Currency.
        InvalidCurrencyConversionValue,
        /// Dao Treasury doesn't have enough funds to be transferred.
        InsufficientDaoTreasuryFunds,
        /// Subnet is on Authority Mode.
        NotVoteMode,
        /// Key has already voted on given Proposal.
        AlreadyVoted,
        /// Key hasn't voted on given Proposal.
        NotVoted,
        /// Key doesn't have enough stake to vote.
        InsufficientStake,
        /// The voter is delegating its voting power to their staked modules. Disable voting power
        /// delegation.
        VoterIsDelegatingVotingPower,
        /// The network vote mode must be authority for changes to be imposed.
        VoteModeIsNotAuthority,
        /// An internal error occurred, probably relating to the size of the bounded sets.
        InternalError,
        /// The application data is too small or empty.
        ApplicationTooSmall,
        /// The application data is too large, exceeding the maximum allowed size.
        InvalidApplicationSize,
        /// The application is not in a pending state.
        ApplicationNotPending,
        /// The application key is already used in another application.
        ApplicationKeyAlreadyUsed,
        /// The application data is invalid or malformed.
        InvalidApplication,
        /// The account doesn't have enough balance to submit an application.
        NotEnoughBalanceToApply,
        /// The operation can only be performed by the curator.
        NotCurator,
        /// The application with the given ID was not found.
        ApplicationNotFound,
        /// The account is already whitelisted and cannot be added again.
        AlreadyWhitelisted,
        /// The account is not whitelisted and cannot be removed from the whitelist.
        NotWhitelisted,
        /// Failed to convert the given value to a balance.
        CouldNotConvertToBalance,
    }
}

// --- Pallet Implementation ---

impl<T: Config> Pallet<T> {
    pub fn validate(
        config: GovernanceConfiguration,
    ) -> Result<GovernanceConfiguration, DispatchError> {
        ensure!(config.proposal_cost > 0, Error::<T>::InvalidProposalCost);
        ensure!(
            config.proposal_expiration > 0,
            Error::<T>::InvalidProposalExpiration
        );
        Ok(config)
    }
}

impl<T: Config> Pallet<T> {
    pub fn is_delegating_voting_power(delegator: &T::AccountId) -> bool {
        !NotDelegatingVotingPower::<T>::get().contains(delegator)
    }

    pub fn update_delegating_voting_power(
        delegator: &T::AccountId,
        delegating: bool,
    ) -> DispatchResult {
        NotDelegatingVotingPower::<T>::mutate(|delegators| {
            if !delegating {
                delegators
                    .try_insert(delegator.clone())
                    .map(|_| ())
                    .map_err(|_| Error::<T>::InternalError.into())
            } else {
                delegators.remove(delegator);
                Ok(())
            }
        })
    }

    pub fn update_global_governance_configuration(
        config: GovernanceConfiguration,
    ) -> DispatchResult {
        let config = Self::validate(config)?;
        GlobalGovernanceConfig::<T>::set(config);
        Ok(())
    }

}
