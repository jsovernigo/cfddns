#!/usr/bin/env python3

import os
import requests
import time
import logging
import dotenv
from pathlib import Path

logging.basicConfig(
    filename='run.log',
    level=logging.INFO,
    format='[%(asctime)s] %(levelname)s - %(message)s'
)

# Set working path
path = Path(__file__).resolve().parent

# Load secrets
with open('/run/secrets/accountid') as f:
    accountid = f.read().strip()

with open('/run/secrets/token') as f:
    token = f.read().strip()

dotenv.load_dotenv()

# Environment variables expected to be set
APIBASE = os.environ.get('APIBASE')
DOMAIN = os.environ.get('DOMAIN')
SUBDOMAIN = os.environ.get('SUBDOMAIN')

ttl = 600

headers = {
    'Authorization': f'Bearer {token}',
    'Content-Type': 'application/json'
}

def get_supported_ip_addresses():
    ipv4 = None
    ipv6 = None

    try:
        ipv4 = requests.get("http://v4.ident.me", timeout=30).text.strip()
    except requests.RequestException as e:
        logging.warning(f"Failed to get a valid IPV4 request. {e}")

    try:
        ipv6 = requests.get("http://v6.ident.me", timeout=30).text.strip()
    except requests.RequestException as e:
        logging.warning(f"Failed to get a valid IPV6 request. {e}")

    return ipv4, ipv6

def get_zone(domain):
    url = f"{APIBASE}/zones?name={domain}"
    response = requests.get(url, headers=headers).json()

    for record in response["result"]:
        if record["name"] == f"{domain}":
            return record["id"]

    return None

def get_dns_record(zone_id, domain, subdomain, type):
    url = f"{APIBASE}/zones/{zone_id}/dns_records"
    response = requests.get(url, headers=headers).json()

    for record in response["result"]:
        if record["name"] == f"{subdomain}.{domain}" and record["type"] == type:
            return record["id"]

    return None

def create_new_dns_record(zone_id, subdomain, type, addr, ttl):
    url = f"{APIBASE}/zones/{zone_id}/dns_records"
    data = {
        "type": type,
        "name": subdomain,
        "content": addr,
        "proxied": False,
        "ttl": ttl
    }
    
    try:
        response = requests.post(url, headers=headers, json=data)

        if response.status_code == 200:
            logging.info(f"Created new {type} record for subdomain {subdomain}")
        else:
            logging.warning(f"Failed to create {type} record for {SUBDOMAIN}.{DOMAIN} with status code {response.status_code}. Error: {response.reason} ")
            return None

        # TODO: return response dns id
        return response.json()["result"]["id"]
    except Exception as e:
        logging.error(f"Encountered unexpected error during request to create {type} record for {subdomain}: {e}")
        return None

def update_dns_record(zone_id, subdomain, type, addr, record_id, ttl):
    url = f"{APIBASE}/zones/{zone_id}/dns_records/{record_id}"
    data = {
        "type": type,
        "name": subdomain,
        "content": addr,
        "proxied": False,
        "ttl": ttl
    }

    try:
        response = requests.patch(url, headers=headers, json=data)
        if response.status_code == 200:
            logging.info(f"Updated {type} record for {SUBDOMAIN}.{DOMAIN} to {ipv4}")
        else:
            logging.warning(f"Failed to update {type} record for {SUBDOMAIN}.{DOMAIN} with status code {response.status_code}. Error: {response.reason} ")
        return record_id
    except Exception as e:
        logging.error(f"Unable to reach {url}: {e}")

    # fallback case - maybe the ID changed or was deleted?
    return None

# first - retrieve global scope ipv4 and ipv6 addresses if possible, and log what is unavailable.
ipv4, ipv6 = get_supported_ip_addresses()

# if no ipv4/ipv6 addresses were reachable, no point continuing.
if not ipv4 and not ipv6:
    logging.error("No valid ipv4 or ipv6 addresses are available on this network. Shutting down.")
    exit(1)

# we must retrieve the zone for the domain provided in the environment. It may not exist.
if not (zone := get_zone(DOMAIN)):
    logging.error(f"No zone exists for {DOMAIN}, exiting.")
    exit(1)

ipv4_record_id = None
ipv6_record_id = None

# discover and assign the ipv4 record if it exists, otherwise attempt to create it.
if ipv4 and not (ipv4_record_id := get_dns_record(zone, DOMAIN, SUBDOMAIN, 'A')):
    logging.warning(f"No 'A' dns record exists for {SUBDOMAIN}.{DOMAIN}")
    ipv4_record_id = create_new_dns_record(zone, SUBDOMAIN, 'A', ipv4, ttl)

# discover and assign the ipv6 record if it exists, otherwise attempt to create it.
if ipv6 and not (ipv6_record_id := get_dns_record(zone, DOMAIN, SUBDOMAIN, 'AAAA')):
    logging.warning(f"No 'AAAA' dns record exists for {SUBDOMAIN}.{DOMAIN}")
    ipv6_record_id = create_new_dns_record(zone, SUBDOMAIN, 'AAAA', ipv6, ttl)

# if neither record id was assigned, something went terribly wrong - unrecoverable, we must exit.
if not ipv4_record_id and ipv6_record_id:
    logging.error(f"Unable to retrieve or create records for {SUBDOMAIN}.{DOMAIN}. Exiting.")
    exit(1)


# Update DNS in a loop
while True:

    # retrieve in a loop in case of isp address reassignment during uptime.
    ipv4, ipv6 = get_supported_ip_addresses()

    if ipv4:
        data = {
            "type": 'A',
            "name": SUBDOMAIN,
            "content": ipv4,
            "proxied": False,
            "ttl": ttl
        }

        # if this record exists we can update it.
        if ipv4_record_id:
            ipv4_record_id = update_dns_record(zone, SUBDOMAIN, 'A', ipv4, ipv4_record_id, ttl)
        else:
            logging.warning(f"No 'A' dns record exists for {SUBDOMAIN}.{DOMAIN}")
            ipv4_record_id = create_new_dns_record(zone, SUBDOMAIN, 'A', ipv4, ttl)
    
    if ipv6:
        data = {
            "type": 'AAAA',
            "name": SUBDOMAIN,
            "content": ipv6,
            "proxied": False,
            "ttl": ttl
        }

        if ipv6_record_id:
            ipv6_record_id = update_dns_record(zone, SUBDOMAIN, 'AAAA', ipv6, ipv6_record_id, ttl)   
        else:
            logging.warning(f"No 'AAAA' dns record exists for {SUBDOMAIN}.{DOMAIN}")
            ipv6_record_id = create_new_dns_record(zone, SUBDOMAIN, 'AAAA', ipv6, ttl)
        

    time.sleep(600)  # 10 minutes
