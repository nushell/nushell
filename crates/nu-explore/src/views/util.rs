use std::borrow::Cow;

use nu_color_config::style_primitive;
use nu_table::{string_width, Alignment, TextStyle};
use tui::{
    buffer::Buffer,
    style::{Color, Modifier, Style},
    text::Span,
};

use crate::nu_common::{truncate_str, NuColor, NuStyle, NuStyleTable, NuText};

pub fn set_span(
    buf: &mut Buffer,
    (x, y): (u16, u16),
    text: &str,
    style: Style,
    max_width: u16,
) -> u16 {
    let mut text = Cow::Borrowed(text);
    let mut text_width = string_width(&text);
    if text_width > max_width as usize {
        let mut s = text.into_owned();
        truncate_str(&mut s, max_width as usize);
        text = Cow::Owned(s);
        text_width = max_width as usize;
    }

    let span = Span::styled(text.as_ref(), style);
    buf.set_span(x, y, &span, text_width as u16);

    text_width as u16
}

pub fn nu_style_to_tui(style: NuStyle) -> tui::style::Style {
    let mut out = tui::style::Style::default();
    if let Some(clr) = style.background {
        out.bg = nu_ansi_color_to_tui_color(clr);
    }

    if let Some(clr) = style.foreground {
        out.fg = nu_ansi_color_to_tui_color(clr);
    }

    if style.is_blink {
        out.add_modifier |= Modifier::SLOW_BLINK;
    }

    if style.is_bold {
        out.add_modifier |= Modifier::BOLD;
    }

    if style.is_dimmed {
        out.add_modifier |= Modifier::DIM;
    }

    if style.is_hidden {
        out.add_modifier |= Modifier::HIDDEN;
    }

    if style.is_italic {
        out.add_modifier |= Modifier::ITALIC;
    }

    if style.is_reverse {
        out.add_modifier |= Modifier::REVERSED;
    }

    if style.is_underline {
        out.add_modifier |= Modifier::UNDERLINED;
    }

    out
}

pub fn nu_ansi_color_to_tui_color(clr: NuColor) -> Option<tui::style::Color> {
    use NuColor::*;

    let clr = match clr {
        Black => Color::Black,
        DarkGray => Color::DarkGray,
        Red => Color::Red,
        LightRed => Color::LightRed,
        Green => Color::Green,
        LightGreen => Color::LightGreen,
        Yellow => Color::Yellow,
        LightYellow => Color::LightYellow,
        Blue => Color::Blue,
        LightBlue => Color::LightBlue,
        Magenta => Color::Magenta,
        LightMagenta => Color::LightMagenta,
        Cyan => Color::Cyan,
        LightCyan => Color::LightCyan,
        White => Color::White,
        Fixed(i) => Color::Indexed(i),
        Rgb(r, g, b) => tui::style::Color::Rgb(r, g, b),
        LightGray => Color::Gray,
        LightPurple => Color::LightMagenta,
        Purple => Color::Magenta,
        Default => return None,
    };

    Some(clr)
}

pub fn text_style_to_tui_style(style: TextStyle) -> tui::style::Style {
    let mut out = tui::style::Style::default();
    if let Some(style) = style.color_style {
        if let Some(clr) = style.background {
            out.bg = nu_ansi_color_to_tui_color(clr);
        }

        if let Some(clr) = style.foreground {
            out.fg = nu_ansi_color_to_tui_color(clr);
        }
    }

    out
}

pub fn make_styled_string(
    text: String,
    text_type: &str,
    col: usize,
    with_index: bool,
    color_hm: &NuStyleTable,
    float_precision: usize,
) -> NuText {
    if col == 0 && with_index {
        return (text, index_text_style(color_hm));
    }

    let style = style_primitive(text_type, color_hm);

    let mut text = text;
    if text_type == "float" {
        text = convert_with_precision(&text, float_precision);
    }

    (text, style)
}

fn index_text_style(color_hm: &std::collections::HashMap<String, NuStyle>) -> TextStyle {
    TextStyle {
        alignment: Alignment::Right,
        color_style: Some(color_hm["row_index"]),
    }
}

fn convert_with_precision(val: &str, precision: usize) -> String {
    // vall will always be a f64 so convert it with precision formatting
    match val.trim().parse::<f64>() {
        Ok(f) => format!("{:.prec$}", f, prec = precision),
        Err(err) => format!("error converting string [{}] to f64; {}", &val, err),
    }
}
