# CFDDNS

A simple docker-wrapped sh script to turn a cloudflare free account into a dynamic DNS.

## Requirements
- cloudflare account
- domain registered under the cloudflare dns for your zone
- docker
- docker-compose

Works on Linux. Not tested on Windows.

## Use

Everything is Makefiled for you. Follow these steps for an easy setup:

1. run `make .env``, fill it out with the correct data.
2. navigate to dash.cloudflare.com, log in.
    - collect your account ID from the lefthand panel under the API section.
    - create and collect an API token under the api token section. record this; it won't be shown again.
3. create a file called `accountid`, put your account id from step 2.1 into it.
4. create a file called `token`, put your api token from step 2.2 into it.
5. run `make build`
6. run `make run`
