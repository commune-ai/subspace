#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use std::collections::BTreeMap;

use alloc::vec::Vec;
use frame_support::{traits::Get, weights};
use frame_system::{
    offchain::{AppCrypto, CreateSignedTransaction, SignedPayload, Signer, SigningTypes},
    pallet_prelude::BlockNumberFor,
};
use pallet_subnet_emission::subnet_consensus::yuma::{YumaEpoch, YumaOutput, YumaParams};
use pallet_subspace::{
    Active, Consensus, CopierMargin, FloorDelegationFee, MaxEncryptionPeriod,
    Pallet as SubspaceModule, Tempo, Weights, N,
};
use parity_scale_codec::{Decode, Encode};
use scale_info::prelude::marker::PhantomData;
use sp_core::crypto::KeyTypeId;
use sp_runtime::{
    offchain::storage::{StorageRetrievalError, StorageValueRef},
    Percent, RuntimeDebug,
};
use substrate_fixed::FixedI128;

/// Defines application identifier for crypto keys of this module.
///
/// Every module that deals with signatures needs to declare its unique identifier for
/// its crypto keys.
/// When offchain worker is signing transactions it's going to request keys of type
/// `KeyTypeId` from the keystore and use the ones it finds to sign the transaction.
/// The keys can be inserted manually via RPC (see `author_insertKey`).
pub const KEY_TYPE: KeyTypeId = KeyTypeId(*b"btc!");

/// Based on the above `KeyTypeId` we need to generate a pallet-specific crypto type wrappers.
/// We can use from supported crypto kinds (`sr25519`, `ed25519` and `ecdsa`) and augment
/// the types with this pallet-specific identifier.
pub mod crypto {
    use super::KEY_TYPE;
    use sp_core::sr25519::Signature as Sr25519Signature;
    use sp_runtime::{
        app_crypto::{app_crypto, sr25519},
        traits::Verify,
        MultiSignature, MultiSigner,
    };
    app_crypto!(sr25519, KEY_TYPE);

    pub struct TestAuthId;

    impl frame_system::offchain::AppCrypto<MultiSigner, MultiSignature> for TestAuthId {
        type RuntimeAppPublic = Public;
        type GenericSignature = sp_core::sr25519::Signature;
        type GenericPublic = sp_core::sr25519::Public;
    }

    // implemented for mock runtime in test
    impl frame_system::offchain::AppCrypto<<Sr25519Signature as Verify>::Signer, Sr25519Signature>
        for TestAuthId
    {
        type RuntimeAppPublic = Public;
        type GenericSignature = sp_core::sr25519::Signature;
        type GenericPublic = sp_core::sr25519::Public;
    }
}

pub use pallet::*;
use substrate_fixed::types::I64F64;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;
    use pallet_subnet_emission::YumaParameters;

    /// This pallet's configuration trait
    #[pallet::config]
    pub trait Config:
        CreateSignedTransaction<Call<Self>>
        + frame_system::Config
        + pallet_subspace::Config
        + pallet_subnet_emission::Config
    {
        /// The identifier type for an offchain worker.
        type AuthorityId: AppCrypto<Self::Public, Self::Signature>;

        /// The overarching event type.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        // Configuration parameters

        /// A grace period after we send transaction.
        ///
        /// To avoid sending too many transactions, we only attempt to send one
        /// every `GRACE_PERIOD` blocks. We use Local Storage to coordinate
        /// sending between distinct runs of this offchain worker.
        #[pallet::constant]
        type GracePeriod: Get<BlockNumberFor<Self>>;

        /// Number of blocks of cooldown after unsigned transaction is included.
        ///
        /// This ensures that we only accept unsigned transactions once, every `UnsignedInterval`
        /// blocks.
        #[pallet::constant]
        type UnsignedInterval: Get<BlockNumberFor<Self>>;

        /// A configuration for base priority of unsigned transactions.
        ///
        /// This is exposed so that it can be tuned for particular runtime, when
        /// multiple pallets send unsigned transactions.
        #[pallet::constant]
        type UnsignedPriority: Get<TransactionPriority>;

        /// Maximum number of prices.
        #[pallet::constant]
        type MaxPrices: Get<u32>;
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        /// Reproducing offchain worker behaivor for testing
        #[cfg(test)]
        fn on_initialize(_block_number: BlockNumberFor<T>) -> Weight {
            log::info!("Hello World from on_initialize!");
            // TODO
            Weight::zero()
        }

        // ! This function is not actually guaranteed to run on every block
        fn offchain_worker(block_number: BlockNumberFor<T>) {
            log::info!("Offchain worker is running");
            for subnet_id in [0u16; 0] {
                // TODO: see if this is needed
                let is_validator = sp_io::offchain::is_validator();
                if !is_validator {
                    log::info!("Not a validator node, skipping offchain computation.");
                    return;
                }

                let block_number: u64 =
                    block_number.try_into().ok().expect("blockchain won't pass 2 ^ 64 blocks");

                // This hook should always run after on_initialize, so YumaParams are already ready
                if pallet_subspace::Pallet::<T>::blocks_until_next_epoch(subnet_id, block_number)
                    > 0
                {
                    return;
                }

                // Get the latest runtime YumaParams
                let latest_rumtime_yuma_params = YumaParameters::<T>::iter_prefix(subnet_id)
                    .max_by_key(|(block_number, _)| *block_number)
                    .map(|(_, params)| params)
                    .unwrap(); // Todo: remove unwrap, handle None

                // TODO: Decrypt Encrypted Weight
                // Potentially return a result here
                let decrypted_weights: Option<Vec<(u16, Vec<(u16, u16)>)>> = Some(Vec::new());

                if let Some(decrypted_weights) = decrypted_weights {
                    let should_decrypt = Self::should_decrpyt(
                        decrypted_weights,
                        latest_rumtime_yuma_params,
                        subnet_id,
                    );

                    if should_decrypt { /* TODO: Send decrypted weights to the runtime */ }
                }

                //|  | 0 | 1 | 2 | 3 | 4 | 5 |
                //|                       ^ choose node F
                //|                   ^ choose node E
                //|               ^ choose node D
                //|           ^ choose node C
                //|       ^ choose node B
                //|   ^ choose node A
            }
        }
    }

    /// A public part of the pallet.
    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::call_index(2)]
        #[pallet::weight({0})]
        pub fn submit_price_unsigned_with_signed_payload(
            origin: OriginFor<T>,
            _price_payload: WeightsPayload<T::Public, T::AccountId, BlockNumberFor<T>>,
            _signature: T::Signature,
        ) -> DispatchResultWithPostInfo {
            // This ensures that the function can only be called via unsigned transaction.
            ensure_none(origin)?;

            // now increment the block number at which we expect next unsigned transaction.
            // let current_block = <system::Pallet<T>>::block_number();
            // <NextUnsignedAt<T>>::put(current_block + T::UnsignedInterval::get());
            Ok(().into())
        }
    }

    /// Events for the pallet.
    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Event generated when new price is accepted to contribute to the average.
        NewPrice {
            price: u32,
            maybe_who: Option<T::AccountId>,
        },
    }

    /// A vector of recently submitted prices.
    ///
    /// This is used to calculate average price, should have bounded size.
    #[pallet::storage]
    pub(super) type Prices<T: Config> = StorageValue<_, BoundedVec<u32, T::MaxPrices>, ValueQuery>;

    /// Defines the block when next unsigned transaction will be accepted.
    ///
    /// To prevent spam of unsigned (and unpaid!) transactions on the network,
    /// we only allow one transaction every `T::UnsignedInterval` blocks.
    /// This storage entry defines when new transaction is going to be accepted.
    #[pallet::storage]
    pub(super) type NextUnsignedAt<T: Config> = StorageValue<_, BlockNumberFor<T>, ValueQuery>;

    #[pallet::type_value]
    pub fn DefaultMeasuredStakeAmount<T: Config>() -> Percent {
        Percent::from_percent(5u8)
    }

    /// The amount of actual consensus sum stake. Used for a simulated consensus.
    /// Weight copying representant
    #[pallet::storage]
    pub type MeasuredStakeAmount<T: Config> =
        StorageValue<_, Percent, ValueQuery, DefaultMeasuredStakeAmount<T>>;
}

/// Payload used by this example crate to hold price
/// data required to submit a transaction.
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, scale_info::TypeInfo)]
pub struct WeightsPayload<Public, AccountId, BlockNumber> {
    subnet_id: u16,
    epoch: BlockNumber,
    module_key: AccountId,
    decrypted_weights: Vec<u8>,
    public: Public,
}

impl<T: SigningTypes> SignedPayload<T>
    for WeightsPayload<T::Public, T::AccountId, BlockNumberFor<T>>
{
    fn public(&self) -> T::Public {
        self.public.clone()
    }
}

impl<T: Config> Pallet<T> {
    #[must_use]
    pub fn should_decrpyt(
        decrypted_weights: Vec<(u16, Vec<(u16, u16)>)>,
        latest_rumtime_yuma_params: YumaParams<T>,
        subnet_id: u16,
    ) -> bool {
        let (copier_uid, simulation_yuma_params) = Pallet::<T>::compute_simulation_yuma_params(
            decrypted_weights,
            latest_rumtime_yuma_params,
            subnet_id,
        );

        // Run simulation
        let simulation_yuma_output =
            YumaEpoch::<T>::new(subnet_id, simulation_yuma_params).run().unwrap(); // TODO: handle unwrap

        // Create a reference to Local Storage value
        let storage_key = format!("consensus_simulation_result:{}", subnet_id).into_bytes();
        let storage = StorageValueRef::persistent(&storage_key);

        // Retrieve the existing simulation result or create a new one
        let mut simulation_result = storage
            .mutate(
                |stored_data: Result<
                    Option<ConsensusSimulationResult<T>>,
                    StorageRetrievalError,
                >|
                 -> Result<ConsensusSimulationResult<T>, StorageRetrievalError> {
                    match stored_data {
                        Ok(Some(data)) => Ok(data),
                        Ok(None) => Ok(ConsensusSimulationResult::default()),
                        Err(e) => Err(e),
                    }
                },
            )
            .unwrap_or_else(|_| ConsensusSimulationResult::default());

        // Update the simulation result
        let tempo = Tempo::<T>::get(subnet_id);
        let delegation_fee = FloorDelegationFee::<T>::get();
        simulation_result.update(simulation_yuma_output, tempo, copier_uid, delegation_fee);

        // Save the updated simulation result to local offchain worker storage
        storage.set(&simulation_result);

        is_copying_irrational::<T>(simulation_result)
    }

    /// Appends copier information to simulated consensus YumaParams
    /// Overwrites onchain decrypted weights with the offchain workers' decrypted weights
    pub fn compute_simulation_yuma_params(
        decrypted_weights: Vec<(u16, Vec<(u16, u16)>)>,
        mut runtime_yuma_params: YumaParams<T>,
        subnet_id: u16,
        // Return copier uid and YumaParams
    ) -> (u16, YumaParams<T>) {
        // TODO:
        // Append registered information of copier to the `YumaParams` struct
        let copier_uid: u16 = N::<T>::get(subnet_id);

        let consensus_weights = Consensus::<T>::get(subnet_id);
        let copier_weights: Vec<(u16, u16)> = consensus_weights
            .into_iter()
            .enumerate()
            .map(|(index, value)| (index as u16, value))
            .collect();

        // Overwrite the runtime yuma params with copier information
        runtime_yuma_params = Self::add_copier_to_yuma_params(
            copier_uid,
            runtime_yuma_params,
            subnet_id,
            copier_weights,
        );

        // Query the onchain weights for subnet_id
        let onchain_weights: Vec<(u16, Vec<(u16, u16)>)> =
            Weights::<T>::iter_prefix(subnet_id).collect();

        // Create a map of uid to decrypted weights for easier lookup
        let decrypted_weights_map: BTreeMap<u16, Vec<(u16, u16)>> =
            decrypted_weights.into_iter().collect();

        // Update the modules in runtime_yuma_params
        for (_, module) in runtime_yuma_params.modules.iter_mut() {
            let uid = module.uid;

            // Use decrypted weights if available, otherwise use onchain weights
            let weights = decrypted_weights_map
                .get(&uid)
                .cloned()
                .or_else(|| {
                    onchain_weights.iter().find(|(w_uid, _)| *w_uid == uid).map(|(_, w)| w.clone())
                })
                .unwrap_or_default();

            // TODO:
            // eventually we will move the decrypted weights out of `YumaParams`,
            // so this is a temporary solution

            // Update the weights_unencrypted field
            module.weights_unencrypted = weights;
        }

        (copier_uid, runtime_yuma_params)
    }

    /// This will mutate YumaParams with copier information, ready for simulation
    pub fn add_copier_to_yuma_params(
        _copier_uid: u16,
        mut _runtime_yuma_params: YumaParams<T>,
        subnet_id: u16,
        _weights: Vec<(u16, u16)>,
    ) -> YumaParams<T> {
        // TODO:

        let _copier_stake = get_copier_stake::<T>(subnet_id);
        // With pseudo code
        // let _ = register_module(netuid, key, copier_stake, false);
        // step_block(1);
        // set_weights(netuid, key, uids, values);

        // todo
        _runtime_yuma_params
    }

    /// Chooses which transaction type to send.
    ///
    /// This function serves mostly to showcase `StorageValue` helper
    /// and local storage usage.
    ///
    /// Returns a type of transaction that should be produced in current run.
    fn local_storage(block_number: BlockNumberFor<T>) {
        /// A friendlier name for the error that is going to be returned in case we are in the grace
        /// period.
        const RECENTLY_SENT: () = ();

        // Start off by creating a reference to Local Storage value.
        // Since the local storage is common for all offchain workers, it's a good practice
        // to prepend your entry with the module name.
        let val = StorageValueRef::persistent(b"example_ocw::last_send");
        // The Local Storage is persisted and shared between runs of the offchain workers,
        // and offchain workers may run concurrently. We can use the `mutate` function, to
        // write a storage entry in an atomic fashion. Under the hood it uses `compare_and_set`
        // low-level method of local storage API, which means that only one worker
        // will be able to "acquire a lock" and send a transaction if multiple workers
        // happen to be executed concurrently.
        let _res = val.mutate(
            |last_send: Result<Option<BlockNumberFor<T>>, StorageRetrievalError>| {
                match last_send {
                    // If we already have a value in storage and the block number is recent enough
                    // we avoid sending another transaction at this time.
                    Ok(Some(block)) if block_number < block + T::GracePeriod::get() => {
                        Err(RECENTLY_SENT)
                    }
                    // In every other case we attempt to acquire the lock and send a transaction.
                    _ => Ok(block_number),
                }
            },
        );
    }

    /// A helper function to fetch the price and send signed transaction.
    fn fetch_price_and_send_signed() -> Result<(), &'static str> {
        let signer = Signer::<T, T::AuthorityId>::all_accounts();
        if !signer.can_sign() {
            return Err(
                "No local accounts available. Consider adding one via `author_insertKey` RPC.",
            );
        }

        // Using `send_signed_transaction` associated type we create and submit a transaction
        // representing the call, we've just created.
        // Submit signed will return a vector of results for all accounts that were found in the
        // local keystore with expected `KEY_TYPE`.
        // let results = signer.send_signed_transaction(|_account| {
        //     // Received price is wrapped into a call to `submit_price` public function of this
        //     // pallet. This means that the transaction, when executed, will simply call that
        //     // function passing `price` as an argument.
        //     Call::submit_price { price }
        // });

        // for (acc, res) in &results {
        //     match res {
        //         Ok(()) => log::info!("[{:?}] Submitted price of {} cents", acc.id, price),
        //         Err(e) => log::error!("[{:?}] Failed to submit transaction: {:?}", acc.id, e),
        //     }
        // }

        Ok(())
    }
}

// Copying profitability calulations
// =================================

/// Determines if the copier's performance is irrational based on cumulative dividends.
///
/// # Arguments
///
/// * `consensus_result` - A `ConsensusSimulationResult` struct containing simulation data.
///
/// # Returns
///
/// * `true` if the copier's cumulative dividends are significantly lower than the adjusted average
///   delegate dividends, and the `epoch_block_sum` is less than `max_encryption_period`.
/// * `false` otherwise, including when `epoch_block_sum` is greater than or equal to
///   `max_encryption_period`.
///
/// # Note
///
/// The function compares `cumulative_copier_divs` against an adjusted
/// `cumulative_avg_delegate_divs`, taking into account the `copying_margin`.
#[must_use]
pub fn is_copying_irrational<T: pallet_subspace::Config>(
    ConsensusSimulationResult {
        black_box_age,
        max_encryption_period,
        copier_margin,
        cumulative_avg_delegate_divs,
        cumulative_copier_divs,
        ..
    }: ConsensusSimulationResult<T>,
) -> bool {
    if black_box_age >= max_encryption_period {
        return true;
    }
    let threshold =
        (I64F64::from_num(1) + copier_margin).saturating_mul(cumulative_avg_delegate_divs);
    cumulative_copier_divs < threshold
}
/// # Arguments
///
/// * `netuid` - The network UID.
/// * `dividends` - A slice of dividend values for each UID.
/// * `copier_uid` - The UID of the copier.
/// * `delegation_fee` - The delegation fee percentage.
///
/// # Returns
///
/// The calculated average delegate dividends as an `I64F64` fixed-point number.
pub fn calculate_avg_delegate_divs<T: pallet_subspace::Config>(
    netuid: u16,
    dividends: &[u16],
    copier_uid: u16,
    delegation_fee: Percent,
) -> Option<I64F64> {
    let copier_idx = copier_uid as usize;
    let fee_factor = I64F64::from_num(100)
        .saturating_sub(I64F64::from_num(delegation_fee.deconstruct()))
        .checked_div(I64F64::from_num(100))?;

    let (total_stake, total_dividends) = dividends
        .iter()
        .enumerate()
        .filter(|&(i, &div)| i != copier_idx && div != 0)
        .try_fold(
            (I64F64::from_num(0), I64F64::from_num(0)),
            |(stake_acc, div_acc), (i, &div)| {
                let stake = I64F64::from_num(get_delegated_stake_on_uid::<T>(netuid, i as u16));
                let dividend = I64F64::from_num(div);
                Some((
                    stake_acc.saturating_add(stake),
                    div_acc.saturating_add(dividend),
                ))
            },
        )?;

    let average_dividends = total_dividends.checked_div(total_stake)?;
    let copier_stake = I64F64::from_num(get_delegated_stake_on_uid::<T>(netuid, copier_uid));

    average_dividends.saturating_mul(fee_factor).saturating_mul(copier_stake).into()
}

pub fn get_copier_stake<T>(netuid: u16) -> u64
where
    T: pallet_subspace::Config + pallet::Config,
{
    let subnet_stake: u64 = Active::<T>::get(netuid)
        .iter()
        .enumerate()
        .filter(|&(_, &is_active)| is_active)
        .map(|(uid, _)| get_delegated_stake_on_uid::<T>(netuid, uid as u16))
        .sum();

    MeasuredStakeAmount::<T>::get().mul_floor(subnet_stake)
}
// TODO:
// get rid of this shit, make it more efficient
#[inline]
pub fn get_delegated_stake_on_uid<T: pallet_subspace::Config>(netuid: u16, module_uid: u16) -> u64 {
    SubspaceModule::<T>::get_key_for_uid(netuid, module_uid)
        .map_or(0, |key| SubspaceModule::<T>::get_delegated_stake(&key))
}

/// Represents the result of a consensus simulation.
///
/// # Type Parameters
///
/// * `T` - The configuration type for the Subspace pallet.
///
/// # Fields
///
/// * `cumulative_copier_divs` - Cumulative dividends for the copier.
/// * `cumulative_avg_delegate_divs` - Cumulative average dividends for delegates.
/// * `copying_margin` - Minimum underperformance threshold.
/// * `epoch_block_sum` - Sum of blocks in the epoch.
/// * `max_encryption_period` - Maximum encryption period.
/// * `_phantom` - PhantomData for the generic type `T`.
#[derive(Clone, Debug, PartialEq, Encode, Decode)]

pub struct ConsensusSimulationResult<T: pallet_subspace::Config> {
    pub cumulative_copier_divs: I64F64,
    pub cumulative_avg_delegate_divs: I64F64,
    pub copier_margin: I64F64,
    pub black_box_age: u64,
    pub max_encryption_period: u64,
    pub _phantom: PhantomData<T>,
}

impl<T: pallet_subspace::Config> Default for ConsensusSimulationResult<T> {
    fn default() -> Self {
        ConsensusSimulationResult {
            cumulative_copier_divs: I64F64::from_num(0),
            cumulative_avg_delegate_divs: I64F64::from_num(0),
            copier_margin: I64F64::from_num(0),
            black_box_age: 0,
            max_encryption_period: 0,
            _phantom: PhantomData,
        }
    }
}
impl<T: pallet_subspace::Config> ConsensusSimulationResult<T> {
    pub fn update(
        &mut self,
        yuma_output: YumaOutput<T>,
        tempo: u16,
        copier_uid: u16,
        delegation_fee: Percent,
    ) {
        let netuid = yuma_output.subnet_id;
        let avg_delegate_divs = calculate_avg_delegate_divs::<T>(
            netuid,
            &yuma_output.dividends,
            copier_uid,
            delegation_fee,
        )
        .unwrap_or_else(|| FixedI128::from(0));
        let copier_divs = I64F64::from_num(yuma_output.dividends[copier_uid as usize]);

        self.cumulative_copier_divs = self.cumulative_copier_divs.saturating_add(copier_divs);
        self.cumulative_avg_delegate_divs =
            self.cumulative_avg_delegate_divs.saturating_add(avg_delegate_divs);
        self.black_box_age = self.black_box_age.saturating_add(tempo as u64);

        self.max_encryption_period = MaxEncryptionPeriod::<T>::get(netuid);
        self.copier_margin = CopierMargin::<T>::get(netuid);
    }
}
