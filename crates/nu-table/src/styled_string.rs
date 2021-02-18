use crate::text_style::TextStyle;

#[derive(Debug, Clone)]
pub struct StyledString {
    pub contents: String,
    pub style: TextStyle,
}

impl StyledString {
    pub fn new(contents: String, style: TextStyle) -> StyledString {
        StyledString { contents, style }
    }

    pub fn set_style(&mut self, style: TextStyle) {
        self.style = style;
    }
}
