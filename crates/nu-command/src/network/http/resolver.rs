use std::{
    error, fmt, io,
    net::{SocketAddr, SocketAddrV4, SocketAddrV6},
};

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
        // Determine the port: use explicit port if provided, otherwise derive from scheme
        let port = uri
            .port_u16()
            .or_else(|| match uri.scheme_str() {
                Some("https") => Some(443),
                Some("http") => Some(80),
                _ => None,
            })
            .unwrap_or(80);

        // Pass None as service to avoid "Service not supported for this socket type" errors
        // in certain environments (e.g., Docker containers on some Linux distributions).
        // We'll set the port manually on each resolved address.
        let addr_info_iter = dns_lookup::getaddrinfo(host, None, None)
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
                // Set the correct port on the resolved address
                let sockaddr_with_port = set_port(sockaddr, port);
                resolved.push(sockaddr_with_port);
                // ArrayVec has a fixed capacity, stop when full
                if resolved.len() >= capacity {
                    break;
                }
            }
        }

        Ok(resolved)
    }
}

/// Set the port on a SocketAddr
fn set_port(addr: SocketAddr, port: u16) -> SocketAddr {
    match addr {
        SocketAddr::V4(v4) => SocketAddr::V4(SocketAddrV4::new(*v4.ip(), port)),
        SocketAddr::V6(v6) => {
            SocketAddr::V6(SocketAddrV6::new(*v6.ip(), port, v6.flowinfo(), v6.scope_id()))
        }
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
