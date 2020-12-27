#!/bin/bash

path=$(dirname $(realpath $0))

api_base="https://api.cloudflare.com/client/v4/"

token=$(cat $path/token)
zone=$(cat $path/zone)
account=$(cat $path/account)
dns_id=$(cat $path/dns_id)
key=$(cat $path/global_key)
myip=$(dig +short myip.opendns.com @resolver1.opendns.com)

EMAIL="juliansovernigo@gmail.com";
TYPE="A";
NAME="vpn";
CONTENT="$myip";
PROXIED="false";
TTL="120";



result=$(curl -X GET "${api_base}user/tokens/verify" -H "Authorization: Bearer ${token}" -H "Content-Type:application/json" 2>/dev/null)

if echo "$result" | grep 'success":true' > /dev/null ; then
	echo "token valid."
else
	echo "token invalid; please update token."
	exit 1
fi


curl -X PUT "https://api.cloudflare.com/client/v4/zones/$zone/dns_records/$dns_id" \
    -H "X-Auth-Email: $EMAIL" \
    -H "X-Auth-Key: $key" \
    -H "Content-Type: application/json" \
    --data '{"type":"'"$TYPE"'","name":"'"$NAME"'","content":"'"$CONTENT"'","proxied":'"$PROXIED"',"ttl":'"$TTL"'}' \
    | python -m json.tool; 

echo "Ran last at $(date)." >> $path/run.log
