use nu_protocol::ShellError;
#[cfg(target_os = "linux")]
use nu_protocol::Value;

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
}

pub trait Clipboard {
    fn copy_text(&self, text: &str) -> Result<(), ShellError> {
        with_clipboard_instance(|clip| clip.set_text(text))
    }

    fn get_text(&self) -> Result<String, ShellError> {
        with_clipboard_instance(|clip| clip.get_text())
    }
}
