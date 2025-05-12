# Treasury Address Migration

## Overview

This document outlines the process for migrating the Commune DAO treasury address to a new address. The migration is necessary due to the original multi-sig holders forking the network and being uncooperative.

## Technical Details

### Current Implementation

The current treasury address is stored in the governance pallet's storage as `DaoTreasuryAddress`. This address is used to direct emissions from the subnet emission pallet.

### Migration Plan

The migration will be implemented as a runtime upgrade that updates the `DaoTreasuryAddress` storage item to the new treasury address. The new treasury address is:

```
5GZfkfjD46SmDrnWZbrzkxkYzeJUWKTAB1HvHBurrPc7XcEj
```

### Implementation Details

The migration is implemented in the governance pallet as a storage migration from V2 to V3. The migration code performs the following steps:

1. Converts the new treasury address from SS58 format to an AccountId
2. Updates the `DaoTreasuryAddress` storage item with the new address
3. Increments the storage version to V3

The runtime spec version has been incremented to trigger the migration:
- Non-testnet version: 132 -> 133
- Testnet version: 515 -> 516

## Building the Runtime

### Prerequisites

- Rust and Cargo installed
- Required dependencies: `protobuf-compiler` and `libclang`

### Build Steps

1. Clone the repository:
   ```bash
   git clone https://github.com/commune-ai/subspace.git
   cd subspace
   ```

2. Build the runtime WASM blob:
   ```bash
   ./scripts/build_runtime_wasm.sh
   ```

3. The WASM blob will be available at:
   ```
   ./runtime_upgrade_wasm/node_subspace_runtime_treasury_migration.compact.compressed.wasm
   ```

## Deployment Process

### For Validators

1. **Verify the WASM blob**:
   - Download the WASM blob from the official repository
   - Verify the SHA-256 hash matches the published hash

2. **Prepare for the upgrade**:
   - Ensure your node is running the latest version before the upgrade
   - Make a backup of your node data

3. **Monitor the upgrade**:
   - The upgrade will be submitted through the on-chain governance process
   - Once approved, the runtime will automatically upgrade at the specified block
   - No manual intervention is required if you're running an up-to-date node

### For Governance Council Members

1. **Submit the runtime upgrade proposal**:
   - Use the Polkadot.js UI to submit a runtime upgrade proposal
   - Attach the WASM blob to the proposal
   - Set an appropriate voting period

2. **Vote on the proposal**:
   - Council members should review and vote on the proposal
   - A majority vote is required for the proposal to pass

3. **Enact the upgrade**:
   - Once passed, the upgrade will be scheduled
   - Monitor the chain to ensure the upgrade is applied successfully

## Upgrade Process

### For Validators

1. **Download the Updated Runtime**: Once the runtime upgrade is approved through governance, download the new WASM blob.

2. **Verify the Runtime**: Verify that the WASM blob matches the expected hash.

3. **Apply the Upgrade**: The upgrade will be applied automatically through the governance process.

### For Users

No action is required from users. The migration will be handled automatically by the network.

## Verification

After the upgrade, you can verify that the treasury address has been updated by querying the `DaoTreasuryAddress` storage item in the governance pallet:

```bash
# Using the Subspace CLI
./target/release/node-subspace query storage --pallet governance --name DaoTreasuryAddress
```

The returned address should match the new treasury address: `5GZfkfjD46SmDrnWZbrzkxkYzeJUWKTAB1HvHBurrPc7XcEj`.

## Timeline

- **Development**: May 12, 2025
- **Testing**: May 13-14, 2025
- **Governance Proposal**: May 15, 2025
- **Expected Upgrade**: May 20, 2025 (subject to governance approval)

## Contact

If you have any questions or concerns about this upgrade, please reach out to the Commune team on Discord or through the governance forum.
