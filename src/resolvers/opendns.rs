use std::net::IpAddr;

use crate::config::IpType;
use crate::dns::DnsResolver;
use crate::errors::DnsError;

pub async fn wan_lookup_ip(ip_type: IpType) -> Result<IpAddr, DnsError> {
    let opendns = OpenDnsResolver::create(ip_type);
    opendns.wan_lookup().await
}

#[derive(Debug)]
struct OpenDnsResolver {
    resolver: DnsResolver,
    ip_type: IpType,
}

impl OpenDnsResolver {
    fn create(ip_type: IpType) -> Self {
        let resolver = DnsResolver::create_opendns(ip_type);
        OpenDnsResolver { resolver, ip_type }
    }

    async fn wan_lookup(&self) -> Result<IpAddr, DnsError> {
        const DOMAIN: &str = "myip.opendns.com.";
        self.resolver.ip_lookup(DOMAIN, self.ip_type).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::errors::DnsErrorKind;

    #[tokio::test]
    async fn opendns_lookup_ipv4_test() {
        // Heads up: this test requires internet connectivity
        match wan_lookup_ip(IpType::V4).await {
            Ok(ip) => {
                assert!(ip.is_ipv4());
                assert!(!ip.is_loopback());
            }
            Err(e) => {
                match e.kind {
                    DnsErrorKind::DnsResolve(e) => {
                        // Check if this is a "no records found" error (e.g., CGNAT scenario)
                        if let hickory_resolver::ResolveErrorKind::Proto(proto_err) = e.kind() {
                            if proto_err.is_no_records_found() {
                                // This is fine, just means we're behind a CGNAT
                                return;
                            }
                        }
                        panic!("unexpected DNS error: {}", e);
                    }
                    DnsErrorKind::UnexpectedResponse(_) => {
                        panic!("unexpected response: {}", e);
                    }
                }
            }
        }
    }

    #[tokio::test]
    #[ignore = "requires IPv6 internet connectivity"]
    async fn opendns_lookup_ipv6_test() {
        // Heads up: this test requires internet connectivity
        match wan_lookup_ip(IpType::V6).await {
            Ok(ip) => {
                assert!(ip.is_ipv6());
                assert!(!ip.is_loopback());
            }
            Err(e) => {
                match e.kind {
                    DnsErrorKind::DnsResolve(e) => {
                        // Check if this is a "no records found" error (e.g., CGNAT scenario)
                        if let hickory_resolver::ResolveErrorKind::Proto(proto_err) = e.kind() {
                            if proto_err.is_no_records_found() {
                                // This is fine, just means we're behind a CGNAT
                                return;
                            }
                        }
                        panic!("unexpected DNS error: {}", e);
                    }
                    DnsErrorKind::UnexpectedResponse(_) => {
                        panic!("unexpected response: {}", e);
                    }
                }
            }
        }
    }
}
