use super::provider::Clipboard;

pub(crate) struct ClipBoardMacos;

impl ClipBoardMacos {
    pub fn new() -> Self {
        Self
    }
}

impl Clipboard for ClipBoardMacos {}
