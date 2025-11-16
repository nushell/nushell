use std::{borrow::Cow, error, fmt, io};

use http::Uri;
use ureq::{
    config::Config,
    unversioned::{
        resolver::{ResolvedSocketAddrs, Resolver},
        transport::NextTimeout,
    },
};

#[derive(Debug)]
pub struct DnsLookupResolver;

impl Resolver for DnsLookupResolver {
    fn resolve(
        &self,
        uri: &Uri,
        _config: &Config,
        _timeout: NextTimeout,
    ) -> Result<ResolvedSocketAddrs, ureq::Error> {
        let host = uri.host();
        let service = uri
            .port()
            .map(|port| Cow::Owned(port.to_string()))
            .or_else(|| uri.scheme_str().map(Cow::Borrowed));
        let service = service.as_ref().map(|s| s.as_ref());
        let addr_info_iter = dns_lookup::getaddrinfo(host, service, None)
            .map_err(|err| ureq::Error::Other(Box::new(LookupError(err))))?;

        let mut resolved = self.empty();
        for addr_info in addr_info_iter {
            let addr_info = addr_info?;
            resolved.push(addr_info.sockaddr);
        }

        Ok(resolved)
    }
}

#[derive(Debug)]
pub struct LookupError(pub dns_lookup::LookupError);

impl Clone for LookupError {
    fn clone(&self) -> Self {
        Self(dns_lookup::LookupError::new(self.0.error_num()))
    }
}

impl fmt::Display for LookupError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let lookup_error = self.clone();
        let io_error = io::Error::from(lookup_error.0);
        fmt::Display::fmt(&io_error, f)
    }
}

impl error::Error for LookupError {}
