
PYTHON=python3
down:
	docker-compose down
stop:
	make down
up:
	docker-compose up -d
restart:
	make down && make up
start:
	make start
enter:
	docker exec -it subspace bash
chmod_scripts:
	chmod +x ./scripts/*.sh
compose:
	docker-compose up -d ${service}

RUST_LOG ?= info,pallet_subspace::migrations=debug

try-runtime-upgrade:
	cargo build --release --features try-runtime
	RUST_BACKTRACE=1; RUST_LOG="${RUST_LOG}"; try-runtime --runtime target/release/wbuild/node-subspace-runtime/node_subspace_runtime.compact.compressed.wasm on-runtime-upgrade live --uri wss://testnet-commune-api-node-1.communeai.net:443

run-benchmarking:
	cargo build -r --features runtime-benchmarks
	./target/release/node-subspace build-spec --disable-default-bootnode --chain local > specs/benchmarks.json
	./target/release/node-subspace benchmark pallet --chain specs/benchmarks.json --pallet pallet_subspace  --extrinsic "*" --steps 50 --repeat 20 --output pallets/subspace/src/weights.rs --template=./.maintain/frame-weight-template.hbs 
	./target/release/node-subspace benchmark pallet --chain specs/benchmarks.json --pallet pallet_governance  --extrinsic "*" --steps 50 --repeat 20 --output pallets/governance/src/weights.rs --template=./.maintain/frame-weight-template.hbs 

specs/mainnet-copy.json:
	python3 scripts/snapshots/builder.py -o specs/mainnet-copy.json

run-localnet:
	cargo xtask run --alice

run-mainnet: specs/mainnet-copy.json
	cargo xtask run --alice --chain-spec specs/mainnet-copy.json
