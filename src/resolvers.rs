use std::net::IpAddr;

use log::error;

use crate::config::DnsConfig;
use crate::core::IpType;
use crate::dns::wan_lookup_ip;
use crate::errors::DnessError;

pub(crate) async fn ipify_resolve_ip(
    client: &reqwest::Client,
    ip_type: IpType,
) -> Result<IpAddr, DnessError> {
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

/// Resolves the WAN IP or exits with a non-zero status code
pub(crate) async fn resolve_ip(
    client: &reqwest::Client,
    config: &DnsConfig,
    ip_type: IpType,
) -> Result<IpAddr, DnessError> {
    match config.ip_resolver.to_ascii_lowercase().as_str() {
        "opendns" => wan_lookup_ip(ip_type).await.map_err(|x| x.into()),
        "ipify" => ipify_resolve_ip(client, ip_type).await,
        _ => {
            error!("unrecognized ip resolver: {}", config.ip_resolver);
            std::process::exit(1)
        }
    }
}
