#!/bin/sh -e

myip=$(dig +short myip.opendns.com @resolver1.opendns.com)
path=$(dirname $(realpath $0))

accountid=$(cat /run/secrets/accountid)
token=$(cat /run/secrets/token)

function hardquit () {
    exit 0
}
trap 'hardquit' SIGINT

function getzone () {
    echo $(curl -X GET "${APIBASE}/zones?name=${DOMAIN}" \
        -H "Authorization: Bearer $token" \
        -H "Content-Type: application/json" \
        | jq -r ".result[$1].id" \
        | sed s/\"//g)
}

function getdns () {
    echo $(curl -X GET "${APIBASE}/zones/$1/dns_records" \
        -H "Authorization: Bearer $token" \
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
        -H "Authorization: Bearer $token" \
        -H "Content-Type: application/json" \
        --data '{"type":"'"$TYPE"'","name":"'"$NAME"'","content":"'"$CONTENT"'","proxied":'"$PROXIED"',"ttl":'"$TTL"'}'
    echo "last ran $(date)." >> $path/run.log
    sleep 10m
done