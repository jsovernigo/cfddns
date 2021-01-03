#!/bin/bash

set -x

api_base="https://api.cloudflare.com/client/v4/"

key=$(cat global_key)
EMAIL="juliansovernigo@gmail.com";

curl -X GET "https://api.cloudflare.com/client/v4/zones?name=sovernigo.ca" \
     -H "X-Auth-Email: $EMAIL" \
     -H "X-Auth-Key: $key" \
     -H "Content-Type: application/json"
