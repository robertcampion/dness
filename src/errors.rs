use anyhow::Error;
use std::fmt;

#[derive(Debug)]
pub enum DnessErrorKind {
    SendHttp { url: String, context: String },
    BadResponse { url: String, context: String },
    Deserialize { url: String, context: String },
    Dns,
}

pub fn send_http(url: &str, context: &str, source: reqwest::Error) -> Error {
    Error::from(source).context(DnessErrorKind::SendHttp {
        url: String::from(url),
        context: String::from(context),
    })
}

pub fn bad_response(url: &str, context: &str, source: reqwest::Error) -> Error {
    Error::from(source).context(DnessErrorKind::BadResponse {
        url: String::from(url),
        context: String::from(context),
    })
}

pub fn deserialize(url: &str, context: &str, source: reqwest::Error) -> Error {
    Error::from(source).context(DnessErrorKind::Deserialize {
        url: String::from(url),
        context: String::from(context),
    })
}

pub fn message(msg: String) -> Error {
    Error::msg(msg)
}

pub fn dns(source: Error) -> Error {
    source.context(DnessErrorKind::Dns)
}

impl fmt::Display for DnessErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DnessErrorKind::SendHttp { url, context, .. } => write!(
                f,
                "unable to send http request for {context}: url attempted: {url}"
            ),
            DnessErrorKind::BadResponse { url, context, .. } => write!(
                f,
                "received bad http response for {context}: url attempted: {url}"
            ),
            DnessErrorKind::Deserialize { url, context, .. } => write!(
                f,
                "unable to deserialize response for {context}: url attempted: {url}"
            ),
            DnessErrorKind::Dns => write!(f, "dns lookup"),
        }
    }
}

#[derive(Debug)]
pub enum DnsErrorKind {
    DnsResolve,
    UnexpectedResponse(usize),
}

impl fmt::Display for DnsErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DnsErrorKind::DnsResolve => write!(f, "could not resolve via dns"),
            DnsErrorKind::UnexpectedResponse(results) => {
                write!(f, "unexpected number of results: {results}")
            }
        }
    }
}
