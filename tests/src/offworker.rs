use crate::mock::*;
use frame_support::traits::Hooks;
use frame_system::{self};
use ow_extensions::{OffworkerExt, OffworkerExtension};
use pallet_offworker::{Call, IrrationalityDelta, Pallet};
use pallet_subnet_emission_api::SubnetConsensus;

use parity_scale_codec::Decode;
use serde_json::Value;
use sp_core::{
    offchain::{testing, OffchainDbExt, OffchainWorkerExt, TransactionPoolExt},
    sr25519, Pair,
};
use sp_keystore::{testing::MemoryKeystore, Keystore, KeystoreExt};
use sp_runtime::{testing::TestXt, BuildStorage, KeyTypeId};
use sp_std::sync::Arc;
use substrate_fixed::types::I64F64;

use pallet_subspace::{
    BondsMovingAverage, FounderShare, LastUpdate, MaxAllowedUids, MaxAllowedWeights,
    MaxEncryptionPeriod, MaxRegistrationsPerBlock, MaxWeightAge, MinValidatorStake,
    RegistrationBlock, Tempo, UseWeightsEncrytyption,
};
use std::{fs::File, io::Read, path::PathBuf};

use pallet_subnet_emission::SubnetConsensusType;

pub struct MockOffworkerExt {
    is_decryption_node: bool,
    encryption_key: Option<(Vec<u8>, Vec<u8>)>,
}

impl MockOffworkerExt {
    pub fn new(is_decryption_node: bool, encryption_key: Option<(Vec<u8>, Vec<u8>)>) -> Self {
        Self {
            is_decryption_node,
            encryption_key,
        }
    }
}

impl OffworkerExtension for MockOffworkerExt {
    fn decrypt_weight(&self, _encrypted: Vec<u8>) -> Option<Vec<(u16, u16)>> {
        None
    }

    fn is_decryption_node(&self) -> bool {
        self.is_decryption_node
    }

    fn get_encryption_key(&self) -> Option<(Vec<u8>, Vec<u8>)> {
        self.encryption_key.clone()
    }
}

/// This is the subnet id specifid in the data/...weights_stake.json
/// We are using real network data to perform the tests
const SAMPLE_SUBNET_ID: &str = "31";
const TEST_NETUID: u16 = 0;

#[test]
fn test_offchain_worker_behavior() {
    let mock_offworker_ext = MockOffworkerExt::new(true, Some((vec![1, 2, 3], vec![4, 5, 6])));
    let (mut ext, pool_state, _offchain_state) = new_test_ext(mock_offworker_ext);

    ext.execute_with(|| {
        let json = load_json_data();
        let first_block =
            json["weights"].as_object().unwrap().keys().next().unwrap().parse().unwrap();

        // Register and setup subnet
        setup_subnet(TEST_NETUID, 360);

        // Register all modules from scratch
        register_modules_from_json(&json, TEST_NETUID);

        // Set block number the simulation will start from
        System::set_block_number(first_block);

        // Overwrite last update and registration blocks
        make_parameter_consensus_overwrites(TEST_NETUID, first_block, &json, None);

        UseWeightsEncrytyption::<Test>::set(TEST_NETUID, true);

        // TODO:
        // make sure that the consensus parameters are saved into the runtime storage
        // `ConsensusParameters`

        let mut decryption_count = 0;
        let mut last_block = 0;

        for (block_number, block_weights) in json["weights"].as_object().unwrap() {
            let block_number: u64 = block_number.parse().unwrap();
            if block_number == first_block {
                continue;
            }

            System::set_block_number(block_number);
            make_parameter_consensus_overwrites(TEST_NETUID, block_number, &json, None);

            let weights: &Value = &block_weights[SAMPLE_SUBNET_ID];

            // Set encrypted weights instead of inserting them
            for (uid, weight_data) in weights.as_object().unwrap() {
                let uid: u16 = uid.parse().unwrap();
                let encrypted_weights = weight_data.to_string().as_bytes().to_vec();
                let decrypted_weights_hash = sp_core::blake2_256(&encrypted_weights).to_vec();

                set_weights_encrypted(
                    TEST_NETUID,
                    SubspaceMod::get_key_for_uid(TEST_NETUID, uid).unwrap(),
                    encrypted_weights,
                    decrypted_weights_hash,
                );
            }

            // Run the offchain worker
            Pallet::<Test>::offchain_worker(block_number.into());

            // Process transactions
            while let Some(tx) = pool_state.write().transactions.pop() {
                if let Ok(call) = TestXt::<Call<Test>, ()>::decode(&mut &tx[..]) {
                    if let Call::send_decrypted_weights {
                        subnet_id,
                        decrypted_weights,
                        delta,
                    } = call.call
                    {
                        assert_eq!(subnet_id, TEST_NETUID);
                        assert!(!decrypted_weights.is_empty());
                        assert_ne!(delta, I64F64::from_num(0));
                        decryption_count += 1;
                    }
                }
            }

            if decryption_count >= 5 {
                last_block = block_number;
                break;
            }
        }

        // Assert that we've processed at least 5 decryptions
        assert!(
            decryption_count >= 5,
            "Expected at least 5 decryptions, got {}",
            decryption_count
        );

        // Check if IrrationalityDelta is set
        assert!(
            IrrationalityDelta::<Test>::contains_key(TEST_NETUID),
            "IrrationalityDelta should be set"
        );

        // Verify the last processed block in offchain storage
        let storage_key = format!("last_processed_block:{}", TEST_NETUID).into_bytes();
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

fn setup_subnet(netuid: u16, tempo: u64) {
    register_subnet(u32::MAX, 0).unwrap();
    zero_min_burn();
    SubnetConsensusType::<Test>::set(netuid, Some(SubnetConsensus::Yuma));
    Tempo::<Test>::insert(netuid, tempo as u16);

    BondsMovingAverage::<Test>::insert(netuid, 0);
    UseWeightsEncrytyption::<Test>::set(netuid, false);

    MaxWeightAge::<Test>::set(netuid, 50_000);
    MinValidatorStake::<Test>::set(netuid, to_nano(10));

    // Things that should never expire / exceed
    MaxEncryptionPeriod::<Test>::set(netuid, u64::MAX);
    MaxRegistrationsPerBlock::<Test>::set(u16::MAX);
    MaxAllowedUids::<Test>::set(netuid, u16::MAX);
    MaxAllowedWeights::<Test>::set(netuid, u16::MAX);
    FounderShare::<Test>::set(netuid, 0);
}

fn load_json_data() -> Value {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("src/data/sn31_weights_stake.json");
    let mut file = File::open(path).expect("Failed to open weights_stake.json");
    let mut contents = String::new();
    file.read_to_string(&mut contents).expect("Failed to read file");
    serde_json::from_str(&contents).expect("Failed to parse JSON")
}

fn register_modules_from_json(json: &Value, netuid: u16) {
    if let Some(stake_map) = json["stake"].as_object() {
        let mut sorted_uids: Vec<u16> =
            stake_map.keys().filter_map(|uid_str| uid_str.parse::<u16>().ok()).collect();
        sorted_uids.sort_unstable();

        sorted_uids.iter().for_each(|&uid| {
            if let Some(stake_value) = stake_map.get(&uid.to_string()) {
                let stake: u64 = stake_value.as_u64().expect("Failed to parse stake value");
                register_module(netuid, uid as u32, stake, false).unwrap();
            }
        });
    }
}

fn make_parameter_consensus_overwrites(
    netuid: u16,
    block: u64,
    json: &Value,
    copier_last_update: Option<u64>,
) {
    let mut last_update_vec = get_value_for_block("last_update", block, &json);
    if let Some(copier_last_update) = copier_last_update {
        last_update_vec.push(copier_last_update);
    }

    LastUpdate::<Test>::set(netuid, last_update_vec);

    let registration_blocks_vec = get_value_for_block("registration_blocks", block, &json);
    registration_blocks_vec.iter().enumerate().for_each(|(i, &block)| {
        RegistrationBlock::<Test>::set(netuid, i as u16, block);
    });
}

fn get_value_for_block(module: &str, block_number: u64, json: &Value) -> Vec<u64> {
    let stuff = json[module].as_object().unwrap();
    let stuff_vec: Vec<u64> = stuff[&block_number.to_string()]
        .as_object()
        .unwrap()
        .values()
        .filter_map(|v| v.as_u64())
        .collect();
    stuff_vec
}

pub const KEY_TYPE: KeyTypeId = KeyTypeId(*b"btc!");

// Helper function to set up the test environment
fn new_test_ext(
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

    let t = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap();

    let mut ext = sp_io::TestExternalities::new(t);
    ext.register_extension(OffchainWorkerExt::new(offchain.clone()));
    ext.register_extension(OffchainDbExt::new(offchain));
    ext.register_extension(TransactionPoolExt::new(pool));
    ext.register_extension(OffworkerExt::new(mock_offworker_ext));
    ext.register_extension(KeystoreExt(Arc::new(keystore)));

    (ext, pool_state, offchain_state)
}
