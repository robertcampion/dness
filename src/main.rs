mod config;
mod core;
mod dns;
mod errors;
mod providers;
mod resolvers;

// Avoid musl's default allocator due to lackluster performance
// https://nickb.dev/blog/default-musl-allocator-considered-harmful-to-performance
#[cfg(target_env = "musl")]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

use crate::config::{parse_config, DnsConfig, IpType};
use crate::core::Updates;
use clap::Parser;
use log::{error, info, LevelFilter};
use std::error;
use std::fmt::Write;
use std::net::IpAddr;
use std::path::{Path, PathBuf};
use std::time::Instant;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Opt {
    /// Sets a custom config file
    #[structopt(short, long)]
    config: Option<PathBuf>,
}

fn log_err(context: &str, err: &dyn error::Error) {
    let mut msg = String::new();
    let _ = writeln!(msg, "{context} ");
    let _ = write!(msg, "\tcaused by: {err}");

    let mut ie = err.source();
    while let Some(cause) = ie {
        let _ = write!(msg, "\n\tcaused by: {cause}");
        ie = cause.source();
    }

    error!("{}", msg);
}

fn init_logging(lvl: LevelFilter) {
    env_logger::Builder::from_default_env()
        .filter_level(lvl)
        .target(env_logger::Target::Stdout)
        .init();
}

/// Parses the TOML configuration. If no configuration file is present, the default configuration
/// is returned so that the WAN IP can still be logged on execution. If there is an error parsing
/// the configuration file, exit with a non-zero status code.
fn init_configuration<T: AsRef<Path>>(file: Option<T>) -> DnsConfig {
    if let Some(config_file) = file {
        let path = config_file.as_ref();
        match parse_config(path) {
            Ok(c) => c,
            Err(e) => {
                // If there is an error during configuration, we assume a log level of Warn so that
                // the user will see the error printed.
                init_logging(LevelFilter::Warn);
                let desc = format!("could not configure application from: {}", path.display());
                log_err(&desc, &e);
                std::process::exit(1)
            }
        }
    } else {
        Default::default()
    }
}

#[tokio::main]
async fn main() {
    let start = Instant::now();
    let opt = Opt::parse();
    let config = init_configuration(opt.config.as_ref());

    init_logging(config.log.level);

    // Use a single HTTP client when updating dns records so that connections can be reused
    let http_client = reqwest::Client::new();

    let mut ip_types: Vec<IpType> = if config.domains.is_empty() {
        vec![IpType::V4]
    } else {
        config
            .domains
            .iter()
            .flat_map(|d| d.get_ip_types())
            .collect()
    };
    ip_types.sort_unstable();
    ip_types.dedup();
    let ip_types = ip_types;

    // Keep track of any failures in ensuring current DNS records. We don't want to fail on the
    // first error, as subsequent domains listed in the config can still be valid, but if there
    // were any failures, we still need to exit with a non-zero exit code
    let mut failure = false;

    let addrs: Vec<Option<IpAddr>> =
        futures::future::join_all(ip_types.iter().map(async |ip_type| {
            let start_resolve = Instant::now();
            match resolvers::resolve_ip(&http_client, &config, *ip_type).await {
                Ok(addr) => {
                    info!(
                        "resolved address to {addr} in {}ms",
                        start_resolve.elapsed().as_millis()
                    );
                    Some(addr)
                }
                Err(e) => {
                    log_err("could not successfully resolve IP", &e);
                    None
                }
            }
        }))
        .await;
    if addrs.iter().any(Option::is_none) {
        failure = true;
    }
    let addrs: Vec<IpAddr> = addrs.iter().copied().flatten().collect();

    let mut total_updates = Updates::default();

    for d in config.domains {
        let ip_types = d.get_ip_types();
        for addr in addrs.iter() {
            if !ip_types.contains(&IpType::from(*addr)) {
                continue;
            }
            let start_update = Instant::now();
            match providers::update_provider(&http_client, *addr, &d).await {
                Ok(updates) => {
                    info!(
                        "processed {d}: ({updates}) in {}ms",
                        start_update.elapsed().as_millis()
                    );
                    total_updates += updates;
                }
                Err(e) => {
                    failure = true;
                    let msg = format!("could not update {d}");
                    log_err(&msg, e.as_ref());
                }
            }
        }
    }

    info!(
        "processed all: ({total_updates}) in {}ms",
        start.elapsed().as_millis()
    );
    if failure {
        error!("at least one update failed, so exiting with non-zero status code");
        std::process::exit(1)
    }
}
