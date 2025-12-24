mod cloudflare;
mod dynu;
mod godaddy;
mod he;
mod namecheap;
mod noip;
mod porkbun;

use crate::config::DomainConfig;
use crate::core::Updates;
use crate::dns::DnsResolver;
use anyhow::Result;
use log::{info, warn};
use std::net::IpAddr;

pub async fn update_provider(
    http_client: &reqwest::Client,
    addr: IpAddr,
    domain: &DomainConfig,
) -> Result<Updates> {
    match domain {
        DomainConfig::Cloudflare(domain_config) => {
            cloudflare::update_domains(http_client, domain_config, addr).await
        }
        DomainConfig::GoDaddy(domain_config) => {
            godaddy::update_domains(http_client, domain_config, addr).await
        }
        DomainConfig::Namecheap(domain_config) => {
            domain_config.update_domains(http_client, addr).await
        }
        DomainConfig::He(domain_config) => domain_config.update_domains(http_client, addr).await,
        DomainConfig::NoIp(domain_config) => domain_config.update_domains(http_client, addr).await,
        DomainConfig::Dynu(domain_config) => domain_config.update_domains(http_client, addr).await,
        DomainConfig::Porkbun(domain_config) => {
            porkbun::update_domains(http_client, domain_config, addr).await
        }
    }
}

trait DnsLookupConfig<'a> {
    type Provider: DnsLookupProvider;

    fn create_provider(&'a self, http_client: &'a reqwest::Client) -> Self::Provider;

    fn records(&'_ self) -> impl Iterator<Item = impl AsRef<str>>;
    fn hostname(&'_ self) -> &str;

    async fn update_domains(
        &'a self,
        http_client: &'a reqwest::Client,
        new_addr: IpAddr,
    ) -> Result<Updates> {
        // Use cloudflare's DNS to query all the configured records. Ideally we'd
        // use dns over tls for privacy purposes.
        //
        // We check all the records with DNS before issuing any requests to update
        // them so that we can be a good netizen. One issue seen with this approach
        // is that in subsequent invocations (cron, timers, etc) -- the dns record
        // won't have propagated yet. I haven't seen any issues with setting the
        // record to an unchanged value, but it is less than ideal.
        let resolver = DnsResolver::create_cloudflare();
        let provider = self.create_provider(http_client);

        let mut results = Updates::default();

        let hostname = self.hostname();
        for record in self.records() {
            let record = record.as_ref();
            let fqdn = if record == "@" {
                format!("{hostname}.")
            } else {
                format!("{record}.{hostname}.")
            };

            let response = resolver.ip_lookup(&fqdn, new_addr.into()).await;

            match response {
                Ok(current_addr) => {
                    if current_addr == new_addr {
                        results.current += 1;
                    } else {
                        provider.update_domain(record, new_addr).await?;
                        info!("{record} from domain {hostname} updated from {current_addr} to {new_addr}");
                        results.updated += 1;
                    }
                }
                Err(e) => {
                    // Could be a network issue or it could be that the record didn't exist.
                    warn!("resolving domain ({fqdn}) encountered an error: {e}");
                    results.missing += 1;
                }
            }
        }

        Ok(results)
    }
}

trait DnsLookupProvider {
    async fn update_domain(&self, record: &str, wan: IpAddr) -> Result<()>;
}
