#!/bin/sh -e

set -x

myip=$(dig +short myip.opendns.com @resolver1.opendns.com)
path=$(dirname $(realpath $0))

globalkey=$(cat /run/secrets/globalkey)
accountid=$(cat /run/secrets/accountid)

function getzone () {
    echo $(curl -X GET "${APIBASE}/zones?name=${DOMAIN}" \
        -H "X-Auth-Email: $EMAIL" \
        -H "X-Auth-Key: $globalkey" \
        -H "Content-Type: application/json" \
        | jq -r ".result[$1].id" \
        | sed s/\"//g)
}

function getdns () {
    echo $(curl -X GET "${APIBASE}/zones/$1/dns_records" \
        -H "X-Auth-Email: $EMAIL" \
        -H "X-Auth-Key: $globalkey" \
        -H "Content-Type: application/json" \
        | jq ".result[]|select(.name==\"${SUBDOMAIN}.${DOMAIN}\")|.id" \
        | sed s/\"//g)
}

zone=$(getzone 0)
dns_id=$(getdns $zone)

TYPE="A";
NAME=$SUBDOMAIN;
CONTENT="$myip";
PROXIED="false";
TTL="600";

while :
do 
    curl -X PUT "${APIBASE}/zones/$zone/dns_records/$dns_id" \
        -H "X-Auth-email: $EMAIL" \
        -H "X-Auth-Key: $globalkey" \
        -H "Content-Type: application/json" \
        --data '{"type":"'"$TYPE"'","name":"'"$NAME"'","content":"'"$CONTENT"'","proxied":'"$PROXIED"',"ttl":'"$TTL"'}'
    echo "last ran $(date)." >> $path/run.log
    sleep 10m
done