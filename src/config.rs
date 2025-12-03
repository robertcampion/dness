use std::collections::HashMap;
use std::fs::File;
use std::io::{Error as IoError, Read};
use std::path::Path;
use std::{error, fmt};

use handlebars::{Handlebars, RenderError, TemplateError};
use log::LevelFilter;
use serde::Deserialize;

use crate::providers::DomainConfig;
use crate::resolvers::Resolver;

#[derive(Debug)]
pub struct ConfigError {
    kind: ConfigErrorKind,
}

#[derive(Debug)]
pub enum ConfigErrorKind {
    FileNotFound(IoError),
    Misread(IoError),
    Parse(toml::de::Error),
    Template(TemplateError),
    Render(RenderError),
}

impl error::Error for ConfigError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self.kind {
            ConfigErrorKind::FileNotFound(ref e) => Some(e),
            ConfigErrorKind::Misread(ref e) => Some(e),
            ConfigErrorKind::Parse(ref e) => Some(e),
            ConfigErrorKind::Template(ref e) => Some(e),
            ConfigErrorKind::Render(ref e) => Some(e),
        }
    }
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "config issue: ")?;
        match self.kind {
            ConfigErrorKind::FileNotFound(ref _e) => write!(f, "file not found"),
            ConfigErrorKind::Misread(ref _e) => write!(f, "unable to read file"),
            ConfigErrorKind::Parse(ref _e) => write!(f, "a parsing error"),
            ConfigErrorKind::Template(ref _e) => write!(f, "config template error"),
            ConfigErrorKind::Render(ref _e) => write!(f, "config template rendering error"),
        }
    }
}

#[derive(Deserialize, Clone, PartialEq, Debug, Default)]
#[serde(deny_unknown_fields)]
pub struct DnsConfig {
    #[serde(default)]
    pub ip_resolver: Resolver,

    #[serde(default)]
    pub log: LogConfig,

    #[serde(default)]
    pub domains: Vec<DomainConfig>,
}

#[derive(Deserialize, Clone, PartialEq, Debug)]
#[serde(deny_unknown_fields)]
pub struct LogConfig {
    #[serde(default = "default_log_level")]
    pub level: LevelFilter,
}

fn default_log_level() -> LevelFilter {
    LevelFilter::Info
}

impl Default for LogConfig {
    fn default() -> LogConfig {
        LogConfig {
            level: default_log_level(),
        }
    }
}

pub fn parse_config<P: AsRef<Path>>(
    path: P,
    env: &HashMap<String, String>,
) -> Result<DnsConfig, ConfigError> {
    let mut f = File::open(path).map_err(|e| ConfigError {
        kind: ConfigErrorKind::FileNotFound(e),
    })?;

    let mut contents = String::new();
    f.read_to_string(&mut contents).map_err(|e| ConfigError {
        kind: ConfigErrorKind::Misread(e),
    })?;

    let mut handlebars = Handlebars::new();

    handlebars
        .register_template_string("dness_config", contents)
        .map_err(|e| ConfigError {
            kind: ConfigErrorKind::Template(e),
        })?;
    handlebars.register_escape_fn(handlebars::no_escape);
    handlebars.set_strict_mode(true);

    let config_contents = handlebars
        .render("dness_config", env)
        .map_err(|e| ConfigError {
            kind: ConfigErrorKind::Render(e),
        })?;

    toml::from_str(&config_contents).map_err(|e| ConfigError {
        kind: ConfigErrorKind::Parse(e),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::IpType;
    use crate::providers::{
        CloudflareConfig, DomainConfig, DynuConfig, GoDaddyConfig, HeConfig, NamecheapConfig,
        NoIpConfig,
    };

    #[test]
    fn deserialize_config_empty() {
        let config: DnsConfig = toml::from_str("").unwrap();
        assert_eq!(
            config,
            DnsConfig {
                ip_resolver: Default::default(),
                log: LogConfig {
                    level: LevelFilter::Info,
                },
                domains: vec![]
            }
        )
    }

    #[test]
    fn deserialize_config_deny_unknown() {
        let err = toml::from_str::<DnsConfig>(r#"log_info = "DEBUG""#).unwrap_err();
        let msg = format!("{}", err);
        assert!(msg.contains("unknown field `log_info`"));
    }

    #[test]
    fn deserialize_config_simple() {
        let toml_str = &include_str!("../assets/base-config.toml");
        let config: DnsConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(
            config,
            DnsConfig {
                ip_resolver: Default::default(),
                log: LogConfig {
                    level: LevelFilter::Info,
                },
                domains: vec![DomainConfig::Cloudflare(CloudflareConfig {
                    email: None,
                    key: None,
                    token: Some("dec0de".to_owned()),
                    zone: "example.com".to_owned(),
                    records: vec!["n.example.com".to_owned()],
                    ip_types: vec![IpType::V4],
                })]
            }
        );
    }

    #[test]
    fn deserialize_config_ipv6() {
        let toml_str = &include_str!("../assets/ipv6-config.toml");
        let config: DnsConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(
            config,
            DnsConfig {
                ip_resolver: Default::default(),
                log: LogConfig {
                    level: LevelFilter::Info,
                },
                domains: vec![DomainConfig::Cloudflare(CloudflareConfig {
                    email: None,
                    key: None,
                    token: Some("dec0de".to_owned()),
                    zone: "example.com".to_owned(),
                    records: vec!["n.example.com".to_owned()],
                    ip_types: vec![IpType::V6],
                })]
            }
        );
    }

    #[test]
    fn deserialize_config_dual_stack() {
        let toml_str = &include_str!("../assets/dual-stack-config.toml");
        let config: DnsConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(
            config,
            DnsConfig {
                ip_resolver: Default::default(),
                log: LogConfig {
                    level: LevelFilter::Info,
                },
                domains: vec![DomainConfig::Cloudflare(CloudflareConfig {
                    email: None,
                    key: None,
                    token: Some("dec0de".to_owned()),
                    zone: "example.com".to_owned(),
                    records: vec!["n.example.com".to_owned()],
                    ip_types: vec![IpType::V4, IpType::V6],
                })]
            }
        )
    }

    #[test]
    fn deserialize_config_godaddy() {
        let toml_str = &include_str!("../assets/godaddy-config.toml");
        let config: DomainConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(
            config,
            DomainConfig::GoDaddy(GoDaddyConfig {
                base_url: "https://api.godaddy.com".to_owned(),
                domain: "example.com".to_owned(),
                key: "abc123".to_owned(),
                secret: "ef".to_owned(),
                records: vec!["@".to_owned()]
            })
        );
    }

    #[test]
    fn deserialize_config_namecheap() {
        let toml_str = &include_str!("../assets/namecheap-config.toml");
        let config: DomainConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(
            config,
            DomainConfig::Namecheap(NamecheapConfig {
                base_url: "https://dynamicdns.park-your-domain.com".to_owned(),
                domain: "test-dness-1.xyz".to_owned(),
                ddns_password: "super_secret_password".to_owned(),
                records: vec!["@".to_owned(), "*".to_owned(), "sub".to_owned()]
            })
        );
    }

    #[test]
    fn deserialize_config_he() {
        let toml_str = &include_str!("../assets/he-config.toml");
        let config: DomainConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(
            config,
            DomainConfig::He(HeConfig {
                base_url: "https://dyn.dns.he.net".to_owned(),
                hostname: "test-dness-1.xyz".to_owned(),
                password: "super_secret_password".to_owned(),
                records: vec!["@".to_owned(), "sub".to_owned()]
            })
        );
    }

    #[test]
    fn deserialize_config_readme() {
        let env = vec![("MY_CLOUDFLARE_TOKEN".to_owned(), "dec0de".to_owned())]
            .into_iter()
            .collect();
        let config = parse_config("assets/readme-config.toml", &env).unwrap();
        assert_eq!(
            config,
            DnsConfig {
                ip_resolver: Default::default(),
                log: LogConfig {
                    level: LevelFilter::Debug,
                },
                domains: vec![
                    DomainConfig::Cloudflare(CloudflareConfig {
                        email: None,
                        key: None,
                        token: Some("dec0de".to_owned()),
                        zone: "example.com".to_owned(),
                        records: vec!["n.example.com".to_owned()],
                        ip_types: vec![IpType::V4],
                    }),
                    DomainConfig::Cloudflare(CloudflareConfig {
                        email: Some("admin@example.com".to_owned()),
                        key: Some("deadbeef".to_owned()),
                        token: None,
                        zone: "example2.com".to_owned(),
                        records: vec!["n.example2.com".to_owned(), "n2.example2.com".to_owned()],
                        ip_types: vec![IpType::V4],
                    })
                ]
            }
        );
    }

    #[test]
    fn deserialize_config_readme_bad() {
        let err = parse_config("assets/readme-config-bad.toml", &Default::default()).unwrap_err();
        let msg = format!("{:?}", err);
        assert!(msg.contains("I_DO_NOT_EXIST"));
    }

    #[test]
    fn deserialize_ipify_config() {
        let toml_str = &include_str!("../assets/ipify-config.toml");
        let config: DnsConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(
            config,
            DnsConfig {
                ip_resolver: Resolver::Ipify,
                log: LogConfig {
                    level: LevelFilter::Info,
                },
                domains: vec![]
            }
        );
    }

    #[test]
    fn deserialize_noip_config() {
        let toml_str = &include_str!("../assets/noip-config.toml");
        let config: DomainConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(
            config,
            DomainConfig::NoIp(NoIpConfig {
                base_url: "https://dynupdate.no-ip.com".to_owned(),
                username: "myemail@example.org".to_owned(),
                hostname: "dnesstest.hopto.org".to_owned(),
                password: "super_secret_password".to_owned(),
            })
        );
    }

    #[test]
    fn deserialize_config_dynu() {
        let toml_str = &include_str!("../assets/dynu-config.toml");
        let config: DomainConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(
            config,
            DomainConfig::Dynu(DynuConfig {
                base_url: "https://api.dynu.com".to_owned(),
                hostname: "test-dness-1.xyz".to_owned(),
                username: "MyUserName".to_owned(),
                password: "IpUpdatePassword".to_owned(),
                records: vec!["@".to_owned(), "sub".to_owned()]
            })
        );
    }
}
