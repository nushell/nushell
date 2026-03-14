use std::{
    collections::HashMap, io, net::{IpAddr, Ipv4Addr, SocketAddr, TcpStream}, sync::mpsc, thread::{self, JoinHandle}
};

use gatekeeper::{Address, ServerConfig, acceptor::TcpBinder, connector::{Connector, TcpUdpConnector}};

pub struct Socks5Proxy {
    addr: SocketAddr,
    th: JoinHandle<Result<(), gatekeeper::error::Error>>,
    tx: mpsc::Sender<gatekeeper::ServerCommand<TcpStream>>,
}

impl Socks5Proxy {
    pub fn builder() -> Socks5ProxyBuilder {
        let addr = reserve_local_addr()?;
        let config = ServerConfig {
            server_ip: addr.ip(),
            server_port: addr.port(),
            ..Default::default()
        };
        Socks5ProxyBuilder { config, redirects: HashMap::new() }
    }
    
    pub fn spawn() -> io::Result<Self> {
        Self::builder().spawn()
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
        Ok(Self { addr, th, tx })
    }

    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    pub fn uri(&self) -> String {
        format!("sock5://{}", self.addr)
    }
}

impl Drop for Socks5Proxy {
    fn drop(&mut self) {
        let _ = self.tx.send(gatekeeper::ServerCommand::Terminate);
        let _ = self.th.join();
    }
}

struct Socks5ProxyBuilder {
    config: ServerConfig,
    redirects: HashMap<Address, Address>,
}

impl Socks5ProxyBuilder {
    pub fn add_redirect(mut self, from: Address, to: Address) -> Self {
        self.redirects.insert(from, to);
        self
    }

    pub fn spawn(self) -> io::Result<Socks5Proxy> {
        Socks5Proxy::spawn_from_builder(self)
    }
}

struct RedirectingTcpConnector {
    redirects: HashMap<Address, Address>,
    connector: TcpUdpConnector,
}

impl RedirectingTcpConnector {
    pub fn new(redirect: HashMap<Address, Address>) -> Self {
        Self {
            redirects,
            connector: TcpUdpConnector::new(None)
        }
    }
}

impl Connector for RedirectingTcpConnector {
    type B = TcpStream;
    type P = (); // unused
    
    fn connect_byte_stream(&self, addr: Address) -> Result<(Self::B, SocketAddr), gatekeeper::model::Error> {
        match self.redirects.get(addr) {
            Some(addr) => self.connector.connect_byte_stream(addr),
            None => self.connector.connect_byte_stream(addr)
        }
    }
    
    fn connect_pkt_stream(&self, addr: Address) -> Result<(Self::P, SocketAddr), gatekeeper::model::Error> {
        unimplemented!("only supports tcp")
    }
}

fn reserve_local_addr() -> io::Result<SocketAddr> {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))?;
    let addr = listener.local_addr()?;
    drop(listener);
    Ok(addr)
}
