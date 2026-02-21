use super::{arboard_provider::with_clipboard_instance, provider::Clipboard};
use nu_protocol::{
    Config, ShellError, Value,
    engine::{EngineState, Stack},
};
use std::sync::{OnceLock, mpsc};
use std::thread;

pub(crate) struct ClipBoardLinux {
    use_resident_thread: bool,
}

enum ResidentThreadMessage {
    SetText(String, mpsc::Sender<Result<(), String>>),
}

struct ClipboardResidentThread {
    tx: mpsc::Sender<ResidentThreadMessage>,
}

impl ClipboardResidentThread {
    fn global() -> &'static ClipboardResidentThread {
        static CLIPBOARD_THREAD: OnceLock<ClipboardResidentThread> = OnceLock::new();
        CLIPBOARD_THREAD.get_or_init(Self::start)
    }

    fn start() -> ClipboardResidentThread {
        let (tx, rx) = mpsc::channel::<ResidentThreadMessage>();

        thread::Builder::new()
            .name("nu-clipboard-holder".into())
            .spawn(move || {
                let clipboard = arboard::Clipboard::new();
                let mut clipboard = match clipboard {
                    Ok(clipboard) => clipboard,
                    Err(err) => {
                        while let Ok(ResidentThreadMessage::SetText(_, ack_tx)) = rx.recv() {
                            let _ = ack_tx.send(Err(err.to_string()));
                        }
                        return;
                    }
                };

                while let Ok(ResidentThreadMessage::SetText(text, ack_tx)) = rx.recv() {
                    let result = clipboard
                        .set_text(text)
                        .map_err(|err| err.to_string())
                        .map(|_| ());
                    let _ = ack_tx.send(result);
                }
            })
            .expect("clipboard background thread failed to start");

        ClipboardResidentThread { tx }
    }

    fn copy_text(&self, text: &str) -> Result<(), ShellError> {
        let (ack_tx, ack_rx) = mpsc::channel();
        self.tx
            .send(ResidentThreadMessage::SetText(text.to_owned(), ack_tx))
            .map_err(|err| ShellError::GenericError {
                error: "Clipboard thread channel failed.".into(),
                msg: err.to_string(),
                span: None,
                help: None,
                inner: vec![],
            })?;

        let result = ack_rx.recv().map_err(|err| ShellError::GenericError {
            error: "Clipboard thread failed.".into(),
            msg: err.to_string(),
            span: None,
            help: None,
            inner: vec![],
        })?;

        result.map_err(|err| ShellError::GenericError {
            error: "Clipboard thread failed.".into(),
            msg: err,
            span: None,
            help: None,
            inner: vec![],
        })?;

        Ok(())
    }
}

impl ClipBoardLinux {
    pub fn new(config: &Config, engine_state: &EngineState, stack: &mut Stack) -> Self {
        Self {
            use_resident_thread: should_use_resident_thread(config, engine_state, stack),
        }
    }
}

impl Clipboard for ClipBoardLinux {
    fn copy_text(&self, text: &str) -> Result<(), ShellError> {
        if self.use_resident_thread {
            ClipboardResidentThread::global().copy_text(text)
        } else {
            with_clipboard_instance(|clip: &mut arboard::Clipboard| clip.set_text(text))
        }
    }

    fn get_text(&self) -> Result<String, ShellError> {
        with_clipboard_instance(|clip| clip.get_text())
    }
}

fn should_use_resident_thread(config: &Config, engine_state: &EngineState, stack: &mut Stack) -> bool {
    // new config
    if config.clip.resident_mode {
        return true;
    }

    // legacy config
    // Backward-compatible override from old plugin config style:
    // `$env.config.plugins.clip.NO_RESIDENT = true`
    if let Some(no_resident) = read_no_resident_legacy(
        crate::platform::clip::get_config::get_clip_config_with_plugin_fallback(
            engine_state,
            stack,
        )
        .as_ref(),
    ) {
        return !no_resident;
    }

    true
}

fn read_no_resident_legacy(value: Option<&Value>) -> Option<bool> {
    match value {
        None => None,
        Some(Value::Record { val, .. }) => {
            if let Some(value) = val
                .get("NO_RESIDENT")
                .or_else(|| val.get("no_resident"))
                .or_else(|| val.get("noResident"))
            {
                read_no_resident_legacy(Some(value))
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
