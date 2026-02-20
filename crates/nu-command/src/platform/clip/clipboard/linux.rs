use super::{arboard_provider::with_clipboard_instance, provider::Clipboard};
use nu_protocol::{ShellError, Value};
use std::sync::{OnceLock, mpsc};
use std::thread;

pub(crate) struct ClipBoardLinux {
    use_daemon: bool,
}

enum DaemonMessage {
    SetText(String, mpsc::Sender<Result<(), String>>),
}

struct ClipboardDaemon {
    tx: mpsc::Sender<DaemonMessage>,
}

impl ClipboardDaemon {
    fn global() -> &'static ClipboardDaemon {
        static DAEMON: OnceLock<ClipboardDaemon> = OnceLock::new();
        DAEMON.get_or_init(Self::start)
    }

    fn start() -> ClipboardDaemon {
        let (tx, rx) = mpsc::channel::<DaemonMessage>();

        thread::Builder::new()
            .name("nu-clipboard-holder".into())
            .spawn(move || {
                let clipboard = arboard::Clipboard::new();
                let mut clipboard = match clipboard {
                    Ok(clipboard) => clipboard,
                    Err(err) => {
                        while let Ok(DaemonMessage::SetText(_, ack_tx)) = rx.recv() {
                            let _ = ack_tx.send(Err(err.to_string()));
                        }
                        return;
                    }
                };

                while let Ok(DaemonMessage::SetText(text, ack_tx)) = rx.recv() {
                    let result = clipboard
                        .set_text(text)
                        .map_err(|err| err.to_string())
                        .map(|_| ());
                    let _ = ack_tx.send(result);
                }
            })
            .expect("clipboard background thread failed to start");

        ClipboardDaemon { tx }
    }

    fn copy_text(&self, text: &str) -> Result<(), ShellError> {
        let (ack_tx, ack_rx) = mpsc::channel();
        self.tx
            .send(DaemonMessage::SetText(text.to_owned(), ack_tx))
            .map_err(|err| ShellError::GenericError {
                error: "Clipboard daemon channel failed.".into(),
                msg: err.to_string(),
                span: None,
                help: None,
                inner: vec![],
            })?;

        let result = ack_rx.recv().map_err(|err| ShellError::GenericError {
            error: "Clipboard daemon failed.".into(),
            msg: err.to_string(),
            span: None,
            help: None,
            inner: vec![],
        })?;

        result.map_err(|err| ShellError::GenericError {
            error: "Clipboard daemon failed.".into(),
            msg: err,
            span: None,
            help: None,
            inner: vec![],
        })?;

        Ok(())
    }
}

impl ClipBoardLinux {
    pub fn new(config: Option<&Value>) -> Self {
        Self {
            use_daemon: should_use_daemon(config),
        }
    }
}

impl Clipboard for ClipBoardLinux {
    fn copy_text(&self, text: &str) -> Result<(), ShellError> {
        if self.use_daemon {
            ClipboardDaemon::global().copy_text(text)
        } else {
            with_clipboard_instance(|clip: &mut arboard::Clipboard| clip.set_text(text))
        }
    }
}

fn should_use_daemon(config: Option<&Value>) -> bool {
    // Backward-compatible override from old plugin config style:
    // `$env.config.plugins.clip.NO_DAEMON = true`
    if let Some(no_daemon) = read_no_daemon(config) {
        return !no_daemon;
    }

    true
}

fn read_no_daemon(value: Option<&Value>) -> Option<bool> {
    match value {
        None => None,
        Some(Value::Record { val, .. }) => {
            if let Some(value) = val
                .get("NO_DAEMON")
                .or_else(|| val.get("no_daemon"))
                .or_else(|| val.get("noDaemon"))
            {
                read_no_daemon(Some(value))
            } else {
                None
            }
        }
        Some(Value::Bool { val, .. }) => Some(*val),
        Some(Value::String { val, .. }) => match val.as_str() {
            "true" | "True" | "1" => Some(true),
            "false" | "False" | "0" => Some(false),
            _ => None,
        },
        Some(Value::Int { val, .. }) => Some(*val == 1),
        _ => None,
    }
}
