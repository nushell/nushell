use miette::{Diagnostic, LabeledSpan};
use thiserror::Error;

use crate::{ShellError, Span, Spanned};

#[derive(Debug, Clone, PartialEq, Error, Diagnostic)]
pub enum NetworkError {
    // Replace ShellError::NetworkFailure with this one
    #[error("Network failure")]
    #[diagnostic(code(nu::shell::network))]
    Generic {
        msg: String,
        #[label("{msg}")]
        span: Span,
    },

    #[error(transparent)]
    #[diagnostic(transparent)]
    Dns(DnsError),
    // TODO: add more precise network errors to avoid generic ones
}

#[derive(Debug, Clone, PartialEq, Error)]
#[error("DNS Error")]
pub struct DnsError {
    pub kind: DnsErrorKind,
    pub span: Span,
    pub query: Spanned<String>,
}

impl Diagnostic for DnsError {
    fn code<'a>(&'a self) -> Option<Box<dyn std::fmt::Display + 'a>> {
        self.kind.code()
    }

    fn labels(&self) -> Option<Box<dyn Iterator<Item = LabeledSpan> + '_>> {
        Some(Box::new(
            [
                LabeledSpan::new_with_span(Some("Could not be resolved".into()), self.query.span),
                LabeledSpan::new_with_span(Some(self.kind.to_string()), self.span),
            ]
            .into_iter(),
        ))
    }

    fn help<'a>(&'a self) -> Option<Box<dyn std::fmt::Display + 'a>> {
        Some(Box::new(format!("While querying \"{}\"", self.query)))
    }
}

#[derive(Debug, Clone, PartialEq, Error, Diagnostic)]
pub enum DnsErrorKind {
    /// Temporary failure in name resolution.
    ///
    /// May also be returned when the DNS server returns SERVFAIL.
    #[error("Temporary failure in name resolution")]
    #[diagnostic(code(nu::shell::network::dns::again))]
    Again,

    /// NAME or SERVICE is unknown.
    ///
    /// May also be returned when the domain does not exist (NXDOMAIN) or
    /// exists but has no address records (NODATA).
    #[error("Name or service is unknown")]
    #[diagnostic(code(nu::shell::network::dns::no_name))]
    NoName,

    /// The specified network host exists, but has no data defined.
    ///
    /// This is no longer a POSIX standard, however it is still returned by
    /// some platforms.
    #[error("Host exists but has no address records")]
    #[diagnostic(code(nu::shell::network::dns::no_data))]
    NoData,

    /// Non recoverable failure in name resolution.
    #[error("Non recoverable failure in name resolution")]
    #[diagnostic(code(nu::shell::network::dns::fail))]
    Fail,
}

impl From<DnsError> for ShellError {
    fn from(value: DnsError) -> Self {
        ShellError::Network(NetworkError::Dns(value))
    }
}
