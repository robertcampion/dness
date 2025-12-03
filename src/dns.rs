use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use hickory_resolver::config::{NameServerConfigGroup, ResolverConfig, ResolverOpts};
use hickory_resolver::TokioAsyncResolver;

use crate::errors::{DnsError, DnsErrorKind};

#[derive(Debug)]
pub struct DnsResolver {
    resolver: TokioAsyncResolver,
}

macro_rules! lookup {
    ($method:ident, $addr_type:ident) => {
        pub async fn $method(&self, host: &str) -> Result<$addr_type, DnsError> {
            let response = self.resolver.$method(host).await.map_err(|e| DnsError {
                kind: Box::new(DnsErrorKind::DnsResolve(e)),
            })?;

            response
                .iter()
                .next()
                .map(|address| address.0)
                .ok_or_else(|| DnsError {
                    kind: Box::new(DnsErrorKind::UnexpectedResponse(0)),
                })
        }
    };
}

impl DnsResolver {
    pub async fn create_opendns() -> Result<Self, DnsError> {
        Self::from_config(config_opendns()).await
    }

    pub async fn create_cloudflare() -> Result<Self, DnsError> {
        Self::from_config(ResolverConfig::cloudflare()).await
    }

    pub async fn from_config(config: ResolverConfig) -> Result<Self, DnsError> {
        let resolver = TokioAsyncResolver::tokio(config, ResolverOpts::default());

        Ok(DnsResolver { resolver })
    }

    lookup!(ipv4_lookup, Ipv4Addr);
    lookup!(ipv6_lookup, Ipv6Addr);
}

pub fn config_opendns() -> ResolverConfig {
    ResolverConfig::from_parts(None, vec![], nameservers_opendns())
}

// pub fn config_opendns_tls() -> ResolverConfig {
//     ResolverConfig::from_parts(None, vec![], nameservers_opendns_tls())
// }

// pub fn config_opendns_https() -> ResolverConfig {
//     ResolverConfig::from_parts(None, vec![], nameservers_opendns_https())
// }

pub fn nameservers_opendns() -> NameServerConfigGroup {
    NameServerConfigGroup::from_ips_clear(OPENDNS_IPS, 53, true)
}

// pub fn nameservers_opendns_tls() -> NameServerConfigGroup {
//     NameServerConfigGroup::from_ips_tls(OPENDNS_IPS, 853, "dns.opendns.com".to_string(), true)
// }

// pub fn nameservers_opendns_https() -> NameServerConfigGroup {
//     NameServerConfigGroup::from_ips_https(OPENDNS_IPS, 443, "doh.opendns.com".to_string(), true)
// }

/// IP address for the OpenDns DNS service
pub const OPENDNS_IPS: &[IpAddr] = &[
    IpAddr::V4(Ipv4Addr::new(208, 67, 222, 222)),
    IpAddr::V4(Ipv4Addr::new(208, 67, 220, 220)),
    IpAddr::V6(Ipv6Addr::new(0x2620, 0x119, 0x35, 0, 0, 0, 0, 0x35)),
    IpAddr::V6(Ipv6Addr::new(0x2620, 0x119, 0x53, 0, 0, 0, 0, 0x53)),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn opendns_lookup_ipv4_test() {
        // Heads up: this test requires internet connectivity
        let resolver = DnsResolver::create_opendns().await.unwrap();
        let ip = resolver.ipv4_lookup("example.com.").await.unwrap();
        assert!(!ip.is_loopback());
    }

    #[tokio::test]
    async fn opendns_lookup_ipv6_test() {
        // Heads up: this test requires internet connectivity
        let resolver = DnsResolver::create_opendns().await.unwrap();
        let ip = resolver.ipv6_lookup("example.com.").await.unwrap();
        assert!(!ip.is_loopback());
    }

    #[tokio::test]
    async fn cloudflare_lookup_ipv4_test() {
        // Heads up: this test requires internet connectivity
        let resolver = DnsResolver::create_cloudflare().await.unwrap();
        let ip = resolver.ipv4_lookup("example.com.").await.unwrap();
        assert!(!ip.is_loopback());
    }

    #[tokio::test]
    async fn cloudflare_lookup_ipv6_test() {
        // Heads up: this test requires internet connectivity
        let resolver = DnsResolver::create_cloudflare().await.unwrap();
        let ip = resolver.ipv6_lookup("example.com.").await.unwrap();
        assert!(!ip.is_loopback());
    }
}
