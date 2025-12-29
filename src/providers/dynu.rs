use anyhow::Result;

use crate::config::DynuConfig;
use crate::providers::DdclientProtocolConfig;
use std::net::IpAddr;

impl DdclientProtocolConfig for DynuConfig {
    fn name() -> &'static str {
        "dynu"
    }

    fn endpoint(&self) -> String {
        let base_url = self.base_url.trim_end_matches('/');
        format!("{base_url}/nic/update")
    }

    fn hostname(&self) -> &str {
        &self.hostname
    }

    fn records(&self) -> &[String] {
        &self.records
    }

    fn build_request(
        &self,
        request: reqwest::RequestBuilder,
        record: &str,
        wan: IpAddr,
    ) -> Result<reqwest::RequestBuilder> {
        let request = request
            .basic_auth(&self.username, Some(&self.password))
            .query(&[("hostname", &self.hostname)]);

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
