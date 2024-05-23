use std::borrow::Cow;

use ansi_str::{get_blocks, AnsiStr};
use nu_table::{string_truncate, string_width};
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::Widget,
};

/// A widget that represents a single line of text with ANSI styles.
pub struct ColoredTextWidget<'a> {
    text: &'a str,
    /// Column to start rendering from
    col: usize,
}

impl<'a> ColoredTextWidget<'a> {
    pub fn new(text: &'a str, col: usize) -> Self {
        Self { text, col }
    }

    /// Return a window of the text that fits into the given width, with ANSI styles stripped.
    pub fn get_plain_text(&self, max_width: usize) -> String {
        cut_string(self.text, self.col, max_width)
            .ansi_strip()
            .into_owned()
    }
}

impl Widget for ColoredTextWidget<'_> {
    fn render(self, area: Rect, buf: &mut ratatui::buffer::Buffer) {
        let text = cut_string(self.text, self.col, area.width as usize);

        let mut offset = 0;
        for block in get_blocks(&text) {
            let text = block.text();
            let style = style_to_tui(block.style());

            let x = area.x + offset;
            let (o, _) = buf.set_stringn(x, area.y, text, area.width as usize, style);

            offset = o
        }
    }
}

fn cut_string(source: &str, skip: usize, width: usize) -> Cow<'_, str> {
    if source.is_empty() {
        return Cow::Borrowed(source);
    }

    let mut text = Cow::Borrowed(source);

    if skip > 0 {
        let skip_chars = source
            .ansi_strip()
            .chars()
            .scan((0usize, 0usize), |acc, c| {
                acc.0 += unicode_width::UnicodeWidthChar::width(c).unwrap_or(0);
                if acc.0 > skip {
                    return None;
                }

                acc.1 = c.len_utf8();

                Some(*acc)
            })
            .map(|(_, b)| b)
            .sum::<usize>();

        let cut_text = source
            .ansi_get(skip_chars..)
            .expect("must be OK")
            .into_owned();
        text = Cow::Owned(cut_text);
    }

    if string_width(&text) > width {
        text = Cow::Owned(string_truncate(&text, width));
    }

    text
}

fn style_to_tui(style: &ansi_str::Style) -> Style {
    let mut out = Style::default();
    if let Some(clr) = style.background() {
        out.bg = ansi_color_to_tui_color(clr);
    }

    if let Some(clr) = style.foreground() {
        out.fg = ansi_color_to_tui_color(clr);
    }

    if style.is_slow_blink() || style.is_rapid_blink() {
        out.add_modifier |= Modifier::SLOW_BLINK;
    }

    if style.is_bold() {
        out.add_modifier |= Modifier::BOLD;
    }

    if style.is_faint() {
        out.add_modifier |= Modifier::DIM;
    }

    if style.is_hide() {
        out.add_modifier |= Modifier::HIDDEN;
    }

    if style.is_italic() {
        out.add_modifier |= Modifier::ITALIC;
    }

    if style.is_inverse() {
        out.add_modifier |= Modifier::REVERSED;
    }

    if style.is_underline() {
        out.add_modifier |= Modifier::UNDERLINED;
    }

    out
}

fn ansi_color_to_tui_color(clr: ansi_str::Color) -> Option<Color> {
    use ansi_str::Color::*;

    let clr = match clr {
        Black => Color::Black,
        BrightBlack => Color::DarkGray,
        Red => Color::Red,
        BrightRed => Color::LightRed,
        Green => Color::Green,
        BrightGreen => Color::LightGreen,
        Yellow => Color::Yellow,
        BrightYellow => Color::LightYellow,
        Blue => Color::Blue,
        BrightBlue => Color::LightBlue,
        Magenta => Color::Magenta,
        BrightMagenta => Color::LightMagenta,
        Cyan => Color::Cyan,
        BrightCyan => Color::LightCyan,
        White => Color::White,
        Fixed(i) => Color::Indexed(i),
        Rgb(r, g, b) => Color::Rgb(r, g, b),
        BrightWhite => Color::Gray,
        BrightPurple => Color::LightMagenta,
        Purple => Color::Magenta,
    };

    Some(clr)
}
