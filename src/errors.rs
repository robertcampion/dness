use hickory_resolver::ResolveError;
use std::error;
use std::fmt;

#[derive(Debug)]
pub enum DnessErrorKind {
    SendHttp { url: String, context: String },
    BadResponse { url: String, context: String },
    Deserialize { url: String, context: String },
    Message(String),
    Dns,
}

#[derive(Debug)]
pub struct DnessError {
    kind: DnessErrorKind,
    source: Option<Box<dyn std::error::Error>>,
}

impl DnessError {
    pub fn send_http(url: &str, context: &str, source: reqwest::Error) -> DnessError {
        DnessError {
            kind: DnessErrorKind::SendHttp {
                url: String::from(url),
                context: String::from(context),
            },
            source: Some(Box::new(source)),
        }
    }

    pub fn bad_response(url: &str, context: &str, source: reqwest::Error) -> DnessError {
        DnessError {
            kind: DnessErrorKind::BadResponse {
                url: String::from(url),
                context: String::from(context),
            },
            source: Some(Box::new(source)),
        }
    }

    pub fn deserialize(url: &str, context: &str, source: reqwest::Error) -> DnessError {
        DnessError {
            kind: DnessErrorKind::Deserialize {
                url: String::from(url),
                context: String::from(context),
            },
            source: Some(Box::new(source)),
        }
    }

    pub fn message(msg: String) -> DnessError {
        DnessError {
            kind: DnessErrorKind::Message(msg),
            source: None,
        }
    }

    pub fn dns(source: DnsError) -> Self {
        DnessError {
            kind: DnessErrorKind::Dns,
            source: Some(Box::new(source)),
        }
    }
}

impl error::Error for DnessError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        self.source.as_deref()
    }
}

impl fmt::Display for DnessError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.kind.fmt(f)
    }
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
            DnessErrorKind::Message(msg) => write!(f, "{msg}"),
        }
    }
}

#[derive(Debug)]
pub struct DnsError {
    pub kind: DnsErrorKind,
}

#[derive(Debug)]
pub enum DnsErrorKind {
    DnsResolve(Box<ResolveError>),
    UnexpectedResponse(usize),
}

impl error::Error for DnsError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self.kind {
            DnsErrorKind::DnsResolve(ref e) => Some(e.as_ref()),
            DnsErrorKind::UnexpectedResponse(_) => None,
        }
    }
}

impl fmt::Display for DnsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.kind {
            DnsErrorKind::DnsResolve(_) => write!(f, "could not resolve via dns"),
            DnsErrorKind::UnexpectedResponse(results) => {
                write!(f, "unexpected number of results: {results}")
            }
        }
    }
}
