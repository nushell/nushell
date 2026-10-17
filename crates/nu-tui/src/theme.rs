//! Colors and border lines for the `tui` widgets, read from `$env.config.tui`.
//!
//! Every color slot is a `color_config`-style value (a color name, `#RRGGBB`,
//! or `{fg, bg, attr}`); see `default_tui` in nu-protocol for the defaults.
//! `border_type` names the `table --theme` whose outline borders draw.
//! Table cells follow `color_config` per value type and `LS_COLORS` for path
//! columns, through the converters shared with `explore`.

use lscolors::LsColors;
use nu_color_config::{StyleComputer, get_color_map};
use nu_explore::style::{get_path_style, nu_style_to_tui, text_style_to_tui_style};
use nu_protocol::{
    ShellError, TableMode, Value,
    engine::{EngineState, Stack},
    shell_error::generic::GenericError,
};
use ratatui::style::{Color, Modifier, Style};
use ratatui::symbols::border;
use std::path::Path;

/// The border of widgets without their own `--border` when
/// `$env.config.tui.border_type` is unset: the plain ratatui lines.
const DEFAULT_BORDER: TableMode = TableMode::Single;

/// Read a `--border` flag: one of [`TableMode::tui_border_names`].
/// `$env.config.tui.border_type` is checked the same way when it is set.
pub fn parse_border(value: &Value) -> Result<TableMode, ShellError> {
    let span = value.span();
    let name = value.as_str()?;
    if let Some(mode) = TableMode::from_tui_border(name) {
        return Ok(mode);
    }
    if name.eq_ignore_ascii_case("none") {
        return Err(ShellError::Generic(
            GenericError::new(
                "`none` is not a tui border",
                "a tui border always takes one cell",
                span,
            )
            .with_help("use `frameless` for a border that draws nothing"),
        ));
    }
    Err(ShellError::InvalidValue {
        valid: TableMode::tui_border_names().collect::<Vec<_>>().join(", "),
        actual: name.to_string(),
        span,
    })
}

/// The closest outline a `table --theme` draws, as ratatui border glyphs.
/// A table line with no sides (`compact`, `with_love`) runs through the
/// corners with blank sides, and a theme whose only lines are inner ones
/// (`light`, `psql`, `frameless`) is blank. Every glyph is one cell wide.
fn outline(mode: TableMode) -> border::Set<'static> {
    let custom = |corners: [&'static str; 4], vertical, horizontal| border::Set {
        top_left: corners[0],
        top_right: corners[1],
        bottom_left: corners[2],
        bottom_right: corners[3],
        vertical_left: vertical,
        vertical_right: vertical,
        horizontal_top: horizontal,
        horizontal_bottom: horizontal,
    };
    match mode {
        TableMode::Thin | TableMode::Single => border::PLAIN,
        TableMode::Rounded => border::ROUNDED,
        TableMode::Heavy => border::THICK,
        TableMode::Double => border::DOUBLE,
        TableMode::Reinforced => border::Set {
            top_left: "┏",
            top_right: "┓",
            bottom_left: "┗",
            bottom_right: "┛",
            ..border::PLAIN
        },
        TableMode::Basic | TableMode::BasicCompact => custom(["+"; 4], "|", "-"),
        TableMode::AsciiRounded => custom([".", ".", "'", "'"], "|", "-"),
        TableMode::Dots => custom([".", ".", ":", ":"], ":", "."),
        TableMode::WithLove => custom(["❤"; 4], " ", "❤"),
        TableMode::Markdown => custom(["|"; 4], "|", " "),
        TableMode::Compact => custom(["─"; 4], " ", "─"),
        TableMode::CompactDouble => custom(["═"; 4], " ", "═"),
        TableMode::Restructured => custom(["="; 4], " ", "="),
        TableMode::Light | TableMode::Psql | TableMode::Frameless | TableMode::None => {
            custom([" "; 4], " ", " ")
        }
    }
}

#[derive(Debug, Clone)]
pub struct Theme {
    /// Opaque fill behind every widget. Reset would show the terminal
    /// through a dialog.
    pub surface: Color,
    /// Full-screen fill behind a dialog.
    pub backdrop: Color,
    /// `false` when `use_ansi_coloring` is off: no colors, only reverse
    /// video for the selection.
    pub colored: bool,
    title: Style,
    status: Style,
    border: Style,
    border_focused: Style,
    selected: Style,
    header: Style,
    muted: Style,
    highlight: Style,
    tab_active: Style,
    tab_inactive: Style,
    progress: Style,
    button: Style,
    /// `$env.config.tui.border_type`: the border of widgets without their
    /// own `--border`.
    border_type: TableMode,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            surface: Color::Rgb(18, 18, 22),
            backdrop: Color::Rgb(8, 8, 12),
            colored: true,
            title: Style::default()
                .fg(Color::White)
                .bg(Color::Blue)
                .add_modifier(Modifier::BOLD),
            status: Style::default().fg(Color::White).bg(Color::DarkGray),
            border: Style::default().fg(Color::DarkGray),
            border_focused: Style::default().fg(Color::Cyan),
            selected: Style::default().add_modifier(Modifier::REVERSED),
            header: Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
            muted: Style::default().fg(Color::DarkGray),
            highlight: Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
            tab_active: Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
            tab_inactive: Style::default().fg(Color::DarkGray),
            progress: Style::default().fg(Color::Green),
            button: Style::default().fg(Color::White).bg(Color::Blue),
            border_type: DEFAULT_BORDER,
        }
    }
}

impl Theme {
    /// No colors at all, for `use_ansi_coloring = false` or `NO_COLOR`.
    pub fn plain() -> Self {
        Self {
            surface: Color::Reset,
            backdrop: Color::Reset,
            colored: false,
            title: Style::default().add_modifier(Modifier::BOLD),
            status: Style::default(),
            border: Style::default(),
            border_focused: Style::default().add_modifier(Modifier::BOLD),
            selected: Style::default().add_modifier(Modifier::REVERSED),
            header: Style::default().add_modifier(Modifier::BOLD),
            muted: Style::default().add_modifier(Modifier::DIM),
            highlight: Style::default().add_modifier(Modifier::BOLD),
            tab_active: Style::default().add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
            tab_inactive: Style::default(),
            progress: Style::default(),
            button: Style::default().add_modifier(Modifier::REVERSED),
            border_type: DEFAULT_BORDER,
        }
    }

    /// Read `$env.config.tui`. Slots missing from the config, or holding a
    /// value that is not a color or border, keep the built-in default.
    pub fn from_config(engine_state: &EngineState, stack: &Stack) -> Self {
        let config = stack.get_config(engine_state);
        let border_type = config
            .tui
            .get("border_type")
            .and_then(|value| value.as_str().ok())
            .and_then(TableMode::from_tui_border)
            .unwrap_or(DEFAULT_BORDER);
        if !config.use_ansi_coloring.get(engine_state) {
            return Self {
                border_type,
                ..Self::plain()
            };
        }
        let colors = get_color_map(&config.tui);
        let mut theme = Self {
            border_type,
            ..Self::default()
        };
        let slot = |name: &str, target: &mut Style| {
            if let Some(style) = colors.get(name) {
                *target = nu_style_to_tui(*style);
            }
        };
        slot("title_bar", &mut theme.title);
        slot("status_bar", &mut theme.status);
        slot("border", &mut theme.border);
        slot("border_focused", &mut theme.border_focused);
        slot("selected", &mut theme.selected);
        slot("header", &mut theme.header);
        slot("muted", &mut theme.muted);
        slot("highlight", &mut theme.highlight);
        slot("tab_active", &mut theme.tab_active);
        slot("tab_inactive", &mut theme.tab_inactive);
        slot("progress", &mut theme.progress);
        slot("button", &mut theme.button);
        if let Some(bg) = colors.get("surface").and_then(|s| s.background) {
            theme.surface = nu_style_to_tui(nu_ansi_term::Style::new().on(bg))
                .bg
                .unwrap_or(theme.surface);
        }
        if let Some(bg) = colors.get("backdrop").and_then(|s| s.background) {
            theme.backdrop = nu_style_to_tui(nu_ansi_term::Style::new().on(bg))
                .bg
                .unwrap_or(theme.backdrop);
        }
        theme
    }

    /// A style painted over the surface: the slot's colors win, the surface
    /// fills in a missing background.
    fn on_surface(&self, style: Style) -> Style {
        self.surface().patch(style)
    }

    pub fn surface(&self) -> Style {
        if self.colored {
            Style::default().bg(self.surface)
        } else {
            Style::default()
        }
    }

    pub fn backdrop(&self) -> Style {
        if self.colored {
            Style::default().bg(self.backdrop).patch(self.muted)
        } else {
            self.muted
        }
    }

    pub fn title(&self) -> Style {
        self.on_surface(self.title)
    }

    pub fn status(&self) -> Style {
        self.on_surface(self.status)
    }

    pub fn border(&self, focused: bool) -> Style {
        self.on_surface(if focused {
            self.border_focused
        } else {
            self.border
        })
    }

    /// Border glyphs for a widget: its own `--border`, or the configured
    /// `border_type`.
    pub fn border_set(&self, own: Option<TableMode>) -> border::Set<'static> {
        outline(own.unwrap_or(self.border_type))
    }

    pub fn selected(&self) -> Style {
        self.selected
    }

    pub fn text(&self) -> Style {
        self.surface()
    }

    pub fn muted(&self) -> Style {
        self.on_surface(self.muted)
    }

    pub fn highlight(&self) -> Style {
        self.on_surface(self.highlight)
    }

    pub fn header(&self) -> Style {
        self.on_surface(self.header)
    }

    pub fn tab(&self, active: bool) -> Style {
        self.on_surface(if active {
            self.tab_active
        } else {
            self.tab_inactive
        })
    }

    pub fn progress(&self) -> Style {
        self.on_surface(self.progress)
    }

    pub fn button(&self, focused: bool) -> Style {
        let base = self.on_surface(self.button);
        if focused {
            base.patch(self.selected)
        } else {
            base
        }
    }

    /// Style a cell from `color_config` for that value's type (`int`,
    /// `filesize`, …).
    pub fn value_cell(&self, value: &Value, computer: &StyleComputer) -> Style {
        if !self.colored {
            return self.text();
        }
        self.on_surface(text_style_to_tui_style(computer.style_primitive(value)))
    }

    /// Style a filesystem path with `LS_COLORS`. Regular files with `fi=0`
    /// get no foreground, the same rule `table` and `explore` use.
    pub fn path_cell(&self, path: &str, cwd: &Path, ls_colors: &LsColors) -> Style {
        if !self.colored {
            return self.text();
        }
        match get_path_style(path, &cwd.to_string_lossy(), ls_colors) {
            Some(style) => self.on_surface(nu_style_to_tui(style)),
            None => self.text(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nu_color_config::ComputableStyle;
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
        let computer = StyleComputer::new(&engine_state, &stack, map);
        let theme = Theme::default();
        let fs = theme.value_cell(&Value::test_filesize(1024), &computer);
        assert_eq!(fs.fg, Some(Color::Cyan));
        assert_eq!(fs.bg, Some(theme.surface));
        let s = theme.value_cell(&Value::test_string("hello"), &computer);
        assert_eq!(s.fg, Some(Color::Red));
    }

    #[test]
    fn regular_file_with_fi_0_has_no_forced_foreground() {
        let ls = get_ls_colors(Some("fi=0:di=0;38;5;81:ex=1;38;5;203".into()));
        let theme = Theme::default();
        let style = theme.path_cell("no-such-file-for-style.txt", Path::new("/tmp"), &ls);
        assert_eq!(style.fg, None);
    }

    #[test]
    fn every_border_name_parses_to_one_cell_glyphs() {
        use unicode_width::UnicodeWidthStr;
        for name in TableMode::tui_border_names() {
            let mode =
                parse_border(&Value::test_string(name)).unwrap_or_else(|e| panic!("{name}: {e:?}"));
            let set = outline(mode);
            for glyph in [
                set.top_left,
                set.top_right,
                set.bottom_left,
                set.bottom_right,
                set.vertical_left,
                set.vertical_right,
                set.horizontal_top,
                set.horizontal_bottom,
            ] {
                assert_eq!(glyph.width(), 1, "{name}: {glyph:?}");
            }
        }
    }

    #[test]
    fn none_default_and_unknown_borders_are_errors() {
        assert!(matches!(
            parse_border(&Value::test_string("none")),
            Err(ShellError::Generic(_))
        ));
        for name in ["default", "wavy"] {
            assert!(
                matches!(
                    parse_border(&Value::test_string(name)),
                    Err(ShellError::InvalidValue { .. })
                ),
                "{name}"
            );
        }
        assert!(parse_border(&Value::test_int(1)).is_err());
        // Case is ignored, like `table --theme`.
        assert_eq!(
            parse_border(&Value::test_string("Double")).ok(),
            Some(TableMode::Double)
        );
    }

    #[test]
    fn plain_theme_has_no_colors() {
        let theme = Theme::plain();
        assert_eq!(theme.title().fg, None);
        assert_eq!(theme.border(true).bg, None);
        assert!(theme.selected().add_modifier.contains(Modifier::REVERSED));
    }
}
