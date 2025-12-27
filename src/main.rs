use dotenvy::dotenv;
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE, HeaderMap, HeaderValue};
use reqwest::blocking::{Client};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use core::time;
use std::time::Duration;
use std::{env};
use std::collections::{HashSet, HashMap};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::thread::sleep;
use log::{info, warn, error};

const IPV4_SERVICES: &[&str] = &[
    "https://api.ipify.org", 
    "https://ipv4.icanhazip.com",
    "https://v4.ident.me",
];

const IPV6_SERVICES: &[&str] = &[
    "https://api64.ipify.org",
    "https://ipv6.icanhazip.com",
    "https://v6.ident.me",
];

/* custom error type - when querying multiple providers we could have a disagreement or an error. */
#[derive(Debug)]
enum IpQueryError {
    ConsensusMismatch {first: IpAddr, conflict: IpAddr},
    NoIpAvailable {reason: &'static str},
}

#[derive(Debug)]
enum CloudflareAPIError {
    ConnectionError {url: String, err: reqwest::Error},
    MissingDomain {domain: String},
    ResponseParseError,
    JsonParseError {json: String},
    JsonFormatError {json: String},
    ResponseError {message: String}
}

#[derive(Debug, Serialize, Deserialize)]
struct DnsRecord {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub record_type: String,
    pub content: String,
    pub proxiable: bool,
    pub proxied: bool,
    pub ttl: u32,
    #[serde(default)]
    pub settings: HashMap<String, serde_json::Value>,
    #[serde(default)]
    pub meta: HashMap<String, serde_json::Value>,
    pub comment: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    pub created_on: String,
    pub modified_on: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct DnsRecordUpdate {
    #[serde(rename = "type")]
    pub record_type: String,
    pub name: String,
    pub content: String,
    pub ttl: u32,
    pub proxied: bool
}

/// split_subdomain
/// Parses a comma-separated string of subdomains into a HashSet.
/// Returns an empty HashSet if the input string is empty.
fn split_subdomain(subdomains: &str) -> HashSet<&str> {
    if subdomains.is_empty() {
        HashSet::new()
    } else {
        subdomains.split(',').collect()
    }
}

/// query_ip_providers
/// Queries multiple IP checking providers to determine the current public IP address.
/// Requires consensus among all reachable providers to return a successful result.
/// Returns an error if providers disagree or if none are reachable.
fn query_ip_providers(
    provider_client: &reqwest::blocking::Client,
    providers: &[&str]
) -> Result<IpAddr, IpQueryError> {

    let mut addr: Option<IpAddr> = None;

    let min_agreement = 2;
    let mut n_agreement = 0;


    for &provider_url in providers {

        info!("Querying provider {provider_url}");

        /* in a case where a provider is unreachable, just abort. */
        let response = match provider_client.get(provider_url).send() {
            Ok(resp) => resp,

            /* this is an error that needs logging. */
            Err(e) => {
                error!("Error when querying {provider_url}: {:#?}", e);
                continue;
            },
        };

        let text = match response.text() {
            Ok(t) => t,
            Err(e) =>  {
                error!("Error when parsing response from {provider_url}: {:#?}", e);
                continue;
            },
        };

        let new_ip = match text
            .trim()
            .parse::<std::net::IpAddr>() {
            Ok(ip) => ip,
            /* similarly here we need to error log */
            Err(e) => {
                error!("Response was not parseable as IpAddr ({text}): {:#?}", e);
                continue;
            }
        };

        /* we must check for consensus, otherwise something is going wrong! */
        match addr {
            Some(current) => {
                if current != new_ip {
                    return Err(IpQueryError::ConsensusMismatch{
                        first: current,
                        conflict: new_ip,
                    });
                }             
                n_agreement += 1;
            },
            None => {
                addr = Some(new_ip);
                n_agreement += 1;
            }
        } 
    }

    if let Some(ip) = addr && n_agreement > min_agreement {
        return Ok(ip);
    }

    return Err(IpQueryError::NoIpAvailable { reason: "No providers were reachable." });
}

/// query_with_retries
/// Wrapper around query_ip_providers that implements a retry mechanism.
/// Attempts to query providers up to the specified number of retries.
/// Returns None if all retry attempts fail.
fn query_with_retries(
    provider_client: &reqwest::blocking::Client,
    providers: &[&str], 
    retries: usize
) -> Option<IpAddr> {

    for _ in 0..retries {
        let result = query_ip_providers(provider_client, providers);

        if let Ok(addr) = result {
            return Some(addr);
        }
                
        let err = result.unwrap_err();
        error!("Error during query, {:?}, retrying", err);

        match err {
            IpQueryError::ConsensusMismatch{first, conflict} => {
                error!("Encountered consensus issue, {conflict} disagrees with {first}");
            },
            IpQueryError::NoIpAvailable{reason} => {
                error!("Couldn't collect ip from providers: {reason}");
            }
        }
    }

    return None
}

/// get_supported_public_ips
/// Queries both IPv4 and IPv6 provider lists to determine supported public IP addresses.
/// Returns a tuple of (Option<Ipv4Addr>, Option<Ipv6Addr>) where either or both may be None
/// if the respective IP version is not available or consensus cannot be reached.
fn get_supported_public_ips(
    provider_client: &reqwest::blocking::Client,
    v4_providers: &[&str], 
    v6_providers: &[&str], 
    retries: usize
) -> (Option<Ipv4Addr>, Option<Ipv6Addr>) {

    let ipv4 = query_with_retries(provider_client, v4_providers, retries)
        .and_then(|ip| match ip {
            IpAddr::V4(v4) => Some(v4),
            IpAddr::V6(_) => None,
        }
    );

    let ipv6 = query_with_retries(provider_client, v6_providers, retries)
        .and_then(|ip| match ip {
            IpAddr::V4(_) => None,
            IpAddr::V6(v6) => Some(v6),
        }
    );

    info!("Acquired IP addresses: v4: {:?}, v6: {:?}", ipv4, ipv6);
    (ipv4, ipv6)
}

/// cloudflare_get_zone_id
/// Retrieves the Cloudflare zone ID for the specified domain.
/// Queries the Cloudflare API zones endpoint and extracts the zone ID from the matching result.
/// Returns an error if the domain is not registered with Cloudflare or if the API request fails.
fn cloudflare_get_zone_id(
    apibase: &str, 
    cloudflare_client: &Client, 
    domain: &str
) -> Result<String, CloudflareAPIError> {

    let url = format!("{apibase}/zones?name={domain}");

    let response = cloudflare_client.get(&url).send()
        .map_err(|e| CloudflareAPIError::ConnectionError { url: url, err: (e) })?;

    let json: Value = response.json()
        .map_err(|_| CloudflareAPIError::ResponseParseError)?;

    /* the json path we want is result.id where result.name == domain */
    let json_results = json.get("result")
        .and_then(|result| result.as_array())
        .ok_or(CloudflareAPIError::JsonFormatError { json: json.to_string() })?;

    let record = json_results.iter()
        .find(|item|  {
            item.get("name")
                .and_then(|item| item.as_str())
                .map(|name| name == domain)
                .unwrap_or(false)
        });

    /* if there is no zone record for the domain name we want, that means the domain is not 
    registered with cloudflare. If this is the case, we must propagate the error and abort - 
    we can't fix this from this program, it needs to be addressed from the cloudflare
    dashboard. */
    let record = match record {
        Some(rec) => rec,
        None => return Err(CloudflareAPIError::MissingDomain { 
            domain: domain.to_string() 
        })
    };

    let zone_id = record.get("id")
        .and_then(|serialized| serialized.as_str())
        .map(|id| id.to_string())
        .ok_or(CloudflareAPIError::JsonFormatError { json: json.to_string() })?;

    Ok(zone_id)
}

/// cloudflare_get_dns_record_id
/// Retrieves all DNS records for a specific full domain name within a Cloudflare zone.
/// Returns a vector of DnsRecord structs, which may be empty if no records exist.
/// The vector can contain both A and AAAA records for the same domain.
fn cloudflare_get_dns_record_id(
    apibase: &str, 
    cloudflare_client: &Client, 
    full_domain: &str, 
    zone_id: &str
) -> Result<Vec<DnsRecord>, CloudflareAPIError> {

    let url = format!("{apibase}/zones/{zone_id}/dns_records");

    let response = cloudflare_client.get(&url).send()
        .map_err(|e| CloudflareAPIError::ConnectionError { url: url, err: e })?;

    let json: serde_json::Value = response.json()
        .map_err(|_| CloudflareAPIError::ResponseParseError)?;

    let json_results = json.get("result")
        .and_then(|result| result.as_array())
        .ok_or_else(|| CloudflareAPIError::JsonParseError { 
            json: json.to_string() 
        })?;

    /* find the specific records for this subdomain */
    let records = json_results.iter()
        .filter(|item| {
            item.get("name")
                .and_then(|name| name.as_str())
                .map(|name| name == full_domain)
                .unwrap_or(false)
        });

    let mut dns_records = Vec::<DnsRecord>::new();

    for record in records {
        let dns_record = serde_json::from_value(record.clone())
                .map_err(|_| CloudflareAPIError::JsonFormatError { json: json.to_string() })?;

        dns_records.push(dns_record);
    }

    /* returning an empty vector is possible - the records may be 
    expired and removed, or this may be a new domain that we haven't
    configured before. */
    Ok(dns_records)
}

/// create_update_records_from_ip_set
/// Generates a DnsRecordUpdate struct for either an A (IPv4) or AAAA (IPv6) record.
/// The record type is determined automatically based on the IP address type.
/// Used to prepare record data for Cloudflare API update or create operations.
fn generate_dns_record(
    ip: &IpAddr,
    full_name: String,
    ttl: u32
) -> DnsRecordUpdate {

    match ip {
        IpAddr::V4(v4) => DnsRecordUpdate{
            record_type: "A".to_string(),
            name: full_name,
            content: v4.to_string(),
            ttl: ttl,
            proxied: false
        },
        IpAddr::V6(v6) => DnsRecordUpdate{
            record_type: "AAAA".to_string(),
            name: full_name,
            content: v6.to_string(),
            ttl: ttl,
            proxied: false
        }
    }
}

/// cloudflare_create_new_dns_record
/// Creates a new DNS record in Cloudflare for the specified domain and IP address.
/// Returns the ID of the newly created record on success.
/// Returns an error if the API request fails or if Cloudflare returns success=false.
fn cloudflare_create_new_dns_record(
    apibase: &str,
    cloudflare_client: &Client,
    full_domain: &str,
    zone_id: &str,
    ip: &IpAddr,
    ttl: u32
) -> Result<String, CloudflareAPIError> {
    let url = format!("{apibase}/zones/{zone_id}/dns_records");

    let update_record = generate_dns_record(&ip, full_domain.to_string(), ttl);

    let response = cloudflare_client
        .post(&url)
        .json(&update_record)
        .send()
        .map_err(|e| CloudflareAPIError::ConnectionError { url: url.to_string(), err: e })?;

    let response_json: serde_json::Value = response
        .json()
        .map_err(|_| CloudflareAPIError::ResponseParseError)?;

    /* we need to extract the success boolean because there is no need to continue
    if there was some kind of error. */
    let success = response_json
        .get("success")
        .and_then(|success| success.as_bool())
        .ok_or(CloudflareAPIError::JsonFormatError { json: response_json.to_string() })?;

    if ! success {
        return Err(CloudflareAPIError::ResponseError { message: "Encountered success=false during record creation.".to_string() });
    }

    let record: DnsRecord = serde_json::from_value(
        response_json
            .get("result")
            .ok_or(CloudflareAPIError::JsonFormatError { json: response_json.to_string() })?
            .clone()
        )
    .map_err(|_| CloudflareAPIError::JsonParseError { json: response_json.to_string() })?;

    Ok(record.id)
}

/// cloudflare_update_dns_record
/// Updates an existing DNS record in Cloudflare with a new IP address.
/// Returns true if the update was successful (Cloudflare returned success=true).
/// Returns false or an error if the update failed.
fn cloudflare_update_dns_record(
    apibase: &str,
    cloudflare_client: &Client,
    full_domain: &str,
    zone_id: &str,
    record_id: &str,
    ip: &IpAddr,
    ttl: u32
) -> Result<bool, CloudflareAPIError> {
    let url = format!("{apibase}/zones/{zone_id}/dns_records/{record_id}");

    let update_record = generate_dns_record(&ip, full_domain.to_string(), ttl);

    let response = cloudflare_client
        .patch(&url)
        .json(&update_record)
        .send()
        .map_err(|e| CloudflareAPIError::ConnectionError { url: url.to_string(), err: e })?;

    let response_json: serde_json::Value = response.json()
        .map_err(|_| CloudflareAPIError::ResponseParseError)?;

    /* we need to extract the success boolean because there is no need to continue
    if there was some kind of error. */
    let success = response_json
        .get("success")
        .and_then(|success| success.as_bool())
        .ok_or(CloudflareAPIError::JsonFormatError { json: response_json.to_string() })?;

    Ok(success)
}

/// update_or_create_record
/// Updates an existing DNS record or creates a new one if it doesn't exist.
/// Returns a tuple of (success: bool, new_id: Option<String>) where new_id is Some
/// only if a new record was created. Logs errors if the operation fails.
fn update_or_create_record(
    apibase: &str,
    cloudflare_client: &Client, 
    full_domain: &str, 
    ip: &IpAddr, 
    ttl: u32,
    zone_id: &str,
    record_id: Option<&String>
) -> (bool, Option<String>) {
    match record_id {
        Some(id) => {
            let result = cloudflare_update_dns_record(
                apibase,
                cloudflare_client, 
                full_domain,
                zone_id, 
                &id, 
                ip, 
                ttl
            );

            if let Ok(success) = result {
                if ! success {
                    error!("Update returned success=false.")
                } 
                /* We sent the update request - now we see if it failed or not. */
                return (success, None);
            } else {
                error!("Error when updating. Encountered {:#?}", result.unwrap_err());
                return (false, None);
            }

        },
        /* no record exists - we must create it. */
        None => {
            let result = cloudflare_create_new_dns_record(
                apibase, 
                cloudflare_client, 
                full_domain, 
                zone_id, 
                ip, 
                ttl
            );

            if let Ok(id) = result {
                return (true, Some(id));
            } else {
                error!("Error when creating record. Encountered {:#?}", result.unwrap_err());
                return (false, None);
            }
        }
    }
}

/// update_dns_record
/// High-level wrapper that handles DNS record updates and caches record IDs.
/// Automatically determines whether to update or create a record based on the cache.
/// Updates the known_ids HashMap with new record IDs when records are created.
/// Returns true if the operation succeeded, false otherwise.
fn update_dns_record(
    apibase: &str,
    client: &Client,
    domain: &str,
    ip: &IpAddr,
    record_type: &str,
    ttl: u32,
    zone_id: &str,
    known_ids: &mut HashMap<(String, String), String>,
) -> bool {
    let key = (domain.to_string(), record_type.to_string());
    let record_id = known_ids.get(&key);
    
    let (success, new_id) = update_or_create_record(
        apibase, client, domain, &ip, ttl, zone_id, record_id
    );
    
    if let Some(id) = new_id {
        known_ids.insert(key, id);
    }
    
    if success {
        info!("Updated {} to {}", domain, ip);
    } else {
        error!("Error encountered while updating {}. No changes made.", domain);
    }
    
    success
} 

fn main() {
    dotenv().ok();
    env_logger::init();

    log::info!("Beginning execution");

    let ttl: u32 = 600;
    let sleep_time: u64 = 600;

    let apibase = env::var("APIBASE")
        .expect("APIBASE must be set");

    let domain = env::var("DOMAIN")
        .expect("DOMAIN must be set");

    let env_subdomains = env::var("SUBDOMAINS")
        .expect("SUBDOMAINS must be set");

    /* subdomains can be blank, or it can be an array of strings */
    let subdomains = split_subdomain(
        &env_subdomains
    );

    let token = env::var("TOKEN")
        .expect("TOKEN must be set");

    info!("Environment variables read. Subdomains are: {:#?}", subdomains);

    let cfclient_headers = HeaderMap::from_iter([
        (
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {token}"))
                .expect("Token must not contain invalid chars.")
        ),
        (
            CONTENT_TYPE, 
            HeaderValue::from_static("application/json")
        )
    ]);

    let cloudflare_client = reqwest::blocking::Client::builder()
        .default_headers(cfclient_headers)
        .timeout(Duration::from_secs(10))
        .connect_timeout(Duration::from_secs(5))
        .build()
        .expect("The client should be able to build.");

    /* next, collect the relevant info from cloudflare's api so we can modify the records. */
    let zone_id = match cloudflare_get_zone_id(&apibase, &cloudflare_client, &domain) {
        Ok(zone) => zone,
        Err(e) => {
            error!("Something went wrong: {:#?}", e);
            panic!("Abort.");
        }
    };

    info!("Collected zone_id from Cloudflare for {domain}: {zone_id}");

    let mut known_dns_ids: HashMap<(String, String), String> = HashMap::new();

    /* we pre-populate the ids in the hashmap for each subdomain. */
    for subdomain in subdomains.clone() {
        let full_domain = format!("{subdomain}.{domain}");

        info!("Checking for subdomain {full_domain}...");

        if let Ok(records) = cloudflare_get_dns_record_id(
            &apibase.as_str(), 
            &cloudflare_client, 
            full_domain.as_str(),
            zone_id.as_str()) {
            
            /* we may get back one or two records */
            for record in records {
                info!("\tCaching {0} record: {1}", record.record_type, record.id);
                known_dns_ids.insert(
                    (
                        full_domain.clone(), 
                        record.record_type.clone()
                    ), 
                    record.id
                );
            }
        } else {
            info!("\tNo records found for {full_domain}");
        }
    }

    /* TODO: maybe make this configurable later. */
    let retries = 5;

    /* Do not collect ip - we must check this on the first iteration of the update loop. */
    let (mut ipv4_cache, mut ipv6_cache) = (None, None);

    let ipquery_client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(10))
        .connect_timeout(Duration::from_secs(5))
        .build()
        .expect("The client should be able to build.");

    let max_failures = 5;
    let mut failure_count = 0;

    info!("Beginning update loop.");

    loop {
        info!("Beginning cycle.");

        let mut cycle_failed: bool = false;

        /* TODO: detect any changes on the network device (netlink, etc) */
        let (ipv4, ipv6) = get_supported_public_ips(&ipquery_client, IPV4_SERVICES, IPV6_SERVICES, retries);

        /* update v4 if needed. */
        let update_v4 = if ipv4_cache != ipv4 {
            info!("IPV4 has changed - update will run this cycle.");
            ipv4_cache = ipv4;
            ipv4_cache.is_some()
        } else {
            false
        };

        /* update v6 if needed. */
        let update_v6 = if ipv6_cache != ipv6 {
            info!("IPV6 has changed - update will run this cycle.");
            ipv6_cache = ipv6;
            ipv6_cache.is_some()
        } else {
            false
        };

        if ipv4_cache.is_none() && ipv6_cache.is_none() {
            warn!("No valid ip addresses were available during this cycle. Failures incremented! ({failure_count})");
            cycle_failed = true;
        } else if !update_v4 && !update_v6 {
            info!("Cached IPs are still valid. No updates will occur this cycle.");
            cycle_failed = false;
        } else {
            /* For each of our subdomains, we need to send separate records for each of A and AAAA. */
            for subdomain in &subdomains {
                let full_domain = format!("{subdomain}.{domain}");

                /* v4 update */
                if let Some(ip) = ipv4_cache && update_v4 {
                    if ! update_dns_record(
                        &apibase, 
                        &cloudflare_client, 
                        &full_domain, 
                        &IpAddr::V4(ip), 
                        "A",
                        ttl, 
                        &zone_id, 
                        &mut known_dns_ids
                    ) {
                        cycle_failed = true;
                    }
                }

                /* v6 update */
                if let Some(ip) = ipv6_cache && update_v6 {
                    if ! update_dns_record(
                        &apibase,
                        &cloudflare_client, 
                        &full_domain, 
                        &IpAddr::V6(ip), 
                        "AAAA", 
                        ttl, 
                        &zone_id,
                        &mut known_dns_ids
                    ) {
                        cycle_failed = true;
                    }
                }
            }
        }

        /* bad case: if we:
        1. failed to update
        2. failed to get any valid ips,
        3. other errors occurred etc.
        we incremement the failure count. */
        if cycle_failed {
            warn!("Cycle failed to update one or more records. Failures incremented! ({failure_count})");
            failure_count += 1;
        } else {
            /* on a success, however, we clear it. */
            failure_count = 0;
        }

        /* exponential backoff - we may have a problem here! */
        if failure_count > max_failures {
            warn!("Failures exceeded max failures - sleeping for 5 cycles.");
            sleep(time::Duration::from_secs(sleep_time * 5));
        } else {
            /* we only want to rest roughly as long as a record ttl,
            since if our ip changes during that time, the cache will 
            probably have expired. */
            info!("Cycle finished, sleeping.");
            sleep(time::Duration::from_secs(sleep_time));
        }
    }
}