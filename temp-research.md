/// CHECK SPELLING!
// WILL BE MOVED TO A GITHUB GIST
// Generate overview
# Implementation Process

## What Modifications or Additions are Needed ?
- Client-side
  - [ ] [Extrinsic call for submitting encrypted weights & hashes](#weight-encryption-extrinsic)
  - [ ] [Weight hashing function](#i-client-side-weight-encryption)
  - [ ] [Weight Encryption function](#i-client-side-weight-encryption)
- Blockchainj
  - [ ] Extrinsic call for submitting encrypted weights & hashes
  - [ ] Storage for encrypted weights & hashes
  - [ ] Consensus output is stored, not applied immediately
  - [ ] Consensus parameter storage, takes all parameters needed to run consensus
- Off-chain worker
  - [ ] Weight decryption function




# Diving Into Code

## I. Client-side Weight Encryption
- Calculate hash of decrypted weights
- Encrypt validator weights
- Send weights to blockchain via extrinsic

### Rust Encryption Reference
(performed by client)
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

### Python Encryption Reference

TODO:
test it
```py
import hashlib
from cryptography.hazmat.primitives.asymmetric import rsa, padding
from cryptography.hazmat.primitives import hashes
import os

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

## II. Blockchain Extrinsic
- Submit encrypted weights and weight hashes to the blockchain
- Store encrypted weights and hashes on-chain

### Weight Setting Extrinsic

```rs
pub fn do_set_weights_encrypted(
    origin: T::RuntimeOrigin,
    netuid: u16,
    encrypted_weights: Vec<u8>,
    decrypted_weights_hash: Vec<u8>,
) -> DispatchResult {
    let key = ensure_signed(origin)?;

    if !pallet_subspace::UseWeightsEncrytyption::<T>::get(netuid) {
        return Err(pallet_subspace::Error::<T>::SubnetNotEncrypted.into());
    }

    let Some(uid) = pallet_subspace::Pallet::<T>::get_uid_for_key(netuid, &key) else {
        return Err(pallet_subspace::Error::<T>::ModuleDoesNotExist.into());
    };

    Self::handle_rate_limiting(uid, netuid, &key)?;
    Self::remove_rootnet_delegation(netuid, key);

    EncryptedWeights::<T>::set(netuid, uid, Some(encrypted_weights));
    DecryptedWeightHashes::<T>::set(netuid, uid, Some(decrypted_weights_hash));

    Ok(())
}
}
```

### Storage Definitions
```rs
#[pallet::storage]
pub type EncryptedWeights<T> = StorageDoubleMap<_, Identity, u16, Identity, u16, Vec<u8>>;

#[pallet::storage]
pub type DecryptedWeightHashes<T> = StorageDoubleMap<_, Identity, u16, Identity, u16, Vec<u8>>;
```

## III. Off-chain Decryption and Processing
- Designated weight decryption nodes:
  a. Decrypt weight data for assigned subnets
  b. Perform consensus calculations
  c. Run DEW Algorithm to check relative copier profitability constraint
  d. If satisfied, submit decrypted weights back to runtimkdye

### Dependencies

```rs
#![cfg_attr(not(feature = "std"), no_std)]
extern crate alloc;
use alloc::vec::Vec;
use frame_support::traits::Get;
use frame_system::{
    offchain::{
        AppCrypto, CreateSignedTransaction, SendSignedTransaction, SendUnsignedTransaction,
        SignedPayload, Signer, SigningTypes,
    },
    pallet_prelude::BlockNumberFor,
};
use pallet_subnet_emission::{
    subnet_consensus::{
        util::{
            consensus::ConsensusOutput,
            params::{ConsensusParams, ModuleKey, ModuleParams},
        },
        yuma::YumaEpoch,
    },
    EncryptedWeights,
};
use pallet_subspace::TotalStake;
use std::collections::BTreeMap;
use pallet_subnet_emission::Weights;
use pallet_subspace::{
    math::{inplace_normalize_64, vec_fixed64_to_fixed32},
    Active, Consensus, CopierMargin, FloorDelegationFee, MaxEncryptionPeriod,
    Pallet as SubspaceModule, StakeFrom, Tempo, N,
};
use parity_scale_codec::{Decode, Encode};
use scale_info::prelude::marker::PhantomData;
use sp_core::crypto::KeyTypeId;
use sp_runtime::{
    offchain::storage::{StorageRetrievalError, StorageValueRef},
    traits::{BlakeTwo256, Hash},
    Percent, RuntimeDebug,
};
use substrate_fixed::{types::I32F32, FixedI128};
```


## IV. Runtime Handling
- Verify weight authenticity by comparing hashes
- Calculate and distribute consensus for epochs with available decrypted weights
- Store decrypted weights in runtime storage
- Optionally reassign decryption responsibilities periodically

## Key Considerations
- Ensure provable non-tampering of weights
- Prevent weight extraction before public decryption
- Regularly rotate and distribute subnet public decryption keys

## Security Measures
- Hash comparison to prevent tampering
- Even distribution of weights across decryption nodes
- Periodic reassignment of decryption responsibilities


TODO:
discuss conesnsus design
