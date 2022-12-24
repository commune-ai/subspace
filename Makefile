up:
	docker compose up -d

down:
	docker compose down 

pull:
	docker compose down -d

restart:
	make down ; make up
build:
	docker compose build

enter:
	docker exec -it subspace bash

exec:
	docker exec -it subspace bash -c "${arg}"

node_up:
	make exec arg="./target/release/subspace-node --dev"

