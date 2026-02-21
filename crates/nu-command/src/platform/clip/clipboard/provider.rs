use nu_protocol::ShellError;
#[cfg(target_os = "linux")]
use nu_protocol::Value;

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
pub fn create_clipboard(config: Option<&Value>) -> impl Clipboard {
    super::linux::ClipBoardLinux::new(config)
}

#[cfg(any(target_os = "macos", target_os = "windows"))]
pub fn create_clipboard(_: Option<&nu_protocol::Value>) -> impl Clipboard {
    arboard_clipboard::ArboardClipboard::new()
}

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
pub fn create_clipboard(_: Option<&nu_protocol::Value>) -> impl Clipboard {
    super::dummy::DummyClipboard::new()
}
