mod cloudflare;
mod dynu;
mod godaddy;
mod he;
mod namecheap;
mod noip;
mod porkbun;

use std::net::IpAddr;

use anyhow::Result;
pub use cloudflare::CloudflareConfig;
pub use dynu::DynuConfig;
pub use godaddy::GoDaddyConfig;
pub use he::HeConfig;
pub use namecheap::NamecheapConfig;
pub use noip::NoIpConfig;
pub use porkbun::PorkbunConfig;
use serde::Deserialize;

use crate::core::{IpType, Updates};

#[derive(Deserialize, Clone, PartialEq, Debug)]
#[serde(tag = "type")]
#[serde(rename_all = "lowercase")]
pub enum DomainConfig {
    Cloudflare(CloudflareConfig),
    GoDaddy(GoDaddyConfig),
    Namecheap(NamecheapConfig),
    He(HeConfig),
    NoIp(NoIpConfig),
    Dynu(DynuConfig),
    Porkbun(PorkbunConfig),
}

impl DomainConfig {
    pub fn display_name(&self) -> String {
        match self {
            DomainConfig::Cloudflare(c) => format!("{} ({})", c.zone, "cloudflare"),
            DomainConfig::GoDaddy(c) => format!("{} ({})", c.domain, "godaddy"),
            DomainConfig::Namecheap(c) => format!("{} ({})", c.domain, "namecheap"),
            DomainConfig::He(c) => format!("{} ({})", c.hostname, "he"),
            DomainConfig::NoIp(c) => format!("{} ({})", c.hostname, "noip"),
            DomainConfig::Dynu(c) => format!("{} ({})", c.hostname, "dynu"),
            DomainConfig::Porkbun(c) => format!("{} ({})", c.domain, "porkbun"),
        }
    }

    pub fn get_ip_types(&self) -> Vec<IpType> {
        match self {
            DomainConfig::Cloudflare(cloudflare_config) => cloudflare_config.ip_types.clone(),
            _ => vec![IpType::V4],
        }
    }

    pub async fn update(&self, http_client: &reqwest::Client, addr: IpAddr) -> Result<Updates> {
        Ok(match self {
            DomainConfig::Cloudflare(domain_config) => {
                cloudflare::update_domains(http_client, domain_config, addr).await?
            }
            DomainConfig::GoDaddy(domain_config) => {
                godaddy::update_domains(http_client, domain_config, addr).await?
            }
            DomainConfig::Namecheap(domain_config) => {
                namecheap::update_domains(http_client, domain_config, addr).await?
            }
            DomainConfig::He(domain_config) => {
                he::update_domains(http_client, domain_config, addr).await?
            }
            DomainConfig::NoIp(domain_config) => {
                noip::update_domains(http_client, domain_config, addr).await?
            }
            DomainConfig::Dynu(domain_config) => {
                dynu::update_domains(http_client, domain_config, addr).await?
            }
            DomainConfig::Porkbun(domain_config) => {
                porkbun::update_domains(http_client, domain_config, addr).await?
            }
        })
    }
}
