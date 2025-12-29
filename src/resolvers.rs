mod ipify;
mod opendns;

use crate::config::{DnsConfig, IpType, ResolverConfig};
use crate::errors::dns;
use anyhow::Result;
use std::net::IpAddr;

/// Resolves the WAN IP or exits with a non-zero status code
pub async fn resolve_ip(
    client: &reqwest::Client,
    config: &DnsConfig,
    ip_type: IpType,
) -> Result<IpAddr> {
    Ok(match config.ip_resolver {
        ResolverConfig::OpenDns => opendns::wan_lookup_ip(ip_type).await.map_err(dns)?,
        ResolverConfig::Ipify => ipify::ipify_resolve_ip(client, ip_type).await?,
    })
}
