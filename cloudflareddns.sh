#!/bin/bash

set -x

path=$(dirname $(realpath $0))

. "$path/vars.sh"


TYPE="A";
NAME="ffh";
CONTENT="$myip";
PROXIED="false";
TTL="120";


result=$(curl -X GET "${api_base}user/tokens/verify" -H "Authorization: Bearer ${token}" -H "Content-Type:application/json" 2>/dev/null)

curl -X PUT "https://api.cloudflare.com/client/v4/zones/$zone/dns_records/$dns_id" \
    -H "X-Auth-email: $email" \
    -H "X-Auth-Key: $key" \
    -H "Content-Type: application/json" \
    --data '{"type":"'"$TYPE"'","name":"'"$NAME"'","content":"'"$CONTENT"'","proxied":'"$PROXIED"',"ttl":'"$TTL"'}' \
    | python -m json.tool; 

#echo "Ran last at $(date)." >> $path/run.log
