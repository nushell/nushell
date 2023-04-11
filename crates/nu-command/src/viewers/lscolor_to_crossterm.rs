// TODO: this can be removed if `lscolor` crate update it's version to fit with crossterm version
// 0.26.1

use lscolors::{Color, FontStyle, Style};

pub fn to_crossterm_style(style: &Style) -> crossterm::style::ContentStyle {
    crossterm::style::ContentStyle {
        foreground_color: style.foreground.as_ref().map(to_crossterm_color),
        background_color: style.background.as_ref().map(to_crossterm_color),
        attributes: to_crossterm_attributes(&style.font_style),
        underline_color: style.underline.as_ref().map(to_crossterm_color),
    }
}

fn to_crossterm_color(color: &Color) -> crossterm::style::Color {
    match color {
        Color::RGB(r, g, b) => crossterm::style::Color::Rgb {
            r: *r,
            g: *g,
            b: *b,
        },
        Color::Fixed(n) => crossterm::style::Color::AnsiValue(*n),
        Color::Black => crossterm::style::Color::Black,
        Color::Red => crossterm::style::Color::DarkRed,
        Color::Green => crossterm::style::Color::DarkGreen,
        Color::Yellow => crossterm::style::Color::DarkYellow,
        Color::Blue => crossterm::style::Color::DarkBlue,
        Color::Magenta => crossterm::style::Color::DarkMagenta,
        Color::Cyan => crossterm::style::Color::DarkCyan,
        Color::White => crossterm::style::Color::Grey,
        Color::BrightBlack => crossterm::style::Color::DarkGrey,
        Color::BrightRed => crossterm::style::Color::Red,
        Color::BrightGreen => crossterm::style::Color::Green,
        Color::BrightYellow => crossterm::style::Color::Yellow,
        Color::BrightBlue => crossterm::style::Color::Blue,
        Color::BrightMagenta => crossterm::style::Color::Magenta,
        Color::BrightCyan => crossterm::style::Color::Cyan,
        Color::BrightWhite => crossterm::style::Color::White,
    }
}

fn to_crossterm_attributes(font_style: &FontStyle) -> crossterm::style::Attributes {
    let mut attributes = crossterm::style::Attributes::default();
    if font_style.bold {
        attributes.set(crossterm::style::Attribute::Bold);
    }
    if font_style.dimmed {
        attributes.set(crossterm::style::Attribute::Dim);
    }
    if font_style.italic {
        attributes.set(crossterm::style::Attribute::Italic);
    }
    if font_style.underline {
        attributes.set(crossterm::style::Attribute::Underlined);
    }
    if font_style.slow_blink {
        attributes.set(crossterm::style::Attribute::SlowBlink);
    }
    if font_style.rapid_blink {
        attributes.set(crossterm::style::Attribute::RapidBlink);
    }
    if font_style.reverse {
        attributes.set(crossterm::style::Attribute::Reverse);
    }
    if font_style.hidden {
        attributes.set(crossterm::style::Attribute::Hidden);
    }
    if font_style.strikethrough {
        attributes.set(crossterm::style::Attribute::CrossedOut);
    }
    attributes
}
