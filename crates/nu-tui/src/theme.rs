//! Colors and border lines for the `tui` widgets, read from `$env.config.tui`.
//!
//! Every color slot is a `color_config`-style value (a color name, `#RRGGBB`,
//! or `{fg, bg, attr}`); see `default_tui` in nu-protocol for the defaults.
//! `border_type` names the `table --theme` whose outline borders draw.
//! Table cells follow `color_config` per value type, through the converters
//! shared with `explore`. Path cells follow the `file_*` slots: yazi's
//! palette by kind of file (directory, executable, image, …), not
//! `LS_COLORS`, whose default theme paints some names on a background.

use nu_color_config::{StyleComputer, get_color_map, lookup_ansi_color_style};
use nu_explore::style::{nu_ansi_color_to_tui_color, nu_style_to_tui, text_style_to_tui_style};
use nu_protocol::{
    ShellError, TableMode, Value,
    engine::{EngineState, Stack},
    shell_error::generic::GenericError,
};
use ratatui::style::{Color, Modifier, Style};
use ratatui::symbols::border;
use std::collections::HashMap;
use std::ops::RangeInclusive;
use unicode_width::UnicodeWidthStr;

/// The border of widgets without their own `--border` when
/// `$env.config.tui.border_type` is unset: the plain ratatui lines.
const DEFAULT_BORDER: TableMode = TableMode::Single;

/// The color slots `$env.config.tui` and `tui theme` take, and the two
/// fills.
pub const COLOR_SLOTS: [&str; 21] = [
    "title_bar",
    "status_bar",
    "border",
    "border_focused",
    "selected",
    "header",
    "muted",
    "highlight",
    "tab_active",
    "tab_inactive",
    "progress",
    "button",
    "surface",
    "backdrop",
    "file_dir",
    "file_orphan",
    "file_exec",
    "file_image",
    "file_media",
    "file_archive",
    "file_document",
];

/// The glyph slots: strings, or `{open, close}` and `{on, off}` records.
/// See [`Glyphs`].
pub const GLYPH_SLOTS: [&str; 9] = [
    "title_caps",
    "status_caps",
    "tab_caps",
    "selected_caps",
    "check",
    "radio",
    "tree_open",
    "tree_closed",
    "progress_fill",
];

/// Every key `$env.config.tui` and `tui theme` take.
pub fn is_slot(name: &str) -> bool {
    name == "border_type" || COLOR_SLOTS.contains(&name) || GLYPH_SLOTS.contains(&name)
}

/// Two glyphs round a run of text, as `{open: "\u{e0b6}", close:
/// "\u{e0b4}"}` (powerline's half circles): each is one cell, or empty
/// for no cap on that side.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Caps {
    pub open: String,
    pub close: String,
}

/// The two faces of a mark, as `{on: "[x]", off: "[ ]"}`, of one width
/// so rows line up.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mark {
    pub on: String,
    pub off: String,
}

impl Mark {
    pub fn get(&self, on: bool) -> &str {
        if on { &self.on } else { &self.off }
    }

    /// Cells the mark takes.
    pub fn width(&self) -> usize {
        self.on.width()
    }
}

/// The glyphs widgets draw, from the string slots of `$env.config.tui`.
/// Plain by default; a theme can give the bars powerline caps, a tree
/// chevrons, a gauge another fill.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Glyphs {
    /// `title_caps`: round the title bar's text, which then no longer
    /// fills the bar: a pill, as yazi's mode.
    pub title_caps: Option<Caps>,
    /// `status_caps`: round each segment of the status bar.
    pub status_caps: Option<Caps>,
    /// `tab_caps`: round each tab's title.
    pub tab_caps: Caps,
    /// `selected_caps`: in a cell kept free at each side of a table,
    /// select, or tree, beside the highlighted row.
    pub selected_caps: Option<Caps>,
    /// `check`: a `--multi` row's mark.
    pub check: Mark,
    /// `radio`: a `tui select` row's mark.
    pub radio: Mark,
    /// `tree_open`, `tree_closed`: before a tree node with children.
    pub tree_open: String,
    pub tree_closed: String,
    /// `progress_fill`: a gauge's bar, in the progress color as far as
    /// the value reaches and muted beyond.
    pub progress_fill: String,
}

impl Default for Glyphs {
    fn default() -> Self {
        Self {
            title_caps: None,
            status_caps: None,
            tab_caps: Caps {
                open: String::new(),
                close: String::new(),
            },
            selected_caps: None,
            check: Mark {
                on: "[x]".into(),
                off: "[ ]".into(),
            },
            radio: Mark {
                on: "(•)".into(),
                off: "( )".into(),
            },
            tree_open: "▾".into(),
            tree_closed: "▸".into(),
            progress_fill: "█".into(),
        }
    }
}

impl Glyphs {
    /// Set the slot `name` from a config value. The error says what the
    /// slot takes; a name that is not a glyph slot changes nothing.
    pub fn apply(&mut self, name: &str, value: &Value) -> Result<(), ShellError> {
        match name {
            "title_caps" => self.title_caps = Some(caps(value)?),
            "status_caps" => self.status_caps = Some(caps(value)?),
            "tab_caps" => self.tab_caps = caps(value)?,
            "selected_caps" => self.selected_caps = Some(caps(value)?),
            "check" => self.check = mark(value)?,
            "radio" => self.radio = mark(value)?,
            "tree_open" => self.tree_open = glyph(value, 1..=2)?,
            "tree_closed" => self.tree_closed = glyph(value, 1..=2)?,
            "progress_fill" => self.progress_fill = glyph(value, 1..=1)?,
            _ => {}
        }
        Ok(())
    }
}

/// A string `width` cells wide.
fn glyph(value: &Value, width: RangeInclusive<usize>) -> Result<String, ShellError> {
    let text = value.as_str()?;
    if !width.contains(&text.width()) {
        return Err(ShellError::InvalidValue {
            valid: match (width.start(), width.end()) {
                (lo, hi) if lo == hi => format!("{lo} cell wide"),
                (lo, hi) => format!("{lo} to {hi} cells wide"),
            },
            actual: text.to_string(),
            span: value.span(),
        });
    }
    Ok(text.to_string())
}

/// A record of two glyphs under `keys`.
fn pair(
    value: &Value,
    keys: [&str; 2],
    width: RangeInclusive<usize>,
) -> Result<(String, String), ShellError> {
    let rec = value.as_record()?;
    let get = |key: &str| {
        let Some(glyph_value) = rec.get(key) else {
            return Err(ShellError::Generic(GenericError::new(
                format!("`{key}` is missing"),
                format!("a record of {{{}, {}}}", keys[0], keys[1]),
                value.span(),
            )));
        };
        glyph(glyph_value, width.clone())
    };
    Ok((get(keys[0])?, get(keys[1])?))
}

fn caps(value: &Value) -> Result<Caps, ShellError> {
    let (open, close) = pair(value, ["open", "close"], 0..=1)?;
    Ok(Caps { open, close })
}

fn mark(value: &Value) -> Result<Mark, ShellError> {
    let (on, off) = pair(value, ["on", "off"], 1..=4)?;
    if on.width() != off.width() {
        return Err(ShellError::InvalidValue {
            valid: "`on` and `off` of one width".into(),
            actual: format!("{on:?} and {off:?}"),
            span: value.span(),
        });
    }
    Ok(Mark { on, off })
}

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

/// A `$env.config.tui` color as a ratatui color. `default` is the terminal's
/// own color (`Color::Reset`); the converter shared with `explore` reads it
/// as "no color", which would let the surface show through instead.
fn config_color(color: nu_ansi_term::Color) -> Color {
    nu_ansi_color_to_tui_color(color).unwrap_or(Color::Reset)
}

/// A `$env.config.tui` style as a ratatui style, reading `default` the way
/// [`config_color`] does.
fn config_style(style: nu_ansi_term::Style) -> Style {
    Style {
        fg: style.foreground.map(config_color),
        bg: style.background.map(config_color),
        ..nu_style_to_tui(style)
    }
}

#[derive(Debug, Clone)]
pub struct Theme {
    /// Fill behind every widget. The default, `Color::Reset`, is the
    /// terminal's own background, so text keeps the terminal's colors on
    /// light and dark terminals alike. Unused when `colored` is false.
    pub surface: Color,
    /// Full-screen fill behind a dialog. Unused when `colored` is false.
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
    /// `file_<kind>`: what a path cell of each [`FileKind`] looks like.
    files: [(FileKind, Style); 7],
    /// `$env.config.tui.border_type`: the border of widgets without their
    /// own `--border`.
    border_type: TableMode,
    /// The glyph slots.
    pub glyphs: Glyphs,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            surface: Color::Reset,
            backdrop: Color::Reset,
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
            files: [
                (FileKind::Dir, Style::default().fg(Color::Blue)),
                (FileKind::Orphan, Style::default().bg(Color::Red)),
                (FileKind::Exec, Style::default().fg(Color::Green)),
                (FileKind::Image, Style::default().fg(Color::Yellow)),
                (FileKind::Media, Style::default().fg(Color::Magenta)),
                (FileKind::Archive, Style::default().fg(Color::Red)),
                (FileKind::Document, Style::default().fg(Color::Cyan)),
            ],
            border_type: DEFAULT_BORDER,
            glyphs: Glyphs::default(),
        }
    }
}

/// What a path cell is, for its color: the kinds yazi's default
/// `[filetype]` rules tell apart. `Other` is every remaining file and
/// draws as plain text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileKind {
    Dir,
    /// A symlink whose target is gone.
    Orphan,
    Exec,
    Image,
    /// Audio or video.
    Media,
    Archive,
    /// PDF, Word, RTF.
    Document,
    Other,
}

impl FileKind {
    /// The kind a mime type names, for a file that is not a directory,
    /// an orphan, or executable: yazi's rules on the type and subtype.
    pub fn from_mime(mime: &str) -> Self {
        let Some((kind, subtype)) = mime.split_once('/') else {
            return Self::Other;
        };
        // `x-` and `vnd.` prefixes say nothing about the kind.
        let subtype = subtype.trim_start_matches("x-").trim_start_matches("vnd.");
        let is = |names: &[&str]| names.iter().any(|name| subtype.starts_with(name));
        match kind {
            "image" => Self::Image,
            "audio" | "video" => Self::Media,
            "application"
                if is(&[
                    "zip", "rar", "7z", "tar", "gzip", "xz", "zstd", "bzip", "lzma", "compress",
                    "archive", "cpio", "arj", "xar", "ms-cab",
                ]) =>
            {
                Self::Archive
            }
            "application" if is(&["pdf", "doc", "msword", "rtf"]) => Self::Document,
            _ => Self::Other,
        }
    }

    /// The `$env.config.tui` slot for this kind, if it has one.
    fn slot(self) -> Option<&'static str> {
        Some(match self {
            Self::Dir => "file_dir",
            Self::Orphan => "file_orphan",
            Self::Exec => "file_exec",
            Self::Image => "file_image",
            Self::Media => "file_media",
            Self::Archive => "file_archive",
            Self::Document => "file_document",
            Self::Other => return None,
        })
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
            files: Self::default()
                .files
                .map(|(kind, _)| (kind, Style::default())),
            border_type: DEFAULT_BORDER,
            glyphs: Glyphs::default(),
        }
    }

    /// Read `$env.config.tui`, with `overrides` (from `tui theme`) on top.
    /// Slots missing from both, or holding a value that is not a color or
    /// border, keep the built-in default.
    pub fn from_config(
        engine_state: &EngineState,
        stack: &Stack,
        overrides: &HashMap<String, Value>,
    ) -> Self {
        let config = stack.get_config(engine_state);
        let mut tui = config.tui.clone();
        tui.extend(overrides.iter().map(|(k, v)| (k.clone(), v.clone())));
        let border_type = tui
            .get("border_type")
            .and_then(|value| value.as_str().ok())
            .and_then(TableMode::from_tui_border)
            .unwrap_or(DEFAULT_BORDER);
        let mut glyphs = Glyphs::default();
        for name in GLYPH_SLOTS {
            if let Some(value) = tui.get(name) {
                // A bad glyph keeps the default, as a bad color does.
                let _ = glyphs.apply(name, value);
            }
        }
        // Colors follow the terminal the TUI draws on, which is not stdout
        // when that is a pipe (`nu browse.nu | get name`).
        let is_terminal = crate::runtime::Tty::open().is_terminal();
        if !config.use_ansi_coloring.get_for(engine_state, is_terminal) {
            return Self {
                border_type,
                glyphs,
                ..Self::plain()
            };
        }
        let colors = get_color_map(&tui);
        let mut theme = Self {
            border_type,
            glyphs,
            ..Self::default()
        };
        let slot = |name: &str, target: &mut Style| {
            if let Some(style) = colors.get(name) {
                *target = config_style(*style);
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
        for (kind, style) in &mut theme.files {
            if let Some(name) = kind.slot() {
                slot(name, style);
            }
        }
        // A fill is one color: a record's `bg`, or a bare color
        // (`surface: default`), which parses as a foreground.
        let fill = |name: &str, target: &mut Color| {
            let style = colors.get(name);
            let color = match tui.get(name) {
                Some(Value::String { .. }) => style.and_then(|s| s.foreground),
                _ => style.and_then(|s| s.background),
            };
            if let Some(color) = color {
                *target = config_color(color);
            }
        };
        fill("surface", &mut theme.surface);
        fill("backdrop", &mut theme.backdrop);
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

    /// The style of a cap glyph beside text drawn in `inner`: the glyph
    /// takes the text's fill color on the surface, so the run ends in the
    /// glyph's shape. Reverse video fills with the foreground; text with
    /// no background of its own (only the surface) gets caps in its
    /// foreground, shapes beside it rather than round it.
    pub fn cap_style(&self, inner: Style) -> Style {
        let reversed = inner.add_modifier.contains(Modifier::REVERSED);
        let bg = inner.bg.filter(|bg| *bg != self.surface);
        let fill = if reversed { inner.fg } else { bg.or(inner.fg) };
        self.on_surface(Style::default().fg(fill.unwrap_or(Color::Reset)))
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

    /// Style a path cell by what the file is. `Other` files are plain
    /// text, as in yazi.
    pub fn path_cell(&self, kind: FileKind) -> Style {
        if !self.colored {
            return self.text();
        }
        self.files
            .iter()
            .find(|(k, _)| *k == kind)
            .map_or_else(|| self.text(), |(_, style)| self.on_surface(*style))
    }

    /// Style for a file icon, from the `#rrggbb` devicons gives it.
    pub fn icon(&self, color: &str) -> Style {
        if !self.colored {
            return self.text();
        }
        self.on_surface(nu_style_to_tui(lookup_ansi_color_style(color)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nu_color_config::ComputableStyle;
    use nu_protocol::engine::{EngineState, Stack};
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
    fn path_cells_follow_yazi_palette_and_plain_files_have_no_foreground() {
        let theme = Theme::default();
        assert_eq!(theme.path_cell(FileKind::Other).fg, None);
        assert_eq!(theme.path_cell(FileKind::Dir).fg, Some(Color::Blue));
        assert_eq!(theme.path_cell(FileKind::Orphan).bg, Some(Color::Red));
        assert_eq!(theme.path_cell(FileKind::Exec).fg, Some(Color::Green));
        // Every kind draws on the surface, as other cells do.
        assert_eq!(theme.path_cell(FileKind::Dir).bg, Some(theme.surface));
        assert_eq!(Theme::plain().path_cell(FileKind::Dir).fg, None);
    }

    #[test]
    fn tui_theme_overrides_go_over_the_config_slots() {
        use nu_protocol::{Config, UseAnsiColoring, record};
        let mut engine_state = EngineState::new();
        // No terminal here; the test is about the slots, not `auto`.
        engine_state.set_config(Config {
            use_ansi_coloring: UseAnsiColoring::True,
            ..Config::default()
        });
        let stack = Stack::new();
        let overrides = HashMap::from([
            ("file_dir".to_string(), Value::test_string("red")),
            (
                "selected".to_string(),
                Value::test_record(record! { "bg" => Value::test_string("white") }),
            ),
        ]);
        let theme = Theme::from_config(&engine_state, &stack, &overrides);
        assert_eq!(theme.path_cell(FileKind::Dir).fg, Some(Color::Red));
        assert_eq!(theme.selected().bg, Some(Color::White));
        // Untouched slots keep the config's value.
        assert_eq!(theme.path_cell(FileKind::Exec).fg, Some(Color::Green));
        assert_eq!(
            Theme::from_config(&engine_state, &stack, &HashMap::new())
                .path_cell(FileKind::Dir)
                .fg,
            Some(Color::Blue)
        );
    }

    #[test]
    fn glyph_slots_take_shaped_values_and_refuse_odd_widths() {
        use nu_protocol::record;
        let pair = |a: &str, b: &str, x: &str, y: &str| {
            Value::test_record(record! { a => Value::test_string(x), b => Value::test_string(y) })
        };
        let mut glyphs = Glyphs::default();
        glyphs
            .apply("tab_caps", &pair("open", "close", "\u{e0b6}", "\u{e0b4}"))
            .expect("one-cell powerline caps");
        assert_eq!(glyphs.tab_caps.open, "\u{e0b6}");
        assert_eq!(glyphs.tab_caps.close, "\u{e0b4}");
        glyphs
            .apply("check", &pair("on", "off", "▍", " "))
            .expect("marks of one width");
        assert_eq!(glyphs.check.get(false), " ");
        assert_eq!(glyphs.check.width(), 1);
        glyphs
            .apply("progress_fill", &Value::test_string("━"))
            .expect("a one-cell fill");
        assert!(
            glyphs
                .apply("check", &pair("on", "off", "x", "oo"))
                .is_err()
        );
        assert!(
            glyphs
                .apply("progress_fill", &Value::test_string("ab"))
                .is_err()
        );
        assert!(
            glyphs
                .apply("selected_caps", &pair("open", "close", "<<", ">"))
                .is_err()
        );
        assert!(glyphs.apply("radio", &Value::test_string("x")).is_err());
        assert!(
            glyphs
                .apply("radio", &pair("on", "close", "x", "y"))
                .is_err()
        );
        // Nothing above changed what it refused.
        assert_eq!(glyphs.check.on, "▍");
        assert_eq!(glyphs.radio, Glyphs::default().radio);
    }

    #[test]
    fn cap_style_takes_the_fill_of_the_text() {
        let theme = Theme::default();
        let solid = Style::default().fg(Color::Black).bg(Color::Blue);
        assert_eq!(theme.cap_style(solid).fg, Some(Color::Blue));
        assert_eq!(theme.cap_style(solid).bg, Some(theme.surface));
        let reversed = Style::default()
            .fg(Color::Red)
            .add_modifier(Modifier::REVERSED);
        assert_eq!(theme.cap_style(reversed).fg, Some(Color::Red));
        assert!(
            !theme
                .cap_style(reversed)
                .add_modifier
                .contains(Modifier::REVERSED)
        );
        // On the surface alone, the foreground is all there is to match.
        let on_surface = theme.on_surface(Style::default().fg(Color::Green));
        assert_eq!(theme.cap_style(on_surface).fg, Some(Color::Green));
    }

    #[test]
    fn mime_types_map_to_yazi_kinds() {
        use FileKind::*;
        for (mime, kind) in [
            ("image/png", Image),
            ("audio/mpeg", Media),
            ("video/mp4", Media),
            ("application/zip", Archive),
            ("application/x-tar", Archive),
            ("application/x-7z-compressed", Archive),
            ("application/vnd.rar", Archive),
            ("application/gzip", Archive),
            ("application/pdf", Document),
            ("application/msword", Document),
            ("application/json", Other),
            ("text/plain", Other),
            ("dir", Other),
        ] {
            assert_eq!(FileKind::from_mime(mime), kind, "{mime}");
        }
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
    fn default_fill_is_terminal_background() {
        let theme = Theme::default();
        assert_eq!(theme.surface, Color::Reset);
        assert_eq!(theme.backdrop, Color::Reset);
        // Plain text sets no foreground, so it keeps the terminal's own.
        assert_eq!(theme.text().fg, None);
        assert_eq!(theme.header().fg, Some(Color::Green));
    }

    #[test]
    fn config_fills_and_default_colors() {
        use nu_protocol::{Config, UseAnsiColoring, record};
        let mut config = Config {
            use_ansi_coloring: UseAnsiColoring::True,
            ..Config::default()
        };
        let mut set = |name: &str, value: Value| config.tui.insert(name.into(), value);
        // A bare color is the fill itself; a record gives it as `bg`.
        set("surface", Value::test_string("#112233"));
        set(
            "backdrop",
            Value::test_record(record! { "bg" => Value::test_string("#445566") }),
        );
        set(
            "title_bar",
            Value::test_record(record! {
                "fg" => Value::test_string("black"),
                "bg" => Value::test_string("default"),
            }),
        );
        set(
            "muted",
            Value::test_record(record! { "fg" => Value::test_string("default") }),
        );
        let mut engine_state = EngineState::new();
        engine_state.set_config(config);
        let theme = Theme::from_config(&engine_state, &Stack::new(), &HashMap::new());
        assert!(theme.colored);
        assert_eq!(theme.surface, Color::Rgb(0x11, 0x22, 0x33));
        assert_eq!(theme.backdrop, Color::Rgb(0x44, 0x55, 0x66));
        // `default` is the terminal's color in every slot, not the surface.
        assert_eq!(theme.title().bg, Some(Color::Reset));
        assert_eq!(theme.title().fg, Some(Color::Black));
        assert_eq!(theme.muted().fg, Some(Color::Reset));
        assert_eq!(theme.muted().bg, Some(Color::Rgb(0x11, 0x22, 0x33)));
    }

    #[test]
    fn plain_theme_has_no_colors() {
        let theme = Theme::plain();
        assert_eq!(theme.title().fg, None);
        assert_eq!(theme.border(true).bg, None);
        assert!(theme.selected().add_modifier.contains(Modifier::REVERSED));
    }
}
