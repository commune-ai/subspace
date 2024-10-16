use crate::mock::*;
use frame_support::{pallet_prelude::BoundedVec, traits::Hooks};
use frame_system::{self};
use ow_extensions::OffworkerExt;
use pallet_offworker::{Call, IrrationalityDelta, Pallet};
use pallet_subnet_emission::Config;
use pallet_subnet_emission_api::SubnetConsensus;
use parity_scale_codec::{Decode, Encode};
use rand::rngs::OsRng;
use rsa::{traits::PublicKeyParts, BigUint, Pkcs1v15Encrypt, RsaPrivateKey, RsaPublicKey};
use serde::{Deserialize, Serialize};
use sp_core::{
    offchain::{testing, OffchainDbExt, OffchainWorkerExt, TransactionPoolExt},
    sr25519, Pair, H256,
};
use sp_keystore::{testing::MemoryKeystore, Keystore, KeystoreExt};
use sp_runtime::{traits::StaticLookup, BuildStorage, KeyTypeId};
use std::collections::BTreeMap;

use pallet_subspace::{
    BondsMovingAverage, FounderShare, LastUpdate, MaxAllowedUids, MaxAllowedWeights,
    MaxEncryptionPeriod, MaxRegistrationsPerBlock, MaxWeightAge, MinValidatorStake,
    RegistrationBlock, Tempo, UseWeightsEncrytyption,
};
use std::{
    fs::File,
    io::{Cursor, Read},
    path::PathBuf,
    sync::Arc,
};

use pallet_subnet_emission::{
    subnet_consensus::{util::params::ConsensusParams, yuma::YumaEpoch},
    types::{DecryptionNodeInfo, PublicKey},
    Authorities, DecryptionNodes, PendingEmission, SubnetConsensusType,
};

pub const KEY_TYPE: KeyTypeId = KeyTypeId(*b"btc!");

struct MockOffworkerExt {
    key: Option<rsa::RsaPrivateKey>,
}

impl Default for MockOffworkerExt {
    fn default() -> Self {
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

        // Generate an RSA key pair
        let mut rng = OsRng;
        let bits = 2048;
        let private_key = RsaPrivateKey::new(&mut rng, bits).expect("Failed to generate RSA key");
        let public_key = RsaPublicKey::from(&private_key);

        // Store the RSA public key components in the keystore
        let n = public_key.n().to_bytes_be();
        let e = public_key.e().to_bytes_be();
        let combined_key = [n, e].concat();
        keystore
            .insert(KEY_TYPE, "rsa_public_key", &combined_key)
            .expect("Failed to store RSA public key");

        Self {
            key: Some(private_key),
        }
    }
}

impl ow_extensions::OffworkerExtension for MockOffworkerExt {
    fn decrypt_weight(&self, encrypted: Vec<u8>) -> Option<(Vec<(u16, u16)>, Vec<u8>)> {
        let Some(key) = &self.key else {
            return None;
        };

        let Some(vec) = encrypted
            .chunks(key.size())
            .map(|chunk| match key.decrypt(Pkcs1v15Encrypt, &chunk) {
                Ok(decrypted) => Some(decrypted),
                Err(_) => None,
            })
            .collect::<Option<Vec<Vec<u8>>>>()
        else {
            return None;
        };

        let decrypted = vec.into_iter().flatten().collect::<Vec<_>>();

        let mut weights = Vec::new();

        let mut cursor = Cursor::new(&decrypted);

        let Some(length) = read_u32(&mut cursor) else {
            return None;
        };
        for _ in 0..length {
            let Some(uid) = read_u16(&mut cursor) else {
                return None;
            };

            let Some(weight) = read_u16(&mut cursor) else {
                return None;
            };

            weights.push((uid, weight));
        }

        let mut key = Vec::new();
        cursor.read_to_end(&mut key).ok()?;

        Some((weights, key))
    }

    fn is_decryption_node(&self) -> bool {
        self.key.is_some()
    }

    fn get_encryption_key(&self) -> Option<(Vec<u8>, Vec<u8>)> {
        let Some(key) = &self.key else {
            return None;
        };

        let public = rsa::RsaPublicKey::from(key);
        Some((public.n().to_bytes_be(), public.e().to_bytes_le()))
    }
}

fn read_u32(cursor: &mut Cursor<&Vec<u8>>) -> Option<u32> {
    let mut buf: [u8; 4] = [0u8; 4];
    match cursor.read_exact(&mut buf[..]) {
        Ok(()) => Some(u32::from_be_bytes(buf)),
        Err(_) => None,
    }
}

fn read_u16(cursor: &mut Cursor<&Vec<u8>>) -> Option<u16> {
    let mut buf = [0u8; 2];
    match cursor.read_exact(&mut buf[..]) {
        Ok(()) => Some(u16::from_be_bytes(buf)),
        Err(_) => None,
    }
}

fn setup_subnet(netuid: u16, tempo: u64) {
    register_subnet(u32::MAX, 0).unwrap();
    zero_min_burn();
    SubnetConsensusType::<Test>::set(netuid, Some(SubnetConsensus::Yuma));
    Tempo::<Test>::insert(netuid, tempo as u16);

    BondsMovingAverage::<Test>::insert(netuid, 0);
    UseWeightsEncrytyption::<Test>::set(netuid, true);

    MaxWeightAge::<Test>::set(netuid, 50_000);
    MinValidatorStake::<Test>::set(netuid, to_nano(10));

    // Things that should never expire / exceed
    MaxEncryptionPeriod::<Test>::set(netuid, u64::MAX);
    MaxRegistrationsPerBlock::<Test>::set(u16::MAX);
    MaxAllowedUids::<Test>::set(netuid, u16::MAX);
    MaxAllowedWeights::<Test>::set(netuid, u16::MAX);
    FounderShare::<Test>::set(netuid, 0);
}

#[derive(Serialize, Deserialize, Debug)]
struct MsgPackValue {
    weights: BTreeMap<String, BTreeMap<String, BTreeMap<String, Vec<Vec<u64>>>>>,
    stake: BTreeMap<String, u64>,
    last_update: BTreeMap<String, BTreeMap<String, u64>>,
    registration_blocks: BTreeMap<String, BTreeMap<String, u64>>,
}

fn load_msgpack_data() -> MsgPackValue {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("src/data/sn31_sim.msgpack");
    let mut file = File::open(path).expect("Failed to open sn31_sim.msgpack");
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).expect("Failed to read file");

    rmp_serde::from_slice(&buffer).expect("Failed to parse msgpack")
}

fn register_modules_from_msgpack(data: &MsgPackValue, netuid: u16) {
    let stake_map = &data.stake;
    let mut sorted_uids: Vec<u16> =
        stake_map.keys().filter_map(|uid_str| uid_str.parse::<u16>().ok()).collect();
    sorted_uids.sort_unstable();

    for uid_str in &sorted_uids {
        if let Some(&stake) = stake_map.get(&uid_str.to_string()) {
            register_module(netuid, *uid_str as u32, stake, false).unwrap();
        }
    }
}

fn make_parameter_consensus_overwrites(
    netuid: u16,
    block: u64,
    data: &MsgPackValue,
    copier_last_update: Option<u64>,
) {
    let mut last_update_vec = get_value_for_block("last_update", block, data);
    if let Some(copier_last_update) = copier_last_update {
        last_update_vec.push(copier_last_update);
    }

    LastUpdate::<Test>::set(netuid, last_update_vec);

    let registration_blocks_vec = get_value_for_block("registration_blocks", block, data);
    registration_blocks_vec.iter().enumerate().for_each(|(i, &block)| {
        RegistrationBlock::<Test>::set(netuid, i as u16, block);
    });
}

fn get_value_for_block(module: &str, block_number: u64, data: &MsgPackValue) -> Vec<u64> {
    let block_str = block_number.to_string();
    match module {
        "last_update" => data
            .last_update
            .get(&block_str)
            .map(|m| m.values().cloned().collect())
            .unwrap_or_default(),
        "registration_blocks" => data
            .registration_blocks
            .get(&block_str)
            .map(|m| m.values().cloned().collect())
            .unwrap_or_default(),
        _ => Vec::new(),
    }
}

fn hash(data: Vec<(u16, u16)>) -> Vec<u8> {
    //can be any sha256 lib, this one is used by substrate.
    sp_io::hashing::sha2_256(&weights_to_blob(&data.clone()[..])[..]).to_vec()
}

fn weights_to_blob(weights: &[(u16, u16)]) -> Vec<u8> {
    let mut encoded = Vec::new();
    encoded.extend((weights.len() as u32).to_be_bytes());
    encoded.extend(weights.iter().flat_map(|(uid, weight)| {
        vec![uid.to_be_bytes(), weight.to_be_bytes()].into_iter().flat_map(|a| a)
    }));

    encoded
}

// the key needs to be retrieved from the blockchain
fn encrypt(key: (Vec<u8>, Vec<u8>), data: Vec<(u16, u16)>, validator_key: Vec<u8>) -> Vec<u8> {
    let rsa_key = RsaPublicKey::new(
        BigUint::from_bytes_be(&key.0),
        BigUint::from_bytes_be(&key.1),
    )
    .expect("Failed to create RSA key");

    let encoded = [
        (data.len() as u32).to_be_bytes().to_vec(),
        data.into_iter()
            .flat_map(|(uid, weight)| {
                uid.to_be_bytes().into_iter().chain(weight.to_be_bytes().into_iter())
            })
            .collect(),
        validator_key,
    ]
    .concat();

    let max_chunk_size = rsa_key.size() - 11; // 11 bytes for PKCS1v15 padding

    encoded
        .chunks(max_chunk_size)
        .flat_map(|chunk| {
            rsa_key.encrypt(&mut OsRng, Pkcs1v15Encrypt, chunk).expect("Encryption failed")
        })
        .collect()
}

// Helper function to set up the test environment
fn new_test_ext(
    mock_offworker_ext: MockOffworkerExt,
) -> (
    sp_io::TestExternalities,
    std::sync::Arc<parking_lot::RwLock<testing::PoolState>>,
    std::sync::Arc<parking_lot::RwLock<testing::OffchainState>>,
    AccountId,
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

    // ! giga gambiarra alert. is this too much voodoo?
    let acc_id: AccountId = H256::from(public.0).using_encoded(|e| {
        let mut buf = [0u8; 4];
        buf.copy_from_slice(&e[0..4]);
        u32::from_le_bytes(buf)
    });

    sp_tracing::try_init_simple();
    let t = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap();
    let mut ext = sp_io::TestExternalities::new(t);
    ext.register_extension(OffchainWorkerExt::new(offchain.clone()));
    ext.register_extension(OffchainDbExt::new(offchain));
    ext.register_extension(TransactionPoolExt::new(pool));
    ext.register_extension(OffworkerExt::new(mock_offworker_ext));
    ext.register_extension(KeystoreExt(Arc::new(keystore)));

    (ext, pool_state, offchain_state, acc_id)
}
/// This is the subnet id specifid in the data/...weights_stake.json
/// We are using real network data to perform the tests
const SAMPLE_SUBNET_ID: &str = "31";
const TEST_SUBNET_ID: u16 = 0;
/// Make sure the tempo 100% matches
const SUBNET_TEMPO: u64 = 360;
const PENDING_EMISSION: u64 = to_nano(1_000);
const EXPECTED_DECRYPTIONS_COUNT: u64 = 2;

#[test]
fn test_offchain_worker_behavior() {
    let mock_offworker_ext = MockOffworkerExt::default();
    let (mut ext, pool_state, _offchain_state, acc_id) = new_test_ext(mock_offworker_ext);

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

        let authorities: BoundedVec<(AccountId, PublicKey), <Test as Config>::MaxAuthorities> =
            vec![(acc_id, (public_key.0.to_vec(), public_key.1.to_vec()))]
                .try_into()
                .expect("Should not exceed max authorities");

        Authorities::<Test>::put(authorities);

        let decryption_info = DecryptionNodeInfo {
            account_id: acc_id,
            public_key,
            last_keep_alive: first_block,
        };
        let decryption_nodes = vec![decryption_info.clone()];
        DecryptionNodes::<Test>::set(decryption_nodes);

        // Run all important things in on initialize hooks
        PendingEmission::<Test>::set(TEST_SUBNET_ID, PENDING_EMISSION);
        step_block(SUBNET_TEMPO as u16);
        let mut decryption_count = 0;
        let mut last_block = 0;

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
                        let encrypted_weights = encrypt(
                            decryption_info.public_key.clone(),
                            weight_vec.clone(),
                            validator_key,
                        );
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
            while let Some(tx) = pool_state.write().transactions.pop() {
                dbg!("processing tx");
                let call = Extrinsic::decode(&mut &*tx).unwrap();
                if let RuntimeCall::OffWorkerMod(Call::send_decrypted_weights {
                    payload,
                    signature,
                }) = call.call
                {
                    assert_eq!(payload.subnet_id, TEST_SUBNET_ID);
                    assert!(!payload.decrypted_weights.is_empty());
                    log::info!("decryption event on block: {}", block_number);

                    // Execute the extrinsic
                    let signer_u64 = match call.signature {
                        Some((account_id, _extra)) => account_id,
                        None => {
                            log::error!("Unsigned extrinsic encountered");
                            continue; // Skip this transaction
                        }
                    };

                    // Convert u64 to AccountId
                    let signer =
                        <Test as frame_system::Config>::Lookup::unlookup(signer_u64 as u32);

                    // Execute the extrinsic
                    let origin = frame_system::RawOrigin::Signed(signer).into();
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
                last_block = block_number;
                break;
            }
        }

        // Assert that we've processed at least 5 decryptions
        //
        // Move to 5 later
        assert!(
            decryption_count >= EXPECTED_DECRYPTIONS_COUNT,
            "Expected at least 5 decryptions, got {}",
            decryption_count
        );

        // Check if IrrationalityDelta is set
        dbg!(IrrationalityDelta::<Test>::iter().collect::<Vec<_>>());
        assert!(
            IrrationalityDelta::<Test>::contains_key(TEST_SUBNET_ID),
            "IrrationalityDelta should be set"
        );

        // Verify the last processed block in offchain storage
        let storage_key = format!("last_processed_block:{}", TEST_SUBNET_ID).into_bytes();
        let last_processed_block = sp_io::offchain::local_storage_get(
            sp_core::offchain::StorageKind::PERSISTENT,
            &storage_key,
        )
        .and_then(|v| u64::decode(&mut &v[..]).ok())
        .unwrap_or(0);

        assert_eq!(
            last_processed_block, last_block,
            "Last processed block should match"
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
