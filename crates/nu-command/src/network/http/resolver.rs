use std::{borrow::Cow, error, fmt, io};

use http::Uri;
use ureq::{
    config::Config,
    unversioned::resolver::{ArrayVec, ResolvedSocketAddrs, Resolver},
    unversioned::transport::NextTimeout,
};

#[derive(Debug)]
pub struct DnsLookupResolver;

impl Resolver for DnsLookupResolver {
    fn resolve(
        &self,
        uri: &Uri,
        config: &Config,
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

        let ip_family = config.ip_family();
        let mut resolved = self.empty();
        let capacity = array_vec_capacity(&resolved);
        for addr_info in addr_info_iter {
            let addr_info = addr_info?;
            let sockaddr = addr_info.sockaddr;
            // Filter addresses based on configured IP family (IPv4 only, IPv6 only, or any)
            let is_wanted = match ip_family {
                ureq::config::IpFamily::Any => true,
                ureq::config::IpFamily::Ipv4Only => sockaddr.is_ipv4(),
                ureq::config::IpFamily::Ipv6Only => sockaddr.is_ipv6(),
            };
            if is_wanted {
                resolved.push(sockaddr);
                // ArrayVec has a fixed capacity, stop when full
                if resolved.len() >= capacity {
                    break;
                }
            }
        }

        Ok(resolved)
    }
}

/// Extract the capacity of an ArrayVec at compile time.
fn array_vec_capacity<T, const N: usize>(_: &ArrayVec<T, N>) -> usize {
    N
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
