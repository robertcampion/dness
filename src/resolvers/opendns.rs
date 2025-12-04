//! Querying the OpenDNS DNS servers for "myip.opendns.com" returns the
//! requester's IP address. Note that this will fail if OpenDNS considers the
//! address to be part of a CGNAT, in which case it will return an empty
//! response with NOERROR.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use anyhow::Result;
use hickory_resolver::config::ResolverConfig;

use crate::core::IpType;
use crate::dns::{config_opendns, DnsResolver};

/// Get current WAN IP of specified type (IPv4 or IPv6) using OpenDNS resolver
pub async fn opendns_resolve_ip(ip_type: IpType) -> Result<IpAddr> {
    // get_config and resolve_ip are separated out for unit testing
    let config = opendns_config(ip_type);
    let resolver = DnsResolver::from_config(config).await?;
    resolve_ip(ip_type, resolver).await
}

/// Get current WAN IP of specified type (IPv4 or IPv6) using provided resolver
async fn resolve_ip<T: DnsResolverTrait>(ip_type: IpType, resolver: T) -> Result<IpAddr> {
    const DOMAIN: &str = "myip.opendns.com.";
    match ip_type {
        IpType::V4 => resolver.ipv4_lookup(DOMAIN).await.map(Into::into),
        IpType::V6 => resolver.ipv6_lookup(DOMAIN).await.map(Into::into),
    }
}

/// Get DNS resolver config for OpenDNS servers of specified IP type
fn opendns_config(ip_type: IpType) -> ResolverConfig {
    let base_config = config_opendns();
    let name_servers: Vec<_> = base_config
        .name_servers()
        .iter()
        .filter(|name_server| IpType::from(name_server.socket_addr.ip()) == ip_type)
        .cloned()
        .collect();
    let config = ResolverConfig::from_parts(
        base_config.domain().cloned(),
        base_config.search().to_vec(),
        name_servers,
    );
    config
}

/// Trait to mock DnsResolver for testing
#[mockall::automock]
trait DnsResolverTrait {
    async fn ipv4_lookup(&self, host: &str) -> Result<Ipv4Addr>;
    async fn ipv6_lookup(&self, host: &str) -> Result<Ipv6Addr>;
}

impl DnsResolverTrait for DnsResolver {
    async fn ipv4_lookup(&self, host: &str) -> Result<Ipv4Addr> {
        Ok(self.ipv4_lookup(host).await?)
    }
    async fn ipv6_lookup(&self, host: &str) -> Result<Ipv6Addr> {
        Ok(self.ipv6_lookup(host).await?)
    }
}

#[cfg(test)]
mod tests {
    use anyhow::anyhow;
    use mockall::predicate;

    use super::*;

    #[test]
    fn ipv4_config() {
        // test that the IPv4 config only uses IPv4 nameservers
        let config = opendns_config(IpType::V4);
        assert!(
            !config.name_servers().is_empty(),
            "name servers should be nonempty"
        );
        for name_server in config.name_servers() {
            let ip = name_server.socket_addr.ip();
            assert!(ip.is_ipv4(), "{} should be an IPv4 address", ip);
        }
    }

    #[test]
    fn ipv6_config() {
        // test that the IPv6 config only uses IPv6 nameservers
        let config = opendns_config(IpType::V6);
        assert!(
            !config.name_servers().is_empty(),
            "name servers should be nonempty"
        );
        for name_server in config.name_servers() {
            let ip = name_server.socket_addr.ip();
            assert!(ip.is_ipv6(), "{} should be an IPv6 address", ip);
        }
    }

    const TEST_ADDR_V4: Ipv4Addr = Ipv4Addr::new(192, 0, 2, 1);
    const TEST_ADDR_V6: Ipv6Addr = Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1);

    #[tokio::test]
    async fn ipv4_lookup() {
        // test that resolve_ip queries the correct hostname
        let mut mock_resolver = MockDnsResolverTrait::new();
        mock_resolver
            .expect_ipv4_lookup()
            .with(predicate::eq("myip.opendns.com."))
            .returning(|_| Ok(TEST_ADDR_V4))
            .times(1);
        mock_resolver.expect_ipv6_lookup().never();

        assert_eq!(
            resolve_ip(IpType::V4, mock_resolver).await.unwrap(),
            TEST_ADDR_V4
        );

        // test that resolve_ip fails if resolution fails, including due to an
        // empty response
        let mut mock_resolver = MockDnsResolverTrait::new();
        mock_resolver
            .expect_ipv4_lookup()
            .with(predicate::eq("myip.opendns.com."))
            .returning(|_| Err(anyhow!("DNS resolution failure")))
            .times(1);
        mock_resolver.expect_ipv6_lookup().never();

        assert!(resolve_ip(IpType::V4, mock_resolver).await.is_err());
    }

    #[tokio::test]
    async fn ipv6_lookup() {
        // test that resolve_ip queries the correct hostname
        let mut mock_resolver = MockDnsResolverTrait::new();
        mock_resolver
            .expect_ipv6_lookup()
            .with(predicate::eq("myip.opendns.com."))
            .returning(|_| Ok(TEST_ADDR_V6))
            .times(1);
        mock_resolver.expect_ipv4_lookup().never();

        assert_eq!(
            resolve_ip(IpType::V6, mock_resolver).await.unwrap(),
            TEST_ADDR_V6
        );

        // test that resolve_ip fails if resolution fails, including due to an
        // empty response
        let mut mock_resolver = MockDnsResolverTrait::new();
        mock_resolver
            .expect_ipv6_lookup()
            .with(predicate::eq("myip.opendns.com."))
            .returning(|_| Err(anyhow!("DNS resolution failure")))
            .times(1);
        mock_resolver.expect_ipv4_lookup().never();

        assert!(resolve_ip(IpType::V6, mock_resolver).await.is_err());
    }
}
