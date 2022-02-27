//! Hotpatch to use lscolors without depending on the unmaintained `ansi-term` crate or crossterm

pub trait ToNuAnsiStyle {
    fn to_nu_ansi_style(&self) -> nu_ansi_term::Style;
}

pub trait ToNuAnsiColor {
    fn to_nu_ansi_color(&self) -> nu_ansi_term::Color;
}

impl ToNuAnsiStyle for lscolors::Style {
    fn to_nu_ansi_style(&self) -> nu_ansi_term::Style {
        nu_ansi_term::Style {
            foreground: self
                .foreground
                .as_ref()
                .map(ToNuAnsiColor::to_nu_ansi_color),
            background: self
                .background
                .as_ref()
                .map(ToNuAnsiColor::to_nu_ansi_color),
            is_bold: self.font_style.bold,
            is_dimmed: self.font_style.dimmed,
            is_italic: self.font_style.italic,
            is_underline: self.font_style.underline,
            is_blink: self.font_style.rapid_blink || self.font_style.slow_blink,
            is_reverse: self.font_style.reverse,
            is_hidden: self.font_style.hidden,
            is_strikethrough: self.font_style.strikethrough,
        }
    }
}

impl ToNuAnsiColor for lscolors::Color {
    fn to_nu_ansi_color(&self) -> nu_ansi_term::Color {
        match self {
            lscolors::Color::RGB(r, g, b) => nu_ansi_term::Color::Rgb(*r, *g, *b),
            lscolors::Color::Fixed(n) => nu_ansi_term::Color::Fixed(*n),
            lscolors::Color::Black => nu_ansi_term::Color::Black,
            lscolors::Color::Red => nu_ansi_term::Color::Red,
            lscolors::Color::Green => nu_ansi_term::Color::Green,
            lscolors::Color::Yellow => nu_ansi_term::Color::Yellow,
            lscolors::Color::Blue => nu_ansi_term::Color::Blue,
            lscolors::Color::Magenta => nu_ansi_term::Color::Purple,
            lscolors::Color::Cyan => nu_ansi_term::Color::Cyan,
            lscolors::Color::White => nu_ansi_term::Color::White,

            // Below items are a rough translations to 256 colors as
            // we do not have bright varients available on ansi-term
            lscolors::Color::BrightBlack => nu_ansi_term::Color::Fixed(8),
            lscolors::Color::BrightRed => nu_ansi_term::Color::Fixed(9),
            lscolors::Color::BrightGreen => nu_ansi_term::Color::Fixed(10),
            lscolors::Color::BrightYellow => nu_ansi_term::Color::Fixed(11),
            lscolors::Color::BrightBlue => nu_ansi_term::Color::Fixed(12),
            lscolors::Color::BrightMagenta => nu_ansi_term::Color::Fixed(13),
            lscolors::Color::BrightCyan => nu_ansi_term::Color::Fixed(14),
            lscolors::Color::BrightWhite => nu_ansi_term::Color::Fixed(15),
        }
    }
}
