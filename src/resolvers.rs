use std::net::IpAddr;

use anyhow::Result;
use hickory_resolver::config::ResolverConfig;
use serde::Deserialize;

use crate::config::DnsConfig;
use crate::core::IpType;
use crate::dns::{config_opendns, DnsResolver};
use crate::errors::{DnessError, DnsError};

#[derive(Deserialize, Clone, Copy, PartialEq, Debug, Default)]
#[serde(rename_all = "lowercase")]
pub enum Resolver {
    #[default]
    OpenDns,
    Ipify,
}

/// Resolves the WAN IP
pub async fn resolve_ip(
    client: &reqwest::Client,
    config: &DnsConfig,
    ip_type: IpType,
) -> Result<IpAddr> {
    match config.ip_resolver {
        Resolver::OpenDns => opendsn_resolve_ip(ip_type).await.map_err(|x| x.into()),
        Resolver::Ipify => ipify_resolve_ip(client, ip_type).await,
    }
}

async fn ipify_resolve_ip(client: &reqwest::Client, ip_type: IpType) -> Result<IpAddr> {
    let ipify_url = match ip_type {
        IpType::V4 => "https://api.ipify.org/",
        IpType::V6 => "https://api6.ipify.org/",
    };
    let ip_text = client
        .get(ipify_url)
        .send()
        .await
        .map_err(|e| DnessError::send_http(ipify_url, "ipify get ip", e))?
        .error_for_status()
        .map_err(|e| DnessError::bad_response(ipify_url, "ipify get ip", e))?
        .text()
        .await
        .map_err(|e| DnessError::deserialize(ipify_url, "ipify get ip", e))?;

    let ip = ip_text
        .parse::<IpAddr>()
        .map_err(|_| DnessError::message(format!("unable to parse {} as an ip", &ip_text)))?;
    Ok(ip)
}

async fn opendsn_resolve_ip(ip_type: IpType) -> Result<IpAddr, DnsError> {
    let base_config = config_opendns();
    let name_servers: Vec<_> = base_config
        .name_servers()
        .iter()
        .filter(|name_server| IpType::from(name_server.socket_addr.ip()) == ip_type)
        .cloned()
        .collect();
    let config = ResolverConfig::from_parts(
        base_config.domain().cloned(),
        base_config.search().to_vec(),
        name_servers,
    );
    let resolver = DnsResolver::from_config(config).await?;
    // When we query opendns for the special domain of "myip.opendns.com" it will return to us
    // our IP
    const DOMAIN: &str = "myip.opendns.com.";
    match ip_type {
        IpType::V4 => resolver.ipv4_lookup(DOMAIN).await.map(Into::into),
        IpType::V6 => resolver.ipv6_lookup(DOMAIN).await.map(Into::into),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::errors::DnsErrorKind;

    #[tokio::test]
    async fn opendns_resolve_ipv4_test() {
        // Heads up: this test requires internet connectivity
        match opendsn_resolve_ip(IpType::V4).await {
            Ok(ip) => {
                assert!(ip.is_ipv4());
                assert!(!ip.is_loopback());
            }
            Err(e) => {
                match e.kind.as_ref() {
                    DnsErrorKind::DnsResolve(e) => {
                        match e.kind() {
                            hickory_resolver::error::ResolveErrorKind::NoRecordsFound {
                                ..
                            } => {
                                // This is fine, just means we're behind a CGNAT
                            }
                            _ => panic!("unexpected error: {}", e),
                        }
                    }
                    DnsErrorKind::UnexpectedResponse(_) => {
                        panic!("unexpected response: {}", e);
                    }
                }
            }
        }
    }

    #[tokio::test]
    async fn opendns_resolve_ipv6_test() {
        // Heads up: this test requires internet connectivity
        match opendsn_resolve_ip(IpType::V6).await {
            Ok(ip) => {
                assert!(ip.is_ipv6());
                assert!(!ip.is_loopback());
            }
            Err(e) => {
                match e.kind.as_ref() {
                    DnsErrorKind::DnsResolve(e) => {
                        match e.kind() {
                            hickory_resolver::error::ResolveErrorKind::NoRecordsFound {
                                ..
                            } => {
                                // This is fine, just means we're behind a CGNAT
                            }
                            _ => panic!("unexpected error: {}", e),
                        }
                    }
                    DnsErrorKind::UnexpectedResponse(_) => {
                        panic!("unexpected response: {}", e);
                    }
                }
            }
        }
    }
}
