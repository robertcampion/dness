use std::fmt;

macro_rules! log_err {
    ($err:expr, $fmt:expr $(, $($arg:tt)*)?) => {
        ::log::error!(
            "{}",
            $crate::errors::format_error($err, format_args!($fmt, $($($arg)*)?))
        )
    };
}
pub(crate) use log_err;

pub fn format_error(err: anyhow::Error, context: fmt::Arguments) -> String {
    use fmt::Write as _;
    let mut msg = format!("{context}");
    for cause in err.chain() {
        let _ = write!(msg, "\n\tcaused by: {cause}");
    }
    msg
}

#[derive(Debug)]
pub enum DnessErrorKind {
    SendHttp { url: String, context: String },
    BadResponse { url: String, context: String },
    Deserialize { url: String, context: String },
    Dns,
}

impl DnessErrorKind {
    pub fn send_http(url: &str, context: &str) -> Self {
        Self::SendHttp {
            url: String::from(url),
            context: String::from(context),
        }
    }

    pub fn bad_response(url: &str, context: &str) -> Self {
        Self::BadResponse {
            url: String::from(url),
            context: String::from(context),
        }
    }

    pub fn deserialize(url: &str, context: &str) -> Self {
        Self::Deserialize {
            url: String::from(url),
            context: String::from(context),
        }
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
