# Subspace

[![Discord Chat](https://img.shields.io/discord/308323056592486420.svg)](https://discord.gg/commune)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Build Status](https://img.shields.io/travis/com/paritytech/substrate/master?label=stable)](https://travis-ci.com/paritytech/substrate)
[![Coverage Status](https://img.shields.io/codecov/c/gh/paritytech/substrate?label=coverage)](https://codecov.io/gh/paritytech/substrate)

Subspace is a FRAME-based [Substrate](https://substrate.io/) blockchain node that provides the foundation for [Commune's](https://commune.network/) decentralized cloud platform. It serves as the trusted base layer responsible for consensus, module advertising, and peer discovery.

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
- [License](#license)
- [Acknowledgements](#acknowledgements)

## Overview
Subspace is built using [Substrate](https://substrate.io/), a framework for developing scalable and upgradeable blockchains. It provides the core functionality and security needed for Commune's platform:
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
```bash
git clone https://github.com/commune-network/subspace.git
cd subspace/
```

## Usage

### Build
To build the node without launching it, run:
```bash
cargo build --release
```

### Run
To run a single development node with ephemeral storage:
```bash
./target/release/node-subspace --dev
```
This will start a Subspace node with a clean state. The node's state will be discarded on exit. 

To retain the node's state between runs, specify a base path:
```bash
mkdir my-chain-state/
./target/release/node-subspace --dev --base-path ./my-chain-state/  
```

Other useful commands:
```bash
# Purge chain state
./target/release/subspace purge-chain --dev

# Detailed logging
RUST_BACKTRACE=1 ./target/release/subspace-ldebug --dev

# Explore parameters and subcommands 
./target/release/node-subspace -h
```

### Test
To run all tests:
```bash
cargo test --all
```

To run specific tests:
```bash
cargo test -p pallet-subspace --test test_voting
```

To run tests with detailed logs:
```bash
SKIP_WASM_BUILD=1 RUST_LOG=runtime=debug cargo test -- --nocapture  
```

## Architecture
Subspace leverages the modular and extensible architecture of Substrate. It uses FRAME pallets to encapsulate domain-specific logic such as consensus, storage, and p2p networking. 

Notable components:
- `/node`: Implementation of the Subspace node including networking, consensus, and RPC 
- `/runtime`: The core blockchain logic responsible for validating and executing state transitions
- `/pallets`: Custom FRAME pallets with Commune-specific logic

## Contributing 
We welcome contributions to Subspace! Feel free to submit issues, fork the repository and send pull requests. 

Please make sure your code follows the house coding style and passes all tests before submitting. See [CONTRIBUTING.md](CONTRIBUTING.md) for detailed guidelines.

Join our [Discord community](https://discord.gg/commune) to discuss the project, ask questions and meet other contributors.

## License
Subspace is licensed under the [MIT License](LICENSE).

## Acknowledgments
Special thanks to the teams at [Parity Technologies](https://www.parity.io/) and [Web3 Foundation](https://web3.foundation/) for their work on Substrate and FRAME.