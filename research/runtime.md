# Runtime

## Weight Setting Extrinsic

```rs
pub fn do_set_weights_encrypted(
    origin: T::RuntimeOrigin,
    netuid: u16,
    encrypted_weights: Vec<u8>,
    decrypted_weights_hash: Vec<u8>,
) -> DispatchResult {
    let key = ensure_signed(origin)?;

    if !pallet_subspace::UseWeightsEncryption::<T>::get(netuid) {
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

## Storage Definitions
```rs
#[pallet::storage]
pub type EncryptedWeights<T> = StorageDoubleMap<_, Identity, u16, Identity, u16, Vec<u8>>;

#[pallet::storage]
pub type DecryptedWeightHashes<T> = StorageDoubleMap<_, Identity, u16, Identity, u16, Vec<u8>>;
```
