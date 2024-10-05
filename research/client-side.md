# Client-side

## Rust Encryption Reference
(performed by client)

### Dependencies

```rs
// TODO:
// add needed imports
```

```rs
// TODO:
// add needed imports
fn hash(data: Vec<(u16, u16)>) -> Vec<u8> {
    //can be any sha256 lib, this one is used by substrate.
    sp_io::hashing::sha2_256(&weights_to_blob(&to_hash.clone()[..])[..]).to_vec()
}

// the key needs to be retrieved from the blockchain
fn encrypt(key: (Vec<u8>, Vec<u8>), data: Vec<(u16, u16)>) -> Vec<u8> {
    let mut blob = weights_to_blob(&data[..]);

    let key = rsa::RsaPublicKey::new(
        BigUint::from_bytes_be(&key.0),
        BigUint::from_bytes_be(&key.1),
    )
    .unwrap();

    let res = encoded
        .chunks(key.size())
        .into_iter()
        .flat_map(|chunk| {
            let enc = key.encrypt(&mut OsRng, Pkcs1v15Encrypt, chunk).unwrap();
            dbg!(enc.len());
            enc
        })
        .collect::<Vec<_>>();

    res
}

fn weights_to_blob(weights: &[(u16, u16)]) -> Vec<u8> {
    let mut encoded = Vec::new();
    encoded.extend((weights.len() as u32).to_be_bytes());
    encoded.extend(weights.iter().flat_map(|(uid, weight)| {
        vec![uid.to_be_bytes(), weight.to_be_bytes()].into_iter().flat_map(|a| a)
    }));

    encoded
}
```

## Python Encryption Reference

### Dependencies

```py
import hashlib
from cryptography.hazmat.primitives.asymmetric import rsa, padding
from cryptography.hazmat.primitives import hashes
import os
import SubstrateInterface
```

TODO:
test it

```py
def hash_data(data: list[tuple[int, int]]) -> bytes:
    blob = weights_to_blob(data)
    return hashlib.sha256(blob).digest()

def encrypt(key: tuple[bytes, bytes], data: list[tuple[int, int]]) -> bytes:
    blob = weights_to_blob(data)

    public_numbers = rsa.RSAPublicNumbers(
        e=int.from_bytes(key[1], 'big'),
        n=int.from_bytes(key[0], 'big')
    )
    public_key = public_numbers.public_key()

    chunk_size = public_key.key_size // 8 - 11  # Adjust for PKCS#1 v1.5 padding
    encrypted = b''.join(
        public_key.encrypt(
            chunk,
            padding.PKCS1v15()
        )
        for chunk in (blob[i:i+chunk_size] for i in range(0, len(blob), chunk_size))
    )

    return encrypted

def weights_to_blob(weights: list[tuple[int, int]]) -> bytes:
    encoded = len(weights).to_bytes(4, 'big')
    for uid, weight in weights:
        encoded += uid.to_bytes(2, 'big') + weight.to_bytes(2, 'big')
    return encoded
```

### Weight Encryption Extrinsic

```py
# TODO
```
