use anyhow::Result;

use crate::config::DynuConfig;
use crate::providers::{DnsLookupConfig, DnsLookupProvider};
use std::net::IpAddr;

#[derive(Debug)]
pub struct DynuProvider<'a> {
    get_url: String,
    config: &'a DynuConfig,
    client: &'a reqwest::Client,
}

impl DnsLookupProvider for DynuProvider<'_> {
    fn name() -> &'static str {
        "dynu"
    }

    fn create_request(&self, record: &str, wan: IpAddr) -> Result<reqwest::RequestBuilder> {
        let request = self
            .client
            .get(&self.get_url)
            .basic_auth(&self.config.username, Some(&self.config.password))
            .query(&[("hostname", &self.config.hostname)]);

        let request = match wan {
            IpAddr::V4(ipv4_addr) => request.query(&(("myip", ipv4_addr), ("myipv6", "no"))),
            IpAddr::V6(ipv6_addr) => request.query(&(("myip", "no"), ("myipv6", ipv6_addr))),
        };

        let request = if record != "@" {
            request.query(&[("alias", String::from(record))])
        } else {
            request
        };

        Ok(request)
    }

    fn response_ok(response: &str) -> bool {
        response.contains("nochg") || response.contains("good")
    }
}

impl<'a> DnsLookupConfig<'a> for DynuConfig {
    type Provider = DynuProvider<'a>;

    fn create_provider(&'a self, client: &'a reqwest::Client) -> Self::Provider {
        let base_url = self.base_url.trim_end_matches('/');
        let get_url = format!("{base_url}/nic/update");

        DynuProvider {
            get_url,
            config: self,
            client,
        }
    }

    fn records(&self) -> impl Iterator<Item = impl AsRef<str>> {
        self.records.iter()
    }

    fn hostname(&self) -> &str {
        &self.hostname
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::IpType;
    use crate::core::Updates;
    use std::net::Ipv4Addr;

    macro_rules! dynu_server {
        () => {{
            use rouille::{Response, Server};

            let server = Server::new("localhost:0", |request| match request.url().as_str() {
                "/nic/update" => Response::from_data("text/plain", b"good 2.2.2.2".to_vec()),
                _ => Response::empty_404(),
            })
            .unwrap();

            let (tx, rx) = std::sync::mpsc::sync_channel(1);
            let addr = server.server_addr().clone();
            std::thread::spawn(move || {
                while let Err(_) = rx.try_recv() {
                    server.poll();
                    std::thread::sleep(std::time::Duration::from_millis(50))
                }
            });
            (tx, addr)
        }};
    }

    #[tokio::test]
    async fn test_dynu_update() {
        let (tx, addr) = dynu_server!();
        let http_client = reqwest::Client::new();
        let new_ip = IpAddr::V4(Ipv4Addr::new(2, 2, 2, 2));
        let config = DynuConfig {
            base_url: format!("http://{}", addr),
            hostname: String::from("root-servers.net"),
            username: String::from("myusername"),
            password: String::from("secret-1"),
            records: vec![String::from("d")],
            ip_types: vec![IpType::V4],
        };

        let summary = config.update_domains(&http_client, new_ip).await.unwrap();
        tx.send(()).unwrap();

        assert_eq!(
            summary,
            Updates {
                current: 0,
                updated: 1,
                missing: 0,
            }
        );
    }
}
