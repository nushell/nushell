use nu_protocol::ShellError;
use nu_protocol::{
    Config,
    engine::{EngineState, Stack},
};

// All clipboard providers must implement this trait.
pub trait Clipboard {
    fn copy_text(&self, text: &str) -> Result<(), ShellError>;
    fn get_text(&self) -> Result<String, ShellError>;
}

// A clipboard provider that uses the `arboard` crate.
// This is the default for platforms that don't have a specialized implementation.
#[cfg(any(target_os = "macos", target_os = "windows"))]
mod arboard_clipboard {
    use super::super::arboard_provider::with_clipboard_instance;
    use super::Clipboard;
    use nu_protocol::ShellError;

    pub(super) struct ArboardClipboard;

    impl ArboardClipboard {
        pub fn new() -> Self {
            Self
        }
    }

    impl Clipboard for ArboardClipboard {
        fn copy_text(&self, text: &str) -> Result<(), ShellError> {
            with_clipboard_instance(|clip| clip.set_text(text))
        }

        fn get_text(&self) -> Result<String, ShellError> {
            with_clipboard_instance(|clip| clip.get_text())
        }
    }
}

#[cfg(target_os = "linux")]
pub fn create_clipboard(
    config: &Config,
    engine_state: &EngineState,
    stack: &mut Stack,
) -> impl Clipboard {
    super::linux::ClipBoardLinux::new(config, engine_state, stack)
}

#[cfg(any(target_os = "macos", target_os = "windows"))]
pub fn create_clipboard(
    _config: &Config,
    _engine_state: &EngineState,
    _stack: &mut Stack,
) -> impl Clipboard {
    arboard_clipboard::ArboardClipboard::new()
}

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
pub fn create_clipboard(
    _config: &Config,
    _engine_state: &EngineState,
    _stack: &mut Stack,
) -> impl Clipboard {
    super::dummy::DummyClipboard::new()
}
