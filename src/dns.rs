use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use anyhow::{anyhow, Context as _, Result};
use hickory_resolver::config::{NameServerConfigGroup, ResolverConfig, ResolverOpts};
use hickory_resolver::TokioAsyncResolver;

#[cfg_attr(test, faux::create)]
#[derive(Debug)]
pub struct DnsResolver {
    resolver: TokioAsyncResolver,
}

macro_rules! lookup {
    ($method:ident, $method_all:ident, $addr_type:path) => {
        #[cfg_attr(test, faux::methods)]
        impl DnsResolver {
            pub async fn $method_all(&self, host: &str) -> Result<Vec<$addr_type>> {
                let response = self
                    .resolver
                    .$method(host)
                    .await
                    .context("could not resolve via dns")?;
                Ok(response.iter().map(|record| record.0).collect())
            }

            pub async fn $method(&self, host: &str) -> Result<$addr_type> {
                let addrs = self.$method_all(host).await?;
                if addrs.len() == 1 {
                    Ok(addrs[0])
                } else {
                    Err(anyhow!("unexpected number of results: {}", addrs.len()))
                }
            }
        }
    };
}

#[cfg_attr(test, faux::methods)]
impl DnsResolver {
    #[allow(dead_code)]
    pub async fn create_opendns() -> Result<Self> {
        Self::from_config(config_opendns()).await
    }

    pub async fn create_cloudflare() -> Result<Self> {
        Self::from_config(ResolverConfig::cloudflare()).await
    }

    pub async fn from_config(config: ResolverConfig) -> Result<Self> {
        let resolver = TokioAsyncResolver::tokio(config, ResolverOpts::default());

        Ok(DnsResolver { resolver })
    }
}

lookup!(ipv4_lookup, ipv4_lookup_all, Ipv4Addr);
lookup!(ipv6_lookup, ipv6_lookup_all, Ipv6Addr);

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

    fn ipv4_is_global(addr: &Ipv4Addr) -> bool {
        !(addr.is_unspecified()
            || addr.is_private()
            || addr.is_loopback()
            || addr.is_link_local()
            || addr.is_documentation()
            || addr.is_broadcast())
    }

    fn ipv6_is_global(addr: &Ipv6Addr) -> bool {
        !(addr.is_unspecified()
            || addr.is_loopback()
            || addr.is_unique_local()
            || addr.is_unicast_link_local())
    }

    #[tokio::test]
    #[ignore]
    async fn opendns_lookup_ipv4_test() {
        // Heads up: this test requires internet connectivity
        let resolver = DnsResolver::create_opendns().await.unwrap();
        let ips = resolver.ipv4_lookup_all("example.com.").await.unwrap();
        assert!(!ips.is_empty(), "ips should be nonemtpy");
        for ip in ips {
            assert!(
                ipv4_is_global(&ip),
                "{} should be a globally routable address",
                ip
            );
        }
    }

    #[tokio::test]
    #[ignore]
    async fn opendns_lookup_ipv6_test() {
        // Heads up: this test requires internet connectivity
        let resolver = DnsResolver::create_opendns().await.unwrap();
        let ips = resolver.ipv6_lookup_all("example.com.").await.unwrap();
        assert!(!ips.is_empty(), "ips should be nonemtpy");
        for ip in ips {
            assert!(
                ipv6_is_global(&ip),
                "{} should be a globally routable address",
                ip
            );
        }
    }

    #[tokio::test]
    #[ignore]
    async fn cloudflare_lookup_ipv4_test() {
        // Heads up: this test requires internet connectivity
        let resolver = DnsResolver::create_cloudflare().await.unwrap();
        let ips = resolver.ipv4_lookup_all("example.com.").await.unwrap();
        assert!(!ips.is_empty(), "ips should be nonemtpy");
        for ip in ips {
            assert!(
                ipv4_is_global(&ip),
                "{} should be a globally routable address",
                ip
            );
        }
    }

    #[tokio::test]
    #[ignore]
    async fn cloudflare_lookup_ipv6_test() {
        // Heads up: this test requires internet connectivity
        let resolver = DnsResolver::create_cloudflare().await.unwrap();
        let ips = resolver.ipv6_lookup_all("example.com.").await.unwrap();
        assert!(!ips.is_empty(), "ips should be nonemtpy");
        for ip in ips {
            assert!(
                ipv6_is_global(&ip),
                "{} should be a globally routable address",
                ip
            );
        }
    }
}
