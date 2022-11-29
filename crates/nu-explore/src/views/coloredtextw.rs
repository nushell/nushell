use std::borrow::Cow;

use ansi_str::{get_blocks, AnsiStr};
use tui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::Widget,
};

pub struct ColoredTextW<'a> {
    text: &'a str,
    col: usize,
}

impl<'a> ColoredTextW<'a> {
    pub fn new(text: &'a str, col: usize) -> Self {
        Self { text, col }
    }

    pub fn what(&self, area: Rect) -> String {
        let text = cut_string(self.text, area, self.col);
        text.ansi_strip().into_owned()
    }
}

impl Widget for ColoredTextW<'_> {
    fn render(self, area: Rect, buf: &mut tui::buffer::Buffer) {
        let text = cut_string(self.text, area, self.col);

        let mut offset = 0;
        for block in get_blocks(&text) {
            let text = block.text();
            let style = style_to_tui(block.style());

            let x = area.x + offset as u16;
            let (o, _) = buf.set_stringn(x, area.y, text, area.width as usize, style);

            offset = o
        }
    }
}

fn cut_string(text: &str, area: Rect, skip: usize) -> Cow<'_, str> {
    let mut text = Cow::Borrowed(text);

    if skip > 0 {
        let n = text
            .ansi_strip()
            .chars()
            .map(|c| c.len_utf8())
            .take(skip)
            .sum::<usize>();

        let s = text.ansi_get(n..).expect("must be OK").into_owned();
        text = Cow::Owned(s);
    }
    if !text.is_empty() && text.len() > area.width as usize {
        let n = text
            .ansi_strip()
            .chars()
            .map(|c| c.len_utf8())
            .take(area.width as usize)
            .sum::<usize>();

        let s = text.ansi_get(..n).expect("must be ok").into_owned();
        text = Cow::Owned(s);
    }

    text
}

fn style_to_tui(style: ansi_str::Style) -> Style {
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
