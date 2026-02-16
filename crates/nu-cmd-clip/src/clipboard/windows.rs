use super::clipboard::Clipboard;

pub(crate) struct ClipBoardWindows;

impl ClipBoardWindows {
    pub fn new() -> Self {
        Self
    }
}

impl Clipboard for ClipBoardWindows {}
