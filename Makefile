
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
compile:
	cargo build --release

deploy:
	./target/release/node-subspace  --dev --ws-port 9944
key_gen:
	./target/release/node-subspace key generate --scheme Sr25519 
purge:
	./target/release/node-subspace purge-chain --dev
logs:
	docker logs -f subspace
docker_unsudo:
	./scripts/docker_unsudo.sh
chmod_scripts:
	chmod +x ./scripts/*.sh