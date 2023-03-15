#!/bin/bash
# This script is meant to be run on Unix/Linux based systems

echo "*** Initializing WASM build environment"

if [ -z $CI_PROJECT_NAME ] ; then
   rustup update nightly
   rustup update stable
fi

rustup toolchain install nightly
rustup override set nightly
rustup target add wasm32-unknown-unknown --toolchain nightly
echo [+] Installed toolchain nightly
