#!/usr/bin/env python3

import sys
import os
from binascii import hexlify

def read_wasm_blob(wasm_path):
    """Read the WASM blob from the given path."""
    if not os.path.exists(wasm_path):
        print(f"Error: WASM file not found at {wasm_path}")
        sys.exit(1)
    
    with open(wasm_path, 'rb') as f:
        return f.read()

def prepare_setcode_extrinsic(wasm_blob):
    """Prepare the system.setCode extrinsic with the WASM blob."""
    hex_blob = '0x' + hexlify(wasm_blob).decode('ascii')
    return hex_blob

def main():
    if len(sys.argv) != 2:
        print("Usage: prepare_runtime_upgrade.py <path_to_wasm_file>")
        print("Example: prepare_runtime_upgrade.py ../target/release/wbuild/node-subspace-runtime/node_subspace_runtime.compact.compressed.wasm")
        sys.exit(1)
    
    wasm_path = sys.argv[1]
    wasm_blob = read_wasm_blob(wasm_path)
    hex_blob = prepare_setcode_extrinsic(wasm_blob)
    
    # Print instructions and the hex blob
    print("\nRuntime Upgrade Instructions:")
    print("-----------------------------")
    print("1. Copy the hex blob below")
    print("2. Go to Polkadot-JS Apps > Developer > Extrinsics")
    print("3. Select 'system' pallet and 'setCode(code)' function")
    print("4. Paste the hex blob into the 'code' field")
    print("5. Submit the extrinsic through governance\n")
    print("Hex Blob for system.setCode:")
    print("-----------------------------")
    print(hex_blob)

if __name__ == "__main__":
    main()
