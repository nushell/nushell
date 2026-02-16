use super::{arboard_provider::with_clipboard_instance, clipboard::Clipboard};
use nu_protocol::ShellError;

pub(crate) struct ClipBoardLinux;

impl ClipBoardLinux {
    pub fn new() -> Self {
        Self
    }
}

impl Clipboard for ClipBoardLinux {
    fn copy_text(&self, text: &str) -> Result<(), ShellError> {
        with_clipboard_instance(|clip: &mut arboard::Clipboard| clip.set_text(text))
    }
}
