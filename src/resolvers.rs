mod ipify;
mod opendns;

use std::net::IpAddr;

use anyhow::Result;
use serde::Deserialize;

use crate::config::DnsConfig;
use crate::core::IpType;

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
        Resolver::OpenDns => opendns::opendns_resolve_ip(ip_type).await,
        Resolver::Ipify => ipify::ipify_resolve_ip(client, ip_type).await,
    }
}
