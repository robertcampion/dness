use crate::config::IpType;
use crate::errors::{DnsError, DnsErrorKind};
use hickory_resolver::config::{NameServerConfigGroup, ResolverConfig};
use hickory_resolver::name_server::TokioConnectionProvider;
use hickory_resolver::TokioResolver;
use std::net::IpAddr;

#[derive(Debug)]
pub struct DnsResolver {
    resolver: TokioResolver,
}

impl DnsResolver {
    pub fn from_ips_clear(ips: &[IpAddr]) -> Self {
        let config = ResolverConfig::from_parts(
            None,
            vec![],
            NameServerConfigGroup::from_ips_clear(ips, 53, false),
        );
        Self::from_config(config)
    }

    pub fn create_cloudflare() -> Self {
        Self::from_config(ResolverConfig::cloudflare())
    }

    pub fn from_config(config: ResolverConfig) -> Self {
        let resolver =
            TokioResolver::builder_with_config(config, TokioConnectionProvider::default()).build();

        DnsResolver { resolver }
    }

    pub async fn ip_lookup(&self, host: &str, ip_type: IpType) -> Result<IpAddr, DnsError> {
        let addrs: Vec<IpAddr> = match ip_type {
            IpType::V4 => self
                .resolver
                .ipv4_lookup(host)
                .await
                .map_err(|e| DnsError {
                    kind: DnsErrorKind::DnsResolve(Box::new(e)),
                })?
                .iter()
                .map(|r| r.0.into())
                .collect(),
            IpType::V6 => self
                .resolver
                .ipv6_lookup(host)
                .await
                .map_err(|e| DnsError {
                    kind: DnsErrorKind::DnsResolve(Box::new(e)),
                })?
                .iter()
                .map(|r| r.0.into())
                .collect(),
        };
        // error unless we got exactly one address
        if let [addr] = addrs.as_slice() {
            Ok(*addr)
        } else {
            Err(DnsError {
                kind: DnsErrorKind::UnexpectedResponse(addrs.len()),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn cloudflare_lookup_ipv4_test() {
        // Heads up: this test requires internet connectivity
        let resolver = DnsResolver::create_cloudflare();
        let ip = resolver
            .ip_lookup("d.root-servers.net.", IpType::V4)
            .await
            .unwrap();
        assert!(!ip.is_loopback());
    }

    #[tokio::test]
    #[ignore = "requires IPv6 internet connectivity"]
    async fn cloudflare_lookup_ipv6_test() {
        // Heads up: this test requires internet connectivity
        let resolver = DnsResolver::create_cloudflare();
        let ip = resolver
            .ip_lookup("d.root-servers.net.", IpType::V6)
            .await
            .unwrap();
        assert!(!ip.is_loopback());
    }
}
