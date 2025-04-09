all: build run

.env:
	! test -f '.env' && echo "DOMAIN=\nSUBDOMAIN=\n" > .env

.PHONY:
build:
	docker-compose build 

.PHONY:
run:
	docker-compose up -d 

.PHONY:
clean:
	docker image rm cfddns
