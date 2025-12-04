use std::net::IpAddr;

use anyhow::{anyhow, Result};

use crate::core::IpType;
use crate::errors::ClientErrorWrapper as _;

pub async fn ipify_resolve_ip(client: &reqwest::Client, ip_type: IpType) -> Result<IpAddr> {
    let ipify_url = match ip_type {
        IpType::V4 => "https://api.ipify.org/",
        IpType::V6 => "https://api6.ipify.org/",
    };
    let ip_text = client.get(ipify_url).send_text("ipify get ip").await?;

    ip_text
        .parse::<IpAddr>()
        .map_err(|_| anyhow!(format!("unable to parse {} as an ip", &ip_text)))
}
