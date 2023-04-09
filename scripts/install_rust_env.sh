
#!/usr/bin/env bash
# This script is meant to be run on Unix/Linux based systems
set -e

# Install cargo and Rust
curl https://sh.rustup.rs -sSf | sh -s -- -y

export PATH="$HOME/.cargo/bin:$PATH"

source "$HOME/.cargo/env"
# Set Rust toolchain to 1.68.1
# rustup default 1.68.1
rustup install nightly-2023-01-21
rustup default nightly
rustup override set nightly

# Add the wasm32-unknown-unknown target for 1.68.1
rustup target add wasm32-unknown-unknown
# rustup target add wasm32-unknown-unknown --toolchain 1.68.1

echo "*** Initialized WASM build environment with Rust 1.68.1"



