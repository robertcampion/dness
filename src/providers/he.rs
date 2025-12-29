use crate::config::HeConfig;
use crate::providers::DdclientProtocolConfig;
use anyhow::Result;
use std::net::IpAddr;

impl DdclientProtocolConfig for HeConfig {
    fn name() -> &'static str {
        "he"
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

    /// <https://dns.he.net/docs.html>
    fn build_request(
        &self,
        request: reqwest::RequestBuilder,
        record: &str,
        wan: IpAddr,
    ) -> Result<reqwest::RequestBuilder> {
        let host = if record == "@" {
            &self.hostname
        } else {
            &format!("{}.{}", record, self.hostname)
        };

        let request = request
            // he.net closes the connection without sending a Connection: close
            // header. So we need to intentionally downgrade from HTTP/1.1,
            // where keep-alive is the default, to HTTP/1.0 so that reqwest will
            // expect this behavior and not attempt to re-use the connection.
            .version(reqwest::Version::HTTP_10)
            .query(&(
                ("hostname", &host),
                ("password", &self.password),
                ("myip", &wan),
            ));

        Ok(request)
    }

    fn response_ok(response: &str) -> bool {
        response.contains("good") || response.contains("nochg")
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
