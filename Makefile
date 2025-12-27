all: build run

.env:
	! test -f '.env' && echo "DOMAIN=\nSUBDOMAINS=\nAPIBASE="https://api.cloudflare.com/client/v4"\nTOKEN=" > .env
        
cfddns:
	cargo build

.PHONY:
build:
	docker-compose build 

.PHONY:
run:
	docker-compose up -d 

.PHONY:
clean:
	docker image rm cfddns
