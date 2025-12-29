use std::net::IpAddr;

use crate::config::IpType;
use crate::errors;
use anyhow::Result;

pub async fn ipify_resolve_ip(client: &reqwest::Client, ip_type: IpType) -> Result<IpAddr> {
    let ipify_url = match ip_type {
        IpType::V4 => "https://api.ipify.org/",
        IpType::V6 => "https://api6.ipify.org/",
    };
    let ip_text = client
        .get(ipify_url)
        .send()
        .await
        .map_err(|e| errors::send_http(ipify_url, "ipify get ip", e))?
        .error_for_status()
        .map_err(|e| errors::bad_response(ipify_url, "ipify get ip", e))?
        .text()
        .await
        .map_err(|e| errors::deserialize(ipify_url, "ipify get ip", e))?;

    let ip = ip_text
        .parse::<IpAddr>()
        .map_err(|_| anyhow::anyhow!("unable to parse {ip_text} as an ip"))?;
    Ok(ip)
}
