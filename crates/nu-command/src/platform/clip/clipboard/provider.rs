use nu_protocol::ShellError;
#[cfg(target_os = "linux")]
use nu_protocol::Value;

#[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
use super::arboard_provider::with_clipboard_instance;

#[cfg(target_os = "linux")]
pub fn create_clipboard(config: Option<&Value>) -> impl Clipboard {
    super::linux::ClipBoardLinux::new(config)
}

#[cfg(not(target_os = "linux"))]
pub fn create_clipboard(_: Option<&nu_protocol::Value>) -> impl Clipboard {
    #[cfg(target_os = "macos")]
    {
        super::mac_os::ClipBoardMacos::new()
    }
    #[cfg(target_os = "windows")]
    {
        super::windows::ClipBoardWindows::new()
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        super::dummy::DummyClipboard::new()
    }
}

pub trait Clipboard {
    #[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
    fn copy_text(&self, text: &str) -> Result<(), ShellError> {
        with_clipboard_instance(|clip| clip.set_text(text))
    }

    #[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
    fn get_text(&self) -> Result<String, ShellError> {
        with_clipboard_instance(|clip| clip.get_text())
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    fn copy_text(&self, text: &str) -> Result<(), ShellError>;

    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    fn get_text(&self) -> Result<String, ShellError>;
}
