<p align="center">
  <picture>
    <img alt="subspace" src="https://raw.githubusercontent.com/LVivona/subspace/refs/heads/chore/LVivona/readme-banner/.github/assets/subspace.png" style="max-width: 100%;">
  </picture>
</p>

<p align="center"> <a href="https://github.com/commune-ai/subspace/blob/main/LICENSE"> <img src="https://img.shields.io/badge/license-MIT-blue.svg" alt="License: MIT"> </a><a href="https://discord.gg/communeai"> <img src="https://img.shields.io/discord/308323056592486420?label=discord&logo=discord" alt="Discord"> </a> </p>


Subspace is a FRAME-based [Substrate](https://substrate.io/) blockchain node
that provides the foundation for [Commune's](https://www.communeai.org/)
network. It serves as the trusted base layer responsible for consensus, module
advertising, and peer discovery.

## Table of Contents

- [Overview](#overview)
- [System Requirements](#system-requirements)
- [Installation](#installation)
- [Usage](#usage)
  - [Build](#build)
  - [Run](#run)
  - [Test](#test)
- [Architecture](#architecture)
- [Contributing](#contributing)
- [Acknowledgments](#acknowledgments)

## Overview

Subspace is built using [Substrate](https://substrate.io/), a framework for
developing scalable and upgradeable blockchains. It provides the core
functionality and security needed for Commune's platform:

1. Implements Commune's consensus mechanism
2. Advertises cluster modules and their IP addresses
3. Enables peer discovery for nodes to connect with each other

## System Requirements

- Supported OSs: Linux, MacOS
- Supported Architectures: x86_64
- Memory: ~ 286MB
- Disk: ~500MB
- Network: Public IPv4 address, TCP ports 9944, 30333 open

## Installation

1. Complete the [basic Rust setup instructions](./docs/rust-setup.md).

2. Clone this repository:

```sh
git clone https://github.com/commune-network/subspace.git
cd subspace/
```

## Usage

### Build

To build the node without launching it, run:

```sh
cargo build --release
```

### Run

To run a single development node with ephemeral storage:

```sh
./target/release/node-subspace --chain specs/local.json
```

This will start a Subspace node with a clean state. The node's state will be
discarded on exit.

To retain the node's state between runs, specify a base path:

```sh
mkdir my-chain-state/
./target/release/node-subspace --dev --base-path ./my-chain-state/  
```

Other useful commands:

```sh
# Purge chain state
./target/release/node-subspace purge-chain --dev

# Detailed logging
RUST_BACKTRACE=1 ./target/release/subspace-ldebug --dev

# Explore parameters and subcommands 
./target/release/node-subspace -h
```

### Test

To run all tests:

```sh
cargo test --all
```

To run specific tests:

```sh
cargo test -p pallet-subspace --test test_voting
```

To run tests with detailed logs:

```sh
SKIP_WASM_BUILD=1 RUST_LOG=runtime=debug cargo test -- --nocapture  
```

## Architecture

Subspace leverages the modular and extensible architecture of Substrate. It uses
FRAME pallets to encapsulate domain-specific logic such as consensus, storage,
and p2p networking.

Notable components:

- `/node`: Implementation of the Subspace node including networking, consensus, and RPC
- `/runtime`: The core blockchain logic responsible for validating and executing state transitions
- `/pallets`: Custom FRAME pallets with Commune-specific logic

## Contributing

We welcome contributions to Subspace! Feel free to submit issues, fork the
repository and send pull requests.

Please make sure your code follows the house coding style and passes all tests
before submitting. See [CONTRIBUTING.md](docs/CONTRIBUTING.md) for detailed
guidelines.

Join our [Discord community](https://discord.gg/communeai) to discuss the
project, ask questions and meet other contributors.

## Acknowledgments

Special thanks to the teams at [Parity Technologies](https://www.parity.io/) and
[Web3 Foundation](https://web3.foundation/) for their work on Substrate and
FRAME.
