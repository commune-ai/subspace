use crate::mock::*;
use rand::{rngs::OsRng, thread_rng, Rng, RngCore};
use rsa::{BigUint, Pkcs1v15Encrypt};

fn encrypt(key: (Vec<u8>, Vec<u8>), data: (Vec<u16>, Vec<u16>)) -> Vec<u8> {
    let mut encoded = Vec::new();
    encoded.extend((data.0.len() as u32).to_be_bytes());
    encoded.extend(data.0.iter().flat_map(|ele| ele.to_be_bytes()));
    encoded.extend((data.1.len() as u32).to_be_bytes());
    encoded.extend(data.1.iter().flat_map(|ele| ele.to_be_bytes()));

    let key = rsa::RsaPublicKey::new(
        BigUint::from_bytes_be(&key.0),
        BigUint::from_bytes_be(&key.1),
    )
    .unwrap();

    let res = encoded
        .chunks(120)
        .into_iter()
        .flat_map(|chunk| {
            let mut random = [0u8; 8];
            thread_rng().fill(&mut random[..]);

            let mut data = Vec::new();
            data.extend(&random[..]);
            data.extend(&chunk[..]);

            dbg!(&data.len());

            key.encrypt(&mut OsRng, Pkcs1v15Encrypt, &data[..]).unwrap()
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
        let encrypted = encrypt(
            testthing::offworker::get_encryption_key(),
            (uids.to_vec(), weights.to_vec()),
        );
        let decrypted = testthing::offworker::decrypt_weight(encrypted).unwrap();

        assert_eq!(decrypted.0, uids);
        assert_eq!(decrypted.1, weights);
    });
}
