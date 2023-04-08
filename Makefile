
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

deploy_chain:
	./target/release/node-subspace  --dev --ws-port 9944
key_gen:
	./target/release/node-subspace key generate --scheme Sr25519 

logs:
	docker logs -f subspace
add_docker_permissions:
	./scripts/add_docker_permissions.sh