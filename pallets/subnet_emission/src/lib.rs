#![allow(non_snake_case)]
#![cfg_attr(not(feature = "std"), no_std)]

use crate::types::{BlockWeights, PublicKey, SubnetDecryptionInfo};
use frame_system::pallet_prelude::OriginFor;
pub use pallet::*;
use parity_scale_codec::{Decode, Encode};
use scale_info::TypeInfo;
use sp_std::{collections::btree_map::BTreeMap, vec::Vec};

// ! Pallet that handles the emission distribution amongst subnets

// Pallet Imports
// ==============

pub mod decryption;
pub mod distribute_emission;
pub mod migrations;
pub mod subnet_pricing {
    pub mod demo;
    pub mod root;
}

pub mod consensus;
pub mod set_weights;
pub mod types;
pub mod weights;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

#[frame_support::pallet]
pub mod pallet {
    pub use crate::weights::WeightInfo;
    use crate::{consensus::util::params::ConsensusParams, *};
    use frame_support::{
        pallet_prelude::{ValueQuery, *},
        sp_runtime::SaturatedConversion,
        storage::with_storage_layer,
        traits::{ConstU64, Currency},
    };
    use frame_system::pallet_prelude::BlockNumberFor;
    use pallet_subnet_emission_api::SubnetConsensus;
    use pallet_subspace::{define_module_includes, define_subnet_includes, TotalStake};
    use subnet_pricing::root::RootPricing;

    #[cfg(feature = "testnet")]
    const STORAGE_VERSION: StorageVersion = StorageVersion::new(16);

    #[cfg(not(feature = "testnet"))]
    const STORAGE_VERSION: StorageVersion = StorageVersion::new(2);

    #[pallet::pallet]
    #[pallet::storage_version(STORAGE_VERSION)]
    #[pallet::without_storage_info]
    pub struct Pallet<T>(_);

    #[pallet::config(with_default)]
    pub trait Config:
        frame_system::Config
        + pallet_subspace::Config
        + pallet_governance_api::GovernanceApi<<Self as frame_system::Config>::AccountId>
        + scale_info::TypeInfo
        + sp_std::fmt::Debug
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

        #[pallet::constant]
        type DecryptionNodeRotationInterval: Get<u64>;

        /// Maximum number of authorities.
        #[pallet::constant]
        type MaxAuthorities: Get<u32>;

        /// The duration (in blocks) for which an offchain worker is banned after being cancelled
        #[pallet::constant]
        type OffchainWorkerBanDuration: Get<u64>;

        /// The number of consecutive missed pings after which a decryption node is considered
        /// inactive
        #[pallet::constant]
        type MissedPingsForInactivity: Get<u8>;

        /// The interval (in blocks) at which the decryption node should send a keep-alive
        #[pallet::constant]
        type PingInterval: Get<u64>;

        /// The extra buffer period in blocks that runtime will wait before banning a decryption
        /// node. So the final count is `MaxEncryptionPeriod + EncryptionPeriodBuffer`
        #[pallet::constant]
        type EncryptionPeriodBuffer: Get<u64>;

        type WeightInfo: WeightInfo;
    }

    type BalanceOf<T> =
        <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

    // Output of every subnet pricing mechanism
    pub type PricedSubnets = BTreeMap<u16, u64>;

    // --- Subnet Related Storage ---

    define_subnet_includes!(
        double_maps: {
            Weights,
            WeightEncryptionData,
            ConsensusParameters
        },
        maps: {
            SubnetDecryptionData,
            SubnetConsensusType
        }
    );

    define_module_includes!(
        vectors: { },
        swap_storages: {
            optional: { },
            required: {
                Weights: Vec<(u16, u16)> = Vec::<(u16, u16)>::new(),
            }
        },
        key_storages: { },
        key_only_storages: { }
    );

    #[pallet::storage]
    pub type Weights<T> = StorageDoubleMap<_, Identity, u16, Identity, u16, Vec<(u16, u16)>>;

    #[derive(Encode, Decode, RuntimeDebug, Default, TypeInfo)]
    pub struct EncryptionMechanism {
        pub encrypted: Vec<u8>,
        pub decrypted_hashes: Vec<u8>,
    }

    #[pallet::storage]
    pub type WeightEncryptionData<T> =
        StorageDoubleMap<_, Identity, u16, Identity, u16, EncryptionMechanism>;

    /// Decryption Node Info assigned to subnet
    #[pallet::storage]
    pub type SubnetDecryptionData<T> = StorageMap<_, Identity, u16, SubnetDecryptionInfo<T>>;

    #[pallet::storage]
    pub type SubnetConsensusType<T> = StorageMap<_, Identity, u16, SubnetConsensus>;

    /// Netuid, to block number to consensus parameters
    #[pallet::storage]
    pub type ConsensusParameters<T> =
        StorageDoubleMap<_, Identity, u16, Identity, u64, ConsensusParams<T>, OptionQuery>;

    // --- Storage Maps ---
    // ? Does not have to be removed upon subnet removal

    /// Stores non responsive decryption nodes
    #[pallet::storage]
    pub type BannedDecryptionNodes<T: Config> =
        StorageMap<_, Identity, T::AccountId, u64, ValueQuery>;

    /// Stores offchain workers that are going to be banned, if their weights aren't received within
    /// the buffer period
    /// Subnet: u16 , Decryption Node: AccountId, Buffer: BlockNumber (current block + buffer)
    #[pallet::storage]
    pub type DecryptionNodeBanQueue<T: Config> =
        StorageDoubleMap<_, Identity, u16, Identity, T::AccountId, u64, ValueQuery>;

    #[pallet::storage]
    pub type PendingEmission<T> = StorageMap<_, Identity, u16, u64, ValueQuery>;

    #[pallet::storage]
    pub type SubnetEmission<T> = StorageMap<_, Identity, u16, u64, ValueQuery>;

    // --- Storage Values ---

    /// This storage is managed dynamically based on the do_keep_alive offchain worker call
    /// It is built from the authority keys
    #[pallet::storage]
    pub type DecryptionNodes<T> = StorageValue<_, Vec<SubnetDecryptionInfo<T>>, ValueQuery>;

    #[pallet::storage]
    pub type DecryptionNodeCursor<T> = StorageValue<_, u16, ValueQuery>;

    /// Association of signing public keys with associated rsa encryption public keys.
    #[pallet::storage]
    pub type Authorities<T: Config> =
        StorageValue<_, BoundedVec<(T::AccountId, PublicKey), T::MaxAuthorities>, ValueQuery>;

    // Subnet Pricing & Consensus
    #[pallet::storage]
    pub type UnitEmission<T> = StorageValue<_, u64, ValueQuery, ConstU64<23148148148>>;

    // Emission Allocation per Block step
    // ==================================

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        // TODO: add a function that discards hanging subnets
        fn on_initialize(block_number: BlockNumberFor<T>) -> Weight {
            let block_number: u64 =
                block_number.try_into().ok().expect("blockchain won't pass 2 ^ 64 blocks");

            log::info!(
                "Running on_initialize at block: {:?}, subnet_emission module",
                block_number
            );

            let cleared = Self::clear_hanging_subnet_state();
            log::info!("Cleared state of {cleared} subnets");

            Self::distribute_subnets_to_nodes(block_number);
            log::info!("Distributed subnets to nodes");
            Self::assign_activation_blocks(block_number);
            Self::cancel_expired_offchain_workers(block_number);
            Self::process_ban_queue(block_number);
            log::info!("Cancelled expired offchain workers");
            let emission_per_block = Self::get_total_emission_per_block();
            log::info!("Emission per block: {:?}", emission_per_block);
            // Make sure to use storage layer,
            // so runtime can never panic in initialization hook
            let res: Result<(), DispatchError> = with_storage_layer(|| {
                Self::process_emission_distribution(block_number, emission_per_block);
                Ok(())
            });
            if let Err(err) = res {
                log::error!("Error in on_initialize emission: {err:?}, skipping...");
            }

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
        /// Subnets tempo has finished or Snapshot has been taken
        EpochFinalized(u16),
        /// Weight copying decryption was canceled
        DecryptionNodeCanceled {
            subnet_id: u16,
            node_id: T::AccountId,
        },
        /// Weight copying decryption node was rotated
        DecryptionNodeRotated {
            subnet_id: u16,
            previous_node_id: T::AccountId,
            new_node_id: T::AccountId,
        },
        /// Decryption node was called by the runtime to send decrypted weights back, if node fails
        /// to do so on time, it will get banned
        DecryptionNodeCallbackScheduled {
            subnet_id: u16,
            node_id: T::AccountId,
            ban_block: u64,
        },
        /// Decryption node was banned, as it failed to send decrypted weights back to the runtime
        DecryptionNodeBanned {
            subnet_id: u16,
            node_id: T::AccountId,
        },
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

        /// Returns emission for every subnet
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
    }

    // add benchmarks
    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::call_index(0)]
        #[pallet::weight((<T as pallet::Config>::WeightInfo::set_weights(), DispatchClass::Normal, Pays::No))]
        pub fn set_weights(
            origin: OriginFor<T>,
            netuid: u16,
            uids: Vec<u16>,
            weights: Vec<u16>,
        ) -> DispatchResult {
            Self::do_set_weights(origin, netuid, uids, weights)
        }

        #[pallet::call_index(1)]
        #[pallet::weight((<T as pallet::Config>::WeightInfo::set_weights_encrypted(), DispatchClass::Normal, Pays::No))]
        pub fn set_weights_encrypted(
            origin: OriginFor<T>,
            netuid: u16,
            encrypted_weights: Vec<u8>,
            decrypted_weights_hash: Vec<u8>,
        ) -> DispatchResult {
            Self::do_set_weights_encrypted(
                origin,
                netuid,
                encrypted_weights,
                decrypted_weights_hash,
                true,
            )
        }

        #[pallet::call_index(2)]
        #[pallet::weight((<T as pallet::Config>::WeightInfo::set_weights_encrypted(), DispatchClass::Normal, Pays::No))]
        pub fn delegate_weight_control(
            origin: OriginFor<T>,
            netuid: u16,
            target: T::AccountId,
        ) -> DispatchResult {
            Self::do_delegate_weight_control(origin, netuid, target)
        }

        #[pallet::call_index(3)]
        #[pallet::weight((<T as pallet::Config>::WeightInfo::set_weights_encrypted(), DispatchClass::Normal, Pays::No))]
        pub fn remove_weight_control(origin: OriginFor<T>, netuid: u16) -> DispatchResult {
            Self::do_remove_weight_control(origin, netuid)
        }
    }
}
