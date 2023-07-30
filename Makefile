
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