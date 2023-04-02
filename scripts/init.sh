#!/usr/bin/env bash
# This script is meant to be run on Unix/Linux based systems
set -e

# Install cargo and Rust
curl https://sh.rustup.rs -sSf | sh -s -- -y
PATH="/root/.cargo/bin:${PATH}"


echo "*** Initializing WASM build environment"

rustup install nightly-2023-01-01

rustup target add wasm32-unknown-unknown --toolchain nightly-2023-01-01-x86_64-unknown-linux-gnu
