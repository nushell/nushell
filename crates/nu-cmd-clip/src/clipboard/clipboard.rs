use nu_protocol::ShellError;

use super::arboard_provider::with_clipboard_instance;

#[cfg(target_os = "linux")]
pub fn create_clipboard() -> impl Clipboard {
    crate::clipboard::linux::ClipBoardLinux::new()
}

#[cfg(not(target_os = "linux"))]
pub fn create_clipboard() -> impl Clipboard {
    #[cfg(target_os = "macos")]
    {
        crate::clipboard::mac_os::ClipBoardMacos::new()
    }
    #[cfg(target_os = "windows")]
    {
        crate::clipboard::windows::ClipBoardWindows::new()
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
