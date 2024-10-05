/// CHECK SPELLING!
// WILL BE MOVED TO A GITHUB GIST
// Generate overview
# Required Steps

## 1. [Client Side]
- [ ] Define needed dependencies
  - Description: List and configure all necessary dependencies for the client-side.
- [ ] Extrinsic call for submitting encrypted weights & hashes
  - Description: Implement the client-side logic for calling the extrinsic to submit encrypted weights and hashes.
- [ ] Weight hashing function
  - Description: Implement a function to hash weights on the client-side.
- [ ] Weight Encryption function
  - Description: Implement a function to encrypt weights on the client-side.

## 2. [Runtime]
- [ ] Define needed dependencies
  - Description: List and configure all necessary dependencies for the runtime.

- [ ] Extrinsic call for submitting encrypted weights & hashes
  - Description: Implement the runtime logic to handle the submission of encrypted weights and hashes.
- [ ] Extrinsic for accepting decrypted weights from off-chain worker
  - Description: Implement an extrinsic to accept decrypted weights from the off-chain worker.

- [ ] Storage for encrypted weights & hashes
  - Description: Implement storage mechanisms for encrypted weights and hashes.
- [ ] Storage for decryption keys
  - Description: Implement storage for decryption keys storage.

- [ ] Decryption key rotation mechanism
  - Description: Implement a mechanism to rotate and manage decryption keys.
- [ ] Implement hash comparison verification
  - Description: Implement a function to compare hashes of decrypted weights.

- [ ] Consensus Executor
  - Description: Implement a function to execute consensus upon receiving decrypted weights from the offchain worker.
- [ ] Consensus output storage
  - Description: Implement storage for consensus output without immediate application.
- [ ] Consensus parameter storage
  - Description: Implement storage for all parameters needed to run consensus.

## 3. [Off-chain worker]
- [ ] Define needed dependencies
  - Description: List and configure all necessary dependencies for the off-chain worker.
- [ ] Weight decryption function
  - Description: Implement a function to decrypt weights in the off-chain worker.
- [ ] Weight copying profitability calculation
  - Description: Implement logic to calculate the profitability of weight copying.
- [ ] Weight submission extrinsic
  - Description: Implement an extrinsic to submit decrypted weights back to the runtime.
