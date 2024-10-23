use rand::rngs::OsRng;
use rsa::{traits::PublicKeyParts, BigUint, Pkcs1v15Encrypt, RsaPrivateKey, RsaPublicKey};
use sp_core::{sr25519, Pair};
use sp_keystore::{testing::MemoryKeystore, Keystore};
use sp_runtime::KeyTypeId;
use std::io::{Cursor, Read};

pub const KEY_TYPE: KeyTypeId = KeyTypeId(*b"wcs!");

pub struct MockOffworkerExt {
    pub key: Option<rsa::RsaPrivateKey>,
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

pub fn hash(data: Vec<(u16, u16)>) -> Vec<u8> {
    //can be any sha256 lib, this one is used by substrate.
    // dbg!(data.clone());
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
pub fn encrypt(key: (Vec<u8>, Vec<u8>), data: Vec<(u16, u16)>, validator_key: Vec<u8>) -> Vec<u8> {
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
