use std::iter::zip;

use crate::mock::*;
use pallet_subnet_emission::Weights;
use rand::Rng;
// use rand::{rngs::OsRng, thread_rng, Rng};
// use rsa::{traits::PublicKeyParts, BigUint, Pkcs1v15Encrypt};

// TODO;
// make this run
// fn encrypt(key: (Vec<u8>, Vec<u8>), data: Vec<(u16, u16)>) -> Vec<u8> {
//     let mut encoded = Vec::new();
//     encoded.extend((data.len() as u32).to_be_bytes());
//     encoded.extend(data.iter().flat_map(|(uid, weight)| {
//         vec![uid.to_be_bytes(), weight.to_be_bytes()].into_iter().flat_map(|a| a)
//     }));

//     let key = rsa::RsaPublicKey::new(
//         BigUint::from_bytes_be(&key.0),
//         BigUint::from_bytes_be(&key.1),
//     )
//     .unwrap();

//     let res = encoded
//         .chunks(key.size())
//         .into_iter()
//         .flat_map(|chunk| {
//             let enc = key.encrypt(&mut OsRng, Pkcs1v15Encrypt, chunk).unwrap();
//             dbg!(enc.len());
//             enc
//         })
//         .collect::<Vec<_>>();

//     res
// }

// #[test]
// fn test_rsa() {
//     new_test_ext().execute_with(|| {
//         let mut uids = [0u16; 16];
//         let mut weights = [0u16; 16];

//         rand::thread_rng().fill(&mut uids[..]);
//         rand::thread_rng().fill(&mut weights[..]);

//         let to_encrypt = zip(uids, weights).collect::<Vec<(_, _)>>();

//         let encrypted = encrypt(
//             ow_extensions::offworker::get_encryption_key().unwrap(),
//             to_encrypt.clone(),
//         );

//         let decrypted = ow_extensions::offworker::decrypt_weight(encrypted).unwrap();

//         assert_eq!(decrypted, to_encrypt);
//     });
// }

#[test]
fn test_hash() {
    new_test_ext().execute_with(|| {
        let mut uids = [0u16; 16];
        let mut weights = [0u16; 16];

        rand::thread_rng().fill(&mut uids[..]);
        rand::thread_rng().fill(&mut weights[..]);

        let to_hash = zip(uids, weights).collect::<Vec<(_, _)>>();

        let hash1 = sp_io::hashing::sha2_256(&weights_to_blob(&to_hash.clone()[..])[..]);
        let hash2 = sp_io::hashing::sha2_256(&weights_to_blob(&to_hash.clone()[..])[..]);

        assert_eq!(hash1, hash2);
    });
}

fn weights_to_blob(weights: &[(u16, u16)]) -> Vec<u8> {
    let mut encoded = Vec::new();
    encoded.extend((weights.len() as u32).to_be_bytes());
    encoded.extend(weights.iter().flat_map(|(uid, weight)| {
        vec![uid.to_be_bytes(), weight.to_be_bytes()].into_iter().flat_map(|a| a)
    }));

    encoded
}

#[test]
fn test_update_decrypted_weights() {
    new_test_ext().execute_with(|| {
        let netuid = 1u16;

        // Set up baseline weights in storage
        Weights::<Test>::insert(netuid, 2u16, vec![(1u16, 2u16)]);
        Weights::<Test>::insert(netuid, 3u16, vec![(2u16, 3u16)]);

        // Verify initial storage state
        let initial_storage: Vec<_> = Weights::<Test>::iter().collect();

        assert_eq!(
            initial_storage,
            vec![
                (netuid, 2u16, vec![(1u16, 2u16)]),
                (netuid, 3u16, vec![(2u16, 3u16)])
            ]
        );

        // Create new valid weights for both blocks
        let block_number_1 = 100u64;
        let block_number_2 = 200u64;
        let new_weights_1 = vec![(5u16, vec![(10u16, 20u16)])];
        let new_weights_2 = vec![(6u16, vec![(15u16, 25u16)])];

        let valid_weights = vec![
            (block_number_1, new_weights_1),
            (block_number_2, new_weights_2),
        ];

        // Call the function
        let result =
            pallet_subnet_emission::Pallet::<Test>::update_decrypted_weights(netuid, valid_weights);

        // Update storage with new weights
        if let Some(weights_map) = result.clone() {
            weights_map.iter().for_each(|(_, inner_weights)| {
                inner_weights.iter().for_each(|(uid, weights)| {
                    Weights::<Test>::set(netuid, *uid, Some(weights.clone()));
                });
            });
        }

        // Verify results
        let updated_weights = result.unwrap();

        // Check block 100
        let (_, block_weights_1) = &updated_weights[0];
        assert!(block_weights_1.contains(&(2u16, vec![(1u16, 2u16)])));
        assert!(block_weights_1.contains(&(3u16, vec![(2u16, 3u16)])));
        assert!(block_weights_1.contains(&(5u16, vec![(10u16, 20u16)])));
        assert_eq!(block_weights_1.len(), 3); // Only baseline + block 100 weights

        // Check block 200
        let (_, block_weights_2) = &updated_weights[1];
        assert!(block_weights_2.contains(&(2u16, vec![(1u16, 2u16)])));
        assert!(block_weights_2.contains(&(3u16, vec![(2u16, 3u16)])));
        assert!(block_weights_2.contains(&(6u16, vec![(15u16, 25u16)])));
        assert_eq!(block_weights_2.len(), 3); // Only baseline + block 200 weights

        // Try to overwrite some already existing weights for both blocks
        let new_weights_1 = vec![(2u16, vec![(30u16, 30u16)])];
        let new_weights_2 = vec![(3u16, vec![(40u16, 40u16)])];

        let new_valid_weights = vec![
            (block_number_1, new_weights_1),
            (block_number_2, new_weights_2),
        ];

        let result = pallet_subnet_emission::Pallet::<Test>::update_decrypted_weights(
            netuid,
            new_valid_weights,
        );

        let updated_weights = result.unwrap();

        dbg!(updated_weights.clone());

        // Check updated block 100
        let (_, block_weights_1) = &updated_weights[0];
        assert!(block_weights_1.contains(&(2u16, vec![(30u16, 30u16)]))); // New value for uid 2
        assert!(block_weights_1.contains(&(3u16, vec![(2u16, 3u16)]))); // Unchanged
        assert!(block_weights_1.contains(&(5u16, vec![(10u16, 20u16)]))); // From first update
        assert_eq!(block_weights_1.len(), 4);

        // Check updated block 200
        let (_, block_weights_2) = &updated_weights[1];
        assert!(block_weights_2.contains(&(2u16, vec![(1u16, 2u16)]))); // Unchanged
        assert!(block_weights_2.contains(&(3u16, vec![(40u16, 40u16)]))); // New value for uid 3
        assert!(block_weights_2.contains(&(6u16, vec![(15u16, 25u16)]))); // From first update
        assert_eq!(block_weights_2.len(), 4);
    });
}
