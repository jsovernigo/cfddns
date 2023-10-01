all: build run

.env:
	! test -f '.env' && echo "DOMAIN=\nSUBDOMAIN=\nEMAIL=\n" > .env

.PHONY:
build:
	docker build -t cfddns .

.PHONY:
run:
	docker-compose run cfddns

.PHONY:
clean:
	docker image rm cfddns
