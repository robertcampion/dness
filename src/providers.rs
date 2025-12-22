mod cloudflare;
mod dynu;
mod godaddy;
mod he;
mod namecheap;
mod noip;
mod porkbun;

use crate::{config::DomainConfig, core::Updates};
use std::net::IpAddr;

pub async fn update_provider(
    http_client: &reqwest::Client,
    addr: IpAddr,
    domain: &DomainConfig,
) -> Result<Updates, Box<dyn std::error::Error>> {
    match domain {
        DomainConfig::Cloudflare(domain_config) => {
            cloudflare::update_domains(http_client, domain_config, addr)
                .await
                .map_err(|e| e.into())
        }
        DomainConfig::GoDaddy(domain_config) => {
            godaddy::update_domains(http_client, domain_config, addr)
                .await
                .map_err(|e| e.into())
        }
        DomainConfig::Namecheap(domain_config) => {
            namecheap::update_domains(http_client, domain_config, addr)
                .await
                .map_err(|e| e.into())
        }
        DomainConfig::He(domain_config) => he::update_domains(http_client, domain_config, addr)
            .await
            .map_err(|e| e.into()),
        DomainConfig::NoIp(domain_config) => noip::update_domains(http_client, domain_config, addr)
            .await
            .map_err(|e| e.into()),
        DomainConfig::Dynu(domain_config) => dynu::update_domains(http_client, domain_config, addr)
            .await
            .map_err(|e| e.into()),
        DomainConfig::Porkbun(domain_config) => {
            porkbun::update_domains(http_client, domain_config, addr)
                .await
                .map_err(|e| e.into())
        }
    }
}
