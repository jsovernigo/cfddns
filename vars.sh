#!/bin/bash

api_base="https://api.cloudflare.com/client/v4/"
email="juliansovernigo@gmail.com";

token=$(cat $path/token)
zone=$(cat $path/zone)
account=$(cat $path/account)
dns_id=$(cat $path/dns_id)
key=$(cat $path/global_key)

myip=$(dig +short myip.opendns.com @resolver1.opendns.com)
