all: build run

.env:
	! test -f '.env' && echo "DOMAIN=\nSUBDOMAIN=\nEMAIL=\n" > .env

.PHONY:
build:
	docker-compose build cfddns

.PHONY:
run:
	docker-compose run cfddns -d

.PHONY:
clean:
	docker image rm cfddns
