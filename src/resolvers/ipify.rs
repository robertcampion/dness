use std::net::IpAddr;

use crate::{config::IpType, errors::DnessError};

pub async fn ipify_resolve_ip(
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
