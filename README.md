# CFDDNS

A docker-wrapped rust program to turn a cloudflare free account into a dynamic DNS.

## Requirements
- cloudflare account
- domain registered under the cloudflare dns for your zone
- python
- docker
- docker-compose

Untested on Windows.

## Use

Follow these steps for an easy setup:
1. run `make .env`, fill it out with the correct data.
2. navigate to dash.cloudflare.com, log in.
    - create and collect an API token under the api token section. record this; it won't be shown again.
3. create a file in the base directory of the project called `token`, put your api token from step 2.1 into it.
4. run `make build`
5. run `make run`
