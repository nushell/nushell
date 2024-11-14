#[cfg(not(target_arch = "wasm32"))]
use std::collections::HashMap;
use std::{
    collections::VecDeque,
    io::Read,
    sync::{
        mpsc::{self, Receiver, TryRecvError},
        Arc, Mutex,
    },
    thread,
    time::{Duration, Instant},
};
use tungstenite::ClientRequestBuilder;
use url::Url;

pub struct ChannelReader {
    rx: Arc<Mutex<Receiver<Vec<u8>>>>,
    deadline: Option<Instant>,
    buf_deque: VecDeque<u8>,
}

impl ChannelReader {
    pub fn new(rx: Receiver<Vec<u8>>, timeout: Option<Duration>) -> Self {
        let mut cr = Self {
            rx: Arc::new(Mutex::new(rx)),
            deadline: None,
            buf_deque: VecDeque::new(),
        };
        if let Some(timeout) = timeout {
            cr.deadline = Some(Instant::now() + timeout);
        }
        cr
    }
}

impl Read for ChannelReader {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let rx = self.rx.lock().expect("Could not get lock on receiver");

        let bytes = match self.deadline {
            Some(deadline) => rx
                .recv_timeout(deadline.duration_since(Instant::now()))
                .map_err(|_| TryRecvError::Disconnected),
            None => rx.recv().map_err(|_| TryRecvError::Disconnected),
        };

        let bytes = match bytes {
            Ok(bytes) => bytes,
            Err(..) => return Ok(0),
        };

        for b in bytes {
            self.buf_deque.push_back(b);
        }

        let mut len = 0;
        for buf in buf {
            if let Some(b) = self.buf_deque.pop_front() {
                *buf = b;
                len += 1;
            } else {
                break;
            }
        }
        Ok(len)
    }
}

pub fn upgrade(
    url: Url,
    timeout: Option<Duration>,
    headers: HashMap<String, String>,
) -> Option<ChannelReader> {
    let mut builder = ClientRequestBuilder::new(url.as_str().parse().ok()?);
    builder = builder.with_header(
        "Origin",
        format!(
            "{}://{}:{}",
            url.scheme(),
            url.host_str().unwrap_or_default(),
            url.port().unwrap_or_default()
        ),
    );
    for (k, v) in headers {
        builder = builder.with_header(k, v);
    }
    match tungstenite::connect(builder) {
        Ok((mut websocket, _)) => {
            let (tx, rx) = mpsc::sync_channel(1024);
            let tx = Arc::new(tx);
            thread::Builder::new()
                .name("websocket response sender".to_string())
                .spawn(move || loop {
                    let tx = tx.clone();
                    match websocket.read() {
                        Ok(msg) => match msg {
                            tungstenite::Message::Text(msg) => {
                                if tx.send(msg.as_bytes().to_vec()).is_err() {
                                    websocket.close(Some(tungstenite::protocol::CloseFrame{
                                    code: tungstenite::protocol::frame::coding::CloseCode::Normal,
                                    reason: std::borrow::Cow::Borrowed("byte stream closed"),
                                })).expect("Could not close connection")
                                }
                            }
                            tungstenite::Message::Binary(msg) => {
                                if tx.send(msg).is_err() {
                                    websocket.close(Some(tungstenite::protocol::CloseFrame{
                                    code: tungstenite::protocol::frame::coding::CloseCode::Normal,
                                    reason: std::borrow::Cow::Borrowed("byte stream closed"),
                                })).expect("Could not close connection")
                                }
                            }
                            tungstenite::Message::Close(..) => {
                                drop(tx);
                                return;
                            }
                            _ => continue,
                        },
                        _ => {
                            drop(tx);
                            return;
                        }
                    }
                })
                .ok()?;
            Some(ChannelReader::new(rx, timeout))
        }
        Err(..) => None,
    }
}
