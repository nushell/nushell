use std::{
    io,
    net::{SocketAddr, ToSocketAddrs},
    sync::{Arc, Mutex, mpsc},
    thread::{self, JoinHandle},
    time::Duration,
};

use fast_socks5::{
    ReplyError, Socks5Command,
    server::{Socks5ServerProtocol, run_tcp_proxy},
};

use tokio::{
    net::{TcpListener, TcpStream},
    runtime::Builder,
    sync::oneshot,
    task::LocalSet,
};

/// A forwarded SOCKS5 CONNECT request handled by the proxy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForwardedRequest {
    /// The requested target address, as seen by the SOCKS5 proxy.
    ///
    /// This is usually a host:port pair.
    pub target: String,
}

/// A small sync-friendly SOCKS5 proxy handle.
///
/// This type starts a SOCKS5 server on a dedicated background thread that runs
/// a Tokio runtime internally. That lets sync code spin up a proxy without
/// needing to own or enter an async runtime.
///
/// The proxy currently supports:
///
/// - no authentication
/// - TCP CONNECT
///
/// The proxy stops accepting new connections when this handle is dropped.
pub struct Socks5Proxy {
    addr: SocketAddr,
    shutdown_tx: Option<oneshot::Sender<()>>,
    thread: Option<JoinHandle<()>>,
    forwarded_requests: Arc<Mutex<Vec<ForwardedRequest>>>,
}

impl Socks5Proxy {
    /// Starts a SOCKS5 proxy on a background thread.
    pub fn spawn(bind_addr: impl ToSocketAddrs) -> io::Result<Self> {
        let bind_addr = bind_addr
            .to_socket_addrs()?
            .next()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "no bind address"))?;

        let (ready_tx, ready_rx) = mpsc::sync_channel::<io::Result<SocketAddr>>(1);
        let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();

        let forwarded_requests = Arc::new(Mutex::new(Vec::new()));
        let thread_forwarded_requests = Arc::clone(&forwarded_requests);

        let thread = thread::spawn(move || {
            let runtime = match Builder::new_current_thread().enable_all().build() {
                Ok(rt) => rt,
                Err(err) => {
                    let _ = ready_tx.send(Err(io::Error::other(format!(
                        "failed to build tokio runtime: {err}"
                    ))));
                    return;
                }
            };

            let local = LocalSet::new();

            local.block_on(&runtime, async move {
                let listener = match TcpListener::bind(bind_addr).await {
                    Ok(listener) => listener,
                    Err(err) => {
                        let _ = ready_tx.send(Err(err));
                        return;
                    }
                };

                let local_addr = match listener.local_addr() {
                    Ok(addr) => addr,
                    Err(err) => {
                        let _ = ready_tx.send(Err(err));
                        return;
                    }
                };

                let _ = ready_tx.send(Ok(local_addr));
                run_accept_loop(listener, shutdown_rx, thread_forwarded_requests).await;
            });
        });

        let addr = ready_rx
            .recv()
            .map_err(|_| io::Error::other("proxy thread exited before init"))??;

        Ok(Self {
            addr,
            shutdown_tx: Some(shutdown_tx),
            thread: Some(thread),
            forwarded_requests,
        })
    }

    /// Returns the local socket address the proxy is listening on.
    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    /// Returns all forwarded SOCKS5 CONNECT requests observed by the proxy so far.
    ///
    /// Each entry represents one forwarded destination request, not a parsed HTTP
    /// request. For example, HTTPS traffic will still only appear as a target
    /// address like `example.com:443`.
    pub fn forwarded_requests(&self) -> Vec<ForwardedRequest> {
        self.forwarded_requests
            .lock()
            .expect("forwarded_requests mutex poisoned")
            .clone()
    }

    /// Returns the number of forwarded SOCKS5 CONNECT requests observed so far.
    pub fn forwarded_request_count(&self) -> usize {
        self.forwarded_requests
            .lock()
            .expect("forwarded_requests mutex poisoned")
            .len()
    }
}

impl Drop for Socks5Proxy {
    fn drop(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }

        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

async fn run_accept_loop(
    listener: TcpListener,
    mut shutdown_rx: oneshot::Receiver<()>,
    forwarded_requests: Arc<Mutex<Vec<ForwardedRequest>>>,
) {
    loop {
        tokio::select! {
            _ = &mut shutdown_rx => break,
            accepted = listener.accept() => {
                let (stream, _) = match accepted {
                    Ok(pair) => pair,
                    Err(err) => {
                        eprintln!("accept error: {err}");
                        continue;
                    }
                };

                let forwarded_requests = Arc::clone(&forwarded_requests);

                tokio::task::spawn_local(async move {
                    if let Err(err) = handle_client(stream, forwarded_requests).await {
                        eprintln!("client error: {err}");
                    }
                });
            }
        }
    }
}

async fn handle_client(
    stream: TcpStream,
    forwarded_requests: Arc<Mutex<Vec<ForwardedRequest>>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let proto = Socks5ServerProtocol::accept_no_auth(stream).await?;
    let (proto, cmd, target_addr) = proto.read_command().await?;

    match cmd {
        Socks5Command::TCPConnect => {
            forwarded_requests
                .lock()
                .expect("forwarded_requests mutex poisoned")
                .push(ForwardedRequest {
                    target: target_addr.to_string(),
                });

            let _ = run_tcp_proxy(proto, &target_addr, Duration::from_secs(30), true).await?;
        }
        Socks5Command::TCPBind | Socks5Command::UDPAssociate => {
            proto.reply_error(&ReplyError::CommandNotSupported).await?;
        }
    }

    Ok(())
}
