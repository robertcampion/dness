use crate::config::HeConfig;
use crate::errors::HttpError;
use crate::providers::{DnsLookupConfig, DnsLookupProvider};
use anyhow::{anyhow, Context as _, Result};
use std::net::IpAddr;

#[derive(Debug)]
pub struct HeProvider<'a> {
    config: &'a HeConfig,
    client: &'a reqwest::Client,
}

impl HeProvider<'_> {
    /// <https://dns.he.net/docs.html>
    pub async fn update_domain(&self, host: &str, wan: IpAddr) -> Result<()> {
        let base = self.config.base_url.trim_end_matches('/');
        let url = format!("{base}/nic/update");
        let params = [
            ("hostname", host),
            ("password", &self.config.password),
            ("myip", &wan.to_string()),
        ];

        let response = self
            .client
            .post(&url)
            // he.net closes the connection without sending a Connection: close
            // header. So we need to intentionally downgrade from HTTP/1.1,
            // where keep-alive is the default, to HTTP/1.0 so that reqwest will
            // expect this behavior and not attempt to re-use the connection.
            .version(reqwest::Version::HTTP_10)
            .form(&params)
            .send()
            .await
            .context(HttpError::send(&url, "he update"))?
            .error_for_status()
            .context(HttpError::bad_response(&url, "he update"))?
            .text()
            .await
            .context(HttpError::deserialize(&url, "he update"))?;

        if !response.contains("good") && !response.contains("nochg") {
            Err(anyhow!("expected zero errors, but received: {response}"))
        } else {
            Ok(())
        }
    }
}

impl<'a> DnsLookupConfig<'a> for HeConfig {
    type Provider = HeProvider<'a>;

    fn create_provider(&'a self, client: &'a reqwest::Client) -> Self::Provider {
        HeProvider {
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

impl DnsLookupProvider for HeProvider<'_> {
    async fn update_domain(&self, record: &str, wan: IpAddr) -> Result<()> {
        let host_record = if record == "@" {
            self.config.hostname.clone()
        } else {
            format!("{}.{}", record, self.config.hostname)
        };

        self.update_domain(&host_record, wan).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::IpType;
    use crate::core::Updates;
    use std::net::Ipv4Addr;

    macro_rules! he_server {
        () => {{
            use rouille::{Response, Server};

            let server = Server::new("localhost:0", |request| match request.url().as_str() {
                "/nic/update" => Response::from_data("text/html", (b"good 2.2.2.2").to_vec()),
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
    async fn test_he_update() {
        let (tx, addr) = he_server!();
        let http_client = reqwest::Client::new();
        let new_ip = IpAddr::V4(Ipv4Addr::new(2, 2, 2, 2));
        let config = HeConfig {
            base_url: format!("http://{}", addr),
            hostname: String::from("root-servers.net"),
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
