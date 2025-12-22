mod ipify;
mod opendns;

use crate::config::{DnsConfig, IpType};
use crate::errors::DnessError;
use log::error;
use std::net::IpAddr;

/// Resolves the WAN IP or exits with a non-zero status code
pub async fn resolve_ip(
    client: &reqwest::Client,
    config: &DnsConfig,
    ip_type: IpType,
) -> Result<IpAddr, DnessError> {
    match config.ip_resolver.to_ascii_lowercase().as_str() {
        "opendns" => opendns::wan_lookup_ip(ip_type).await.map_err(|x| x.into()),
        "ipify" => ipify::ipify_resolve_ip(client, ip_type).await,
        _ => {
            error!("unrecognized ip resolver: {}", config.ip_resolver);
            std::process::exit(1)
        }
    }
}
