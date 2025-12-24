use crate::config::NoIpConfig;
use crate::errors::HttpError;
use crate::providers::{DnsLookupConfig, DnsLookupProvider};
use anyhow::{anyhow, Context as _, Result};
use std::net::IpAddr;

#[derive(Debug)]
pub struct NoIpProvider<'a> {
    client: &'a reqwest::Client,
    config: &'a NoIpConfig,
}

impl NoIpProvider<'_> {
    /// <https://www.noip.com/integrate/request>
    pub async fn update_domain(&self, wan: IpAddr) -> Result<()> {
        let base = self.config.base_url.trim_end_matches('/');
        let get_url = format!("{base}/nic/update");
        let response = self
            .client
            .get(&get_url)
            .query(&[
                ("hostname", &self.config.hostname),
                ("myip", &wan.to_string()),
            ])
            .basic_auth(&self.config.username, Some(&self.config.password))
            .send()
            .await
            .context(HttpError::send(&get_url, "noip update"))?
            .error_for_status()
            .context(HttpError::bad_response(&get_url, "noip update"))?
            .text()
            .await
            .context(HttpError::deserialize(&get_url, "noip update"))?;

        if !response.contains("good") {
            Err(anyhow!("expected zero errors, but received: {response}"))
        } else {
            Ok(())
        }
    }
}

impl<'a> DnsLookupConfig<'a> for NoIpConfig {
    type Provider = NoIpProvider<'a>;

    fn create_provider(&'a self, client: &'a reqwest::Client) -> Self::Provider {
        NoIpProvider {
            config: self,
            client,
        }
    }

    fn records(&self) -> impl Iterator<Item = impl AsRef<str>> {
        std::iter::once("@")
    }

    fn hostname(&self) -> &str {
        &self.hostname
    }
}

impl DnsLookupProvider for NoIpProvider<'_> {
    async fn update_domain(&self, record: &str, wan: IpAddr) -> Result<()> {
        let _ = record; // we only have one record to update
        self.update_domain(wan).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::IpType;
    use crate::core::Updates;
    use std::net::Ipv4Addr;

    macro_rules! noip_server {
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
    async fn test_noip_update() {
        let (tx, addr) = noip_server!();
        let http_client = reqwest::Client::new();
        let new_ip = IpAddr::V4(Ipv4Addr::new(2, 2, 2, 2));
        let config = NoIpConfig {
            base_url: format!("http://{}", addr),
            hostname: String::from("d.root-servers.net"),
            username: String::from("me@example.com"),
            password: String::from("my-pass"),
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
