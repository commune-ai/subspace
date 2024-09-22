use std::iter::zip;

use crate::mock::*;
use rand::{rngs::OsRng, thread_rng, Rng};
use rsa::{BigUint, Pkcs1v15Encrypt};

fn encrypt(key: (Vec<u8>, Vec<u8>), data: Vec<(u16, u16)>) -> Vec<u8> {
    let mut encoded = Vec::new();
    encoded.extend((data.len() as u32).to_be_bytes());
    encoded.extend(data.iter().flat_map(|(uid, weight)| {
        vec![uid.to_be_bytes(), weight.to_be_bytes()].into_iter().flat_map(|a| a)
    }));

    let key = rsa::RsaPublicKey::new(
        BigUint::from_bytes_be(&key.0),
        BigUint::from_bytes_be(&key.1),
    )
    .unwrap(); // todo remove unwrap

    dbg!(&key.size());

    let res = encoded
        .chunks(key.size())
        .into_iter()
        .flat_map(|chunk| {
            let enc = key.encrypt(&mut OsRng, Pkcs1v15Encrypt, chunk).unwrap();
            dbg!(enc.len());
            enc
        })
        .collect::<Vec<_>>();

    dbg!(&res.len());
    res
}

#[test]
fn test_rsa() {
    new_test_ext().execute_with(|| {
        let mut uids = [0u16; 16];
        let mut weights = [0u16; 16];

        rand::thread_rng().fill(&mut uids[..]);
        rand::thread_rng().fill(&mut weights[..]);

        let to_encrypt = zip(uids, weights).collect::<Vec<(_, _)>>();

        let encrypted = encrypt(
            ow_extensions::offworker::get_encryption_key().unwrap(),
            to_encrypt.clone(),
        );

        let decrypted = ow_extensions::offworker::decrypt_weight(encrypted).unwrap();

        assert_eq!(decrypted, to_encrypt);
    });
}

#[test]
fn test_hash() {
    new_test_ext().execute_with(|| {
        let mut uids = [0u16; 16];
        let mut weights = [0u16; 16];

        rand::thread_rng().fill(&mut uids[..]);
        rand::thread_rng().fill(&mut weights[..]);

        let to_hash = zip(uids, weights).collect::<Vec<(_, _)>>();

        let hash1 = ow_extensions::offworker::hash_weight(to_hash.clone()).unwrap();
        let hash2 = ow_extensions::offworker::hash_weight(to_hash.clone()).unwrap();

        assert_eq!(hash1, hash2);
    });
}
