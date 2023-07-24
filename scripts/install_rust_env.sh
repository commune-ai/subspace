
#!/usr/bin/env bash
# This script is meant to be run on Unix/Linux based systems
set -e

# Install cargo and Rust
curl https://sh.rustup.rs -sSf | sh -s -- -y

export PATH="$HOME/.cargo/bin:$PATH"
source "$HOME/.cargo/env"
echo "*** Initialized WASM build environment with Rust 1.68.1"

# Install cargo and Rust nightly
rustup update
rustup update nightly

rustup install nightly-2023-01-01
rustup override set nightly-2023-01-01
rustup target add wasm32-unknown-unknown

# apt-get install libclang-dev
# apt-get install protobuf-compiler


