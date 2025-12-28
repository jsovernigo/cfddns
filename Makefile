all: build run

.env:
	! test -f '.env' && echo "DOMAIN=\nSUBDOMAINS=\nAPIBASE="https://api.cloudflare.com/client/v4"\nTOKEN=" > .env
        
cfddns:
	cargo build --release

.PHONY:
build: cfddns
	docker-compose build 

.PHONY:
run:
	docker-compose up -d 

stop:
	docker-compose down

logs:
	docker-compose logs -f cfddns

restart: stop run

.PHONY:
clean:
	docker-compose down
	docker image rm cfddns || true
	docker image prune -f
