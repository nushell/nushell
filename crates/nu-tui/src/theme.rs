//! Colors and border lines for the `tui` widgets, read from `$env.config.tui`.
//!
//! Every color slot is a `color_config`-style value (a color name, `#RRGGBB`,
//! or `{fg, bg, attr}`); see `default_tui` in nu-protocol for the defaults.
//! `border_type` names the `table --theme` whose outline borders draw.
//! Table cells follow `color_config` per value type, through the converters
//! shared with `explore`. Path cells follow `file_styles`, an ordered list of
//! rules on the kind of file, its mime type, extension, and mode, not
//! `LS_COLORS`, whose default theme paints some names on a background.

use nu_color_config::{
    StyleComputer, color_record_to_nustyle, get_color_map, lookup_ansi_color_style,
};
use nu_explore::style::{nu_ansi_color_to_tui_color, nu_style_to_tui, text_style_to_tui_style};
use nu_glob::Pattern;
use nu_protocol::{
    Config, ShellError, TableMode, Value,
    engine::{EngineState, Stack},
    shell_error::generic::GenericError,
};
use ratatui::style::{Color, Modifier, Style};
use ratatui::symbols::border;
use std::collections::HashMap;
use std::ops::RangeInclusive;
use std::sync::{Arc, LazyLock};
use unicode_width::UnicodeWidthStr;

/// The border of widgets without their own `--border` when
/// `$env.config.tui.border_type` is unset: the plain ratatui lines.
const DEFAULT_BORDER: TableMode = TableMode::Single;

/// The color slots `$env.config.tui` and `tui theme` take, and the two
/// fills.
pub const COLOR_SLOTS: [&str; 14] = [
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
    matches!(name, "border_type" | "file_styles")
        || COLOR_SLOTS.contains(&name)
        || GLYPH_SLOTS.contains(&name)
}

/// Check one slot as `tui theme` and `$env.config.tui` take it: a known
/// name, and for a border, the rules, or a glyph, a value of the right
/// shape. Colors are not checked, as in `color_config`.
pub fn check_slot(name: &str, value: &Value) -> Result<(), ShellError> {
    if !is_slot(name) {
        return Err(ShellError::InvalidValue {
            valid: format!(
                "one of border_type, file_styles, {}, {}",
                COLOR_SLOTS.join(", "),
                GLYPH_SLOTS.join(", ")
            ),
            actual: name.to_string(),
            span: value.span(),
        });
    }
    match name {
        "border_type" => parse_border(value).map(drop),
        "file_styles" => parse_file_styles(value).map(drop),
        _ => Glyphs::default().apply(name, value),
    }
}

/// Check `$env.config.tui` when a TUI starts, so a bad slot is an error
/// rather than a silent default. `image_protocol` is config-only and
/// checked where it is set.
pub fn check_config(tui: &HashMap<String, Value>) -> Result<(), ShellError> {
    for (name, value) in tui {
        if name == "image_protocol" {
            continue;
        }
        check_slot(name, value).map_err(|err| {
            ShellError::Generic(
                GenericError::new(
                    format!("invalid $env.config.tui.{name}"),
                    "set here",
                    value.span(),
                )
                .with_help("`tui theme` takes the same slots; see `help tui theme`")
                .with_inner([err]),
            )
        })?;
    }
    Ok(())
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
    /// `file_styles`: how path cells look, the first matching rule winning.
    /// Shared, since the theme is cloned for every frame.
    file_styles: Arc<[FileStyle]>,
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
            file_styles: default_file_styles(),
            border_type: DEFAULT_BORDER,
            glyphs: Glyphs::default(),
        }
    }
}

/// What a path cell's file is on disk. A symlink is not followed for this:
/// one that resolves is [`Symlink`](Self::Symlink), whatever it points at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileKind {
    Dir,
    File,
    Symlink,
    /// A symlink whose target is gone.
    Orphan,
}

impl FileKind {
    const NAMES: [&str; 4] = ["dir", "file", "symlink", "orphan"];

    fn parse(name: &str) -> Option<Self> {
        match name {
            "dir" => Some(Self::Dir),
            "file" => Some(Self::File),
            "symlink" => Some(Self::Symlink),
            "orphan" => Some(Self::Orphan),
            _ => None,
        }
    }
}

/// What a `file_styles` rule can look at. A field that is `None` (a
/// directory's mime type, a mode on Windows) matches no rule that asks for it.
#[derive(Debug)]
pub struct FileFacts<'a> {
    pub kind: FileKind,
    pub mime: Option<&'a str>,
    /// The extension, lowercased.
    pub ext: Option<String>,
    /// As `ls -l` shows it: `rwxr-xr-x`.
    pub mode: Option<String>,
}

/// One rule of `$env.config.tui.file_styles`: what a file must be, all of it,
/// and how it then looks. The first rule a file matches is the one it gets.
#[derive(Debug, Default)]
pub struct FileStyle {
    kind: Option<FileKind>,
    mime_types: Vec<Pattern>,
    exts: Vec<String>,
    modes: Vec<Pattern>,
    style: Style,
    icon: Option<String>,
}

impl FileStyle {
    fn parse(value: &Value) -> Result<Self, ShellError> {
        let mut rule = Self {
            style: config_style(color_record_to_nustyle(value)),
            ..Self::default()
        };
        for (field, value) in value.as_record()? {
            match field.as_str() {
                "kind" => {
                    let name = value.as_str()?;
                    rule.kind =
                        Some(
                            FileKind::parse(name).ok_or_else(|| ShellError::InvalidValue {
                                valid: format!("one of {}", FileKind::NAMES.join(", ")),
                                actual: name.into(),
                                span: value.span(),
                            })?,
                        );
                }
                "mime_type" => rule.mime_types = patterns(value)?,
                "mode" => rule.modes = patterns(value)?,
                "ext" => {
                    rule.exts = strings(value)?.into_iter().map(str::to_lowercase).collect();
                }
                "icon" => rule.icon = Some(value.as_str()?.to_string()),
                "fg" | "bg" | "attr" => {}
                _ => {
                    return Err(ShellError::InvalidValue {
                        valid: "one of kind, mime_type, ext, mode, fg, bg, attr, icon".into(),
                        actual: field.clone(),
                        span: value.span(),
                    });
                }
            }
        }
        Ok(rule)
    }

    fn matches(&self, file: &FileFacts) -> bool {
        let glob = |patterns: &[Pattern], text: Option<&str>| {
            patterns.is_empty() || text.is_some_and(|text| patterns.iter().any(|p| p.matches(text)))
        };
        self.kind.is_none_or(|kind| kind == file.kind)
            && glob(&self.mime_types, file.mime)
            && glob(&self.modes, file.mode.as_deref())
            && (self.exts.is_empty()
                || file.ext.as_ref().is_some_and(|ext| self.exts.contains(ext)))
    }

    /// The glyph that replaces `devicons`' under `--icons`.
    pub fn icon(&self) -> Option<&str> {
        self.icon.as_deref()
    }
}

/// A field that takes one string or a list of them.
fn strings(value: &Value) -> Result<Vec<&str>, ShellError> {
    match value {
        Value::List { vals, .. } => vals.iter().map(Value::as_str).collect(),
        other => Ok(vec![other.as_str()?]),
    }
}

/// A field that takes one glob or a list of them.
fn patterns(value: &Value) -> Result<Vec<Pattern>, ShellError> {
    strings(value)?
        .into_iter()
        .map(|glob| {
            Pattern::new(glob).map_err(|err| ShellError::InvalidValue {
                valid: format!("a glob ({err})"),
                actual: glob.into(),
                span: value.span(),
            })
        })
        .collect()
}

/// Read a `file_styles` list, as `tui theme` and `Theme::from_config` do.
pub fn parse_file_styles(value: &Value) -> Result<Vec<FileStyle>, ShellError> {
    value.as_list()?.iter().map(FileStyle::parse).collect()
}

/// The rules in `default_tui`, read once: the theme is built with them
/// before any config applies.
fn default_file_styles() -> Arc<[FileStyle]> {
    static DEFAULT: LazyLock<Arc<[FileStyle]>> = LazyLock::new(|| {
        Config::default()
            .tui
            .get("file_styles")
            .and_then(|value| parse_file_styles(value).ok())
            .unwrap_or_default()
            .into()
    });
    DEFAULT.clone()
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
            file_styles: default_file_styles(),
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
        // Bad rules keep the default list, as a bad color does. `tui theme`
        // reports them; a plain theme still takes the rules' icons.
        let file_styles = tui
            .get("file_styles")
            .and_then(|value| parse_file_styles(value).ok())
            .map_or_else(default_file_styles, Into::into);
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
                file_styles,
                glyphs,
                ..Self::plain()
            };
        }
        let colors = get_color_map(&tui);
        let mut theme = Self {
            border_type,
            file_styles,
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

    /// The first `file_styles` rule the file matches.
    pub fn file_style(&self, file: &FileFacts) -> Option<&FileStyle> {
        self.file_styles.iter().find(|rule| rule.matches(file))
    }

    /// Style a path cell by its rule. A file no rule matches is plain text.
    pub fn path_cell(&self, rule: Option<&FileStyle>) -> Style {
        match rule {
            Some(rule) if self.colored => self.on_surface(rule.style),
            _ => self.text(),
        }
    }

    /// Style for a rule's own icon: the rule's `fg`, else `color`, the
    /// `#rrggbb` devicons gives the file.
    pub fn rule_icon(&self, rule: &FileStyle, color: &str) -> Style {
        match rule.style.fg {
            Some(fg) if self.colored => self.on_surface(Style::default().fg(fg)),
            _ => self.icon(color),
        }
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

    /// A regular file with this mime type.
    fn file(mime: &str) -> FileFacts<'_> {
        FileFacts {
            kind: FileKind::File,
            mime: Some(mime),
            ext: None,
            mode: Some("rw-r--r--".into()),
        }
    }

    /// The `fg` of the rule the default list picks.
    fn default_fg(file: &FileFacts) -> Option<Color> {
        let theme = Theme::default();
        theme.path_cell(theme.file_style(file)).fg
    }

    fn rules(nuon: &[Value]) -> Vec<FileStyle> {
        parse_file_styles(&Value::test_list(nuon.to_vec())).expect("valid rules")
    }

    #[test]
    fn default_rules_by_kind_mode_and_mime() {
        let theme = Theme::default();
        let of = |kind| FileFacts {
            kind,
            mime: None,
            ext: None,
            mode: None,
        };
        assert_eq!(default_fg(&of(FileKind::Dir)), Some(Color::Blue));
        assert_eq!(default_fg(&of(FileKind::Symlink)), Some(Color::Cyan));
        assert_eq!(
            theme.path_cell(theme.file_style(&of(FileKind::Orphan))).bg,
            Some(Color::Red)
        );
        let exec = FileFacts {
            mode: Some("rwxr-xr-x".into()),
            ..file("image/png")
        };
        assert_eq!(default_fg(&exec), Some(Color::Green));
        // No rule: plain text on the surface.
        assert_eq!(default_fg(&file("text/plain")), None);
        assert_eq!(
            theme.path_cell(theme.file_style(&file("text/plain"))).bg,
            Some(theme.surface)
        );
        // A plain theme has no colors, whatever the rule.
        let plain = Theme::plain();
        assert_eq!(
            plain.path_cell(plain.file_style(&of(FileKind::Dir))).fg,
            None
        );
        for (mime, fg) in [
            ("image/png", Some(Color::Yellow)),
            ("audio/mpeg", Some(Color::Magenta)),
            ("video/mp4", Some(Color::Magenta)),
            ("application/zip", Some(Color::Red)),
            ("application/x-tar", Some(Color::Red)),
            ("application/x-7z-compressed", Some(Color::Red)),
            ("application/vnd.rar", Some(Color::Red)),
            ("application/gzip", Some(Color::Red)),
            ("application/pdf", Some(Color::Cyan)),
            ("application/msword", Some(Color::Cyan)),
            ("application/json", None),
            ("text/plain", None),
        ] {
            assert_eq!(default_fg(&file(mime)), fg, "{mime}");
        }
    }

    #[test]
    fn the_first_matching_rule_wins_and_every_field_must_match() {
        use nu_protocol::record;
        let s = Value::test_string;
        let list = rules(&[
            Value::test_record(record! {
                "kind" => s("dir"), "ext" => s("nu"), "fg" => s("red"),
            }),
            Value::test_record(record! { "ext" => s("NU"), "fg" => s("green") }),
            Value::test_record(record! {
                "mime_type" => Value::test_list(vec![s("text/*"), s("image/*")]),
                "fg" => s("blue"),
            }),
            Value::test_record(record! { "mode" => s("??x*"), "fg" => s("cyan") }),
            Value::test_record(record! { "fg" => s("white") }),
        ]);
        let picked = |file: &FileFacts| {
            let index = list.iter().position(|rule| rule.matches(file));
            index.expect("the last rule matches anything")
        };
        let nu = FileFacts {
            ext: Some("nu".into()),
            ..file("text/plain")
        };
        assert_eq!(
            picked(&nu),
            1,
            "file, so not the dir rule; ext ignores case"
        );
        assert_eq!(picked(&file("image/png")), 2);
        let exec = FileFacts {
            mode: Some("rwx------".into()),
            ..file("application/json")
        };
        assert_eq!(picked(&exec), 3);
        // A field the file lacks never matches a rule that asks for it.
        let no_facts = FileFacts {
            kind: FileKind::Dir,
            mime: None,
            ext: None,
            mode: None,
        };
        assert_eq!(picked(&no_facts), 4);
    }

    #[test]
    fn bad_rules_are_errors() {
        use nu_protocol::record;
        let s = Value::test_string;
        for rule in [
            Value::test_record(record! { "kind" => s("folder") }),
            Value::test_record(record! { "extension" => s("nu") }),
            Value::test_record(record! { "mime_type" => s("[") }),
            Value::test_record(record! { "ext" => Value::test_int(1) }),
            s("dir"),
        ] {
            assert!(
                parse_file_styles(&Value::test_list(vec![rule.clone()])).is_err(),
                "{rule:?}"
            );
        }
        assert!(parse_file_styles(&s("dir")).is_err());
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
            (
                "file_styles".to_string(),
                Value::test_list(vec![Value::test_record(record! {
                    "kind" => Value::test_string("dir"),
                    "fg" => Value::test_string("red"),
                })]),
            ),
            (
                "selected".to_string(),
                Value::test_record(record! { "bg" => Value::test_string("white") }),
            ),
        ]);
        let theme = Theme::from_config(&engine_state, &stack, &overrides);
        let dir = FileFacts {
            kind: FileKind::Dir,
            mime: None,
            ext: None,
            mode: None,
        };
        assert_eq!(theme.path_cell(theme.file_style(&dir)).fg, Some(Color::Red));
        assert_eq!(theme.selected().bg, Some(Color::White));
        // The list replaces the config's: its other rules are gone.
        assert_eq!(theme.file_style(&file("image/png")).map(|_| ()), None);
        // Untouched slots keep the config's value.
        let config = Theme::from_config(&engine_state, &stack, &HashMap::new());
        assert_eq!(
            config.path_cell(config.file_style(&dir)).fg,
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
