use crate::config::NamecheapConfig;
use crate::providers::{DnsLookupConfig, DnsLookupProvider};
use anyhow::{anyhow, Result};
use std::net::IpAddr;

#[derive(Debug)]
pub struct NamecheapProvider<'a> {
    get_url: String,
    client: &'a reqwest::Client,
    config: &'a NamecheapConfig,
}

impl DnsLookupProvider for NamecheapProvider<'_> {
    fn name() -> &'static str {
        "namecheap"
    }
    /// <https://www.namecheap.com/support/knowledgebase/article.aspx/29/11/how-do-i-use-a-browser-to-dynamically-update-the-hosts-ip>
    fn create_request(&self, record: &str, wan: IpAddr) -> Result<reqwest::RequestBuilder> {
        let IpAddr::V4(wan) = wan else {
            return Err(anyhow!("IPv6 not supported for Namecheap"));
        };

        let request = self.client.get(&self.get_url).query(&(
            ("host", record),
            ("domain", &self.config.domain),
            ("password", &self.config.ddns_password),
            ("ip", &wan),
        ));

        Ok(request)
    }

    fn response_ok(response: &str) -> bool {
        response.contains("<ErrCount>0</ErrCount>")
    }
}

impl<'a> DnsLookupConfig<'a> for NamecheapConfig {
    type Provider = NamecheapProvider<'a>;

    fn create_provider(&'a self, client: &'a reqwest::Client) -> Self::Provider {
        let base_url = self.base_url.trim_end_matches('/').to_string();
        let get_url = format!("{base_url}/update");

        NamecheapProvider {
            get_url,
            config: self,
            client,
        }
    }

    fn records(&self) -> impl Iterator<Item = impl AsRef<str>> {
        self.records.iter()
    }

    fn hostname(&self) -> &str {
        &self.domain
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::Updates;
    use std::net::Ipv4Addr;

    macro_rules! namecheap_server {
        () => {{
            use rouille::{Response, Server};

            let server = Server::new("localhost:0", |request| match request.url().as_str() {
                "/update" => Response::from_data(
                    "text/html",
                    include_bytes!("../../assets/namecheap-update.xml").to_vec(),
                ),
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
    async fn test_namecheap_update() {
        let (tx, addr) = namecheap_server!();
        let http_client = reqwest::Client::new();
        let new_ip = IpAddr::V4(Ipv4Addr::new(2, 2, 2, 2));
        let config = NamecheapConfig {
            base_url: format!("http://{}", addr),
            domain: String::from("root-servers.net"),
            ddns_password: String::from("secret-1"),
            records: vec![String::from("d")],
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
