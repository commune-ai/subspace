#!/bin/bash
# Script to build the WASM runtime for the treasury address migration

set -e

echo "Starting WASM runtime build for treasury address migration..."

# Navigate to the project root
cd "$(dirname "$0")/.."

# Build the runtime with WASM target
echo "Building runtime with WASM target..."
cargo build --release -p node-subspace-runtime --features=runtime-benchmarks

# Check if the build was successful
if [ $? -eq 0 ]; then
    echo "WASM runtime build successful!"
    
    # Copy the WASM blob to a more accessible location
    WASM_PATH="./target/release/wbuild/node-subspace-runtime/node_subspace_runtime.compact.compressed.wasm"
    DEST_PATH="./runtime_upgrade_wasm"
    
    mkdir -p "$DEST_PATH"
    cp "$WASM_PATH" "$DEST_PATH/node_subspace_runtime_treasury_migration.compact.compressed.wasm"
    
    echo "WASM blob copied to $DEST_PATH/node_subspace_runtime_treasury_migration.compact.compressed.wasm"
    
    # Calculate and display the hash of the WASM blob
    WASM_HASH=$(sha256sum "$DEST_PATH/node_subspace_runtime_treasury_migration.compact.compressed.wasm" | cut -d ' ' -f 1)
    echo "WASM blob hash (SHA-256): $WASM_HASH"
    
    echo "This WASM blob can be used for the runtime upgrade proposal."
else
    echo "WASM runtime build failed!"
    exit 1
fi
