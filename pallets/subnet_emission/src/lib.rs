#![allow(non_snake_case)]
#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;
use sp_std::collections::btree_map::BTreeMap;
// ! Pallet that handles the emisson distribution amongs subnets

// Pallet Imports
// ==============

pub mod distribute_emission;
pub mod migrations;
pub mod post_runtime;
pub mod subnet_pricing {
    pub mod demo;
    pub mod root;
}

pub mod subnet_consensus;

// TODO:
// move some import outside of the macro
#[frame_support::pallet]
pub mod pallet {
    use crate::*;
    use frame_support::{
        pallet_prelude::*,
        sp_runtime::SaturatedConversion,
        storage::with_storage_layer,
        traits::{ConstU64, Currency},
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
            Weight::zero()
        }

        fn on_idle(_n: BlockNumberFor<T>, remaining: Weight) -> Weight {
            log::info!("on_idle: {remaining:?}");
            Self::deregister_excess_modules(remaining)
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
    }
}
