use std::{
    io::{Cursor, Read},
    path::{Path, PathBuf},
};

use rsa::{pkcs1::DecodeRsaPrivateKey, traits::PublicKeyParts, Pkcs1v15Encrypt};

pub struct Decrypter {
    key: Option<rsa::RsaPrivateKey>,
}

impl Decrypter {
    pub fn new(rsa_key_path: Option<PathBuf>) -> Self {
        let decryption_key_path = if let Some(rsa_key_path) = rsa_key_path {
            rsa_key_path
        } else {
            Path::new("decryption.pem").to_path_buf()
        };

        if !decryption_key_path.exists() {
            eprintln!("node does not have a decryption key configured");
            return Self { key: None };
        }

        let Ok(content) = std::fs::read_to_string(decryption_key_path) else {
            eprintln!("node does not have a decryption key configured");
            return Self { key: None };
        };

        match rsa::RsaPrivateKey::from_pkcs1_pem(&content) {
            Ok(key) => {
                eprintln!("node started with decryption key configured");
                Self { key: Some(key) }
            }
            Err(err) => {
                eprintln!("failed to load decryption key for node: {err:?}");
                Self { key: None }
            }
        }
    }
}

impl ow_extensions::OffworkerExtension for Decrypter {
    fn decrypt_weight(&self, encrypted: Vec<u8>) -> Option<(Vec<(u16, u16)>, Vec<u8>)> {
        let Some(key) = &self.key else {
            return None;
        };

        let vec = encrypted
            .chunks(key.size())
            .map(|chunk| key.decrypt(Pkcs1v15Encrypt, chunk))
            .collect::<Result<Vec<Vec<u8>>, _>>()
            .ok()?;

        let decrypted = vec.into_iter().flatten().collect::<Vec<_>>();

        let mut cursor = Cursor::new(&decrypted);
        let length = read_u32(&mut cursor)?;

        let mut res = Vec::with_capacity(length as usize);
        for _ in 0..length {
            let uid = read_u16(&mut cursor)?;
            let weight = read_u16(&mut cursor)?;

            res.push((uid, weight));
        }

        let mut key = Vec::new();
        cursor.read_to_end(&mut key).ok()?;

        Some((res, key))
    }

    fn is_decryption_node(&self) -> bool {
        self.key.is_some()
    }

    fn get_encryption_key(&self) -> Option<(Vec<u8>, Vec<u8>)> {
        let Some(key) = &self.key else {
            return None;
        };

        let public = rsa::RsaPublicKey::from(key);
        Some((public.n().to_bytes_be(), public.e().to_bytes_be()))
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
