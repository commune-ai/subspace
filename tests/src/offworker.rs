use crate::mock::*;
use frame_support::traits::Hooks;
use frame_system::{self, Config};
use ow_extensions::{OffworkerExt, OffworkerExtension};
use pallet_offworker::{Call, IrrationalityDelta, Pallet};
use pallet_subnet_emission::{
    decryption::SubnetDecryptionInfo, ConsensusParameters, SubnetDecryptionData, Weights,
};
use pallet_subspace::{UseWeightsEncrytyption, N};
use parity_scale_codec::Decode;
use sp_core::offchain::{testing, OffchainDbExt, OffchainWorkerExt, TransactionPoolExt};
use sp_keystore::{testing::MemoryKeystore, KeystoreExt};
use sp_runtime::{testing::TestXt, traits::TrailingZeroInput, BuildStorage};
use sp_std::sync::Arc;
use substrate_fixed::types::{I32F32, I64F64};

type AccountId = <Test as Config>::AccountId;

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
        Some(vec![(1, 100), (2, 200), (3, 300)])
    }

    fn is_decryption_node(&self) -> bool {
        self.is_decryption_node
    }

    fn get_encryption_key(&self) -> Option<(Vec<u8>, Vec<u8>)> {
        self.encryption_key.clone()
    }
}

#[test]
fn test_offchain_worker_behavior() {
    let mock_offworker_ext = MockOffworkerExt::new(true, Some((vec![1, 2, 3], vec![4, 5, 6])));
    let (mut ext, pool_state, _offchain_state) = new_test_ext(mock_offworker_ext);

    ext.execute_with(|| {
        let block_number = 1u64;
        let public_key = (vec![1, 2, 3], vec![4, 5, 6]);
        let subnet_id = 0u16;

        UseWeightsEncrytyption::<Test>::set(subnet_id, true);
        SubnetDecryptionData::<Test>::insert(
            subnet_id,
            SubnetDecryptionInfo {
                node_id: 1u16,
                node_public_key: public_key.clone(),
                block_assigned: block_number,
            },
        );
        N::<Test>::set(subnet_id, 5);

        let dummy_params =
            pallet_subnet_emission::subnet_consensus::util::params::ConsensusParams {
                subnet_id,
                current_block: block_number,
                modules: Default::default(),
                token_emission: to_nano(10_000),
                kappa: I32F32::from_num(0.5),
                founder_key: pallet_subnet_emission::subnet_consensus::util::params::AccountKey(
                    AccountId::decode(&mut TrailingZeroInput::zeroes()).unwrap(),
                ),
                founder_emission: to_nano(1_000),
                activity_cutoff: 10_000,
                use_weights_encryption: true,
                max_allowed_validators: Some(100),
                bonds_moving_average: 900_000,
                alpha_values: (I32F32::from_num(0.5), I32F32::from_num(0.5)),
            };
        ConsensusParameters::<Test>::insert(subnet_id, block_number, dummy_params);

        let dummy_weights = vec![(2u16, 100u16), (3u16, 200u16)];
        Weights::<Test>::insert(subnet_id, 1u16, dummy_weights);

        run_to_block(block_number + 1);

        Pallet::<Test>::offchain_worker(block_number.into());

        let storage_key = b"last_keep_alive";
        assert!(
            sp_io::offchain::local_storage_get(
                sp_core::offchain::StorageKind::PERSISTENT,
                storage_key
            )
            .is_some(),
            "Offchain storage should contain last_keep_alive"
        );

        assert!(
            IrrationalityDelta::<Test>::contains_key(0u16),
            "IrrationalityDelta should be set"
        );
    });

    let tx = pool_state.write().transactions.pop().expect("Should have one transaction");
    if let Ok(call) = TestXt::<Call<Test>, ()>::decode(&mut &tx[..]) {
        if let Call::send_keep_alive { public_key: pk } = call.call {
            assert_eq!(
                pk,
                (vec![1, 2, 3], vec![4, 5, 6]),
                "Expected send_keep_alive call with correct public key"
            );
        } else {
            panic!("Expected send_keep_alive call");
        }
    } else {
        panic!("Failed to decode transaction");
    }

    let tx = pool_state.write().transactions.pop().expect("Should have another transaction");
    if let Ok(call) = TestXt::<Call<Test>, ()>::decode(&mut &tx[..]) {
        if let Call::send_decrypted_weights {
            subnet_id: sent_subnet_id,
            decrypted_weights,
            delta,
        } = call.call
        {
            assert_eq!(sent_subnet_id, 0u16);
            assert!(
                !decrypted_weights.is_empty(),
                "Decrypted weights should not be empty"
            );
            assert_ne!(delta, I64F64::from_num(0), "Delta should not be zero");
        } else {
            panic!("Expected send_decrypted_weights call");
        }
    } else {
        panic!("Failed to decode transaction");
    }
}

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
    let store = MemoryKeystore::new();

    let t = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap();

    let mut ext = sp_io::TestExternalities::new(t);
    ext.register_extension(OffchainWorkerExt::new(offchain.clone()));
    ext.register_extension(OffchainDbExt::new(offchain));
    ext.register_extension(TransactionPoolExt::new(pool));
    ext.register_extension(OffworkerExt::new(mock_offworker_ext));
    ext.register_extension(KeystoreExt(Arc::new(store)));

    (ext, pool_state, offchain_state)
}
