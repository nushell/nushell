use std::{
    collections::HashMap,
    hash::Hash,
    io, mem,
    net::{SocketAddr, TcpStream},
    sync::{Arc, Mutex, mpsc},
    thread::{self, JoinHandle},
};

use gatekeeper::{
    Address, ServerConfig,
    connector::{Connector, TcpUdpConnector},
};

pub struct Socks5Proxy {
    addr: SocketAddr,
    th: Option<JoinHandle<Result<(), gatekeeper::error::Error>>>,
    tx: mpsc::Sender<gatekeeper::ServerCommand<TcpStream>>,
}

impl Socks5Proxy {
    pub fn builder() -> io::Result<Socks5ProxyBuilder> {
        let addr = nu_utils::net::reserve_local_addr()?;
        let config = ServerConfig {
            server_ip: addr.ip(),
            server_port: addr.port(),
            ..Default::default()
        };
        Ok(Socks5ProxyBuilder {
            config,
            redirects: HashMap::new(),
        })
    }

    pub fn spawn() -> io::Result<Self> {
        Self::builder()?.spawn()
    }

    fn spawn_from_builder(builder: Socks5ProxyBuilder) -> io::Result<Self> {
        let Socks5ProxyBuilder { config, redirects } = builder;

        let (tx_done, rx_done) = mpsc::sync_channel(1);
        let (mut server, tx) = gatekeeper::Server::with_binder(
            config.clone(),
            gatekeeper::acceptor::TcpBinder::new(
                config.client_rw_timeout,
                Arc::new(Mutex::new(rx_done)),
                config.accept_timeout,
            ),
            tx_done,
            RedirectingTcpConnector::new(redirects),
        );

        let th = thread::spawn(move || server.serve());

        let addr = SocketAddr::new(config.server_ip, config.server_port);
        Ok(Self {
            addr,
            th: Some(th),
            tx,
        })
    }

    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    pub fn uri(&self) -> String {
        format!("socks5://{}", self.addr)
    }
}

impl Drop for Socks5Proxy {
    fn drop(&mut self) {
        let _ = self.tx.send(gatekeeper::ServerCommand::Terminate);
        let _ = self.th.take().map(|th| th.join());
    }
}

#[derive(Debug, Clone)]
pub struct Socks5ProxyBuilder {
    config: ServerConfig,
    redirects: HashMap<HashableAddress, Address>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct HashableAddress(Address);

impl Hash for HashableAddress {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match &self.0 {
            addr @ Address::IpAddr(ip_addr, port) => {
                mem::discriminant(addr).hash(state);
                ip_addr.hash(state);
                port.hash(state);
            }
            addr @ Address::Domain(domain, port) => {
                mem::discriminant(addr).hash(state);
                domain.hash(state);
                port.hash(state);
            }
        }
    }
}

impl Socks5ProxyBuilder {
    pub fn add_redirect(mut self, from: Address, to: Address) -> Self {
        self.redirects.insert(HashableAddress(from), to);
        self
    }

    pub fn spawn(self) -> io::Result<Socks5Proxy> {
        Socks5Proxy::spawn_from_builder(self)
    }
}

#[derive(Debug, Clone)]
struct RedirectingTcpConnector {
    redirects: HashMap<HashableAddress, Address>,
    connector: TcpUdpConnector,
}

impl RedirectingTcpConnector {
    pub fn new(redirects: HashMap<HashableAddress, Address>) -> Self {
        Self {
            redirects,
            connector: TcpUdpConnector::new(None),
        }
    }
}

impl Connector for RedirectingTcpConnector {
    type B = <TcpUdpConnector as Connector>::B;
    type P = <TcpUdpConnector as Connector>::P;

    fn connect_byte_stream(
        &self,
        addr: Address,
    ) -> Result<(Self::B, SocketAddr), gatekeeper::model::Error> {
        let addr = HashableAddress(addr);
        match self.redirects.get(&addr) {
            Some(addr) => self.connector.connect_byte_stream(addr.clone()),
            None => self.connector.connect_byte_stream(addr.0),
        }
    }

    fn connect_pkt_stream(
        &self,
        _addr: Address,
    ) -> Result<(Self::P, SocketAddr), gatekeeper::model::Error> {
        unimplemented!("only supports tcp")
    }
}
