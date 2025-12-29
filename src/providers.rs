mod cloudflare;
mod dynu;
mod godaddy;
mod he;
mod namecheap;
mod noip;
mod porkbun;

use crate::core::Updates;
use crate::dns::DnsResolver;
use crate::{config::DomainConfig, errors::HttpError};
use anyhow::{anyhow, Context as _, Result};
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

trait DdclientProtocolConfig {
    fn name() -> &'static str;
    fn endpoint(&self) -> String;

    fn hostname(&self) -> &str;
    fn records(&self) -> &[String];

    fn build_request(
        &self,
        request: reqwest::RequestBuilder,
        record: &str,
        wan: IpAddr,
    ) -> Result<reqwest::RequestBuilder>;
    fn response_ok(response: &str) -> bool;

    async fn update_domain(
        &self,
        client: &reqwest::Client,
        record: &str,
        wan: IpAddr,
    ) -> Result<()> {
        let request = client.get(self.endpoint());
        let request = self.build_request(request, record, wan)?.build();
        let context = || format!("{} update", Self::name());

        let request = request.map_err(|e| {
            let url = e.url().map_or("", |u| u.as_str()).to_owned();
            anyhow!(e).context(HttpError::send(&url, &context()))
        })?;
        let url = request.url().as_str().to_owned();

        let response = client
            .execute(request)
            .await
            .with_context(|| HttpError::send(&url, &context()))?
            .error_for_status()
            .with_context(|| HttpError::bad_response(&url, &context()))?
            .text()
            .await
            .with_context(|| HttpError::deserialize(&url, &context()))?;

        if !Self::response_ok(&response) {
            Err(anyhow!("expected zero errors, but received: {response}"))
        } else {
            Ok(())
        }
    }

    async fn update_domains(
        &self,
        http_client: &reqwest::Client,
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
                        self.update_domain(http_client, record, new_addr).await?;
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
