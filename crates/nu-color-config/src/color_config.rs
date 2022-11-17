use crate::nu_style::{color_from_hex, lookup_style};
use nu_ansi_term::Style;

pub fn lookup_ansi_color_style(s: &str) -> Style {
    if s.starts_with('#') {
        color_from_hex(s)
            .ok()
            .and_then(|c| c.map(|c| c.normal()))
            .unwrap_or_default()
    } else {
        lookup_style(s)
    }
}
