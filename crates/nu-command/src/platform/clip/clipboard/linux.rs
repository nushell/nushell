use super::{arboard_provider::with_clipboard_instance, provider::Clipboard};
use nu_protocol::{
    Config, ShellError, Value,
    engine::{EngineState, Stack},
    shell_error::generic::GenericError,
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
    /// Returns the shared holder thread, starting it on first use.
    fn global() -> Result<&'static ClipboardResidentThread, ShellError> {
        static CLIPBOARD_THREAD: OnceLock<ClipboardResidentThread> = OnceLock::new();
        if let Some(thread) = CLIPBOARD_THREAD.get() {
            return Ok(thread);
        }
        // `OnceLock::get_or_try_init` is unstable, so start the thread before storing it. If
        // another caller stores theirs first, ours is dropped, its channel closes and it exits.
        let started = Self::start()?;
        Ok(CLIPBOARD_THREAD.get_or_init(|| started))
    }

    fn start() -> Result<ClipboardResidentThread, ShellError> {
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
            .map_err(|err| {
                ShellError::Generic(GenericError::new_internal(
                    "Clipboard thread failed to start.",
                    err.to_string(),
                ))
            })?;

        Ok(ClipboardResidentThread { tx })
    }

    fn copy_text(&self, text: &str) -> Result<(), ShellError> {
        let (ack_tx, ack_rx) = mpsc::channel();
        self.tx
            .send(ResidentThreadMessage::SetText(text.to_owned(), ack_tx))
            .map_err(|err| {
                ShellError::Generic(GenericError::new_internal(
                    "Clipboard thread channel failed.",
                    err.to_string(),
                ))
            })?;

        let result = ack_rx.recv().map_err(|err| {
            ShellError::Generic(GenericError::new_internal(
                "Clipboard thread failed.",
                err.to_string(),
            ))
        })?;

        result.map_err(|err| {
            ShellError::Generic(GenericError::new_internal("Clipboard thread failed.", err))
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
            ClipboardResidentThread::global()?.copy_text(text)
        } else {
            with_clipboard_instance(|clip: &mut arboard::Clipboard| clip.set_text(text))
        }
    }

    fn get_text(&self) -> Result<String, ShellError> {
        with_clipboard_instance(|clip| clip.get_text())
    }
}

fn should_use_resident_thread(
    config: &Config,
    engine_state: &EngineState,
    stack: &mut Stack,
) -> bool {
    // The old plugin config style, `$env.config.plugins.clip.NO_RESIDENT`, overrides
    // `$env.config.clip.resident_mode` when it is set.
    read_no_resident_legacy(
        crate::platform::clip::get_config::get_clip_config_with_plugin_fallback(
            engine_state,
            stack,
        )
        .as_ref(),
    )
    .map_or(config.clip.resident_mode, |no_resident| !no_resident)
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

#[cfg(test)]
mod tests {
    use super::*;
    use nu_protocol::record;

    /// Runs `should_use_resident_thread` with `clip.resident_mode` and an optional
    /// `plugins.clip` value.
    fn uses_resident_thread(resident_mode: bool, plugins_clip: Option<Value>) -> bool {
        let mut config = Config::default();
        config.clip.resident_mode = resident_mode;
        if let Some(value) = plugins_clip {
            config.plugins.insert("clip".into(), value);
        }
        let mut engine_state = EngineState::new();
        engine_state.set_config(config);
        let config = engine_state.get_config().clone();
        should_use_resident_thread(&config, &engine_state, &mut Stack::new())
    }

    fn no_resident(val: bool) -> Value {
        Value::test_record(record! { "NO_RESIDENT" => Value::test_bool(val) })
    }

    #[test]
    fn resident_mode_decides_without_legacy_key() {
        assert!(uses_resident_thread(true, None));
        assert!(!uses_resident_thread(false, None));
    }

    #[test]
    fn legacy_no_resident_overrides_resident_mode() {
        assert!(!uses_resident_thread(true, Some(no_resident(true))));
        assert!(uses_resident_thread(false, Some(no_resident(false))));
    }
}
