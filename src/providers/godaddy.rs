use std::collections::{BTreeMap as Map, HashSet};
use std::net::{IpAddr, Ipv4Addr};

use anyhow::Result;
use log::{debug, info, warn};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::core::Updates;
use crate::errors::ClientErrorWrapper as _;

#[derive(Deserialize, Clone, PartialEq, Debug)]
#[serde(deny_unknown_fields)]
pub struct GoDaddyConfig {
    #[serde(default = "godaddy_base_url")]
    pub base_url: String,
    pub key: String,
    pub secret: String,
    pub domain: String,
    pub records: Vec<String>,
}

fn godaddy_base_url() -> String {
    "https://api.godaddy.com".to_owned()
}

#[derive(Deserialize, Serialize, PartialEq, Clone, Debug)]
struct GoRecord {
    data: String,
    name: String,

    #[serde(flatten)]
    other: Map<String, Value>,
}

#[derive(Clone, Debug)]
struct GoClient<'a> {
    base_url: String,
    domain: String,
    key: String,
    secret: String,
    records: HashSet<String>,
    client: &'a reqwest::Client,
}

impl GoClient<'_> {
    fn log_missing_domains(&self, remote_domains: &[GoRecord]) -> usize {
        let actual = remote_domains
            .iter()
            .map(|x| &x.name)
            .cloned()
            .collect::<HashSet<String>>();
        crate::core::log_missing_domains(&self.records, &actual, "GoDaddy", &self.domain)
    }

    fn auth_header(&self) -> String {
        format!("sso-key {}:{}", self.key, self.secret)
    }

    async fn fetch_records(&self) -> Result<Vec<GoRecord>> {
        let get_url = format!("{}/v1/domains/{}/records/A", self.base_url, self.domain);
        self.client
            .get(&get_url)
            .header("Authorization", self.auth_header())
            .send_json("godaddy fetch records")
            .await
    }

    async fn update_record(&self, record: &GoRecord, addr: Ipv4Addr) -> Result<()> {
        let put_url = format!(
            "{}/v1/domains/{}/records/A/{}",
            self.base_url, self.domain, record.name
        );

        self.client
            .put(&put_url)
            .header("Authorization", self.auth_header())
            .json(&vec![GoRecord {
                data: addr.to_string(),
                ..record.clone()
            }])
            .send_err("godaddy update records")
            .await?;

        Ok(())
    }

    async fn ensure_current_ip(&self, record: &GoRecord, addr: Ipv4Addr) -> Result<Updates> {
        let mut current = 0;
        let mut updated = 0;
        match record.data.parse::<Ipv4Addr>() {
            Ok(ip) => {
                if ip != addr {
                    updated += 1;
                    self.update_record(record, addr).await?;

                    info!(
                        "{} from domain {} updated from {} to {}",
                        record.name, self.domain, record.data, addr
                    )
                } else {
                    current += 1;
                    debug!(
                        "{} from domain {} is already current",
                        record.name, self.domain
                    )
                }
            }
            Err(ref e) => {
                updated += 1;
                warn!("could not parse domain {} address {} as ipv4 -- will replace it. Original error: {}", record.name, record.data, e);
                self.update_record(record, addr).await?;

                info!(
                    "{} from domain {} updated from {} to {}",
                    record.name, self.domain, record.data, addr
                )
            }
        }

        Ok(Updates {
            updated,
            current,
            ..Updates::default()
        })
    }
}

/// GoDaddy dynamic dns service works as the following:
///
/// 1. Send a GET request to find all records in the domain
/// 2. Find all the expected records (and log those that are missing) and check their current IP
/// 3. Update the remote IP as needed, ensuring that original properties are preserved in the
///    upload, so that we don't overwrite a property like TTL.
pub async fn update_domains(
    client: &reqwest::Client,
    config: &GoDaddyConfig,
    addr: IpAddr,
) -> Result<Updates> {
    let IpAddr::V4(addr) = addr else {
        unimplemented!("IPv6 not supported for GoDaddy")
    };
    let go_client = GoClient {
        base_url: config.base_url.trim_end_matches('/').to_string(),
        domain: config.domain.clone(),
        key: config.key.clone(),
        secret: config.secret.clone(),
        records: config.records.iter().cloned().collect(),
        client,
    };

    let records = go_client.fetch_records().await?;
    let missing = go_client.log_missing_domains(&records) as i32;
    let mut summary = Updates {
        missing,
        ..Updates::default()
    };

    for record in records {
        if go_client.records.contains(&record.name) {
            summary += go_client.ensure_current_ip(&record, addr).await?;
        }
    }

    Ok(summary)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn deserialize_go_records() {
        let json_str = &include_str!("../../assets/godaddy-get-records.json");
        let response: Vec<GoRecord> = serde_json::from_str(json_str).unwrap();
        let mut expected = Map::new();
        expected.insert("ttl".to_owned(), Value::Number(600.into()));
        expected.insert("type".to_owned(), Value::String("A".to_owned()));
        assert_eq!(
            response,
            vec![GoRecord {
                name: "@".to_owned(),
                data: "256.256.256.256".to_owned(),
                other: expected,
            }]
        );
    }

    #[test]
    fn serialize_go_records() {
        let mut other = Map::new();
        other.insert("ttl".to_owned(), Value::Number(600.into()));
        let rec = GoRecord {
            data: "256.256.256.256".to_owned(),
            name: "@".to_owned(),
            other,
        };

        let actual = serde_json::to_string(&rec).unwrap();
        let expected = serde_json::to_string(&json!({
            "name": "@",
            "data": "256.256.256.256",
            "ttl": 600
        }))
        .unwrap();
        assert_eq!(actual, expected);
    }

    macro_rules! godaddy_rouille_server {
        () => {{
            use rouille::{Response, Server};

            let server = Server::new("localhost:0", |request| match request.url().as_str() {
                "/v1/domains/domain-1.com/records/A" => Response::from_data(
                    "application/json",
                    include_bytes!("../../assets/godaddy-get-records.json").to_vec(),
                ),
                "/v1/domains/domain-1.com/records/A/@" => Response::text("Nice job!"),
                "/v1/domains/domain-2.com/records/A" => Response::from_data(
                    "application/json",
                    r#"[{"name": "@", "data": "2.2.2.2"}, {"name": "a", "data": "2.1.2.2"}]"#,
                ),
                "/v1/domains/domain-2.com/records/A/@" => Response::text("Nice job!"),
                "/v1/domains/domain-2.com/records/A/a" => Response::text("Nice job!"),
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
    async fn test_godaddy_unparseable_ipv4() {
        let (tx, addr) = godaddy_rouille_server!();
        let http_client = reqwest::Client::new();
        let new_ip = IpAddr::V4(Ipv4Addr::new(2, 2, 2, 2));
        let config = GoDaddyConfig {
            base_url: format!("http://{}", addr),
            domain: "domain-1.com".to_owned(),
            key: "key-1".to_owned(),
            secret: "secret-1".to_owned(),
            records: vec!["@".to_owned()],
        };

        let summary = update_domains(&http_client, &config, new_ip).await.unwrap();
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

    #[tokio::test]
    async fn test_godaddy_grabbag() {
        let (tx, addr) = godaddy_rouille_server!();
        let http_client = reqwest::Client::new();
        let new_ip = IpAddr::V4(Ipv4Addr::new(2, 2, 2, 2));
        let config = GoDaddyConfig {
            base_url: format!("http://{}", addr),
            domain: "domain-2.com".to_owned(),
            key: "key-1".to_owned(),
            secret: "secret-1".to_owned(),
            records: vec!["@".to_owned(), "a".to_owned(), "b".to_owned()],
        };

        let summary = update_domains(&http_client, &config, new_ip).await.unwrap();
        tx.send(()).unwrap();

        assert_eq!(
            summary,
            Updates {
                current: 1,
                updated: 1,
                missing: 1,
            }
        );
    }
}
