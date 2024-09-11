use crate::mock::*;
use rand::{rngs::OsRng, RngCore};
use rsa::{BigUint, Pkcs1v15Encrypt};

fn encrypt(key: (Vec<u8>, Vec<u8>), data: Vec<u8>) -> Vec<u8> {
    let key = rsa::RsaPublicKey::new(
        BigUint::from_bytes_be(&key.0),
        BigUint::from_bytes_be(&key.1),
    )
    .unwrap();

    key.encrypt(&mut OsRng, Pkcs1v15Encrypt, &data[..]).unwrap()
}

#[test]
fn test_rsa() {
    new_test_ext().execute_with(|| {
        let mut data = Vec::with_capacity(256);

        rand::thread_rng().fill_bytes(&mut data);
        let encrypted = encrypt(testthing::offworker::get_encryption_key(), data.clone());
        let decrypted = testthing::offworker::decrypt_weight(encrypted).unwrap();

        assert_eq!(decrypted, data);
    });
}
