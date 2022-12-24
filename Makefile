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