
#!/bin/bash

source .env

target/release/node-subspace --dev --bridge-enable --bridge-l1-rpc $ETHEREUM_SEPOLIA_RPC --bridge-pk $BRIDGE_PK --bridge-l1-token $L1_TOKEN --bridge-l2-gas $BRIDGE_L2_GAS --bridge-substrate-decimals 12 --bridge-erc20-decimals 18 --bridge-minter $BRIDGE_MINTER

