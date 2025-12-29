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
pub struct HttpError {
    kind: HttpErrorKind,
    url: String,
    context: String,
}

#[derive(Debug)]
pub enum HttpErrorKind {
    Send,
    BadResponse,
    Deserialize,
}

impl HttpError {
    pub fn send(url: &str, context: &str) -> Self {
        Self {
            kind: HttpErrorKind::Send,
            url: String::from(url),
            context: String::from(context),
        }
    }

    pub fn bad_response(url: &str, context: &str) -> Self {
        Self {
            kind: HttpErrorKind::BadResponse,
            url: String::from(url),
            context: String::from(context),
        }
    }

    pub fn deserialize(url: &str, context: &str) -> Self {
        Self {
            kind: HttpErrorKind::Deserialize,
            url: String::from(url),
            context: String::from(context),
        }
    }
}

impl fmt::Display for HttpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} for {}: url attempted: {}",
            match self.kind {
                HttpErrorKind::Send => "unable to send http request",
                HttpErrorKind::BadResponse => "received bad http response",
                HttpErrorKind::Deserialize => "unable to deserialize response",
            },
            self.context,
            self.url
        )
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
