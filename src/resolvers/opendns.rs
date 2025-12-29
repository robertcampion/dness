use crate::config::IpType;
use crate::dns::DnsResolver;
use anyhow::Result;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

pub async fn wan_lookup_ip(ip_type: IpType) -> Result<IpAddr> {
    let ips = match ip_type {
        IpType::V4 => [
            IpAddr::V4(Ipv4Addr::new(208, 67, 222, 222)),
            IpAddr::V4(Ipv4Addr::new(208, 67, 220, 220)),
        ],
        IpType::V6 => [
            IpAddr::V6(Ipv6Addr::new(0x2620, 0x119, 0x35, 0, 0, 0, 0, 0x35)),
            IpAddr::V6(Ipv6Addr::new(0x2620, 0x119, 0x53, 0, 0, 0, 0, 0x53)),
        ],
    };
    let resolver = DnsResolver::from_ips_clear(&ips);
    resolver.ip_lookup("myip.opendns.com.", ip_type).await
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
                eprintln!("Error: {}", e);
                let context: &DnsErrorKind = e.downcast_ref().unwrap();
                match context {
                    DnsErrorKind::DnsResolve => {
                        // Check if this is a "no records found" error (e.g., CGNAT scenario)
                        if let hickory_resolver::ResolveErrorKind::Proto(proto_err) = e
                            .source()
                            .unwrap()
                            .downcast_ref::<hickory_resolver::ResolveError>()
                            .unwrap()
                            .kind()
                        {
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
                eprintln!("Error: {}", e);
                let context: &DnsErrorKind = e.downcast_ref().unwrap();
                match context {
                    DnsErrorKind::DnsResolve => {
                        // Check if this is a "no records found" error (e.g., CGNAT scenario)
                        if let hickory_resolver::ResolveErrorKind::Proto(proto_err) = e
                            .source()
                            .unwrap()
                            .downcast_ref::<hickory_resolver::ResolveError>()
                            .unwrap()
                            .kind()
                        {
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
