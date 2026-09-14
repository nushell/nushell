//! Colors used by TUI widgets. Interactive runs follow `$env.config.color_config`.
//! Table/list cells follow value-type styles and, for path columns, `LS_COLORS`.

use std::path::Path;

use lscolors::LsColors;
use nu_color_config::StyleComputer;
use nu_protocol::{
    Span, Value,
    engine::{EngineState, Stack},
};
use ratatui::style::{Color, Modifier, Style};

#[derive(Debug, Clone)]
pub struct Theme {
    pub title_fg: Color,
    pub title_bg: Color,
    pub status_fg: Color,
    pub status_bg: Color,
    pub border: Color,
    pub border_focused: Color,
    pub text: Color,
    pub muted: Color,
    pub highlight: Color,
    pub tab_active: Color,
    pub tab_inactive: Color,
    /// Opaque fill. Reset would show the terminal through a dialog.
    pub surface: Color,
    /// Full-screen fill behind a dialog (alternate screen).
    pub backdrop: Color,
    header: Style,
    selection: Style,
}

impl Default for Theme {
    fn default() -> Self {
        let surface = Color::Rgb(18, 18, 22);
        Self {
            title_fg: Color::White,
            title_bg: Color::Blue,
            status_fg: Color::White,
            status_bg: Color::DarkGray,
            border: Color::DarkGray,
            border_focused: Color::Cyan,
            text: Color::White,
            muted: Color::DarkGray,
            highlight: Color::Yellow,
            tab_active: Color::Cyan,
            tab_inactive: Color::DarkGray,
            surface,
            backdrop: Color::Rgb(8, 8, 12),
            header: Style::default()
                .fg(Color::Green)
                .bg(surface)
                .add_modifier(Modifier::BOLD),
            selection: Style::default().add_modifier(Modifier::REVERSED),
        }
    }
}

impl Theme {
    pub fn from_config(engine_state: &EngineState, stack: &Stack) -> Self {
        let computer = StyleComputer::from_config(engine_state, stack);
        let nothing = Value::nothing(Span::unknown());
        let header = computer.compute("header", &nothing);
        let hints = computer.compute("hints", &nothing);
        let separator = computer.compute("separator", &nothing);
        let filepath = computer.compute("shape_filepath", &nothing);
        let directory = computer.compute("shape_directory", &nothing);
        let selection = computer.compute("selection", &nothing);
        let row_index = computer.compute("row_index", &nothing);

        let mut theme = Self::default();
        if let Some(fg) = named_fg(header.foreground) {
            theme.title_fg = fg;
        }
        if let Some(bg) = named_fg(header.background) {
            theme.title_bg = bg;
        }
        if let Some(fg) = named_fg(hints.foreground) {
            theme.status_fg = fg;
            theme.muted = fg;
            theme.tab_inactive = fg;
        }
        if let Some(bg) = named_fg(hints.background) {
            theme.status_bg = bg;
        }
        if let Some(fg) = named_fg(separator.foreground) {
            theme.border = fg;
        }
        if let Some(fg) = named_fg(filepath.foreground).or_else(|| named_fg(directory.foreground)) {
            theme.border_focused = fg;
            theme.tab_active = fg;
        }
        if let Some(fg) = named_fg(row_index.foreground) {
            theme.highlight = fg;
        }
        theme.header = nu_style_to_ratatui(header, Some(theme.surface));
        // Selection is reverse in the default color_config. Do not use
        // `search_result` (red background) here; that paints the whole row.
        theme.selection = nu_style_to_ratatui(selection, None);
        if theme.selection == Style::default() {
            theme.selection = Style::default().add_modifier(Modifier::REVERSED);
        }
        theme
    }

    pub fn title(&self) -> Style {
        Style::default()
            .fg(self.title_fg)
            .bg(self.title_bg)
            .add_modifier(Modifier::BOLD)
    }

    pub fn status(&self) -> Style {
        Style::default().fg(self.status_fg).bg(self.status_bg)
    }

    pub fn surface(&self) -> Style {
        Style::default().bg(self.surface).fg(self.text)
    }

    pub fn backdrop(&self) -> Style {
        Style::default().bg(self.backdrop).fg(self.muted)
    }

    pub fn border(&self, focused: bool) -> Style {
        Style::default()
            .fg(if focused {
                self.border_focused
            } else {
                self.border
            })
            .bg(self.surface)
    }

    pub fn selected(&self) -> Style {
        self.selection
    }

    pub fn text(&self) -> Style {
        Style::default().fg(self.text).bg(self.surface)
    }

    pub fn muted(&self) -> Style {
        Style::default().fg(self.muted).bg(self.surface)
    }

    pub fn highlight(&self) -> Style {
        Style::default()
            .fg(self.highlight)
            .bg(self.surface)
            .add_modifier(Modifier::BOLD)
    }

    pub fn header(&self) -> Style {
        self.header
    }

    pub fn table_body(&self) -> Style {
        Style::default().bg(self.surface)
    }
}

/// Style a cell from `color_config` for that value's type (`int`, `filesize`, …).
pub fn value_cell_style(value: &Value, computer: &StyleComputer, surface: Color) -> Style {
    let text = computer.style_primitive(value);
    match text.color_style {
        Some(style) => nu_style_to_ratatui(style, Some(surface)),
        None => Style::default().bg(surface),
    }
}

/// Style a filesystem path with `LS_COLORS`.
///
/// Regular files with `fi=0` get no foreground so they are not painted with
/// `color_config.string` (the same rule `table` uses).
pub fn path_cell_style(
    path: &str,
    cwd: &Path,
    ls_colors: &LsColors,
    surface: Color,
    use_ls_colors: bool,
) -> Style {
    if !use_ls_colors {
        return Style::default().bg(surface);
    }
    let stripped = nu_utils::strip_ansi_unlikely(path);
    let full = {
        let p = Path::new(stripped.as_ref());
        if p.is_absolute() {
            p.to_path_buf()
        } else {
            cwd.join(p)
        }
    };
    let meta = std::fs::symlink_metadata(&full).ok();
    let style = ls_colors
        .style_for_path_with_metadata(stripped.as_ref(), meta.as_ref())
        .map(lscolors::Style::to_nu_ansi_term_style);
    match style {
        Some(style) => nu_style_to_ratatui(style, Some(surface)),
        None => Style::default().bg(surface),
    }
}

pub fn nu_style_to_ratatui(style: nu_ansi_term::Style, fallback_bg: Option<Color>) -> Style {
    let mut out = Style::default();
    if let Some(fg) = named_fg(style.foreground) {
        out = out.fg(fg);
    }
    if let Some(bg) = named_fg(style.background) {
        out = out.bg(bg);
    } else if let Some(bg) = fallback_bg {
        out = out.bg(bg);
    }
    if style.is_bold {
        out = out.add_modifier(Modifier::BOLD);
    }
    if style.is_dimmed {
        out = out.add_modifier(Modifier::DIM);
    }
    if style.is_italic {
        out = out.add_modifier(Modifier::ITALIC);
    }
    if style.is_underline {
        out = out.add_modifier(Modifier::UNDERLINED);
    }
    if style.is_blink {
        out = out.add_modifier(Modifier::SLOW_BLINK);
    }
    if style.is_reverse {
        out = out.add_modifier(Modifier::REVERSED);
    }
    if style.is_hidden {
        out = out.add_modifier(Modifier::HIDDEN);
    }
    if style.is_strikethrough {
        out = out.add_modifier(Modifier::CROSSED_OUT);
    }
    out
}

fn named_fg(color: Option<nu_ansi_term::Color>) -> Option<Color> {
    color.and_then(|c| {
        if c == nu_ansi_term::Color::Default {
            None
        } else {
            Some(nu_color_to_ratatui(c))
        }
    })
}

fn nu_color_to_ratatui(color: nu_ansi_term::Color) -> Color {
    use nu_ansi_term::Color as C;
    match color {
        C::Black => Color::Black,
        C::DarkGray => Color::DarkGray,
        C::Red => Color::Red,
        C::LightRed => Color::LightRed,
        C::Green => Color::Green,
        C::LightGreen => Color::LightGreen,
        C::Yellow => Color::Yellow,
        C::LightYellow => Color::LightYellow,
        C::Blue => Color::Blue,
        C::LightBlue => Color::LightBlue,
        C::Purple | C::Magenta => Color::Magenta,
        C::LightPurple | C::LightMagenta => Color::LightMagenta,
        C::Cyan => Color::Cyan,
        C::LightCyan => Color::LightCyan,
        C::White => Color::Gray,
        C::LightGray => Color::Gray,
        C::Fixed(n) => Color::Indexed(n),
        C::Rgb(r, g, b) => Color::Rgb(r, g, b),
        C::Default => Color::White,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nu_color_config::{ComputableStyle, StyleComputer};
    use nu_protocol::engine::{EngineState, Stack};
    use nu_utils::get_ls_colors;
    use std::collections::HashMap;

    #[test]
    fn filesize_uses_filesize_color_not_string() {
        let engine_state = EngineState::new();
        let stack = Stack::new();
        let mut map = HashMap::new();
        map.insert(
            "filesize".into(),
            ComputableStyle::Static(nu_ansi_term::Color::Cyan.normal()),
        );
        map.insert(
            "string".into(),
            ComputableStyle::Static(nu_ansi_term::Color::Red.normal()),
        );
        map.insert(
            "int".into(),
            ComputableStyle::Static(nu_ansi_term::Color::Purple.normal()),
        );
        let computer = StyleComputer::new(&engine_state, &stack, map);
        let surface = Color::Rgb(18, 18, 22);
        let fs = value_cell_style(&Value::test_filesize(1024), &computer, surface);
        assert_eq!(fs.fg, Some(Color::Cyan));
        let n = value_cell_style(&Value::test_int(7), &computer, surface);
        assert_eq!(n.fg, Some(Color::Magenta));
        let s = value_cell_style(&Value::test_string("hello"), &computer, surface);
        assert_eq!(s.fg, Some(Color::Red));
    }

    #[test]
    fn regular_file_with_fi_0_has_no_forced_foreground() {
        let ls = get_ls_colors(Some("fi=0:di=0;38;5;81:ex=1;38;5;203".into()));
        let style = path_cell_style(
            "no-such-file-for-style.txt",
            Path::new("/tmp"),
            &ls,
            Color::Rgb(18, 18, 22),
            true,
        );
        assert_eq!(style.fg, None);
    }

    #[test]
    fn default_color_is_not_painted_as_white() {
        let style = nu_style_to_ratatui(
            nu_ansi_term::Style {
                foreground: Some(nu_ansi_term::Color::Default),
                background: Some(nu_ansi_term::Color::Default),
                ..Default::default()
            },
            Some(Color::Rgb(18, 18, 22)),
        );
        assert_eq!(style.fg, None);
        assert_eq!(style.bg, Some(Color::Rgb(18, 18, 22)));
    }
}
