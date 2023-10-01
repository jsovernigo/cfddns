#!/bin/bash

set -x

globalkey=$(cat /run/secrets/globalkey)

function getzone () {
     echo $(curl -X GET "${APIBASE}/zones?name=${DOMAIN}" \
          -H "X-Auth-Email: $EMAIL" \
          -H "X-Auth-Key: $globalkey" \
          -H "Content-Type: application/json" | jq -r ".result[$1].id") \
          | jq
}

gezone $0