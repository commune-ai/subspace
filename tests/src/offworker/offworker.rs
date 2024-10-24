use crate::{
    mock::*,
    offworker::{
        data::{
            load_msgpack_data, make_parameter_consensus_overwrites, register_modules_from_msgpack,
        },
        encryption::{encrypt, hash, MockOffworkerExt, KEY_TYPE},
        util::{initialize_authorities, setup_subnet, update_authority_and_decryption_node},
    },
};
use frame_support::traits::Hooks;
use frame_system::{
    self,
    offchain::{SignedPayload, SigningTypes},
    pallet_prelude::BlockNumberFor,
};
use ow_extensions::OffworkerExt;
use pallet_offworker::{types::DecryptedWeightsPayload, Call, IrrationalityDelta, Pallet};
use parity_scale_codec::{Decode, Encode};
use sp_core::{
    offchain::{testing, OffchainDbExt, OffchainWorkerExt, TransactionPoolExt},
    sr25519, Pair,
};
use sp_keystore::{testing::MemoryKeystore, Keystore, KeystoreExt};
use sp_runtime::{traits::IdentifyAccount, BuildStorage};

use std::sync::Arc;

use pallet_subnet_emission::{
    subnet_consensus::{util::params::ConsensusParams, yuma::YumaEpoch},
    PendingEmission,
};

/// This is the subnet id specifid in the data/...weights_stake.json
/// We are using real network data to perform the tests
const SAMPLE_SUBNET_ID: &str = "31";
const TEST_SUBNET_ID: u16 = 0;
/// Make sure the tempo 100% matches
const SUBNET_TEMPO: u64 = 360;
const PENDING_EMISSION: u64 = to_nano(1_000);
const EXPECTED_DECRYPTIONS_COUNT: u64 = 1;

// Helper function to set up the test environment
fn new_offworker_test_ext(
    mock_offworker_ext: MockOffworkerExt,
) -> (
    sp_io::TestExternalities,
    std::sync::Arc<parking_lot::RwLock<testing::PoolState>>,
    std::sync::Arc<parking_lot::RwLock<testing::OffchainState>>,
) {
    let (offchain, offchain_state) = testing::TestOffchainExt::new();
    let (pool, pool_state) = testing::TestTransactionPoolExt::new();
    let keystore = MemoryKeystore::new();

    // Generate a new key pair and add it to the keystore
    let (pair, _, _) = sr25519::Pair::generate_with_phrase(None);
    let public = pair.public();
    keystore
        .sr25519_generate_new(
            KEY_TYPE,
            Some(&format!("//{}", hex::encode(public.as_ref() as &[u8]))),
        )
        .expect("Failed to add key to keystore");

    sp_tracing::try_init_simple();
    let t = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap();
    let mut ext = sp_io::TestExternalities::new(t);
    ext.register_extension(OffchainWorkerExt::new(offchain.clone()));
    ext.register_extension(OffchainDbExt::new(offchain));
    ext.register_extension(TransactionPoolExt::new(pool));
    ext.register_extension(OffworkerExt::new(mock_offworker_ext));
    ext.register_extension(KeystoreExt(Arc::new(keystore)));

    (ext, pool_state, offchain_state)
}

#[test]
fn test_offchain_worker_behavior() {
    let mock_offworker_ext = MockOffworkerExt::default();
    let (mut ext, pool_state, _offchain_state) = new_offworker_test_ext(mock_offworker_ext);

    ext.execute_with(|| {
        let data = load_msgpack_data();
        let first_block = data
            .weights
            .keys()
            .next()
            .and_then(|s| s.parse::<u64>().ok())
            .expect("Failed to parse first block number");

        // Register and setup subnet
        setup_subnet(TEST_SUBNET_ID, SUBNET_TEMPO);

        // Register all modules from scratch
        register_modules_from_msgpack(&data, TEST_SUBNET_ID);

        // Set block number the simulation will start from
        System::set_block_number(first_block - SUBNET_TEMPO);

        let Some(public_key) = ow_extensions::offworker::get_encryption_key() else {
            panic!("No encryption key found")
        };

        let decryption_info = initialize_authorities(public_key, first_block);

        // Run all important things in on initialize hooks
        PendingEmission::<Test>::set(TEST_SUBNET_ID, PENDING_EMISSION);
        step_block(SUBNET_TEMPO as u16);
        let mut decryption_count = 0;

        for (block_number_str, block_weights) in &data.weights {
            let block_number: u64 = block_number_str.parse().unwrap();
            dbg!(block_number);

            PendingEmission::<Test>::set(TEST_SUBNET_ID, PENDING_EMISSION);
            step_block(SUBNET_TEMPO as u16);
            make_parameter_consensus_overwrites(TEST_SUBNET_ID, block_number, &data, None);

            let weights = &block_weights[SAMPLE_SUBNET_ID];

            let mut input_decrypted_weights: Vec<(u16, Vec<(u16, u16)>)> = Vec::new();

            // Set encrypted weights
            for (uid_str, weight_data) in weights {
                if let Ok(uid) = uid_str.parse::<u16>() {
                    let weight_vec: Vec<(u16, u16)> = weight_data
                        .iter()
                        .filter_map(|w| {
                            if w.len() == 2 {
                                Some((w[0] as u16, w[1] as u16))
                            } else {
                                None
                            }
                        })
                        .collect();

                    if !weight_vec.is_empty() && weight_vec.iter().any(|(_, value)| *value != 0) {
                        let validator_key =
                            SubspaceMod::get_key_for_uid(TEST_SUBNET_ID, uid).unwrap().encode();

                        // Pring all encrpyt inputs
                        let encrypted_weights = encrypt(
                            decryption_info.public_key.clone(),
                            weight_vec.clone(),
                            validator_key,
                        );
                        // Pring all encrpyt outputs
                        let decrypted_weights_hash = hash(weight_vec.clone());

                        if let Some(key) = SubspaceMod::get_key_for_uid(TEST_SUBNET_ID, uid) {
                            set_weights_encrypted(
                                TEST_SUBNET_ID,
                                key,
                                encrypted_weights,
                                decrypted_weights_hash,
                            );
                        }
                        input_decrypted_weights.push((uid, weight_vec));
                    }
                }
            }

            if block_number == first_block {
                let params =
                    ConsensusParams::<Test>::new(TEST_SUBNET_ID, PENDING_EMISSION).unwrap();
                YumaEpoch::<Test>::new(TEST_SUBNET_ID, params)
                    .run(input_decrypted_weights)
                    .unwrap()
                    .apply();
                continue;
            }

            // Run the offchain worker
            Pallet::<Test>::offchain_worker(block_number.into());

            // Process transactions
            // ! This is not actually running the validate unsigned function
            // we need to do all verification, and transaction processing manually
            while let Some(tx) = pool_state.write().transactions.pop() {
                log::info!("processing tx");
                let call = Extrinsic::decode(&mut &*tx).unwrap();
                if let RuntimeCall::OffWorkerMod(Call::send_decrypted_weights {
                    payload,
                    signature,
                }) = call.call
                {
                    let signature_valid = <DecryptedWeightsPayload<
                        <Test as SigningTypes>::Public,
                        BlockNumberFor<Test>,
                    > as SignedPayload<Test>>::verify::<TestAuthId>(
                        &payload, signature.clone()
                    );

                    assert!(signature_valid);

                    assert_eq!(payload.subnet_id, TEST_SUBNET_ID);
                    assert!(!payload.decrypted_weights.is_empty());
                    log::info!("decryption event on block: {}", block_number);

                    let new_acc_id: AccountId = payload.public.clone().into_account().into();
                    update_authority_and_decryption_node::<Test>(TEST_SUBNET_ID, new_acc_id);

                    // Execute the extrinsic
                    let origin = frame_system::RawOrigin::None.into();
                    let result = Pallet::<Test>::send_decrypted_weights(origin, payload, signature);

                    // Handle the result
                    match result {
                        Ok(_) => {
                            log::info!("Transaction executed successfully");
                            decryption_count += 1;
                        }
                        Err(e) => {
                            log::error!("Transaction execution failed: {:?}", e);
                            // Handle the error as needed
                        }
                    }
                }
            }

            if decryption_count >= EXPECTED_DECRYPTIONS_COUNT {
                break;
            }
        }

        assert!(
            decryption_count >= EXPECTED_DECRYPTIONS_COUNT,
            "Expected at least {} decryptions, got {}",
            EXPECTED_DECRYPTIONS_COUNT,
            decryption_count
        );

        // Check if IrrationalityDelta is set
        assert!(
            IrrationalityDelta::<Test>::contains_key(TEST_SUBNET_ID),
            "IrrationalityDelta should be set"
        );

        // we actually want this **not** to be set, as whenever weights are sent, the subnet state
        // is **nuked**
        let storage_key = format!("subnet_state:{}", TEST_SUBNET_ID).into_bytes();
        let subnet_state = sp_io::offchain::local_storage_get(
            sp_core::offchain::StorageKind::PERSISTENT,
            &storage_key,
        );

        assert!(
            subnet_state.is_none(),
            "Subnet state should not be set in offchain storage"
        );

        // Verify keep-alive functionality
        let keep_alive_key = b"last_keep_alive";
        assert!(
            sp_io::offchain::local_storage_get(
                sp_core::offchain::StorageKind::PERSISTENT,
                keep_alive_key
            )
            .is_some(),
            "Offchain storage should contain last_keep_alive"
        );
    });
}
