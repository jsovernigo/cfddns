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
    - to obtain your token, create and collect an API token on your Cloudflare dashboard.
3. run `make build`
4. run `make run`
