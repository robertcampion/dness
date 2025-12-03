use std::{error, fmt};

use anyhow::{anyhow, Error};
use hickory_resolver::error::ResolveError;

#[derive(Debug)]
pub struct DnessError;

impl DnessError {
    pub fn send_http(url: &str, context: &str, source: reqwest::Error) -> Error {
        anyhow!(source).context(format!(
            "unable to send http request for {context}: url attempted: {url}",
        ))
    }

    pub fn bad_response(url: &str, context: &str, source: reqwest::Error) -> Error {
        anyhow!(source).context(format!(
            "received bad http response for {context}: url attempted: {url}",
        ))
    }

    pub fn deserialize(url: &str, context: &str, source: reqwest::Error) -> Error {
        anyhow!(source).context(format!(
            "unable to deserialize response for {context}: url attempted: {url}",
        ))
    }

    pub fn message(msg: String) -> Error {
        anyhow!(msg)
    }
}

#[derive(Debug)]
pub struct DnsError {
    pub kind: Box<DnsErrorKind>,
}

#[derive(Debug)]
pub enum DnsErrorKind {
    DnsResolve(ResolveError),
    UnexpectedResponse(usize),
}

impl error::Error for DnsError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match *self.kind {
            DnsErrorKind::DnsResolve(ref e) => Some(e),
            DnsErrorKind::UnexpectedResponse(_) => None,
        }
    }
}

impl fmt::Display for DnsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &*self.kind {
            DnsErrorKind::DnsResolve(_) => write!(f, "could not resolve via dns"),
            DnsErrorKind::UnexpectedResponse(results) => {
                write!(f, "unexpected number of results: {}", results)
            }
        }
    }
}
