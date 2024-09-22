#![allow(non_snake_case)]
#![cfg_attr(not(feature = "std"), no_std)]

use core::marker::PhantomData;
use frame_system::pallet_prelude::OriginFor;
pub use pallet::*;
use parity_scale_codec::{Decode, Encode};
use scale_info::TypeInfo;
use sp_std::{collections::btree_map::BTreeMap, vec::Vec};
// ! Pallet that handles the emisson distribution amongs subnets

// Pallet Imports
// ==============

pub mod decryption;
pub mod distribute_emission;
pub mod migrations;
pub mod subnet_pricing {
    pub mod demo;
    pub mod root;
}

pub mod set_weights;
pub mod subnet_consensus;

pub type PublicKey = (Vec<u8>, Vec<u8>);

// TODO:
// move some import outside of the macro
#[frame_support::pallet]
pub mod pallet {
    use crate::{
        decryption::{DecryptionNodeInfo, SubnetDecryptionInfo},
        subnet_consensus::util::params::ConsensusParams,
        *,
    };
    use frame_support::{
        pallet_prelude::{ValueQuery, *},
        sp_runtime::SaturatedConversion,
        storage::with_storage_layer,
        traits::{ConstU64, Currency},
        // Identity,
    };
    use frame_system::pallet_prelude::BlockNumberFor;
    use pallet_subnet_emission_api::SubnetConsensus;
    use pallet_subspace::TotalStake;
    use subnet_pricing::root::RootPricing;

    const STORAGE_VERSION: StorageVersion = StorageVersion::new(0);

    #[pallet::pallet]
    #[pallet::storage_version(STORAGE_VERSION)]
    #[pallet::without_storage_info]
    pub struct Pallet<T>(_);

    #[pallet::config(with_default)]
    pub trait Config:
        frame_system::Config
        + pallet_subspace::Config
        + pallet_governance_api::GovernanceApi<<Self as frame_system::Config>::AccountId>
    {
        /// The events emitted on proposal changes.
        #[pallet::no_default_bounds]
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// Currency type that will be used to place deposits on modules
        type Currency: Currency<Self::AccountId> + Send + Sync;

        // Commune uses 9 token decimals.
        #[pallet::constant]
        type Decimals: Get<u8>;

        #[pallet::constant]
        type HalvingInterval: Get<u64>;

        /// The maximum token supply.
        #[pallet::constant]
        type MaxSupply: Get<u64>;
    }

    // Storage
    // ==========

    #[pallet::storage]
    pub type UnitEmission<T> = StorageValue<_, u64, ValueQuery, ConstU64<23148148148>>;

    #[pallet::storage]
    pub type PendingEmission<T> = StorageMap<_, Identity, u16, u64, ValueQuery>;

    #[pallet::storage]
    pub type SubnetEmission<T> = StorageMap<_, Identity, u16, u64, ValueQuery>;

    #[pallet::storage]
    pub type SubnetConsensusType<T> = StorageMap<_, Identity, u16, SubnetConsensus>;

    /// Netuid, to block number to consensus parameters
    #[pallet::storage]
    pub type ConsensusParameters<T> =
        StorageDoubleMap<_, Identity, u16, Identity, u64, ConsensusParams<T>, OptionQuery>;

    #[pallet::storage]
    pub type Weights<T> = StorageDoubleMap<_, Identity, u16, Identity, u16, Vec<(u16, u16)>>;

    #[pallet::storage]
    pub type EncryptedWeights<T> = StorageDoubleMap<_, Identity, u16, Identity, u16, Vec<u8>>;

    #[pallet::storage]
    pub type EncryptedWeightHashes<T> = StorageDoubleMap<_, Identity, u16, Identity, u16, Vec<u8>>;

    #[pallet::storage]
    pub type AuthorizedPublicKeys<T> = StorageValue<_, Vec<PublicKey>, ValueQuery>;

    #[pallet::storage]
    pub type DecryptionNodes<T> = StorageValue<_, Vec<DecryptionNodeInfo>, ValueQuery>;

    #[pallet::storage]
    pub type SubnetDecryptionData<T> = StorageMap<_, Identity, u16, SubnetDecryptionInfo>;

    #[pallet::storage]
    pub type DecryptionNodeCursor<T> = StorageValue<_, u16, ValueQuery>;

    #[pallet::storage]
    pub type DecryptedWeights<T> =
        StorageMap<_, Identity, u16, Vec<(u64, Vec<(u16, Vec<(u16, u16)>)>)>>;

    type BalanceOf<T> =
        <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

    // Output of every subnet pricing mechanism
    pub type PricedSubnets = BTreeMap<u16, u64>;

    // Emission Allocation per Block step
    // ==================================

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_initialize(block_number: BlockNumberFor<T>) -> Weight {
            let block_number: u64 =
                block_number.try_into().ok().expect("blockchain won't pass 2 ^ 64 blocks");

            Self::distribute_subnets_to_nodes(block_number);

            let emission_per_block = Self::get_total_emission_per_block();
            // Make sure to use storage layer,
            // so runtime can never panic in initialization hook
            let res: Result<(), DispatchError> = with_storage_layer(|| {
                Self::process_emission_distribution(block_number, emission_per_block);
                Ok(())
            });
            if let Err(err) = res {
                log::error!("Error in on_initialize emission: {err:?}, skipping...");
            }

            Self::copy_delegated_weights(block_number);

            for netuid in pallet_subspace::N::<T>::iter_keys() {
                if pallet_subspace::Pallet::<T>::blocks_until_next_epoch(netuid, block_number) > 0 {
                    continue;
                }

                // Clear weights for normal subnets
                Self::clear_set_weight_rate_limiter(netuid);
            }

            Weight::zero()
        }
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub fn deposit_event)]
    pub enum Event<T: Config> {
        /// Subnets tempo has finished
        EpochFinished(u16),
    }

    #[derive(Debug)]
    #[allow(dead_code)]
    pub enum EmissionError {
        EmittedMoreThanExpected { emitted: u64, expected: u64 },
        HasEmissionRemaining { emitted: u64 },
        BalanceConversionFailed,
        Other(&'static str),
    }

    impl From<&'static str> for EmissionError {
        fn from(v: &'static str) -> Self {
            Self::Other(v)
        }
    }

    // Subnet Emission distribution
    // =============================

    impl<T: Config> Pallet<T> {
        fn get_total_free_balance() -> BalanceOf<T> {
            <T as Config>::Currency::total_issuance().saturated_into()
        }

        fn get_total_issuence_as_u64() -> u64
        where
            <<T as pallet::Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance:
                TryInto<u64>,
        {
            let total_free_balance = Self::get_total_free_balance();
            let total_staked_balance = TotalStake::<T>::get();
            total_free_balance
                .try_into()
                .unwrap_or_default()
                .saturating_add(total_staked_balance)
        }

        // Halving Logic / Emission distributed per block
        // ===============================================

        // Halving occurs every 250 million minted tokens, until reaching a maximum supply of 1
        // billion tokens.
        #[must_use]
        pub fn get_total_emission_per_block() -> u64 {
            let total_issuance = Self::get_total_issuence_as_u64();
            let unit_emission = UnitEmission::<T>::get();
            let halving_interval = T::HalvingInterval::get();
            let max_supply = T::MaxSupply::get();
            let decimals = T::Decimals::get() as u32;

            let halving_interval = match halving_interval.checked_mul(10_u64.pow(decimals)) {
                Some(val) => val,
                None => {
                    log::error!(
                        "Critical error: halving_interval overflow in get_total_emission_per_block"
                    );
                    return 0;
                }
            };

            let max_supply = match max_supply.checked_mul(10_u64.pow(decimals)) {
                Some(val) => val,
                None => {
                    log::error!(
                        "Critical error: max_supply overflow in get_total_emission_per_block"
                    );
                    return 0;
                }
            };

            if total_issuance >= max_supply {
                0
            } else {
                match total_issuance.checked_div(halving_interval) {
                    Some(halving_count) => unit_emission >> halving_count,
                    None => {
                        log::error!(
                            "Critical error: Division failed in get_total_emission_per_block"
                        );
                        0
                    }
                }
            }
        }

        // Emission Distribution per Subnet
        // =================================

        // Returns emisison for every network
        // TODO
        // later
        // change this to also have the governacne processes, of picking the right subnet pricing
        #[must_use]
        pub fn get_subnet_pricing(token_emission: u64) -> PricedSubnets {
            let rootnet_id = Self::get_consensus_netuid(SubnetConsensus::Root).unwrap_or(0);
            let pricing = RootPricing::<T>::new(rootnet_id, token_emission);
            let priced_subnets = match pricing.run() {
                Ok(priced_subnets) => priced_subnets,
                Err(err) => {
                    log::debug!("could not get priced subnets: {err:?}");
                    PricedSubnets::default()
                }
            };

            for (netuid, emission) in priced_subnets.iter() {
                SubnetEmission::<T>::insert(netuid, emission);
            }

            priced_subnets
        }
        // --- Returns the transaction priority for setting weights.
        pub fn get_priority_set_weights(key: &T::AccountId, netuid: u16) -> u64 {
            if let Some(uid) = pallet_subspace::Uids::<T>::get(netuid, key) {
                let last_update =
                    pallet_subspace::Pallet::<T>::get_last_update_for_uid(netuid, uid);
                pallet_subspace::Pallet::<T>::get_current_block_number().saturating_add(last_update)
            } else {
                0
            }
        }
        // --- Returns the transaction priority for setting weights.
        pub fn get_priority_stake(key: &T::AccountId, netuid: u16) -> u64 {
            if pallet_subspace::Uids::<T>::contains_key(netuid, key) {
                return pallet_subspace::Pallet::<T>::get_delegated_stake(key);
            }
            0
        }

        pub fn handle_decrypted_weights(
            netuid: u16,
            weights: Vec<(u64, Vec<(u16, Vec<(u16, u16)>)>)>,
        ) {
            Self::do_handle_decrypted_weights(netuid, weights);
        }

        pub fn handle_authority_node_keep_alive(public_key: (Vec<u8>, Vec<u8>)) {
            Self::do_handle_authority_node_keep_alive(public_key);
        }
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::call_index(0)]
        #[pallet::weight((0, DispatchClass::Normal, Pays::No))]
        pub fn set_weights(
            origin: OriginFor<T>,
            netuid: u16,
            uids: Vec<u16>,
            weights: Vec<u16>,
        ) -> DispatchResult {
            Self::do_set_weights(origin, netuid, uids, weights)
        }

        #[pallet::call_index(1)]
        #[pallet::weight(0)] // TODO: add benchmark
        pub fn set_weights_encrypted(
            origin: OriginFor<T>,
            netuid: u16,
            encrypted_weights: Vec<u8>,
            encrypted_weights_hash: Vec<u8>,
        ) -> DispatchResult {
            Self::do_set_weights_encrypted(
                origin,
                netuid,
                encrypted_weights,
                encrypted_weights_hash,
            )
        }

        #[pallet::call_index(2)]
        #[pallet::weight((0, DispatchClass::Normal, Pays::No))]
        pub fn delegate_rootnet_control(
            origin: OriginFor<T>,
            target: T::AccountId,
        ) -> DispatchResult {
            Self::do_delegate_rootnet_control(origin, target)
        }
    }
}

#[derive(Debug, PartialEq, Default)]
pub enum CallType {
    SetWeights,
    SetEncryptedWeights,
    DelegateRootnetControl,
    #[default]
    Other,
}

use frame_support::{
    dispatch,
    dispatch::{DispatchInfo, PostDispatchInfo},
    traits::{Currency, IsSubType},
};
use sp_runtime::{
    traits::{DispatchInfoOf, Dispatchable, PostDispatchInfoOf, SignedExtension},
    transaction_validity::{TransactionValidity, TransactionValidityError, ValidTransaction},
};

#[derive(Encode, Decode, Clone, Eq, PartialEq, TypeInfo)]
pub struct SubnetEmissionSignedExtension<T: Config + Send + Sync + TypeInfo>(pub PhantomData<T>);

impl<T: Config + Send + Sync + TypeInfo> Default for SubnetEmissionSignedExtension<T>
where
    T::RuntimeCall: Dispatchable<Info = DispatchInfo, PostInfo = PostDispatchInfo>,
    <T as frame_system::Config>::RuntimeCall: IsSubType<Call<T>>,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Config + Send + Sync + TypeInfo> SubnetEmissionSignedExtension<T>
where
    T::RuntimeCall: Dispatchable<Info = DispatchInfo, PostInfo = PostDispatchInfo>,
    <T as frame_system::Config>::RuntimeCall: IsSubType<Call<T>>,
{
    pub fn new() -> Self {
        Self(Default::default())
    }

    pub fn get_priority_vanilla(who: &T::AccountId) -> u64 {
        // Return high priority so that every extrinsic except set_weights function will
        // have a higher priority than the set_weights call
        // get the current block number
        let current_block_number = pallet_subspace::Pallet::<T>::get_current_block_number();
        let balance = pallet_subspace::Pallet::<T>::get_balance_u64(who);

        // this is the current block number minus the last update block number
        current_block_number.saturating_add(balance)
    }

    pub fn get_priority_set_weights(who: &T::AccountId, netuid: u16) -> u64 {
        // Return the non vanilla priority for a set weights call.
        Pallet::<T>::get_priority_set_weights(who, netuid)
    }

    #[must_use]
    pub fn u64_to_balance(
        input: u64,
    ) -> Option<
        <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance,
    > {
        input.try_into().ok()
    }
}

impl<T: Config + Send + Sync + TypeInfo> sp_std::fmt::Debug for SubnetEmissionSignedExtension<T> {
    fn fmt(&self, f: &mut sp_std::fmt::Formatter) -> sp_std::fmt::Result {
        write!(f, "SubspaceSignedExtension")
    }
}

impl<T: Config + Send + Sync + TypeInfo> SignedExtension for SubnetEmissionSignedExtension<T>
where
    T::RuntimeCall: Dispatchable<Info = DispatchInfo, PostInfo = PostDispatchInfo>,
    <T as frame_system::Config>::RuntimeCall: IsSubType<Call<T>>,
{
    const IDENTIFIER: &'static str = "SubspaceSignedExtension";

    type AccountId = T::AccountId;
    type Call = T::RuntimeCall;
    type AdditionalSigned = ();
    type Pre = (CallType, u64, Self::AccountId);

    fn additional_signed(&self) -> Result<Self::AdditionalSigned, TransactionValidityError> {
        Ok(())
    }

    fn validate(
        &self,
        who: &Self::AccountId,
        call: &Self::Call,
        _info: &DispatchInfoOf<Self::Call>,
        _len: usize,
    ) -> TransactionValidity {
        match call.is_sub_type() {
            Some(Call::set_weights { netuid, .. }) => {
                let priority: u64 = Self::get_priority_set_weights(who, *netuid);
                Ok(ValidTransaction {
                    priority,
                    longevity: 1,
                    ..Default::default()
                })
            }
            _ => Ok(ValidTransaction {
                priority: Self::get_priority_vanilla(who),
                ..Default::default()
            }),
        }
    }

    // NOTE: Add later when we put in a pre and post dispatch step.
    fn pre_dispatch(
        self,
        who: &Self::AccountId,
        call: &Self::Call,
        _info: &DispatchInfoOf<Self::Call>,
        _len: usize,
    ) -> Result<Self::Pre, TransactionValidityError> {
        let who = who.clone();
        match call.is_sub_type() {
            Some(Call::set_weights { .. }) => Ok((CallType::SetWeights, 0, who)),
            _ => Ok((CallType::Other, 0, who)),
        }
    }

    fn post_dispatch(
        maybe_pre: Option<Self::Pre>,
        _info: &DispatchInfoOf<Self::Call>,
        _post_info: &PostDispatchInfoOf<Self::Call>,
        _len: usize,
        _result: &dispatch::DispatchResult,
    ) -> Result<(), TransactionValidityError> {
        if let Some((call_type, _transaction_fee, _who)) = maybe_pre {
            match call_type {
                CallType::SetWeights => {
                    log::debug!("Not Implemented!");
                }
                _ => {
                    log::debug!("Not Implemented!");
                }
            }
        }
        Ok(())
    }
}
