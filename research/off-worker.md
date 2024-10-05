
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
