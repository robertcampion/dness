use crate::config::IpType;
use crate::errors::{DnsError, DnsErrorKind};
use hickory_resolver::config::{NameServerConfigGroup, ResolverConfig};
use hickory_resolver::name_server::TokioConnectionProvider;
use hickory_resolver::TokioResolver;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

#[derive(Debug)]
pub struct DnsResolver {
    resolver: TokioResolver,
}

impl DnsResolver {
    pub fn create_opendns(ip_type: IpType) -> Self {
        let ips = // OpenDNS nameservers:
                // https://en.wikipedia.org/wiki/OpenDNS#Name_server_IP_addresses
                match ip_type {
                    IpType::V4 => [
                        IpAddr::V4(Ipv4Addr::new(208, 67, 222, 222)),
                        IpAddr::V4(Ipv4Addr::new(208, 67, 220, 220)),
                    ],
                    IpType::V6 => [
                        IpAddr::V6(Ipv6Addr::new(0x2620, 0x119, 0x35, 0, 0, 0, 0, 0x35)),
                        IpAddr::V6(Ipv6Addr::new(0x2620, 0x119, 0x53, 0, 0, 0, 0, 0x53)),
                    ],
                };

        let config = ResolverConfig::from_parts(
            None,
            vec![],
            NameServerConfigGroup::from_ips_clear(&ips, 53, false),
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
