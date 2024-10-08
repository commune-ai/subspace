#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use alloc::vec::Vec;
use frame_support::{pallet_macros::import_section, sp_runtime::DispatchError, traits::Get};
use frame_system::{
    offchain::{AppCrypto, CreateSignedTransaction, SendSignedTransaction, Signer},
    pallet_prelude::BlockNumberFor,
};
use pallet_subnet_emission::subnet_consensus::{
    util::{
        consensus::ConsensusOutput,
        params::{ConsensusParams, ModuleKey, ModuleParams},
    },
    yuma::YumaEpoch,
};

use std::collections::BTreeMap;

use pallet_subnet_emission::{ConsensusParameters, Weights};
use pallet_subspace::{
    math::{inplace_normalize_64, vec_fixed64_to_fixed32},
    Active, Consensus, CopierMargin, FloorDelegationFee, MaxEncryptionPeriod,
    Pallet as SubspaceModule, Tempo, N,
};
use parity_scale_codec::{Decode, Encode};
use scale_info::prelude::marker::PhantomData;
use sp_core::crypto::KeyTypeId;
use sp_runtime::{
    offchain::storage::StorageValueRef,
    traits::{BlakeTwo256, Hash},
    Percent,
};
use substrate_fixed::types::I32F32;
use types::{ConsensusSimulationResult, ShouldDecryptResult};
use util::process_consensus_params;

mod dispatches;
mod process;
mod profitability;
mod types;
mod util;

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

#[import_section(dispatches::dispatches)]
#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::{
        pallet_prelude::{ValueQuery, *},
        Identity,
    };
    use frame_system::pallet_prelude::*;

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

        type MaxEncryptionTime: Get<u64>;
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        #[cfg(test)]
        fn on_initialize(_block_number: BlockNumberFor<T>) -> Weight {
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

            if let Err(e) = Self::do_send_keep_alive(block_number) {
                log::error!("Error sending keep alive: {:?}", e);
                return;
            }

            let Some(public_key) = ow_extensions::offworker::get_encryption_key() else {
                return;
            };

            let subnets = Self::get_valid_subnets(public_key);
            Self::process_subnets(subnets);
        }
    }

    #[pallet::event]
    pub enum Event<T: Config> {}

    // 5 % of total active stake
    #[pallet::type_value]
    pub fn DefaultMeasuredStakeAmount<T: Config>() -> Percent {
        Percent::from_percent(5u8)
    }

    /// The amount of actual consensus sum stake. Used for a simulated consensus.
    /// Weight copying representant
    #[pallet::storage]
    pub type MeasuredStakeAmount<T: Config> =
        StorageValue<_, Percent, ValueQuery, DefaultMeasuredStakeAmount<T>>;

    /// The amount of delta between comulative copier dividends and compulative delegator dividends.
    #[pallet::storage]
    pub type IrrationalityDelta<T: Config> = StorageMap<_, Identity, u16, I64F64, ValueQuery>;
}

impl<T: Config> Pallet<T> {
    fn do_send_weights(
        netuid: u16,
        decrypted_weights: Vec<(u64, Vec<(u16, Vec<(u16, u16)>)>)>,
        delta: I64F64,
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
            delta,
        });

        Ok(())
    }

    fn do_send_keep_alive(current_block: u64) -> Result<(), DispatchError> {
        let public_key =
            ow_extensions::offworker::get_encryption_key().ok_or("Failed to get encryption key")?;

        let storage_key = b"last_keep_alive";
        let storage = StorageValueRef::persistent(storage_key);
        let last_keep_alive = storage.get::<u64>().ok().flatten().unwrap_or(0);
        if last_keep_alive != 0 && current_block.saturating_sub(last_keep_alive) < 50 {
            return Ok(());
        }

        let signer = Signer::<T, T::AuthorityId>::all_accounts();
        if !signer.can_sign() {
            dbg!("no local accs");
            log::error!(
                "No local accounts available. Consider adding one via `author_insertKey` RPC."
            );
            return Err("No local accounts available".into());
        }
        let result = signer.send_signed_transaction(|_| Call::send_keep_alive {
            public_key: public_key.clone(),
        });

        for (_account, result) in result {
            if let Err(e) = result {
                log::error!("Failed to send keep-alive transaction: {:?}", e);
                return Err("Failed to send keep-alive transaction".into());
            }
        }

        storage.set(&current_block);
        Ok(())
    }
}
