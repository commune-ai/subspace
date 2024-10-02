/// CHECK SPELLING!
// WILL BE MOVED TO A GITHUB GIST
# Implementation Process

## 1. Client-side Weight Encryption
- Calculate hash of decrypted weights
- Encrypt validator weights using asymmetric encryption
- Incorporate randomization to prevent rainbow table attacks

**Rust Encryption Reference** (performed by client)
```rs
// TODO: implement this
// hash
// encrypt
```

**Python Encryption Reference**
```py
# TODO: implement this
```

## 2. Blockchain Extrinsic
- Submit encrypted weights and weight hashes to the blockchain
- Store encrypted weights and hashes on-chain

**Weight Setting Extrinsic**
```rs
pub fn do_set_weights_encrypted(
    origin: T::RuntimeOrigin,
    netuid: u16,
    encrypted_weights: Vec<u8>,
    decrypted_weights_hash: Vec<u8>,
) -> DispatchResult {
    let key = ensure_signed(origin)?;

    if !pallet_subspace::UseWeightsEncrytyption::<T>::get(netuid) {
        return Err(pallet_subspace::Error::<T>::SubnetNotEncrypted.into());
    }

    let Some(uid) = pallet_subspace::Pallet::<T>::get_uid_for_key(netuid, &key) else {
        return Err(pallet_subspace::Error::<T>::ModuleDoesNotExist.into());
    };

    Self::handle_rate_limiting(uid, netuid, &key)?;
    Self::remove_rootnet_delegation(netuid, key);

    EncryptedWeights::<T>::set(netuid, uid, Some(encrypted_weights));
    DecryptedWeightHashes::<T>::set(netuid, uid, Some(decrypted_weights_hash));

    Ok(())
}
}
```

**Storage Definitions**
```rs
#[pallet::storage]
pub type EncryptedWeights<T> = StorageDoubleMap<_, Identity, u16, Identity, u16, Vec<u8>>;

#[pallet::storage]
pub type DecryptedWeightHashes<T> = StorageDoubleMap<_, Identity, u16, Identity, u16, Vec<u8>>;
```

## 3. Off-chain Decryption and Processing
- Designated weight decryption nodes:
  a. Decrypt weight data for assigned subnets
  b. Perform consensus calculations
  c. Run DEW Algorithm to check relative copier profitability constraint
  d. If satisfied, submit decrypted weights back to runtime

**Off-chain Worker**

```rs
#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use alloc::vec::Vec;
use frame_support::traits::Get;
use frame_system::{
    offchain::{
        AppCrypto, CreateSignedTransaction, SendSignedTransaction, SendUnsignedTransaction,
        SignedPayload, Signer, SigningTypes,
    },
    pallet_prelude::BlockNumberFor,
};
use pallet_subnet_emission::{
    subnet_consensus::{
        util::{
            consensus::ConsensusOutput,
            params::{ConsensusParams, ModuleKey, ModuleParams},
        },
        yuma::YumaEpoch,
    },
    EncryptedWeights,
};
use pallet_subspace::TotalStake;

use std::collections::BTreeMap;

use pallet_subnet_emission::Weights;
use pallet_subspace::{
    math::{inplace_normalize_64, vec_fixed64_to_fixed32},
    Active, Consensus, CopierMargin, FloorDelegationFee, MaxEncryptionPeriod,
    Pallet as SubspaceModule, StakeFrom, Tempo, N,
};
use parity_scale_codec::{Decode, Encode};
use scale_info::prelude::marker::PhantomData;
use sp_core::crypto::KeyTypeId;
use sp_runtime::{
    offchain::storage::{StorageRetrievalError, StorageValueRef},
    traits::{BlakeTwo256, Hash},
    Percent, RuntimeDebug,
};
use substrate_fixed::{types::I32F32, FixedI128};

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

    // TODO:
    // rename from test to prod naming
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
    use frame_support::{
        pallet_prelude::{ValueQuery, *},
        Identity,
    };
    use frame_system::pallet_prelude::*;
    use pallet_subnet_emission::ConsensusParameters;

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

        // TODO:
        // add hardcoded limit of how many black boxes we can hold
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

            if !ow_extensions::offworker::is_decryption_node() {
                return;
            }

            let block_number =
                block_number.try_into().ok().expect("blockchain won't pass 2^64 blocks");

            Self::do_send_keep_alive(block_number);

            let public_key: (Vec<u8>, Vec<u8>) = (vec![], vec![]);

            let subnets = pallet_subnet_emission::SubnetDecryptionData::<T>::iter()
                .filter_map(|(netuid, data)| {
                    if !pallet_subspace::UseWeightsEncrytyption::<T>::get(netuid) {
                        return None;
                    }

                    if data.node_public_key != public_key {
                        return None;
                    }

                    Some(netuid)
                })
                .collect::<Vec<_>>();

            for subnet_id in subnets {
                // Create a reference to Local Storage value for the last processed block
                let storage_key = format!("last_processed_block:{}", subnet_id).into_bytes();
                let storage = StorageValueRef::persistent(&storage_key);

                // Retrieve the last processed block or use 0 if not found
                let last_processed_block: u64 = storage.get::<u64>().ok().flatten().unwrap_or(0);

                // TODO: if last processed block is missing then just return everything
                // Get all new ConsensusParameters since the last processed block
                let new_params: Vec<(u64, ConsensusParams<T>)> =
                    ConsensusParameters::<T>::iter_prefix(subnet_id)
                        .filter(|(block, _)| {
                            *block > last_processed_block && *block <= block_number
                        })
                        .collect();

                let mut epochs = Vec::new();
                for (param_block, params) in new_params {
                    let decrypted_weights = params
                        .modules
                        .iter()
                        .filter_map(|(key, params)| {
                            // TODO check if were able to use this here
                            let Some(uid) =
                                pallet_subspace::Pallet::<T>::get_uid_for_key(subnet_id, &key.0)
                            else {
                                return None;
                            };

                            if params.weight_encrypted.is_empty() {
                                return Some((uid, Vec::new()));
                            }

                            match ow_extensions::offworker::decrypt_weight(
                                params.weight_encrypted.clone(),
                            ) {
                                Some(decrypted) => Some((uid, decrypted)),
                                None => None,
                            }
                        })
                        .collect::<Vec<_>>();

                    let should_decrypt: bool =
                        Self::should_decrpyt(&decrypted_weights, params, subnet_id);

                    if should_decrypt {
                        epochs.push((param_block, decrypted_weights));
                    }

                    // Update the last processed block in local storage
                    storage.set(&param_block);
                }

                if let Err(err) = Self::do_send_weights(subnet_id, epochs) {
                    log::error!("couldn't send weights to runtime: {err}");
                }
            }
        }
    }

    /// A public part of the pallet.
    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::call_index(2)]
        #[pallet::weight({0})]
        pub fn send_decrypted_weights(
            origin: OriginFor<T>,
            subnet_id: u16,
            decrypted_weights: Vec<(u64, Vec<(u16, Vec<(u16, u16)>)>)>,
        ) -> DispatchResultWithPostInfo {
            ensure_none(origin)?;

            pallet_subnet_emission::Pallet::<T>::handle_decrypted_weights(
                subnet_id,
                decrypted_weights,
            );

            Ok(().into())
        }

        #[pallet::call_index(3)]
        #[pallet::weight({0})]
        pub fn send_keep_alive(
            origin: OriginFor<T>,
            public_key: (Vec<u8>, Vec<u8>),
        ) -> DispatchResultWithPostInfo {
            ensure_none(origin)?;

            pallet_subnet_emission::Pallet::<T>::handle_authority_node_keep_alive(public_key);

            Ok(().into())
        }
    }

    /// Events for the pallet.
    #[pallet::event]
    // #[pallet::generate_deposit(pub(super) fn deposit_event)]
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

    #[pallet::storage]
    pub type IrrationalityDelta<T: Config> =
        StorageMap<_, Identity, u16, Option<I64F64>, ValueQuery>;
}

impl<T: Config> Pallet<T> {
    #[must_use]
    pub fn should_decrpyt(
        decrypted_weights: &Vec<(u16, Vec<(u16, u16)>)>,
        latest_rumtime_yuma_params: ConsensusParams<T>,
        subnet_id: u16,
    ) -> bool {
        let (copier_uid, simulation_yuma_params) = Pallet::<T>::compute_simulation_yuma_params(
            decrypted_weights,
            latest_rumtime_yuma_params,
            subnet_id,
        );

        // Run simulation
        let simulation_yuma_output = YumaEpoch::<T>::new(subnet_id, simulation_yuma_params)
            .run(decrypted_weights.clone())
            .unwrap(); // TODO: handle unwrap

        // Create a reference to Local Storage value, we don't want to store the results in offchain
        // worker memory
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

        // TODO:
        // send delta back to the runtime
        let (is_irrational, _) = is_copying_irrational::<T>(simulation_result);

        is_irrational
    }

    /// Appends copier information to simulated consensus ConsensusParams
    /// Overwrites onchain decrypted weights with the offchain workers' decrypted weights
    pub fn compute_simulation_yuma_params(
        decrypted_weights: &Vec<(u16, Vec<(u16, u16)>)>,
        mut runtime_yuma_params: ConsensusParams<T>,
        subnet_id: u16,
        // Return copier uid and ConsensusParams
    ) -> (u16, ConsensusParams<T>) {
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
            decrypted_weights.iter().cloned().collect();

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
        }

        (copier_uid, runtime_yuma_params)
    }

    /// This will mutate ConsensusParams with copier information, ready for simulation
    pub fn add_copier_to_yuma_params(
        copier_uid: u16,
        mut runtime_yuma_params: ConsensusParams<T>,
        subnet_id: u16,
        weights: Vec<(u16, u16)>,
    ) -> ConsensusParams<T> {
        let copier_stake = get_copier_stake::<T>(subnet_id);
        let current_block = runtime_yuma_params.current_block;

        let mut all_stakes: Vec<I64F64> = runtime_yuma_params
            .modules
            .values()
            .map(|m| m.stake_original)
            .chain(std::iter::once(I64F64::from_num(copier_stake)))
            .collect();

        inplace_normalize_64(&mut all_stakes);

        let normalized_stakes = vec_fixed64_to_fixed32(all_stakes.clone());

        let copier_stake_normalized = normalized_stakes.last().cloned().unwrap_or_default();

        let copier_module = ModuleParams {
            uid: copier_uid,
            last_update: current_block,
            block_at_registration: current_block.saturating_sub(1),
            validator_permit: true,
            stake_normalized: copier_stake_normalized,
            stake_original: I64F64::from_num(copier_stake),
            bonds: Vec::new(),
            weight_encrypted: Vec::new(),
            weight_hash: Vec::new(),
        };

        let seed = (b"copier", subnet_id, copier_uid).using_encoded(BlakeTwo256::hash);
        let copier_account_id = T::AccountId::decode(&mut seed.as_ref())
            .expect("32 bytes should be sufficient for any AccountId");

        let copier_key = ModuleKey(copier_account_id);

        runtime_yuma_params.modules.insert(copier_key.clone(), copier_module);

        for (index, module) in runtime_yuma_params.modules.values_mut().enumerate() {
            module.stake_normalized =
                normalized_stakes.get(index).cloned().unwrap_or_else(|| I32F32::from_num(0));
            module.stake_original =
                all_stakes.get(index).cloned().unwrap_or_else(|| I64F64::from_num(0));
        }
        if let Some(copier_module) = runtime_yuma_params.modules.get_mut(&copier_key) {
            copier_module.stake_original = I64F64::from_num(copier_stake);
        }

        runtime_yuma_params
    }

    fn do_send_weights(
        netuid: u16,
        decrypted_weights: Vec<(u64, Vec<(u16, Vec<(u16, u16)>)>)>,
    ) -> Result<(), &'static str> {
        let signer = Signer::<T, T::AuthorityId>::all_accounts();
        if !signer.can_sign() {
            return Err(
                "No local accounts available. Consider adding one via `author_insertKey` RPC.",
            );
        }

        signer.send_signed_transaction(|_| Call::send_decrypted_weights {
            decrypted_weights: decrypted_weights.clone(),
            subnet_id: netuid,
        });

        Ok(())
    }

    fn do_send_keep_alive(current_block: u64) {
        let Some(public_key) = ow_extensions::offworker::get_encryption_key() else {
            return;
        };

        let storage_key = "last_keep_alive".as_bytes();
        let storage = StorageValueRef::persistent(&storage_key);
        let last_keep_alive = storage.get::<u64>().ok().flatten().unwrap_or(0);

        if last_keep_alive != 0 && current_block - last_keep_alive < 50 {
            return;
        }

        let signer = Signer::<T, T::AuthorityId>::all_accounts();
        if !signer.can_sign() {
            log::error!(
                "No local accounts available. Consider adding one via `author_insertKey` RPC.",
            );
        }

        signer.send_signed_transaction(|_| Call::send_keep_alive {
            public_key: public_key.clone(),
        });

        storage.set(&current_block);
    }
}

// Copying Profitbility Math
// =========================

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
) -> (bool, I64F64) {
    if black_box_age >= max_encryption_period {
        return (true, I64F64::from_num(0));
    }
    let one = I64F64::from_num(1);
    let threshold = one.saturating_add(copier_margin).saturating_mul(cumulative_avg_delegate_divs);
    let delta = cumulative_copier_divs.saturating_sub(threshold);
    (delta.is_negative(), delta)
}

pub fn calculate_avg_delegate_divs<T: pallet_subspace::Config + pallet_subnet_emission::Config>(
    yuma_output: &ConsensusOutput<T>,
    copier_uid: u16,
    delegation_fee: Percent,
) -> Option<I64F64> {
    let subnet_id = yuma_output.subnet_id;
    let copier_idx = copier_uid as usize;
    let fee_factor = I64F64::from_num(100)
        .saturating_sub(I64F64::from_num(delegation_fee.deconstruct()))
        .checked_div(I64F64::from_num(100))?;

    let (total_stake, total_dividends) = yuma_output
        .dividends
        .iter()
        .enumerate()
        .filter(|&(i, &div)| i != copier_idx && div != 0)
        .try_fold(
            (I64F64::from_num(0), I64F64::from_num(0)),
            |(stake_acc, div_acc), (i, &div)| {
                let stake = I64F64::from_num(get_uid_deleg_stake::<T>(i as u16, subnet_id));
                let dividend = I64F64::from_num(div);
                Some((
                    stake_acc.saturating_add(stake),
                    div_acc.saturating_add(dividend),
                ))
            },
        )?;

    let average_dividends = total_dividends.checked_div(total_stake)?;
    let copier_stake = I64F64::from_num(get_uid_deleg_stake::<T>(copier_uid, subnet_id));

    average_dividends.saturating_mul(fee_factor).saturating_mul(copier_stake).into()
}

// TODO:
// Optimization possible here
#[inline]
fn get_uid_deleg_stake<T>(module_id: u16, subnet_id: u16) -> u64
where
    T: pallet_subspace::Config,
{
    let deleg_stake = SubspaceModule::<T>::get_key_for_uid(subnet_id, module_id)
        .map_or(0, |key| SubspaceModule::<T>::get_delegated_stake(&key));

    deleg_stake
}

// TODO:
// The way stake is calculated here is wrong
pub fn get_copier_stake<T>(subnet_id: u16) -> u64
where
    T: pallet_subspace::Config + pallet::Config,
{
    let subnet_stake: u64 = Active::<T>::get(subnet_id)
        .iter()
        .enumerate()
        .filter(|&(_, &is_active)| is_active)
        .map(|(uid, _)| get_uid_deleg_stake::<T>(uid as u16, subnet_id))
        .sum();

    MeasuredStakeAmount::<T>::get().mul_floor(subnet_stake)
}

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
impl<T: pallet_subspace::Config + pallet_subnet_emission::Config> ConsensusSimulationResult<T> {
    pub fn update(
        &mut self,
        yuma_output: ConsensusOutput<T>,
        tempo: u16,
        copier_uid: u16,
        delegation_fee: Percent,
    ) {
        let avg_delegate_divs =
            calculate_avg_delegate_divs::<T>(&yuma_output, copier_uid, delegation_fee)
                .unwrap_or_else(|| FixedI128::from(0));

        let copier_divs = yuma_output
            .dividends
            .get(copier_uid as usize)
            .map(|&div| I64F64::from_num(div))
            .unwrap_or_else(|| I64F64::from_num(0));

        self.cumulative_copier_divs = self.cumulative_copier_divs.saturating_add(copier_divs);
        self.cumulative_avg_delegate_divs =
            self.cumulative_avg_delegate_divs.saturating_add(avg_delegate_divs);
        self.black_box_age = self.black_box_age.saturating_add(u64::from(tempo));

        self.max_encryption_period = MaxEncryptionPeriod::<T>::get(yuma_output.subnet_id);
        self.copier_margin = CopierMargin::<T>::get(yuma_output.subnet_id);
    }
}
```

## 4. Runtime Handling
- Verify weight authenticity by comparing hashes
- Calculate and distribute consensus for epochs with available decrypted weights
- Store decrypted weights in runtime storage
- Optionally reassign decryption responsibilities periodically

## Key Considerations
- Ensure provable non-tampering of weights
- Prevent weight extraction before public decryption
- Regularly rotate and distribute subnet public decryption keys

## Security Measures
- Hash comparison to prevent tampering
- Even distribution of weights across decryption nodes
- Periodic reassignment of decryption responsibilities


TODO:
discuss conesnsus design
