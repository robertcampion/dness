use std::net::IpAddr;

use crate::config::IpType;
use crate::errors::DnessErrorKind;
use anyhow::{Context as _, Result};

pub async fn ipify_resolve_ip(client: &reqwest::Client, ip_type: IpType) -> Result<IpAddr> {
    let ipify_url = match ip_type {
        IpType::V4 => "https://api.ipify.org/",
        IpType::V6 => "https://api6.ipify.org/",
    };
    let ip_text = client
        .get(ipify_url)
        .send()
        .await
        .context(DnessErrorKind::send_http(ipify_url, "ipify get ip"))?
        .error_for_status()
        .context(DnessErrorKind::bad_response(ipify_url, "ipify get ip"))?
        .text()
        .await
        .context(DnessErrorKind::deserialize(ipify_url, "ipify get ip"))?;

    let ip = ip_text
        .parse::<IpAddr>()
        .map_err(|_| anyhow::anyhow!("unable to parse {ip_text} as an ip"))?;
    Ok(ip)
}
