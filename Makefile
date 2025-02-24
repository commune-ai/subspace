PYTHON=python3
RUST_LOG ?= info,pallet_subspace::migrations=debug

.PHONY: down stop up restart start enter chmod_scripts compose try-runtime-upgrade-testnet try-runtime-upgrade-mainnet run-benchmarking run-localnet run-mainnet

down:
	docker-compose down

stop: down

up:
	docker-compose up -d

restart: down up

start: up

enter:
	docker exec -it subspace bash

chmod_scripts:
	chmod +x ./scripts/*.sh

compose:
	docker-compose up -d ${service}

try-runtime-upgrade-testnet:
	cargo build --release --features "try-runtime,testnet"
	RUST_BACKTRACE=1; RUST_LOG="${RUST_LOG}"; try-runtime --runtime target/release/wbuild/node-subspace-runtime/node_subspace_runtime.compact.compressed.wasm on-runtime-upgrade --blocktime 8000 live --uri wss://testnet.api.communeai.net:443

try-runtime-upgrade-mainnet:
	cargo build --release --features try-runtime
	RUST_BACKTRACE=1; RUST_LOG="${RUST_LOG}"; try-runtime --runtime target/release/wbuild/node-subspace-runtime/node_subspace_runtime.compact.compressed.wasm on-runtime-upgrade --blocktime 8000 live --uri wss://api.communeai.net:443

try-runtime-upgrade-devnet:
	cargo build --release --features try-runtime
	RUST_BACKTRACE=1; RUST_LOG="${RUST_LOG}"; try-runtime --runtime target/release/wbuild/node-subspace-runtime/node_subspace_runtime.compact.compressed.wasm on-runtime-upgrade --blocktime 8000 live --uri wss://devnet-commune-api-node-0.communeai.net:443


run-benchmarking:
	cargo build -r --features runtime-benchmarks
	./target/release/node-subspace build-spec --disable-default-bootnode --chain local > specs/benchmarks.json
	./target/release/node-subspace benchmark pallet --chain specs/local.json --pallet pallet_subspace  --extrinsic "*" --steps 50 --repeat 20 --output pallets/subspace/src/weights.rs --template=./.maintain/frame-weight-template.hbs
	./target/release/node-subspace benchmark pallet --chain specs/local.json --pallet pallet_governance  --extrinsic "*" --steps 50 --repeat 20 --output pallets/governance/src/weights.rs --template=./.maintain/frame-weight-template.hbs
	./target/release/node-subspace benchmark pallet --chain specs/local.json --pallet pallet_emission  --extrinsic "*" --steps 50 --repeat 20 --output pallets/emission/src/weights.rs --template=./.maintain/frame-weight-template.hbs

specs/mainnet-copy.json:
	$(PYTHON) scripts/snapshots/builder.py -o specs/mainnet-copy.json

run-localnet:
	cargo xtask run --alice

run-mainnet: specs/mainnet-copy.json
	cargo xtask run --alice --chain-spec specs/mainnet-copy.json
