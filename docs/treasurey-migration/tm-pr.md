## Pull Request Checklist

Before submitting this PR, please make sure:

- [x] You have run cargo clippy and addressed any warnings
- [x] You have added appropriate tests (if applicable)
- [x] You have updated the documentation (if applicable)
- [x] You have reviewed your own code
- [x] You have updated changelog (if applicable)

## Overview

This PR implements a runtime upgrade to redirect emissions to a new treasury key and addresses critical security issues in the implementation. The upgrade is necessary because the original multi-sig holders have forked the network and are being uncooperative.

## Implementation Details

1. **Migration Module**: Created a new MigrateToV3 struct in the governance pallet that implements the OnRuntimeUpgrade trait.

2. **Treasury Address Update**: The migration updates the DaoTreasuryAddress storage item to the new address: `5GZfkfjD46SmDrnWZbrzkxkYzeJUWKTAB1HvHBurrPc7XcEj`.

3. **Fixed Public Key Format**:
   - Corrected the public key bytes in the migration code to match the SS58 address
   - The previous implementation used ASCII values of the address characters, which would result in an invalid account ID
   - The correct binary representation has been verified using the substrate-interface library

4. **Enhanced Validation and Error Handling**:
   - Added validation to ensure the public key format is correct before using it
   - Implemented checks to prevent migration if the new treasury address is invalid or unchanged
   - Enhanced error logging with detailed messages
   - Added fallback to default account with clear error messages if decoding fails

5. **Added Security Audit Trail**:
   - Implemented event emission for the treasury address update, providing an on-chain audit trail
   - This allows for verification of the migration through block explorers

6. **Weight Calculation Analysis**:
   - Conducted a thorough investigation of weight calculations in the codebase
   - Analyzed benchmarking code and weight patterns in the governance pallet
   - Added detailed comments explaining the weight calculation rationale

7. **Created Validation Tool**:
   - Developed a Python script (`validate_replacement_key.py`) that:
     - Validates the public key bytes against the SS58 address
     - Formats the output in a clear, tabular format for easy comparison
     - Can automatically write the correct bytes to the migration code
     - Features a rich-formatted help menu

8. **Runtime Version**: Incremented the runtime spec version to trigger the migration:
   - Non-testnet: from 132 to 133
   - Testnet: from 515 to 516

9. **Documentation**: Enhanced the treasury migration documentation to include:
   - Additional validation steps in the implementation details
   - Information about the weight calculation analysis
   - Information about the new validation tool
   - Usage examples for the validation tool
   - Comprehensive explanation of the migration process, build instructions, and deployment steps for validators

## Testing

- Validated the public key bytes using the new validation tool
- Verified that the bytes match the SS58 address `5GZfkfjD46SmDrnWZbrzkxkYzeJUWKTAB1HvHBurrPc7XcEj`
- Tested the validation tool's write functionality to ensure it correctly updates the migration code
- Created a test script to verify the migration works correctly in a development environment

## Security Considerations

- The migration now includes multiple layers of validation to ensure the treasury address is correctly updated
- Error handling has been improved to prevent silent failures
- Event emission provides an audit trail for the migration

## Related Issues

- Addresses security concerns identified in the treasury migration security audit
- Resolves the issue with the current treasury multi-sig holders forking the network, which is preventing proper emission distribution

## WASM Build

The WASM blob for this runtime upgrade has been built and is available in the runtime_upgrade_wasm directory. The SHA-256 hash of the WASM blob is:
```
75db3b9a397a30ac70371dd70f06d1b29ae3cc888e62d8a7165195a9bbbe54dd
```