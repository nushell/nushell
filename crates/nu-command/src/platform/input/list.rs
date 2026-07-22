use crossterm::{
    cursor::{Hide, MoveDown, MoveToColumn, MoveUp, Show},
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    execute,
    style::Print,
    terminal::{
        self, BeginSynchronizedUpdate, Clear, ClearType, EndSynchronizedUpdate, disable_raw_mode,
        enable_raw_mode,
    },
};
use nu_ansi_term::{Style, ansi::RESET};
use nu_color_config::{Alignment, StyleComputer, TextStyle};
use nu_engine::{ClosureEval, command_prelude::*, get_columns};
use nu_protocol::engine::Closure;
use nu_protocol::{Config, ListStream, Signals, TableMode, shell_error::io::IoError};
use nu_table::common::nu_value_to_string;
use nucleo_matcher::{
    Config as NucleoConfig, Matcher as NucleoMatcher, Utf32Str,
    pattern::{Atom, AtomKind, CaseMatching, Normalization},
};
use std::{
    borrow::Cow,
    collections::HashSet,
    io::{self, Stderr, Write},
    sync::mpsc::{self, Receiver, RecvTimeoutError, TryRecvError},
    thread,
    time::Duration,
};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum CaseSensitivity {
    #[default]
    Smart,
    CaseSensitive,
    CaseInsensitive,
}

#[derive(Debug, Clone)]
struct InputListConfig {
    match_text: Style,                 // For fuzzy match highlighting
    footer: Style,                     // For footer "[1-5 of 10]"
    separator: Style,                  // For separator line
    prompt_marker: Style,              // For prompt marker (">") in fuzzy mode
    selected_marker: Style,            // For selection marker (">") in item list
    table_header: Style,               // For table column headers
    table_separator: Style,            // For table column separators
    show_footer: bool,                 // Whether to show the footer
    separator_char: String,            // Character(s) for separator line between search and results
    show_separator: bool,              // Whether to show the separator line
    prompt_marker_text: String,        // Text for prompt marker (default: "> ")
    selected_marker_char: char,        // Single character for selection marker (default: '>')
    table_column_separator: char,      // Character for table column separator (default: '│')
    table_header_separator: char, // Horizontal line character for header separator (default: '─')
    table_header_intersection: char, // Intersection character for header separator (default: '┼')
    case_sensitivity: CaseSensitivity, // Fuzzy match case sensitivity
}

const DEFAULT_PROMPT_MARKER: &str = "> ";
const DEFAULT_SELECTED_MARKER: char = '>';

const DEFAULT_TABLE_COLUMN_SEPARATOR: char = '│';

// Streaming behavior tuning knobs.
//
// Keeping these as constants makes behavior easy to tweak and avoids hidden magic numbers.
// - INITIAL_STREAM_COLLECT_TIMEOUT: maximum time to spend trying to collect a finite input before
//   falling back to live streaming.
// - INITIAL_STREAM_MAX_ITEMS: safety cap for very fast unbounded streams during initial collection.
// - STREAM_LOAD_BATCH: rows to fetch for each incremental refill.
// - STREAM_PREFETCH_MARGIN: how far from the end we begin prefetching.
// - STREAM_CHANNEL_CAPACITY: rows the background reader can collect before the UI drains them.
// - STREAM_POLL_INTERVAL: render cadence while a stream is still loading.
// - STREAM_FOOTER_UPDATE_INTERVAL: visible footer animation/count cadence while rows stream in.
const INITIAL_STREAM_COLLECT_TIMEOUT: Duration = Duration::from_millis(250);
const INITIAL_STREAM_MAX_ITEMS: usize = 100_000;
const STREAM_LOAD_BATCH: usize = 512;
const STREAM_PREFETCH_MARGIN: usize = 2;
const STREAM_CHANNEL_CAPACITY: usize = 8192;
const STREAM_SPINNER_FRAMES: &[&str] = &["-", "\\", "|", "/"];
const STREAM_DRAIN_TIME_BUDGET: Duration = Duration::from_millis(16);
const STREAM_POLL_INTERVAL: Duration = Duration::from_millis(16);
const STREAM_FOOTER_UPDATE_INTERVAL: Duration = Duration::from_millis(125);
const IDLE_POLL_INTERVAL: Duration = Duration::from_millis(100);
const FUZZY_FILTER_INTERRUPT_CHECK_INTERVAL: usize = 1024;
const FUZZY_FILTER_MIN_INTERRUPT_TIME: Duration = Duration::from_millis(16);

fn io_context(context: &'static str) -> impl FnOnce(io::Error) -> io::Error {
    move |err| io::Error::new(err.kind(), format!("{context}: {err}"))
}

fn terminal_char_width(c: char, current_column: usize) -> usize {
    match c {
        '\t' => {
            let next_tab_stop = ((current_column / 8) + 1) * 8;
            next_tab_stop - current_column
        }
        c if c.is_control() => 0,
        c => UnicodeWidthChar::width(c).unwrap_or(0),
    }
}

fn terminal_text_width_from(text: &str, start_column: usize) -> usize {
    let mut current_column = start_column;
    let mut chars = text.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            skip_ansi_escape(&mut chars);
        } else {
            current_column += terminal_char_width(c, current_column);
        }
    }

    current_column - start_column
}

// These display segments keep terminal control text and user text separate. Existing ANSI helpers
// like strip_ansi_* are useful when ANSI can be discarded entirely, but input list needs to keep
// color escapes in the rendered output while still mapping fuzzy matches back to the original
// source characters.
struct DisplaySegment {
    source_index: Option<usize>,
    text: String,
}

struct SanitizedText {
    segments: Vec<DisplaySegment>,
    text: String,
    source_chars: usize,
    truncated: bool,
}

// Skip ANSI CSI/OSC sequences while measuring terminal width. Existing strip/cut helpers do not
// account for tab stops from an arbitrary starting column, so width calculation stays local to the
// input list renderer.
fn skip_ansi_escape<I>(chars: &mut std::iter::Peekable<I>)
where
    I: Iterator<Item = char>,
{
    match chars.next() {
        Some('[') => {
            for c in chars.by_ref() {
                if ('@'..='~').contains(&c) {
                    break;
                }
            }
        }
        Some(']') => {
            while let Some(c) = chars.next() {
                if c == '\u{7}' {
                    break;
                }
                if c == '\u{1b}' && chars.next_if_eq(&'\\').is_some() {
                    break;
                }
            }
        }
        Some(_) | None => {}
    }
}

// Preserve ANSI CSI/OSC sequences as zero-width display segments. This lets rendering retain
// upstream styling without treating escape bytes as searchable/displayable characters.
fn collect_ansi_escape<I>(chars: &mut std::iter::Peekable<I>) -> Option<String>
where
    I: Iterator<Item = char>,
{
    let mut escape = String::from('\u{1b}');

    match chars.next() {
        Some('[') => {
            escape.push('[');
            for c in chars.by_ref() {
                escape.push(c);
                if ('@'..='~').contains(&c) {
                    return Some(escape);
                }
            }
            Some(escape)
        }
        Some(']') => {
            escape.push(']');
            while let Some(c) = chars.next() {
                escape.push(c);
                if c == '\u{7}' {
                    return Some(escape);
                }
                if c == '\u{1b}' && chars.next_if_eq(&'\\').is_some() {
                    escape.push('\\');
                    return Some(escape);
                }
            }
            Some(escape)
        }
        Some(c) => {
            escape.push(c);
            Some(escape)
        }
        None => Some(escape),
    }
}

fn sanitize_text_for_display(
    text: &str,
    target_width: usize,
    start_column: usize,
) -> SanitizedText {
    let mut current_column = start_column;
    let max_column = start_column + target_width;
    let mut segments = Vec::new();
    let mut sanitized = String::new();
    let mut chars = text.chars().peekable();
    let mut source_index = 0;
    let mut truncated = false;

    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            if let Some(escape) = collect_ansi_escape(&mut chars) {
                sanitized.push_str(&escape);
                segments.push(DisplaySegment {
                    source_index: None,
                    text: escape,
                });
            }
            continue;
        }

        let char_width = terminal_char_width(c, current_column);
        if current_column + char_width > max_column {
            truncated = true;
            break;
        }

        let mut display = String::new();
        if c == '\t' {
            display.extend(std::iter::repeat_n(' ', char_width));
        } else if !c.is_control() {
            display.push(c);
        }

        if !display.is_empty() {
            sanitized.push_str(&display);
            segments.push(DisplaySegment {
                source_index: Some(source_index),
                text: display,
            });
        }
        current_column += char_width;
        source_index += 1;
    }

    SanitizedText {
        segments,
        text: sanitized,
        source_chars: source_index,
        truncated,
    }
}

#[cfg(test)]
fn truncate_ansi_aware_text(text: &str, available_width: usize) -> Cow<'_, str> {
    truncate_ansi_aware_text_at(text, available_width, 0)
}

fn truncate_ansi_aware_text_at(
    text: &str,
    available_width: usize,
    start_column: usize,
) -> Cow<'_, str> {
    let sanitized = sanitize_text_for_display(text, available_width, start_column);
    if !sanitized.truncated {
        Cow::Owned(sanitized.text)
    } else if available_width <= 1 {
        Cow::Borrowed("…")
    } else {
        let target_width = available_width - 1;
        let mut sanitized = sanitize_text_for_display(text, target_width, start_column).text;
        sanitized.push('…');
        Cow::Owned(sanitized)
    }
}

/// Maps TableMode to the appropriate vertical separator character
fn table_mode_to_separator(mode: TableMode) -> char {
    match mode {
        // ASCII-based themes
        TableMode::Basic | TableMode::BasicCompact | TableMode::Psql | TableMode::Markdown => '|',
        TableMode::AsciiRounded => '|',
        // Modern unicode (single line)
        TableMode::Thin
        | TableMode::Rounded
        | TableMode::Single
        | TableMode::Compact
        | TableMode::Frameless => '│',
        TableMode::Reinforced | TableMode::Light => '│',
        // Heavy borders
        TableMode::Heavy => '┃',
        // Double line
        TableMode::Double | TableMode::CompactDouble => '║',
        // Special themes
        TableMode::WithLove => '❤',
        TableMode::Dots => ':',
        // Minimal/no borders
        TableMode::Restructured | TableMode::None => ' ',
    }
}

/// Maps TableMode to (horizontal_line_char, intersection_char) for header separator
fn table_mode_to_header_separator(mode: TableMode) -> (char, char) {
    match mode {
        // ASCII-based themes
        TableMode::Basic | TableMode::BasicCompact | TableMode::Psql => ('-', '+'),
        TableMode::AsciiRounded => ('-', '+'),
        TableMode::Markdown => ('-', '|'),
        // Modern unicode (single line)
        TableMode::Thin
        | TableMode::Rounded
        | TableMode::Single
        | TableMode::Compact
        | TableMode::Frameless => ('─', '┼'),
        TableMode::Reinforced => ('─', '┼'),
        TableMode::Light => ('─', '─'), // Light has no vertical lines, so no intersection
        // Heavy borders
        TableMode::Heavy => ('━', '╋'),
        // Double line
        TableMode::Double | TableMode::CompactDouble => ('═', '╬'),
        // Special themes
        TableMode::WithLove => ('❤', '❤'),
        TableMode::Dots => ('.', ':'),
        // Minimal/no borders - use simple dashes
        TableMode::Restructured | TableMode::None => (' ', ' '),
    }
}

impl Default for InputListConfig {
    fn default() -> Self {
        Self {
            match_text: Style::new().fg(nu_ansi_term::Color::Yellow),
            footer: Style::new().fg(nu_ansi_term::Color::DarkGray),
            separator: Style::new().fg(nu_ansi_term::Color::DarkGray),
            prompt_marker: Style::new().fg(nu_ansi_term::Color::Green),
            selected_marker: Style::new().fg(nu_ansi_term::Color::Green),
            table_header: Style::new().bold(),
            table_separator: Style::new().fg(nu_ansi_term::Color::DarkGray),
            show_footer: true,
            separator_char: "─".to_string(),
            show_separator: true,
            prompt_marker_text: DEFAULT_PROMPT_MARKER.to_string(),
            selected_marker_char: DEFAULT_SELECTED_MARKER,
            table_column_separator: DEFAULT_TABLE_COLUMN_SEPARATOR,
            table_header_separator: '─',
            table_header_intersection: '┼',
            case_sensitivity: CaseSensitivity::default(),
        }
    }
}

impl InputListConfig {
    fn from_nu_config(
        config: &nu_protocol::Config,
        style_computer: &StyleComputer,
        span: Span,
    ) -> Self {
        let mut ret = Self::default();

        // Get styles from color_config (same as regular table command and find)
        let color_config_header = style_computer.compute("header", &Value::string("", span));
        let color_config_separator = style_computer.compute("separator", &Value::nothing(span));
        let color_config_search_result =
            style_computer.compute("search_result", &Value::string("", span));
        let color_config_hints = style_computer.compute("hints", &Value::nothing(span));
        let color_config_row_index = style_computer.compute("row_index", &Value::string("", span));

        ret.table_header = color_config_header;
        ret.table_separator = color_config_separator;
        ret.separator = color_config_separator;
        ret.match_text = color_config_search_result;
        ret.footer = color_config_hints;
        ret.prompt_marker = color_config_row_index;
        ret.selected_marker = color_config_row_index;

        // Derive table separators from user's table mode
        ret.table_column_separator = table_mode_to_separator(config.table.mode);
        let (header_sep, header_int) = table_mode_to_header_separator(config.table.mode);
        ret.table_header_separator = header_sep;
        ret.table_header_intersection = header_int;

        ret
    }
}

enum InteractMode {
    Single(Option<usize>),
    Multi(Option<Vec<usize>>),
}

struct SelectItem {
    name: String, // Search text (concatenated cells in table mode)
    cells: Option<Vec<(String, TextStyle)>>, // Cell values with TextStyle for type-based styling (None = single-line mode)
    value: Value,                            // Original value to return
}

/// Display mode for key-based conversion in streaming mode
#[derive(Clone)]
enum DisplayMode {
    Default,
    CellPath(Vec<nu_protocol::ast::PathMember>),
    Closure(Closure),
}

/// Layout information for table rendering
struct TableLayout {
    columns: Vec<String>,   // Column names
    col_widths: Vec<usize>, // Computed width per column (content only, not separators)
    truncated_cols: usize, // Number of columns that fit in terminal starting from horizontal_offset
}

#[derive(Clone)]
pub struct InputList;

const INTERACT_ERROR: &str = "Interact error, could not process options";

impl Command for InputList {
    fn name(&self) -> &str {
        "input list"
    }

    fn signature(&self) -> Signature {
        Signature::build("input list")
            .input_output_types(vec![
                (Type::List(Box::new(Type::Any)), Type::Any),
                (Type::Range, Type::Int),
            ])
            .optional("prompt", SyntaxShape::String, "The prompt to display.")
            .switch(
                "multi",
                "Use multiple results, you can press a to toggle all, Ctrl+R to refine.",
                Some('m'),
            )
            .switch("fuzzy", "Use a fuzzy select.", Some('f'))
            .switch("index", "Returns list indexes.", Some('i'))
            .switch(
                "no-footer",
                "Hide the footer showing item count and selection count.",
                Some('n'),
            )
            .switch(
                "no-separator",
                "Hide the separator line between the search box and results.",
                None,
            )
            .named(
                "case-sensitive",
                SyntaxShape::OneOf(vec![SyntaxShape::Boolean, SyntaxShape::String]),
                "Case sensitivity for fuzzy matching: true, false, or 'smart' (case-insensitive unless query has uppercase)",
                Some('s'),
            )
            .named(
                "display",
                SyntaxShape::OneOf(vec![
                    SyntaxShape::CellPath,
                    SyntaxShape::Closure(Some(vec![SyntaxShape::Any])),
                ]),
                "Field or closure to generate display value for search (returns original value when selected)",
                Some('d'),
            )
            .switch(
                "no-table",
                "Disable table rendering for table input (show as single lines).",
                Some('t'),
            )
            .switch(
                "per-column",
                "Match filter text against each column independently (table mode only).",
                Some('c'),
            )
            .allow_variants_without_examples(true)
            .category(Category::Platform)
    }

    fn description(&self) -> &str {
        "Display an interactive list for user selection."
    }

    fn extra_description(&self) -> &str {
        r#"Presents an interactive list in the terminal for selecting items.

Four modes are available:
- Single (default): Select one item with arrow keys, confirm with Enter
- Multi (--multi): Select multiple items with Space, toggle all with 'a'
- Fuzzy (--fuzzy): Type to filter, matches are highlighted
- Fuzzy Multi (--fuzzy --multi): Type to filter AND select multiple items with Tab, toggle all with Alt+A

Multi mode features:
- The footer always shows the selection count (e.g., "[1-5 of 10, 3 selected]")
- Use Ctrl+R to "refine" the list: narrow down to only selected items, keeping them
  selected so you can deselect the ones you don't want. Can be used multiple times.

Table rendering:
When piping a table (list of records), items are displayed with aligned columns.
Use Left/Right arrows (or h/l) to scroll horizontally when columns exceed terminal width.
In fuzzy mode, use Shift+Left/Right for horizontal scrolling.
Ellipsis (…) shows when more columns are available in each direction.
In fuzzy mode, the ellipsis is highlighted when matches exist in hidden columns.
Use --no-table to disable table rendering and show records as single lines.
Use --per-column to match filter text against each column independently (best match wins).
This prevents false positives from matches spanning column boundaries.
Use --display to specify a column or closure for display/search text (disables table mode).
The --display flag accepts either a cell path (e.g., -d name) or a closure (e.g., -d {|it| $it.name}).
The closure receives each item and should return the string to display and search on.
The original value is always returned when selected, regardless of what --display shows.

Keyboard shortcuts:
- Up/Down, j/k, Ctrl+n/p: Navigate items
- Left/Right, h/l: Scroll columns horizontally (table mode, single/multi)
- Shift+Left/Right: Scroll columns horizontally (fuzzy mode)
- Home/End: Jump to first/last item
- PageUp/PageDown: Navigate by page
- Space: Toggle selection (multi mode)
- Tab: Toggle selection and move down (fuzzy multi mode)
- Shift+Tab: Toggle selection and move up (fuzzy multi mode)
- a: Toggle all items (multi mode), Alt+A in fuzzy multi mode
- Ctrl+R: Refine list to only selected items (multi modes)
- Alt+C: Cycle case sensitivity (smart -> CASE -> nocase) in fuzzy modes
- Alt+P: Toggle per-column matching in fuzzy table mode
- Enter: Confirm selection
- Esc: Cancel (all modes)
- q: Cancel (single/multi modes only)
- Ctrl+C: Cancel (all modes)

Fuzzy mode supports readline-style editing:
- Ctrl+A/E: Beginning/end of line
- Ctrl+B/F, Left/Right: Move cursor
- Alt+B/F: Move by word
- Ctrl+U/K: Kill to beginning/end of line
- Ctrl+W, Alt+Backspace: Delete previous word
- Ctrl+D, Delete: Delete character at cursor

Styling (inherited from $env.config.color_config):
- search_result: Match highlighting in fuzzy mode
- hints: Footer text
- separator: Separator line and table column separators
- row_index: Prompt marker and selection marker
- header: Table column headers
- Table column characters inherit from $env.config.table.mode

Use --no-footer and --no-separator to hide the footer and separator line."#
    }

    fn search_terms(&self) -> Vec<&str> {
        vec![
            "prompt", "ask", "menu", "select", "pick", "choose", "fzf", "fuzzy",
        ]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;
        let prompt: Option<String> = call.opt(engine_state, stack, 0)?;
        let multi = call.has_flag(engine_state, stack, "multi")?;
        let fuzzy = call.has_flag(engine_state, stack, "fuzzy")?;
        let index = call.has_flag(engine_state, stack, "index")?;
        let display_flag: Option<Value> = call.get_flag(engine_state, stack, "display")?;
        let no_footer = call.has_flag(engine_state, stack, "no-footer")?;
        let no_separator = call.has_flag(engine_state, stack, "no-separator")?;
        let case_sensitive: Option<Value> = call.get_flag(engine_state, stack, "case-sensitive")?;
        let no_table = call.has_flag(engine_state, stack, "no-table")?;
        let per_column = call.has_flag(engine_state, stack, "per-column")?;
        let config = stack.get_config(engine_state);
        let style_computer = StyleComputer::from_config(engine_state, stack);
        let mut input_list_config = InputListConfig::from_nu_config(&config, &style_computer, head);
        if no_footer {
            input_list_config.show_footer = false;
        }
        if no_separator {
            input_list_config.show_separator = false;
        }
        if let Some(cs) = case_sensitive {
            input_list_config.case_sensitivity = match &cs {
                Value::Bool { val: true, .. } => CaseSensitivity::CaseSensitive,
                Value::Bool { val: false, .. } => CaseSensitivity::CaseInsensitive,
                Value::String { val, .. } if val == "smart" => CaseSensitivity::Smart,
                Value::String { val, .. } if val == "true" => CaseSensitivity::CaseSensitive,
                Value::String { val, .. } if val == "false" => CaseSensitivity::CaseInsensitive,
                _ => {
                    return Err(ShellError::InvalidValue {
                        valid: "true, false, or 'smart'".to_string(),
                        actual: cs.to_abbreviated_string(&config),
                        span: cs.span(),
                    });
                }
            };
        }

        let (initial_values, pending_stream) =
            Self::initial_values_from_input(input, head, engine_state.signals().clone())?;

        // Map display_mode from display_flag
        let display_mode = match &display_flag {
            Some(Value::CellPath { val: cellpath, .. }) => {
                DisplayMode::CellPath(cellpath.members.clone())
            }
            Some(Value::Closure { val: closure, .. }) => {
                DisplayMode::Closure(Closure::clone(closure))
            }
            _ => DisplayMode::Default,
        };

        // Detect table mode
        let columns = if matches!(display_mode, DisplayMode::Default) && !no_table {
            get_columns(&initial_values)
        } else {
            vec![]
        };
        let is_table_mode = !columns.is_empty();

        // Build initial SelectItem list
        let options: Vec<SelectItem> = initial_values
            .into_iter()
            .map(|val| {
                InputList::make_select_item(
                    val,
                    &columns,
                    &display_mode,
                    &config,
                    engine_state,
                    stack,
                    head,
                )
            })
            .collect();

        let table_layout = if is_table_mode {
            Some(Self::calculate_table_layout(&columns, &options))
        } else {
            None
        };

        if options.is_empty() && pending_stream.is_none() {
            return Err(ShellError::TypeMismatch {
                err_message: "expected a list or table, it can also be a problem with the inner type of your list.".to_string(),
                span: head,
            });
        }

        let mode = if multi && fuzzy {
            SelectMode::FuzzyMulti
        } else if multi {
            SelectMode::Multi
        } else if fuzzy {
            SelectMode::Fuzzy
        } else {
            SelectMode::Single
        };

        let config_clone = config.clone();
        let columns_clone = columns.clone();
        let display_mode_clone = display_mode.clone();

        // Build conversion logic once and reuse it for all lazily-loaded rows.
        // This guarantees that rows loaded later follow the exact same display rules as rows
        // loaded during initial priming.
        let item_generator: Box<dyn FnMut(Value) -> SelectItem + '_> =
            Box::new(move |val: Value| {
                InputList::make_select_item(
                    val,
                    &columns_clone,
                    &display_mode_clone,
                    &config_clone,
                    engine_state,
                    stack,
                    head,
                )
            });

        let mut widget = SelectWidget::new(
            mode,
            prompt.as_deref(),
            options,
            input_list_config,
            table_layout,
            per_column,
            StreamState {
                stream_reader: pending_stream,
                item_generator: Some(item_generator),
            },
        );
        let answer = widget.run().map_err(|err| {
            IoError::new_with_additional_context(err, call.head, None, INTERACT_ERROR)
        })?;

        Ok(match answer {
            InteractMode::Multi(res) => {
                if index {
                    match res {
                        Some(opts) => Value::list(
                            opts.into_iter()
                                .map(|s| Value::int(s as i64, head))
                                .collect(),
                            head,
                        ),
                        None => Value::nothing(head),
                    }
                } else {
                    match res {
                        Some(opts) => Value::list(
                            opts.iter()
                                .map(|s| widget.items[*s].value.clone())
                                .collect(),
                            head,
                        ),
                        None => Value::nothing(head),
                    }
                }
            }
            InteractMode::Single(res) => {
                if index {
                    match res {
                        Some(opt) => Value::int(opt as i64, head),
                        None => Value::nothing(head),
                    }
                } else {
                    match res {
                        Some(opt) => widget.items[opt].value.clone(),
                        None => Value::nothing(head),
                    }
                }
            }
        }
        .into_pipeline_data())
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Return a single value from a list.",
                example: "[1 2 3 4 5] | input list 'Rate it'",
                result: None,
            },
            Example {
                description: "Return multiple values from a list.",
                example: "[Banana Kiwi Pear Peach Strawberry] | input list --multi 'Add fruits to the basket'",
                result: None,
            },
            Example {
                description: "Return a single record from a table with fuzzy search.",
                example: "ls | input list --fuzzy 'Select the target'",
                result: None,
            },
            Example {
                description: "Choose an item from a range.",
                example: "1..10 | input list",
                result: None,
            },
            Example {
                description: "Return the index of a selected item.",
                example: "[Banana Kiwi Pear Peach Strawberry] | input list --index",
                result: None,
            },
            Example {
                description: "Choose an item from a table using a column as display value.",
                example: "[[name price]; [Banana 12] [Kiwi 4] [Pear 7]] | input list -d name",
                result: None,
            },
            Example {
                description: "Choose an item using a closure to generate display text",
                example: r#"[[name price]; [Banana 12] [Kiwi 4] [Pear 7]] | input list -d {|it| $"($it.name): $($it.price)"}"#,
                result: None,
            },
            Example {
                description: "Fuzzy search with case-sensitive matching",
                example: "[abc ABC aBc] | input list --fuzzy --case-sensitive true",
                result: None,
            },
            Example {
                description: "Fuzzy search without the footer showing item count",
                example: "ls | input list --fuzzy --no-footer",
                result: None,
            },
            Example {
                description: "Fuzzy search without the separator line",
                example: "ls | input list --fuzzy --no-separator",
                result: None,
            },
            Example {
                description: "Fuzzy search with custom match highlighting color",
                example: r#"$env.config.color_config.search_result = "red"; ls | input list --fuzzy"#,
                result: None,
            },
            Example {
                description: "Display a table with column rendering",
                example: r#"[[name size]; [file1.txt "1.2 KB"] [file2.txt "3.4 KB"]] | input list"#,
                result: None,
            },
            Example {
                description: "Display a table as single lines (no table rendering)",
                example: "ls | input list --no-table",
                result: None,
            },
            Example {
                description: "Fuzzy search with multiple selection (use Tab to toggle)",
                example: "ls | input list --fuzzy --multi",
                result: None,
            },
        ]
    }
}

impl InputList {
    /// Extract initial values from supported input.
    ///
    /// Already materialized lists are returned directly. Only true streams and ranges go through
    /// the timed initial read so slow or unbounded input can keep the UI responsive.
    fn initial_values_from_input(
        input: PipelineData,
        head: Span,
        signals: Signals,
    ) -> Result<(Vec<Value>, Option<StreamReader>), ShellError> {
        match input {
            PipelineData::ListStream(stream, ..) => Ok(Self::read_initial_stream_values(stream)),
            PipelineData::Value(Value::List { vals, .. }, ..) => Ok((vals.into_owned(), None)),
            input @ PipelineData::Value(Value::Range { .. }, ..) => {
                let stream = ListStream::new(input.into_iter(), head, signals);
                Ok(Self::read_initial_stream_values(stream))
            }
            _ => Err(ShellError::TypeMismatch {
                err_message: "expected a list, a table, or a range".to_string(),
                span: head,
            }),
        }
    }

    /// Read initial values from the upstream stream.
    ///
    /// Returns any values available before the initial timeout/cap and a reader for the remaining
    /// stream when it is not exhausted yet.
    fn read_initial_stream_values(stream: ListStream) -> (Vec<Value>, Option<StreamReader>) {
        let mut reader = StreamReader::new(stream);
        let values =
            reader.drain_available_until(INITIAL_STREAM_MAX_ITEMS, INITIAL_STREAM_COLLECT_TIMEOUT);
        let pending_stream = if reader.is_finished() {
            None
        } else {
            Some(reader)
        };

        (values, pending_stream)
    }

    /// Convert a raw input `Value` into a `SelectItem`, used for streaming growth
    fn make_select_item(
        value: Value,
        columns: &[String],
        display_mode: &DisplayMode,
        config: &Config,
        engine_state: &EngineState,
        stack: &mut Stack,
        span: Span,
    ) -> SelectItem {
        if !columns.is_empty() {
            // Build style computer on demand so streamed rows preserve the same type-aware
            // formatting behavior as eagerly materialized rows.
            let style_computer = StyleComputer::from_config(engine_state, stack);

            let cells: Vec<(String, TextStyle)> = columns
                .iter()
                .map(|col| {
                    if let Value::Record { val: record, .. } = &value {
                        record
                            .get(col)
                            .map(|v| nu_value_to_string(v, config, &style_computer))
                            .unwrap_or_else(|| (String::new(), TextStyle::default()))
                    } else {
                        (String::new(), TextStyle::default())
                    }
                })
                .collect();

            let name = cells
                .iter()
                .map(|(s, _)| s.as_str())
                .collect::<Vec<_>>()
                .join(" ");
            SelectItem {
                name,
                cells: Some(cells),
                value,
            }
        } else {
            let display_value = match display_mode {
                DisplayMode::CellPath(cellpath) => value
                    .follow_cell_path(cellpath)
                    .map(|v| v.to_expanded_string(", ", config))
                    .unwrap_or_else(|_| value.to_expanded_string(", ", config)),
                DisplayMode::Closure(closure) => {
                    let mut closure_eval =
                        ClosureEval::new(engine_state, stack, Closure::clone(closure));
                    closure_eval
                        .run_with_value(value.clone())
                        .and_then(|data| data.into_value(span))
                        .map(|v| v.to_expanded_string(", ", config))
                        .unwrap_or_else(|_| value.to_expanded_string(", ", config))
                }
                DisplayMode::Default => value.to_expanded_string(", ", config),
            };
            SelectItem {
                name: display_value,
                cells: None,
                value,
            }
        }
    }

    /// Calculate column widths for table rendering
    fn calculate_table_layout(columns: &[String], options: &[SelectItem]) -> TableLayout {
        let mut layout = TableLayout {
            columns: columns.to_vec(),
            col_widths: columns.iter().map(|c| c.width()).collect(),
            truncated_cols: 0, // Will be calculated when terminal width is known
        };

        Self::update_table_layout_with_items(&mut layout, options);
        layout
    }

    fn update_table_layout_with_items(layout: &mut TableLayout, items: &[SelectItem]) -> bool {
        let mut changed = false;
        for item in items {
            if let Some(cells) = &item.cells {
                for (i, (cell_text, _)) in cells.iter().enumerate() {
                    if i < layout.col_widths.len() {
                        let cell_width = terminal_text_width_from(cell_text, 0);
                        if cell_width > layout.col_widths[i] {
                            layout.col_widths[i] = cell_width;
                            changed = true;
                        }
                    }
                }
            }
        }
        changed
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SelectMode {
    Single,
    Multi,
    Fuzzy,
    FuzzyMulti,
}

/// Streaming-specific state injected into `SelectWidget`.
///
/// Keeping stream concerns grouped in one struct reduces constructor parameter noise and
/// keeps the non-streaming widget state easier to reason about.
struct StreamState<'a> {
    stream_reader: Option<StreamReader>,
    item_generator: Option<Box<dyn FnMut(Value) -> SelectItem + 'a>>,
}

enum StreamMessage {
    Item(Value),
    End,
}

struct StreamReader {
    receiver: Receiver<StreamMessage>,
    finished: bool,
}

impl StreamReader {
    fn new(stream: ListStream) -> Self {
        let (sender, receiver) = mpsc::sync_channel(STREAM_CHANNEL_CAPACITY);

        thread::spawn(move || {
            for value in stream {
                if sender.send(StreamMessage::Item(value)).is_err() {
                    return;
                }
            }

            let _ = sender.send(StreamMessage::End);
        });

        Self {
            receiver,
            finished: false,
        }
    }

    fn is_finished(&self) -> bool {
        self.finished
    }

    fn drain_available(&mut self, count: usize) -> Vec<Value> {
        let mut values = Vec::new();

        while values.len() < count && !self.finished {
            match self.receiver.try_recv() {
                Ok(StreamMessage::Item(value)) => values.push(value),
                Ok(StreamMessage::End) | Err(TryRecvError::Disconnected) => {
                    self.finished = true;
                    break;
                }
                Err(TryRecvError::Empty) => break,
            }
        }

        values
    }

    fn drain_available_for(&mut self, max_duration: Duration) -> Vec<Value> {
        let start = nu_utils::time::Instant::now();
        let mut values = Vec::new();

        while !self.finished {
            match self.receiver.try_recv() {
                Ok(StreamMessage::Item(value)) => values.push(value),
                Ok(StreamMessage::End) | Err(TryRecvError::Disconnected) => {
                    self.finished = true;
                    break;
                }
                Err(TryRecvError::Empty) => break,
            }

            if start.elapsed() >= max_duration {
                break;
            }
        }

        values
    }

    fn drain_available_until(&mut self, count: usize, max_duration: Duration) -> Vec<Value> {
        let start = nu_utils::time::Instant::now();
        let mut values = Vec::new();

        while values.len() < count && !self.finished {
            let elapsed = start.elapsed();
            let Some(remaining) = max_duration.checked_sub(elapsed) else {
                break;
            };

            match self.receiver.recv_timeout(remaining) {
                Ok(StreamMessage::Item(value)) => values.push(value),
                Ok(StreamMessage::End) | Err(RecvTimeoutError::Disconnected) => {
                    self.finished = true;
                    break;
                }
                Err(RecvTimeoutError::Timeout) => break,
            }
        }

        values
    }
}

struct SelectWidget<'a> {
    mode: SelectMode,
    prompt: Option<&'a str>,
    items: Vec<SelectItem>,
    cursor: usize,
    selected: HashSet<usize>,
    filter_text: String,
    filtered_indices: Vec<usize>,
    scroll_offset: usize,
    stream_reader: Option<StreamReader>,
    item_generator: Option<Box<dyn FnMut(Value) -> SelectItem + 'a>>,
    visible_height: u16,
    matcher: NucleoMatcher,
    last_filter_text: String,
    force_full_filter: bool,
    rendered_lines: usize,
    /// Previous cursor position for efficient cursor-only updates
    prev_cursor: usize,
    /// Previous scroll offset to detect if we need full redraw
    prev_scroll_offset: usize,
    /// Whether this is the first render
    first_render: bool,
    /// In fuzzy mode, cursor is positioned at filter line; this tracks how far up from end
    fuzzy_cursor_offset: usize,
    /// Whether filter results changed since last render
    results_changed: bool,
    /// Whether filter text changed since last render
    filter_text_changed: bool,
    /// Item that was toggled in multi-mode (for checkbox-only update)
    toggled_item: Option<usize>,
    /// Whether all items were toggled (for bulk checkbox update)
    toggled_all: bool,
    /// Cursor position within filter_text (byte offset)
    filter_cursor: usize,
    /// Configuration for input list styles
    config: InputListConfig,
    /// Cached terminal width for separator line
    term_width: u16,
    /// Cached separator line (regenerated on terminal resize)
    separator_line: String,
    /// Table layout for table mode (None if single-line mode)
    table_layout: Option<TableLayout>,
    /// First visible column index (for horizontal scrolling)
    horizontal_offset: usize,
    /// Whether horizontal scroll changed since last render
    horizontal_scroll_changed: bool,
    /// Whether terminal width changed since last render
    width_changed: bool,
    /// Whether streamed rows changed table column widths since last render
    table_layout_changed: bool,
    /// Whether the list has been refined to only show selected items (Multi/FuzzyMulti)
    refined: bool,
    /// Whether streamed rows should keep the cursor pinned to the loaded tail
    follow_stream_to_end: bool,
    /// Current footer spinner frame while upstream rows are still pending
    stream_spinner_frame: usize,
    /// Last item count shown in the streaming footer
    stream_footer_item_count: usize,
    /// Last time the streaming footer spinner/count was advanced
    last_stream_footer_update: nu_utils::time::Instant,
    /// Base indices for refined mode (the subset to filter from in FuzzyMulti)
    refined_base_indices: Vec<usize>,
    /// Whether to match filter text against each column independently (table mode only)
    per_column: bool,
    /// Whether settings changed since last render (for footer update)
    settings_changed: bool,
    /// Cached selected marker string (computed once, doesn't change at runtime)
    selected_marker_cached: String,
    /// Cached visible columns calculation (cols_visible, has_more_right)
    /// Invalidated when horizontal_offset, term_width, or table_layout changes
    visible_columns_cache: Option<(usize, bool)>,
}

impl<'a> SelectWidget<'a> {
    fn make_matcher() -> NucleoMatcher {
        NucleoMatcher::new({
            let mut config = NucleoConfig::DEFAULT;
            config.prefer_prefix = true;
            config
        })
    }

    fn new(
        mode: SelectMode,
        prompt: Option<&'a str>,
        items: Vec<SelectItem>,
        config: InputListConfig,
        table_layout: Option<TableLayout>,
        per_column: bool,
        stream_state: StreamState<'a>,
    ) -> Self {
        let filtered_indices: Vec<usize> = (0..items.len()).collect();
        let matcher = Self::make_matcher();
        // Pre-compute the selected marker string (doesn't change at runtime)
        let selected_marker_cached = format!(
            "{} ",
            config
                .selected_marker
                .paint(config.selected_marker_char.to_string())
        );
        let initial_item_count = items.len();
        Self {
            mode,
            prompt,
            items,
            cursor: 0,
            selected: HashSet::new(),
            filter_text: String::new(),
            filtered_indices,
            scroll_offset: 0,
            visible_height: 10,
            matcher,
            last_filter_text: String::new(),
            force_full_filter: false,
            rendered_lines: 0,
            prev_cursor: 0,
            prev_scroll_offset: 0,
            first_render: true,
            fuzzy_cursor_offset: 0,
            results_changed: true,
            filter_text_changed: false,
            toggled_item: None,
            toggled_all: false,
            filter_cursor: 0,
            config,
            term_width: 0,
            separator_line: String::new(),
            table_layout,
            horizontal_offset: 0,
            horizontal_scroll_changed: false,
            width_changed: false,
            table_layout_changed: false,
            refined: false,
            follow_stream_to_end: false,
            stream_spinner_frame: 0,
            stream_footer_item_count: initial_item_count,
            last_stream_footer_update: nu_utils::time::Instant::now(),
            refined_base_indices: Vec::new(),
            per_column,
            settings_changed: false,
            selected_marker_cached,
            stream_reader: stream_state.stream_reader,
            item_generator: stream_state.item_generator,
            visible_columns_cache: None,
        }
    }

    /// Generate the separator line based on current terminal width
    fn generate_separator_line(&mut self) {
        let sep_width = self.config.separator_char.width();
        let repeat_count = (self.term_width as usize)
            .checked_div(sep_width)
            .unwrap_or(self.term_width as usize);
        self.separator_line = self.config.separator_char.repeat(repeat_count);
    }

    /// Get the styled prompt marker string (for fuzzy mode filter line)
    fn prompt_marker(&self) -> String {
        self.config
            .prompt_marker
            .paint(&self.config.prompt_marker_text)
            .to_string()
    }

    /// Get the width of the prompt marker in characters
    fn prompt_marker_width(&self) -> usize {
        self.config.prompt_marker_text.width()
    }

    /// Position terminal cursor within the fuzzy filter text
    fn position_fuzzy_cursor(&self, stderr: &mut Stderr) -> io::Result<()> {
        let text_before_cursor = &self.filter_text[..self.filter_cursor];
        let cursor_col = self.prompt_marker_width() + text_before_cursor.width();
        execute!(stderr, MoveToColumn(cursor_col as u16))
    }

    /// Get the styled selection marker string (for active items)
    fn selected_marker(&self) -> &str {
        &self.selected_marker_cached
    }

    /// Check if we're in table mode
    fn is_table_mode(&self) -> bool {
        self.table_layout.is_some()
    }

    /// Check if we're in a multi-selection mode
    fn is_multi_mode(&self) -> bool {
        self.mode == SelectMode::Multi || self.mode == SelectMode::FuzzyMulti
    }

    /// Check if we're in a fuzzy mode
    fn is_fuzzy_mode(&self) -> bool {
        self.mode == SelectMode::Fuzzy || self.mode == SelectMode::FuzzyMulti
    }

    /// Try to convert a value into a SelectItem via the configured generator
    fn make_select_item(&mut self, value: Value) -> SelectItem {
        if let Some(r#gen) = self.item_generator.as_mut() {
            r#gen(value)
        } else {
            // Defensive fallback for test-only widget construction paths.
            // In normal command execution the generator is always present whenever streaming is
            // active, so this branch should remain cold.
            SelectItem {
                name: value.to_expanded_string(", ", &Config::default()),
                cells: None,
                value,
            }
        }
    }

    /// Load more items from upstream stream when near the end of the loaded list.
    fn load_more_items(&mut self, count: usize) -> bool {
        let Some(reader) = self.stream_reader.as_mut() else {
            return false;
        };

        let values = reader.drain_available(count);
        let stream_finished = reader.is_finished();
        self.append_streamed_values(values, stream_finished)
    }

    fn load_more_items_for(&mut self, max_duration: Duration) -> bool {
        let Some(reader) = self.stream_reader.as_mut() else {
            return false;
        };

        let values = reader.drain_available_for(max_duration);
        let stream_finished = reader.is_finished();
        self.append_streamed_values(values, stream_finished)
    }

    fn append_streamed_values(&mut self, values: Vec<Value>, stream_finished: bool) -> bool {
        if stream_finished {
            self.stream_reader = None;
            self.stream_footer_item_count = self.items.len() + values.len();
            self.settings_changed = true;
        }

        if values.is_empty() {
            if stream_finished {
                return true;
            }
            return false;
        }

        let old_filtered_indices = if self.filter_text.is_empty() && !self.refined {
            None
        } else {
            Some(self.filtered_indices.clone())
        };
        let start_index = self.items.len();
        for value in values {
            let item = self.make_select_item(value);
            self.items.push(item);
        }

        if self.items.len() > start_index {
            // Table widths may have expanded as more rows are loaded
            if self.is_table_mode()
                && let Some(layout) = &mut self.table_layout
                && InputList::update_table_layout_with_items(layout, &self.items[start_index..])
            {
                self.table_layout_changed = true;
                self.update_table_layout();
            }

            if self.filter_text.is_empty() && !self.refined {
                self.filtered_indices.extend(start_index..self.items.len());
            } else {
                self.force_full_filter = true;
                self.update_filter();
            }

            if let Some(old_filtered_indices) = old_filtered_indices {
                self.results_changed =
                    self.results_changed || old_filtered_indices != self.filtered_indices;
            }
            true
        } else {
            false
        }
    }

    /// Ensure we have enough items to show around the cursor; stream if needed.
    fn maybe_load_more(&mut self) -> bool {
        if self.stream_reader.is_none() {
            return false;
        }

        // Prefetch a little before hitting the end of loaded rows to avoid visible refill latency.
        let threshold = self.scroll_offset + self.visible_height as usize + STREAM_PREFETCH_MARGIN;
        if self.is_fuzzy_mode() && !self.filter_text.is_empty() || threshold >= self.items.len() {
            self.load_more_items(STREAM_LOAD_BATCH)
        } else {
            false
        }
    }

    /// Cycle case sensitivity: Smart -> CaseSensitive -> CaseInsensitive -> Smart
    fn toggle_case_sensitivity(&mut self) {
        self.config.case_sensitivity = match self.config.case_sensitivity {
            CaseSensitivity::Smart => CaseSensitivity::CaseSensitive,
            CaseSensitivity::CaseSensitive => CaseSensitivity::CaseInsensitive,
            CaseSensitivity::CaseInsensitive => CaseSensitivity::Smart,
        };
        self.rebuild_matcher();
        // Re-run filter with new matcher
        if !self.filter_text.is_empty() {
            self.force_full_filter = true;
            self.update_filter();
        }
        self.settings_changed = true;
    }

    /// Toggle per-column matching (only meaningful in table mode)
    fn toggle_per_column(&mut self) {
        if self.is_table_mode() {
            self.per_column = !self.per_column;
            // Re-run filter with new matching mode
            if !self.filter_text.is_empty() {
                self.force_full_filter = true;
                self.update_filter();
            }
            self.settings_changed = true;
        }
    }

    /// Reset the fuzzy matcher's scratch state after matching settings change.
    fn rebuild_matcher(&mut self) {
        self.matcher = Self::make_matcher();
    }

    /// Get the settings indicator string for the footer (fuzzy modes only)
    /// Returns empty string if not in fuzzy mode, otherwise returns " [settings]"
    fn settings_indicator(&self) -> String {
        if !self.is_fuzzy_mode() {
            return String::new();
        }

        let case_str = match self.config.case_sensitivity {
            CaseSensitivity::Smart => "smart",
            CaseSensitivity::CaseSensitive => "CASE",
            CaseSensitivity::CaseInsensitive => "nocase",
        };

        if self.is_table_mode() && self.per_column {
            format!(" [{} col]", case_str)
        } else {
            format!(" [{}]", case_str)
        }
    }

    fn stream_is_pending(&self) -> bool {
        self.stream_reader.is_some()
    }

    fn stream_spinner(&self) -> &'static str {
        STREAM_SPINNER_FRAMES[self.stream_spinner_frame % STREAM_SPINNER_FRAMES.len()]
    }

    fn update_stream_footer(&mut self) {
        if !self.stream_is_pending() {
            return;
        }

        if self.last_stream_footer_update.elapsed() >= STREAM_FOOTER_UPDATE_INTERVAL {
            self.stream_spinner_frame =
                (self.stream_spinner_frame + 1) % STREAM_SPINNER_FRAMES.len();
            self.stream_footer_item_count = self.items.len();
            self.last_stream_footer_update = nu_utils::time::Instant::now();
            self.settings_changed = true;
        }
    }

    /// Generate the footer string, truncating if necessary to fit terminal width
    fn generate_footer(&self) -> String {
        let total_count = self.current_list_len();
        let end = (self.scroll_offset + self.visible_height as usize).min(total_count);
        let settings = self.settings_indicator();
        let stream_is_pending = self.stream_is_pending();
        let count_text = if stream_is_pending {
            format!(
                "{} {}",
                self.stream_footer_item_count,
                self.stream_spinner()
            )
        } else {
            total_count.to_string()
        };

        let position_part = if self.is_multi_mode() {
            format!(
                "[{}-{} of {}, {} selected]",
                self.scroll_offset + 1,
                end.min(total_count),
                count_text,
                self.selected.len()
            )
        } else {
            format!(
                "[{}-{} of {}]",
                self.scroll_offset + 1,
                end.min(total_count),
                count_text
            )
        };

        let full_footer = format!("{}{}", position_part, settings);

        // Truncate if footer exceeds terminal width
        let max_width = self.term_width as usize;
        if full_footer.width() <= max_width {
            full_footer
        } else if max_width <= 3 {
            // Too narrow, just show ellipsis
            "…".to_string()
        } else {
            // Try to fit position part + truncated settings, or just position part
            if position_part.width() <= max_width {
                // Position fits, truncate or drop settings
                let remaining = max_width - position_part.width();
                if remaining <= 4 {
                    // Not enough room for meaningful settings, just show position
                    position_part
                } else {
                    // Truncate settings portion
                    let target_width = remaining - 2; // Reserve space for "…]"
                    let mut current_width = 0;
                    let mut end_pos = 0;

                    // Skip the leading " [" in settings
                    for (byte_pos, c) in settings.char_indices().skip(2) {
                        if c == ']' {
                            break;
                        }
                        let char_width = UnicodeWidthChar::width(c).unwrap_or(0);
                        if current_width + char_width > target_width {
                            break;
                        }
                        end_pos = byte_pos + c.len_utf8();
                        current_width += char_width;
                    }
                    if end_pos > 2 {
                        format!("{} [{}…]", position_part, &settings[2..end_pos])
                    } else {
                        position_part
                    }
                }
            } else {
                // Even position part doesn't fit, truncate it
                let target_width = max_width - 2; // Reserve space for "…]"
                let mut current_width = 0;
                let mut end_pos = 0;

                for (byte_pos, c) in position_part.char_indices() {
                    if c == ']' {
                        break;
                    }
                    let char_width = UnicodeWidthChar::width(c).unwrap_or(0);
                    if current_width + char_width > target_width {
                        break;
                    }
                    end_pos = byte_pos + c.len_utf8();
                    current_width += char_width;
                }
                format!("{}…]", &position_part[..end_pos])
            }
        }
    }

    /// Check if footer should be shown
    /// Footer is always shown in fuzzy modes (for settings display), multi modes (for selection
    /// count), or when the list fills the item area reserved above the footer.
    fn has_footer(&self) -> bool {
        self.config.show_footer
            && (self.is_fuzzy_mode()
                || self.is_multi_mode()
                || self.current_list_len() >= self.visible_height as usize
                || self.stream_is_pending())
    }

    /// Render just the footer text at current cursor position (for optimized updates)
    fn render_footer_inline(&self, stderr: &mut Stderr) -> io::Result<()> {
        let indicator = self.generate_footer();
        execute!(
            stderr,
            MoveToColumn(0),
            Print(self.config.footer.paint(&indicator)),
            Clear(ClearType::UntilNewLine),
        )
    }

    /// Get the row prefix width (selection marker + optional checkbox)
    fn row_prefix_width(&self) -> usize {
        match self.mode {
            SelectMode::Multi | SelectMode::FuzzyMulti => 6, // "> [x] " or "  [ ] "
            _ => 2,                                          // "> " or "  "
        }
    }

    /// Get the table column separator string (e.g., " │ ")
    fn table_column_separator(&self) -> String {
        format!(" {} ", self.config.table_column_separator)
    }

    /// Get the width of the table column separator (char width + 2 for surrounding spaces)
    fn table_column_separator_width(&self) -> usize {
        UnicodeWidthChar::width(self.config.table_column_separator).unwrap_or(1) + 2
    }

    /// Calculate how many columns fit starting from horizontal_offset
    /// Returns (number of columns that fit, whether there are more columns to the right)
    /// Uses cached value if available (cache is updated by update_table_layout)
    fn calculate_visible_columns(&self) -> (usize, bool) {
        // Use cache if available (populated by update_table_layout)
        if let Some(cached) = self.visible_columns_cache {
            return cached;
        }

        // Fallback to computation (should rarely happen after first render)
        let Some(layout) = &self.table_layout else {
            return (0, false);
        };

        Self::calculate_visible_columns_for_layout(
            layout,
            self.horizontal_offset,
            self.term_width as usize,
            self.row_prefix_width(),
            self.table_column_separator_width(),
        )
    }

    /// Static helper to calculate visible columns without borrowing self
    fn calculate_visible_columns_for_layout(
        layout: &TableLayout,
        horizontal_offset: usize,
        term_width: usize,
        prefix_width: usize,
        separator_width: usize,
    ) -> (usize, bool) {
        // Account for scroll indicators: "… │ " on left (1 + separator_width)
        let scroll_indicator_width = if horizontal_offset > 0 {
            1 + separator_width
        } else {
            0
        };
        let available = term_width
            .saturating_sub(prefix_width)
            .saturating_sub(scroll_indicator_width);

        let mut used_width = 0;
        let mut cols_fit = 0;

        for (i, &col_width) in layout.col_widths.iter().enumerate().skip(horizontal_offset) {
            // Add separator width for all but first visible column
            let sep_width = if i > horizontal_offset {
                separator_width
            } else {
                0
            };
            let needed = col_width + sep_width;

            // Reserve space for right scroll indicator if not the last column: " │ …" (separator_width + 1)
            let reserve_right = if i + 1 < layout.col_widths.len() {
                separator_width + 1
            } else {
                0
            };

            if used_width + needed + reserve_right <= available {
                used_width += needed;
                cols_fit += 1;
            } else {
                break;
            }
        }

        let has_more_right = horizontal_offset + cols_fit < layout.col_widths.len();
        (cols_fit.max(1), has_more_right) // Always show at least 1 column
    }

    /// Update table layout's truncated_cols based on current terminal width
    /// Also updates the visible_columns_cache
    fn update_table_layout(&mut self) {
        let prefix_width = self.row_prefix_width();
        let term_width = self.term_width as usize;
        let horizontal_offset = self.horizontal_offset;
        let separator_width = self.table_column_separator_width();

        if let Some(layout) = &mut self.table_layout {
            let result = Self::calculate_visible_columns_for_layout(
                layout,
                horizontal_offset,
                term_width,
                prefix_width,
                separator_width,
            );
            layout.truncated_cols = result.0;
            self.visible_columns_cache = Some(result);
        } else {
            self.visible_columns_cache = Some((0, false));
        }
    }

    /// Header lines for fuzzy modes (prompt + filter + separator + table header)
    fn fuzzy_header_lines(&self) -> u16 {
        let mut header_lines: u16 = if self.prompt.is_some() { 2 } else { 1 };
        if self.config.show_separator {
            header_lines += 1;
        }
        if self.is_table_mode() {
            header_lines += 2;
        }
        header_lines
    }

    /// Filter line row index for fuzzy modes
    fn fuzzy_filter_row(&self) -> u16 {
        if self.prompt.is_some() { 1 } else { 0 }
    }

    /// Update terminal dimensions and recalculate visible height
    fn update_term_size(&mut self, width: u16, height: u16) {
        // Subtract 1 to avoid issues with writing to the very last terminal column
        let new_width = width.saturating_sub(1);
        let width_changed = self.term_width != new_width;
        self.term_width = new_width;

        // Track width change for full redraw
        if width_changed {
            self.width_changed = true;
        }

        // Regenerate separator line if width changed
        if width_changed && self.config.show_separator {
            self.generate_separator_line();
        }

        // Update table layout if width changed
        if width_changed {
            self.update_table_layout();
        }

        // Recalculate visible height
        let mut reserved: u16 = if self.prompt.is_some() { 1 } else { 0 };
        if self.mode == SelectMode::Fuzzy || self.mode == SelectMode::FuzzyMulti {
            reserved += 1; // filter line
            if self.config.show_separator {
                reserved += 1; // separator line
            }
        }
        if self.is_table_mode() {
            reserved += 2; // table header + header separator
        }
        if self.config.show_footer {
            reserved += 1; // footer
        }
        self.visible_height = height.saturating_sub(reserved).max(1);
    }

    fn run(&mut self) -> io::Result<InteractMode> {
        let mut stderr = io::stderr();

        enable_raw_mode().map_err(io_context("enable raw mode"))?;
        scopeguard::defer! {
            let _ = disable_raw_mode();
        }

        // Only hide cursor for non-fuzzy modes (fuzzy modes need visible cursor for text input)
        if self.mode != SelectMode::Fuzzy && self.mode != SelectMode::FuzzyMulti {
            execute!(stderr, Hide).map_err(io_context("hide terminal cursor"))?;
        }
        scopeguard::defer! {
            let _ = execute!(io::stderr(), Show);
        }

        // Get initial terminal size and cache it
        let (term_width, term_height) =
            terminal::size().map_err(io_context("read terminal size"))?;
        self.update_term_size(term_width, term_height);

        self.render(&mut stderr)
            .map_err(io_context("render input list"))?;

        loop {
            let poll_interval = if self.stream_is_pending() {
                STREAM_POLL_INTERVAL
            } else {
                IDLE_POLL_INTERVAL
            };
            let has_event =
                event::poll(poll_interval).map_err(io_context("poll terminal event"))?;

            if has_event {
                match event::read().map_err(io_context("read terminal event"))? {
                    Event::Key(key_event) => {
                        match self.handle_key(key_event) {
                            KeyAction::Continue => {}
                            KeyAction::Cancel => {
                                self.clear_display(&mut stderr)
                                    .map_err(io_context("clear input list after cancel"))?;
                                return Ok(match self.mode {
                                    SelectMode::Multi => InteractMode::Multi(None),
                                    _ => InteractMode::Single(None),
                                });
                            }
                            KeyAction::Confirm => {
                                self.clear_display(&mut stderr)
                                    .map_err(io_context("clear input list after confirm"))?;
                                return Ok(self.get_result());
                            }
                        }
                        self.render(&mut stderr)
                            .map_err(io_context("render input list after key event"))?;
                    }
                    Event::Resize(width, height) => {
                        // Clear old content first - terminal reflow may have corrupted positions
                        self.clear_display(&mut stderr)
                            .map_err(io_context("clear input list after resize"))?;
                        self.update_term_size(width, height);
                        // Force full redraw on resize
                        self.first_render = true;
                        self.render(&mut stderr)
                            .map_err(io_context("render input list after resize"))?;
                    }
                    _ => {}
                }
            } else if self.stream_is_pending() {
                self.render(&mut stderr)
                    .map_err(io_context("render input list after stream update"))?;
            }
        }
    }

    fn handle_key(&mut self, key: KeyEvent) -> KeyAction {
        // Only handle key press and repeat events, not release
        // This is important on Windows where crossterm sends press, repeat, and release events
        // We need Repeat events for key repeat to work when holding down a key on Windows
        if key.kind == KeyEventKind::Release {
            return KeyAction::Continue;
        }

        // Ctrl+C always cancels
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            return KeyAction::Cancel;
        }

        match self.mode {
            SelectMode::Single => self.handle_single_key(key),
            SelectMode::Multi => self.handle_multi_key(key),
            SelectMode::Fuzzy => self.handle_fuzzy_key(key),
            SelectMode::FuzzyMulti => self.handle_fuzzy_multi_key(key),
        }
    }

    fn handle_single_key(&mut self, key: KeyEvent) -> KeyAction {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);

        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => KeyAction::Cancel,
            KeyCode::Enter => KeyAction::Confirm,
            KeyCode::Char('p' | 'P') if ctrl => {
                self.navigate_up();
                KeyAction::Continue
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.navigate_up();
                KeyAction::Continue
            }
            KeyCode::Char('n' | 'N') if ctrl => {
                self.navigate_down();
                KeyAction::Continue
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.navigate_down();
                KeyAction::Continue
            }
            KeyCode::Left | KeyCode::Char('h') => {
                self.scroll_columns_left();
                KeyAction::Continue
            }
            KeyCode::Right | KeyCode::Char('l') => {
                self.scroll_columns_right();
                KeyAction::Continue
            }
            KeyCode::Home => {
                self.navigate_home();
                KeyAction::Continue
            }
            KeyCode::End => {
                self.navigate_end();
                KeyAction::Continue
            }
            KeyCode::PageUp => {
                self.navigate_page_up();
                KeyAction::Continue
            }
            KeyCode::PageDown => {
                self.navigate_page_down();
                KeyAction::Continue
            }
            KeyCode::Tab => {
                self.navigate_down();
                KeyAction::Continue
            }
            KeyCode::BackTab => {
                self.navigate_up();
                KeyAction::Continue
            }
            _ => KeyAction::Continue,
        }
    }

    fn handle_multi_key(&mut self, key: KeyEvent) -> KeyAction {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);

        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => KeyAction::Cancel,
            KeyCode::Enter => KeyAction::Confirm,
            // Ctrl+R: Refine list to only show selected items
            KeyCode::Char('r' | 'R') if ctrl => {
                self.refine_list();
                KeyAction::Continue
            }
            KeyCode::Char('p' | 'P') if ctrl => {
                self.navigate_up();
                KeyAction::Continue
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.navigate_up();
                KeyAction::Continue
            }
            KeyCode::Char('n' | 'N') if ctrl => {
                self.navigate_down();
                KeyAction::Continue
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.navigate_down();
                KeyAction::Continue
            }
            KeyCode::Left | KeyCode::Char('h') => {
                self.scroll_columns_left();
                KeyAction::Continue
            }
            KeyCode::Right | KeyCode::Char('l') => {
                self.scroll_columns_right();
                KeyAction::Continue
            }
            KeyCode::Char(' ') => {
                self.toggle_current();
                KeyAction::Continue
            }
            KeyCode::Char('a') => {
                self.toggle_all();
                KeyAction::Continue
            }
            KeyCode::Home => {
                self.navigate_home();
                KeyAction::Continue
            }
            KeyCode::End => {
                self.navigate_end();
                KeyAction::Continue
            }
            KeyCode::PageUp => {
                self.navigate_page_up();
                KeyAction::Continue
            }
            KeyCode::PageDown => {
                self.navigate_page_down();
                KeyAction::Continue
            }
            KeyCode::Tab => {
                self.toggle_current();
                self.navigate_down();
                KeyAction::Continue
            }
            KeyCode::BackTab => {
                self.navigate_up();
                self.toggle_current();
                KeyAction::Continue
            }
            _ => KeyAction::Continue,
        }
    }

    fn handle_fuzzy_key(&mut self, key: KeyEvent) -> KeyAction {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        let alt = key.modifiers.contains(KeyModifiers::ALT);
        let shift = key.modifiers.contains(KeyModifiers::SHIFT);

        match key.code {
            KeyCode::Esc => KeyAction::Cancel,
            KeyCode::Enter => KeyAction::Confirm,

            // Tab: navigate down (mirrors single/multi mode behavior)
            KeyCode::Tab | KeyCode::Char('\t') => {
                self.navigate_down();
                KeyAction::Continue
            }
            KeyCode::BackTab => {
                self.navigate_up();
                KeyAction::Continue
            }

            // List navigation
            KeyCode::Up | KeyCode::Char('p' | 'P') if ctrl => {
                self.navigate_up();
                KeyAction::Continue
            }
            KeyCode::Down | KeyCode::Char('n' | 'N') if ctrl => {
                self.navigate_down();
                KeyAction::Continue
            }
            KeyCode::Up => {
                self.navigate_up();
                KeyAction::Continue
            }
            KeyCode::Down => {
                self.navigate_down();
                KeyAction::Continue
            }

            // Horizontal scrolling for table mode (Shift+Left/Right)
            KeyCode::Left if shift => {
                self.scroll_columns_left();
                KeyAction::Continue
            }
            KeyCode::Right if shift => {
                self.scroll_columns_right();
                KeyAction::Continue
            }

            // Readline: Cursor movement
            KeyCode::Char('a' | 'A') if ctrl => {
                // Ctrl-A: Move to beginning of line
                self.filter_cursor = 0;
                self.filter_text_changed = true;
                KeyAction::Continue
            }
            KeyCode::Char('e' | 'E') if ctrl => {
                // Ctrl-E: Move to end of line
                self.filter_cursor = self.filter_text.len();
                self.filter_text_changed = true;
                KeyAction::Continue
            }
            KeyCode::Char('b' | 'B') if ctrl => {
                // Ctrl-B: Move back one character
                self.move_filter_cursor_left();
                self.filter_text_changed = true;
                KeyAction::Continue
            }
            KeyCode::Char('f' | 'F') if ctrl => {
                // Ctrl-F: Move forward one character
                self.move_filter_cursor_right();
                self.filter_text_changed = true;
                KeyAction::Continue
            }
            KeyCode::Char('b' | 'B') if alt => {
                // Alt-B: Move back one word
                self.move_filter_cursor_word_left();
                self.filter_text_changed = true;
                KeyAction::Continue
            }
            KeyCode::Char('f' | 'F') if alt => {
                // Alt-F: Move forward one word
                self.move_filter_cursor_word_right();
                self.filter_text_changed = true;
                KeyAction::Continue
            }
            // Settings toggles
            KeyCode::Char('c' | 'C') if alt => {
                // Alt-C: Toggle case sensitivity
                self.toggle_case_sensitivity();
                KeyAction::Continue
            }
            KeyCode::Char('p' | 'P') if alt => {
                // Alt-P: Toggle per-column matching (table mode only)
                self.toggle_per_column();
                KeyAction::Continue
            }
            KeyCode::Left if ctrl || alt => {
                // Ctrl/Alt-Left: Move back one word
                self.move_filter_cursor_word_left();
                self.filter_text_changed = true;
                KeyAction::Continue
            }
            KeyCode::Right if ctrl || alt => {
                // Ctrl/Alt-Right: Move forward one word
                self.move_filter_cursor_word_right();
                self.filter_text_changed = true;
                KeyAction::Continue
            }
            KeyCode::Left => {
                self.move_filter_cursor_left();
                self.filter_text_changed = true;
                KeyAction::Continue
            }
            KeyCode::Right => {
                self.move_filter_cursor_right();
                self.filter_text_changed = true;
                KeyAction::Continue
            }

            // Readline: Deletion
            KeyCode::Char('u' | 'U') if ctrl => {
                // Ctrl-U: Kill to beginning of line
                self.filter_text.drain(..self.filter_cursor);
                self.filter_cursor = 0;
                self.update_filter();
                KeyAction::Continue
            }
            KeyCode::Char('k' | 'K') if ctrl => {
                // Ctrl-K: Kill to end of line
                self.filter_text.truncate(self.filter_cursor);
                self.update_filter();
                KeyAction::Continue
            }
            KeyCode::Char('d' | 'D') if ctrl => {
                // Ctrl-D: Delete character at cursor
                if self.filter_cursor < self.filter_text.len() {
                    self.filter_text.remove(self.filter_cursor);
                    self.update_filter();
                }
                KeyAction::Continue
            }
            KeyCode::Delete => {
                // Delete: Delete character at cursor
                if self.filter_cursor < self.filter_text.len() {
                    self.filter_text.remove(self.filter_cursor);
                    self.update_filter();
                }
                KeyAction::Continue
            }
            KeyCode::Char('d' | 'D') if alt => {
                // Alt-D: Delete word forward
                self.delete_word_forwards();
                self.update_filter();
                KeyAction::Continue
            }
            // Ctrl-W or Ctrl-H (Ctrl-Backspace) to delete previous word
            KeyCode::Char('w' | 'W' | 'h' | 'H') if ctrl => {
                self.delete_word_backwards();
                self.update_filter();
                KeyAction::Continue
            }
            // Alt-Backspace: delete previous word
            KeyCode::Backspace if alt => {
                self.delete_word_backwards();
                self.update_filter();
                KeyAction::Continue
            }
            KeyCode::Backspace => {
                // Delete character before cursor (handle UTF-8)
                if self.filter_cursor > 0 {
                    // Find previous char boundary
                    let mut new_pos = self.filter_cursor - 1;
                    while new_pos > 0 && !self.filter_text.is_char_boundary(new_pos) {
                        new_pos -= 1;
                    }
                    self.filter_cursor = new_pos;
                    self.filter_text.remove(self.filter_cursor);
                    self.update_filter();
                }
                KeyAction::Continue
            }
            // Ctrl-T: Transpose characters
            KeyCode::Char('t' | 'T') if ctrl => {
                let old_text = self.filter_text.clone();
                self.transpose_chars();
                if self.filter_text != old_text {
                    self.update_filter();
                }
                KeyAction::Continue
            }

            // Character input
            KeyCode::Char(c) => {
                self.filter_text.insert(self.filter_cursor, c);
                self.filter_cursor += c.len_utf8();
                self.update_filter();
                KeyAction::Continue
            }

            // List navigation with Home/End/PageUp/PageDown
            KeyCode::Home => {
                self.navigate_home();
                KeyAction::Continue
            }
            KeyCode::End => {
                self.navigate_end();
                KeyAction::Continue
            }
            KeyCode::PageUp => {
                self.navigate_page_up();
                KeyAction::Continue
            }
            KeyCode::PageDown => {
                self.navigate_page_down();
                KeyAction::Continue
            }
            _ => KeyAction::Continue,
        }
    }

    fn handle_fuzzy_multi_key(&mut self, key: KeyEvent) -> KeyAction {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        let alt = key.modifiers.contains(KeyModifiers::ALT);
        let shift = key.modifiers.contains(KeyModifiers::SHIFT);

        match key.code {
            KeyCode::Esc => KeyAction::Cancel,
            KeyCode::Enter => KeyAction::Confirm,

            // Ctrl+R: Refine list to only show selected items
            KeyCode::Char('r' | 'R') if ctrl => {
                self.refine_list();
                KeyAction::Continue
            }

            // Tab: Toggle selection of current item and move down
            // Note: Some terminals may report Tab as Char('\t')
            KeyCode::Tab | KeyCode::Char('\t') => {
                self.toggle_current_fuzzy();
                self.navigate_down();
                KeyAction::Continue
            }

            // Shift-Tab: Toggle selection and move up
            KeyCode::BackTab => {
                self.navigate_up();
                self.toggle_current_fuzzy();
                KeyAction::Continue
            }

            // List navigation
            KeyCode::Up | KeyCode::Char('p' | 'P') if ctrl => {
                self.navigate_up();
                KeyAction::Continue
            }
            KeyCode::Down | KeyCode::Char('n' | 'N') if ctrl => {
                self.navigate_down();
                KeyAction::Continue
            }
            KeyCode::Up => {
                self.navigate_up();
                KeyAction::Continue
            }
            KeyCode::Down => {
                self.navigate_down();
                KeyAction::Continue
            }

            // Horizontal scrolling for table mode (Shift+Left/Right)
            KeyCode::Left if shift => {
                self.scroll_columns_left();
                KeyAction::Continue
            }
            KeyCode::Right if shift => {
                self.scroll_columns_right();
                KeyAction::Continue
            }

            // Readline: Cursor movement
            KeyCode::Char('a' | 'A') if ctrl => {
                self.filter_cursor = 0;
                self.filter_text_changed = true;
                KeyAction::Continue
            }
            KeyCode::Char('e' | 'E') if ctrl => {
                self.filter_cursor = self.filter_text.len();
                self.filter_text_changed = true;
                KeyAction::Continue
            }
            KeyCode::Char('b' | 'B') if ctrl => {
                self.move_filter_cursor_left();
                self.filter_text_changed = true;
                KeyAction::Continue
            }
            KeyCode::Char('f' | 'F') if ctrl => {
                self.move_filter_cursor_right();
                self.filter_text_changed = true;
                KeyAction::Continue
            }
            KeyCode::Char('b' | 'B') if alt => {
                self.move_filter_cursor_word_left();
                self.filter_text_changed = true;
                KeyAction::Continue
            }
            KeyCode::Char('f' | 'F') if alt => {
                self.move_filter_cursor_word_right();
                self.filter_text_changed = true;
                KeyAction::Continue
            }
            // Settings toggles
            KeyCode::Char('c' | 'C') if alt => {
                // Alt-C: Toggle case sensitivity
                self.toggle_case_sensitivity();
                KeyAction::Continue
            }
            KeyCode::Char('p' | 'P') if alt => {
                // Alt-P: Toggle per-column matching (table mode only)
                self.toggle_per_column();
                KeyAction::Continue
            }
            KeyCode::Left if ctrl || alt => {
                self.move_filter_cursor_word_left();
                self.filter_text_changed = true;
                KeyAction::Continue
            }
            KeyCode::Right if ctrl || alt => {
                self.move_filter_cursor_word_right();
                self.filter_text_changed = true;
                KeyAction::Continue
            }
            KeyCode::Left => {
                self.move_filter_cursor_left();
                self.filter_text_changed = true;
                KeyAction::Continue
            }
            KeyCode::Right => {
                self.move_filter_cursor_right();
                self.filter_text_changed = true;
                KeyAction::Continue
            }

            // Readline: Deletion
            KeyCode::Char('u' | 'U') if ctrl => {
                self.filter_text.drain(..self.filter_cursor);
                self.filter_cursor = 0;
                self.update_filter();
                KeyAction::Continue
            }
            KeyCode::Char('k' | 'K') if ctrl => {
                self.filter_text.truncate(self.filter_cursor);
                self.update_filter();
                KeyAction::Continue
            }
            KeyCode::Char('d' | 'D') if ctrl => {
                if self.filter_cursor < self.filter_text.len() {
                    self.filter_text.remove(self.filter_cursor);
                    self.update_filter();
                }
                KeyAction::Continue
            }
            KeyCode::Delete => {
                if self.filter_cursor < self.filter_text.len() {
                    self.filter_text.remove(self.filter_cursor);
                    self.update_filter();
                }
                KeyAction::Continue
            }
            KeyCode::Char('d' | 'D') if alt => {
                self.delete_word_forwards();
                self.update_filter();
                KeyAction::Continue
            }
            KeyCode::Char('w' | 'W' | 'h' | 'H') if ctrl => {
                self.delete_word_backwards();
                self.update_filter();
                KeyAction::Continue
            }
            KeyCode::Backspace if alt => {
                self.delete_word_backwards();
                self.update_filter();
                KeyAction::Continue
            }
            KeyCode::Backspace => {
                if self.filter_cursor > 0 {
                    let mut new_pos = self.filter_cursor - 1;
                    while new_pos > 0 && !self.filter_text.is_char_boundary(new_pos) {
                        new_pos -= 1;
                    }
                    self.filter_cursor = new_pos;
                    self.filter_text.remove(self.filter_cursor);
                    self.update_filter();
                }
                KeyAction::Continue
            }
            KeyCode::Char('t' | 'T') if ctrl => {
                let old_text = self.filter_text.clone();
                self.transpose_chars();
                if self.filter_text != old_text {
                    self.update_filter();
                }
                KeyAction::Continue
            }

            // Alt-A: Toggle all filtered items in fuzzy multi mode
            KeyCode::Char('a' | 'A') if alt => {
                self.toggle_all_fuzzy();
                KeyAction::Continue
            }

            // Character input
            KeyCode::Char(c) => {
                self.filter_text.insert(self.filter_cursor, c);
                self.filter_cursor += c.len_utf8();
                self.update_filter();
                KeyAction::Continue
            }

            // List navigation with Home/End/PageUp/PageDown
            KeyCode::Home => {
                self.navigate_home();
                KeyAction::Continue
            }
            KeyCode::End => {
                self.navigate_end();
                KeyAction::Continue
            }
            KeyCode::PageUp => {
                self.navigate_page_up();
                KeyAction::Continue
            }
            KeyCode::PageDown => {
                self.navigate_page_down();
                KeyAction::Continue
            }
            _ => KeyAction::Continue,
        }
    }

    /// Move cursor up with wrapping
    fn navigate_up(&mut self) {
        self.follow_stream_to_end = false;
        let list_len = self.current_list_len();
        if self.cursor > 0 {
            self.cursor -= 1;
            self.adjust_scroll_up();
        } else if list_len > 0 {
            self.maybe_load_more();
            let list_len = self.current_list_len();
            self.cursor = list_len.saturating_sub(1);
            self.adjust_scroll_down();
        }
    }

    /// Move cursor down with wrapping
    fn navigate_down(&mut self) {
        self.follow_stream_to_end = false;
        self.maybe_load_more();

        let list_len = self.current_list_len();
        if self.cursor + 1 < list_len {
            self.cursor += 1;
            self.adjust_scroll_down();
        } else {
            // If we still have a pending stream, attempt to load more and stay in place
            if self.stream_reader.is_some() {
                self.load_more_items(STREAM_LOAD_BATCH);
                let list_len = self.current_list_len();
                if self.cursor + 1 < list_len {
                    self.cursor += 1;
                    self.adjust_scroll_down();
                    return;
                }
            }

            // Wrap to top
            self.cursor = 0;
            self.scroll_offset = 0;
        }
    }

    fn adjust_scroll_down(&mut self) {
        let max_visible = self.scroll_offset + self.visible_height as usize;
        if self.cursor >= max_visible {
            self.scroll_offset = self.cursor - self.visible_height as usize + 1;
        }
    }

    fn adjust_scroll_up(&mut self) {
        if self.cursor < self.scroll_offset {
            self.scroll_offset = self.cursor;
        }
    }

    /// Get the current list length (filtered for fuzzy modes or refined multi, full for others)
    fn current_list_len(&self) -> usize {
        match self.mode {
            SelectMode::Fuzzy | SelectMode::FuzzyMulti => self.filtered_indices.len(),
            SelectMode::Multi if self.refined => self.filtered_indices.len(),
            _ => self.items.len(),
        }
    }

    /// Navigate to the start of the list
    fn navigate_home(&mut self) {
        self.follow_stream_to_end = false;
        self.cursor = 0;
        self.scroll_offset = 0;
    }

    /// Navigate to the end of the list
    fn navigate_end(&mut self) {
        self.follow_stream_to_end = true;
        self.load_more_items(STREAM_CHANNEL_CAPACITY);
        self.cursor = self.current_list_len().saturating_sub(1);
        self.adjust_scroll_down();
    }

    /// Navigate page up: go to top of current page, or previous page if already at top
    fn navigate_page_up(&mut self) {
        self.follow_stream_to_end = false;
        let page_top = self.scroll_offset;
        if self.cursor == page_top {
            // Already at top of page, go to previous page
            self.cursor = self.cursor.saturating_sub(self.visible_height as usize);
            self.adjust_scroll_up();
        } else {
            // Go to top of current page
            self.cursor = page_top;
        }
    }

    /// Navigate page down: go to bottom of current page, or next page if already at bottom
    fn navigate_page_down(&mut self) {
        self.follow_stream_to_end = false;
        self.maybe_load_more();

        let list_len = self.current_list_len();
        let page_bottom =
            (self.scroll_offset + self.visible_height as usize - 1).min(list_len.saturating_sub(1));
        if self.cursor == page_bottom {
            // Already at bottom of page, go to next page
            self.cursor =
                (self.cursor + self.visible_height as usize).min(list_len.saturating_sub(1));
            self.adjust_scroll_down();
        } else {
            // Go to bottom of current page
            self.cursor = page_bottom;
        }

        self.maybe_load_more();
    }

    /// Scroll table columns left (show earlier columns)
    fn scroll_columns_left(&mut self) -> bool {
        if !self.is_table_mode() || self.horizontal_offset == 0 {
            return false;
        }
        self.horizontal_offset -= 1;
        self.horizontal_scroll_changed = true;
        self.update_table_layout();
        true
    }

    /// Scroll table columns right (show later columns)
    fn scroll_columns_right(&mut self) -> bool {
        let Some(layout) = &self.table_layout else {
            return false;
        };
        let (cols_visible, has_more_right) = self.calculate_visible_columns();
        if !has_more_right {
            return false;
        }
        // Don't scroll past the last column
        if self.horizontal_offset + cols_visible >= layout.col_widths.len() {
            return false;
        }
        self.horizontal_offset += 1;
        self.horizontal_scroll_changed = true;
        self.update_table_layout();
        true
    }

    fn toggle_current(&mut self) {
        // Guard against empty list when refined
        if self.refined && self.filtered_indices.is_empty() {
            return;
        }
        // Get the real item index (may differ from cursor when refined)
        let real_idx = if self.refined {
            self.filtered_indices[self.cursor]
        } else {
            self.cursor
        };
        self.toggle_index(real_idx);
    }

    /// Toggle selection of a specific item by its real index
    fn toggle_index(&mut self, real_idx: usize) {
        if self.selected.contains(&real_idx) {
            self.selected.remove(&real_idx);
        } else {
            self.selected.insert(real_idx);
        }
        self.toggled_item = Some(self.cursor);
    }

    /// Toggle selection of current item in fuzzy multi mode (uses filtered_indices)
    /// Returns true if an item was toggled, false if list was empty
    fn toggle_current_fuzzy(&mut self) -> bool {
        if self.filtered_indices.is_empty() {
            return false;
        }
        let real_idx = self.filtered_indices[self.cursor];
        self.toggle_index(real_idx);
        true
    }

    fn toggle_all(&mut self) {
        // Check if all current items are selected
        let all_selected = if self.refined {
            self.filtered_indices
                .iter()
                .all(|i| self.selected.contains(i))
        } else {
            (0..self.items.len()).all(|i| self.selected.contains(&i))
        };

        if all_selected {
            // Deselect all current items
            if self.refined {
                for i in &self.filtered_indices {
                    self.selected.remove(i);
                }
            } else {
                self.selected.clear();
            }
        } else {
            // Select all current items
            if self.refined {
                self.selected.extend(self.filtered_indices.iter().copied());
            } else {
                self.selected.extend(0..self.items.len());
            }
        }
        self.toggled_all = true;
    }

    /// Toggle all items in fuzzy multi mode (only the currently filtered items)
    fn toggle_all_fuzzy(&mut self) {
        if self.filtered_indices.is_empty() {
            return;
        }

        // Check if all filtered items are selected
        let all_selected = self
            .filtered_indices
            .iter()
            .all(|i| self.selected.contains(i));

        if all_selected {
            // Deselect all filtered items
            for i in &self.filtered_indices {
                self.selected.remove(i);
            }
        } else {
            // Select all filtered items
            self.selected.extend(self.filtered_indices.iter().copied());
        }
        self.toggled_all = true;
    }

    /// Refine the list to only show currently selected items
    /// This allows users to narrow down to their selections and continue selecting
    fn refine_list(&mut self) {
        if self.selected.is_empty() {
            return;
        }

        // Set filtered_indices to sorted selected indices
        let mut indices: Vec<usize> = self.selected.iter().copied().collect();
        indices.sort();

        // Store as base indices for filtering in FuzzyMulti mode
        // Clone once for both vectors instead of cloning refined_base_indices
        self.filtered_indices = indices.clone();
        self.refined_base_indices = indices;

        // Reset cursor and scroll
        self.cursor = 0;
        self.scroll_offset = 0;

        // Keep all items selected (don't clear selection)
        // User can deselect items they don't want

        // Clear filter text in FuzzyMulti mode
        if self.mode == SelectMode::FuzzyMulti {
            self.filter_text.clear();
            self.filter_cursor = 0;
            self.last_filter_text.clear();
            self.force_full_filter = true;
            self.filter_text_changed = true;
        }

        // Mark as refined (for Multi mode rendering)
        self.refined = true;

        // Force full redraw
        self.first_render = true;
    }

    // Filter cursor movement helpers
    fn move_filter_cursor_left(&mut self) {
        if self.filter_cursor > 0 {
            // Move back one character (handle UTF-8)
            let mut new_pos = self.filter_cursor - 1;
            while new_pos > 0 && !self.filter_text.is_char_boundary(new_pos) {
                new_pos -= 1;
            }
            self.filter_cursor = new_pos;
        }
    }

    fn move_filter_cursor_right(&mut self) {
        if self.filter_cursor < self.filter_text.len() {
            // Move forward one character (handle UTF-8)
            let mut new_pos = self.filter_cursor + 1;
            while new_pos < self.filter_text.len() && !self.filter_text.is_char_boundary(new_pos) {
                new_pos += 1;
            }
            self.filter_cursor = new_pos;
        }
    }

    fn move_filter_cursor_word_left(&mut self) {
        if self.filter_cursor == 0 {
            return;
        }
        let bytes = self.filter_text.as_bytes();
        let mut pos = self.filter_cursor;
        // Skip whitespace
        while pos > 0 && bytes[pos - 1].is_ascii_whitespace() {
            pos -= 1;
        }
        // Skip word characters
        while pos > 0 && !bytes[pos - 1].is_ascii_whitespace() {
            pos -= 1;
        }
        self.filter_cursor = pos;
    }

    fn move_filter_cursor_word_right(&mut self) {
        let len = self.filter_text.len();
        if self.filter_cursor >= len {
            return;
        }
        let bytes = self.filter_text.as_bytes();
        let mut pos = self.filter_cursor;
        // Skip current word characters
        while pos < len && !bytes[pos].is_ascii_whitespace() {
            pos += 1;
        }
        // Skip whitespace
        while pos < len && bytes[pos].is_ascii_whitespace() {
            pos += 1;
        }
        self.filter_cursor = pos;
    }

    fn delete_word_backwards(&mut self) {
        if self.filter_cursor == 0 {
            return;
        }
        let start = self.filter_cursor;
        // Skip whitespace
        while self.filter_cursor > 0
            && self.filter_text.as_bytes()[self.filter_cursor - 1].is_ascii_whitespace()
        {
            self.filter_cursor -= 1;
        }
        // Skip word characters
        while self.filter_cursor > 0
            && !self.filter_text.as_bytes()[self.filter_cursor - 1].is_ascii_whitespace()
        {
            self.filter_cursor -= 1;
        }
        self.filter_text.drain(self.filter_cursor..start);
    }

    fn delete_word_forwards(&mut self) {
        let len = self.filter_text.len();
        if self.filter_cursor >= len {
            return;
        }
        let start = self.filter_cursor;
        let bytes = self.filter_text.as_bytes();
        let mut end = start;
        // Skip word characters
        while end < len && !bytes[end].is_ascii_whitespace() {
            end += 1;
        }
        // Skip whitespace
        while end < len && bytes[end].is_ascii_whitespace() {
            end += 1;
        }
        self.filter_text.drain(start..end);
    }

    fn transpose_chars(&mut self) {
        // Ctrl-T: swap the two characters before the cursor
        // If at end of line, swap last two chars
        // If at position 1 or beyond with at least 2 chars, swap char before cursor with one before that
        let len = self.filter_text.len();
        if len < 2 {
            return;
        }

        // If cursor is at start, nothing to transpose
        if self.filter_cursor == 0 {
            return;
        }

        // If cursor is at end, transpose last two characters and keep cursor at end
        // Otherwise, transpose char at cursor-1 with char at cursor, then move cursor right
        let pos = if self.filter_cursor >= len {
            len - 1
        } else {
            self.filter_cursor
        };

        if pos == 0 {
            return;
        }

        // Only transpose if both positions are ASCII (single-byte) characters.
        // For multi-byte UTF-8 characters, transposition is more complex and skipped.
        if self.filter_text.is_char_boundary(pos - 1)
            && self.filter_text.is_char_boundary(pos)
            && pos < len
            && self.filter_text.is_char_boundary(pos + 1)
        {
            // Check both chars are single-byte ASCII
            let bytes = self.filter_text.as_bytes();
            if bytes[pos - 1].is_ascii() && bytes[pos].is_ascii() {
                // SAFETY: We verified both bytes are ASCII, so swapping them is safe
                let bytes = unsafe { self.filter_text.as_bytes_mut() };
                bytes.swap(pos - 1, pos);

                // Move cursor right if not at end
                if self.filter_cursor < len {
                    self.filter_cursor += 1;
                }
            }
        }
    }

    fn case_matching(&self) -> CaseMatching {
        match self.config.case_sensitivity {
            CaseSensitivity::Smart => CaseMatching::Smart,
            CaseSensitivity::CaseSensitive => CaseMatching::Respect,
            CaseSensitivity::CaseInsensitive => CaseMatching::Ignore,
        }
    }

    fn fuzzy_atom(&self) -> Atom {
        Atom::new(
            &self.filter_text,
            self.case_matching(),
            Normalization::Smart,
            AtomKind::Fuzzy,
            false,
        )
    }

    fn score_text(
        matcher: &mut NucleoMatcher,
        atom: &Atom,
        text: &str,
        buf: &mut Vec<char>,
    ) -> Option<u16> {
        atom.score(Utf32Str::new(text, buf), matcher)
    }

    fn fuzzy_text_matches(&self, text: &str) -> bool {
        let atom = self.fuzzy_atom();
        let mut matcher = Self::make_matcher();
        let mut buf = Vec::new();
        Self::score_text(&mut matcher, &atom, text, &mut buf).is_some()
    }

    fn fuzzy_match_indices(&self, text: &str) -> Option<Vec<usize>> {
        let atom = self.fuzzy_atom();
        let mut matcher = Self::make_matcher();
        let mut buf = Vec::new();
        let mut indices = Vec::new();
        atom.indices(Utf32Str::new(text, &mut buf), &mut matcher, &mut indices)?;

        let mut indices = indices
            .into_iter()
            .map(usize::try_from)
            .collect::<Result<Vec<_>, _>>()
            .ok()?;
        indices.sort_unstable();
        indices.dedup();
        Some(indices)
    }

    /// Score an item using per-column matching (best column wins)
    fn score_per_column(
        matcher: &mut NucleoMatcher,
        atom: &Atom,
        item: &SelectItem,
        buf: &mut Vec<char>,
    ) -> Option<u16> {
        item.cells.as_ref().and_then(|cells| {
            cells
                .iter()
                .filter_map(|(cell_text, _)| Self::score_text(matcher, atom, cell_text, buf))
                .max()
        })
    }

    /// Score an item - uses per-column matching if enabled and in table mode
    fn score_item(
        matcher: &mut NucleoMatcher,
        atom: &Atom,
        per_column: bool,
        item: &SelectItem,
        buf: &mut Vec<char>,
    ) -> Option<u16> {
        if per_column && item.cells.is_some() {
            Self::score_per_column(matcher, atom, item, buf)
        } else {
            Self::score_text(matcher, atom, &item.name, buf)
        }
    }

    fn should_yield_filter(start: nu_utils::time::Instant, checked: usize) -> bool {
        checked > 0
            && checked.is_multiple_of(FUZZY_FILTER_INTERRUPT_CHECK_INTERVAL)
            && start.elapsed() >= FUZZY_FILTER_MIN_INTERRUPT_TIME
            && event::poll(Duration::ZERO).is_ok_and(|has_event| has_event)
    }

    fn score_filter_candidates<I>(
        &mut self,
        candidates: I,
        atom: &Atom,
        start: nu_utils::time::Instant,
    ) -> Option<Vec<(usize, u16)>>
    where
        I: Iterator<Item = usize>,
    {
        let mut scored = Vec::new();
        let mut buf = Vec::new();
        for (checked, i) in candidates.enumerate() {
            if Self::should_yield_filter(start, checked) {
                return None;
            }

            if let Some(score) = Self::score_item(
                &mut self.matcher,
                atom,
                self.per_column,
                &self.items[i],
                &mut buf,
            ) {
                scored.push((i, score));
            }
        }

        Some(scored)
    }

    fn update_filter(&mut self) {
        let old_indices = std::mem::take(&mut self.filtered_indices);
        let start = nu_utils::time::Instant::now();

        // Determine whether to filter from refined subset or all items
        let use_refined = self.refined && !self.refined_base_indices.is_empty();

        if self.filter_text.is_empty() {
            // When empty, copy the base indices
            self.filtered_indices = if use_refined {
                self.refined_base_indices.clone()
            } else {
                (0..self.items.len()).collect()
            };
            self.last_filter_text.clear();
            self.force_full_filter = false;
        } else {
            let atom = self.fuzzy_atom();
            let can_reuse_previous = !self.force_full_filter
                && !self.last_filter_text.is_empty()
                && self.filter_text.starts_with(&self.last_filter_text);

            let mut scored = if can_reuse_previous {
                self.score_filter_candidates(old_indices.iter().copied(), &atom, start)
            } else if use_refined {
                let refined_base_indices = self.refined_base_indices.clone();
                self.score_filter_candidates(refined_base_indices.into_iter(), &atom, start)
            } else {
                self.score_filter_candidates(0..self.items.len(), &atom, start)
            };

            let Some(mut scored) = scored.take() else {
                self.filtered_indices = old_indices;
                self.results_changed = false;
                self.filter_text_changed = true;
                return;
            };
            // Sort by score descending
            scored.sort_by_key(|entry| std::cmp::Reverse(entry.1));
            self.filtered_indices = scored.into_iter().map(|(i, _)| i).collect();
            self.last_filter_text = self.filter_text.clone();
            self.force_full_filter = false;
        }

        // Check if results actually changed
        self.results_changed = old_indices != self.filtered_indices;
        self.filter_text_changed = true;

        // Only reset cursor/scroll if results changed
        if self.results_changed {
            self.cursor = 0;
            self.scroll_offset = 0;
        }

        // In table mode, auto-scroll horizontally to show the first column with matches
        if self.is_table_mode() && !self.filter_text.is_empty() && !self.filtered_indices.is_empty()
        {
            self.auto_scroll_to_match_column();
        }
    }

    /// In table mode, scroll horizontally to ensure the first column with matches is visible
    fn auto_scroll_to_match_column(&mut self) {
        let Some(layout) = &self.table_layout else {
            return;
        };

        // Look at the top result to find which column has the best match
        let first_idx = self.filtered_indices[0];
        let item = &self.items[first_idx];
        let Some(cells) = &item.cells else {
            return;
        };

        // Find the first column (leftmost) that has a match
        let mut first_match_col: Option<usize> = None;
        for (col_idx, (cell_text, _)) in cells.iter().enumerate() {
            if self.per_column {
                // Per-column mode: check each cell individually
                if self.fuzzy_text_matches(cell_text) {
                    first_match_col = Some(col_idx);
                    break;
                }
            } else {
                // Standard mode: check if this cell's portion of item.name has matches
                // Calculate the character offset for this cell in the concatenated name
                let cell_start: usize = cells[..col_idx]
                    .iter()
                    .map(|(s, _)| s.chars().count() + 1) // +1 for space separator
                    .sum();
                let cell_char_count = cell_text.chars().count();

                if let Some(indices) = self.fuzzy_match_indices(&item.name) {
                    // Check if any match indices fall within this cell
                    if indices
                        .iter()
                        .any(|&idx| idx >= cell_start && idx < cell_start + cell_char_count)
                    {
                        first_match_col = Some(col_idx);
                        break;
                    }
                }
            }
        }

        // If we found a matching column, ensure it's visible
        if let Some(match_col) = first_match_col {
            let (cols_visible, _) = self.calculate_visible_columns();
            let visible_start = self.horizontal_offset;
            let visible_end = self.horizontal_offset + cols_visible;

            if match_col < visible_start {
                // Match is to the left, scroll left
                self.horizontal_offset = match_col;
                self.horizontal_scroll_changed = true;
                self.update_table_layout();
            } else if match_col >= visible_end {
                // Match is to the right, scroll right
                // Set offset so match_col is the first visible column
                self.horizontal_offset = match_col;
                // But don't scroll past what's possible
                let max_offset = layout.col_widths.len().saturating_sub(1);
                self.horizontal_offset = self.horizontal_offset.min(max_offset);
                self.horizontal_scroll_changed = true;
                self.update_table_layout();
            }
        }
    }

    fn get_result(&self) -> InteractMode {
        match self.mode {
            SelectMode::Single => InteractMode::Single(Some(self.cursor)),
            SelectMode::Multi => {
                let mut indices: Vec<usize> = self.selected.iter().copied().collect();
                indices.sort();
                InteractMode::Multi(Some(indices))
            }
            SelectMode::Fuzzy => {
                if self.filtered_indices.is_empty() {
                    InteractMode::Single(None)
                } else {
                    InteractMode::Single(Some(self.filtered_indices[self.cursor]))
                }
            }
            SelectMode::FuzzyMulti => {
                // Return all selected items regardless of current filter
                // This allows selecting items across multiple filter searches
                let mut indices: Vec<usize> = self.selected.iter().copied().collect();
                indices.sort();
                InteractMode::Multi(Some(indices))
            }
        }
    }

    /// Check if we can do a toggle-only update in multi mode
    /// (just toggled a single visible item, no cursor movement)
    fn can_do_multi_toggle_only_update(&self) -> bool {
        if self.first_render || self.width_changed || self.mode != SelectMode::Multi {
            return false;
        }
        if self.table_layout_changed {
            return false;
        }
        // If the cursor also moved (e.g. Tab toggles and navigates), a full redraw
        // is needed so the ">" indicator follows the cursor.
        if self.cursor != self.prev_cursor {
            return false;
        }
        if let Some(toggled) = self.toggled_item {
            // Check if toggled item is visible
            let visible_start = self.scroll_offset;
            let visible_end = self.scroll_offset + self.visible_height as usize;
            toggled >= visible_start && toggled < visible_end
        } else {
            false
        }
    }

    /// Check if we can do a toggle+move update in fuzzy multi mode
    /// (toggled an item and moved cursor, both visible, no scroll change)
    fn can_do_fuzzy_multi_toggle_update(&self) -> bool {
        if self.first_render || self.width_changed || self.mode != SelectMode::FuzzyMulti {
            return false;
        }
        if self.table_layout_changed {
            return false;
        }
        if self.scroll_offset != self.prev_scroll_offset {
            return false; // Scrolled, need full redraw
        }
        if self.filter_text_changed || self.results_changed {
            return false; // Filter changed, need full redraw
        }
        if let Some(toggled) = self.toggled_item {
            // Check if both toggled item and new cursor are visible
            let visible_start = self.scroll_offset;
            let visible_end = self.scroll_offset + self.visible_height as usize;
            let toggled_visible = toggled >= visible_start && toggled < visible_end;
            let cursor_visible = self.cursor >= visible_start && self.cursor < visible_end;
            toggled_visible && cursor_visible
        } else {
            false
        }
    }

    /// Check if we can do a toggle-all update in fuzzy multi mode
    /// (toggled all filtered items with Alt+A)
    fn can_do_fuzzy_multi_toggle_all_update(&self) -> bool {
        !self.first_render
            && !self.width_changed
            && self.mode == SelectMode::FuzzyMulti
            && self.toggled_all
            && !self.filter_text_changed
            && !self.results_changed
            && self.scroll_offset == self.prev_scroll_offset
            && !self.horizontal_scroll_changed
            && !self.table_layout_changed
    }

    /// Check if we can do a toggle-all update in multi mode
    /// (toggled all items with 'a' key)
    fn can_do_multi_toggle_all_update(&self) -> bool {
        !self.first_render
            && !self.width_changed
            && self.mode == SelectMode::Multi
            && self.toggled_all
            && !self.table_layout_changed
    }

    /// FuzzyMulti mode: update toggled row and new cursor row
    fn render_fuzzy_multi_toggle_update(&mut self, stderr: &mut Stderr) -> io::Result<()> {
        let toggled = self.toggled_item.expect("toggled_item must be Some");
        execute!(stderr, BeginSynchronizedUpdate)?;

        // Calculate header lines (prompt + filter + separator + table header)
        let header_lines = self.fuzzy_header_lines();

        let toggled_display_row = (toggled - self.scroll_offset) as u16;
        let cursor_display_row = (self.cursor - self.scroll_offset) as u16;

        let toggled_item_row = header_lines + toggled_display_row;
        let cursor_item_row = header_lines + cursor_display_row;

        // We're at the filter line
        let filter_row = self.fuzzy_filter_row();

        // Move to toggled row and redraw it (checkbox changed, marker removed)
        let down_to_toggled = toggled_item_row.saturating_sub(filter_row);
        execute!(stderr, MoveDown(down_to_toggled), MoveToColumn(0))?;

        // Redraw toggled row (now without marker, checkbox state changed)
        let toggled_real_idx = self.filtered_indices[toggled];
        let toggled_item = &self.items[toggled_real_idx];
        let toggled_checked = self.selected.contains(&toggled_real_idx);
        if self.is_table_mode() {
            self.render_table_row_fuzzy_multi(stderr, toggled_item, toggled_checked, false)?;
        } else {
            self.render_fuzzy_multi_item_inline(
                stderr,
                &toggled_item.name,
                toggled_checked,
                false,
            )?;
        }

        // Move to cursor row and redraw it (marker added)
        if cursor_item_row > toggled_item_row {
            let lines_down = cursor_item_row - toggled_item_row;
            execute!(stderr, MoveDown(lines_down), MoveToColumn(0))?;
        } else if cursor_item_row < toggled_item_row {
            let lines_up = toggled_item_row - cursor_item_row;
            execute!(stderr, MoveUp(lines_up), MoveToColumn(0))?;
        }

        let cursor_real_idx = self.filtered_indices[self.cursor];
        let cursor_item = &self.items[cursor_real_idx];
        let cursor_checked = self.selected.contains(&cursor_real_idx);
        if self.is_table_mode() {
            self.render_table_row_fuzzy_multi(stderr, cursor_item, cursor_checked, true)?;
        } else {
            self.render_fuzzy_multi_item_inline(stderr, &cursor_item.name, cursor_checked, true)?;
        }

        // Update footer to reflect new selection count
        if self.has_footer() {
            // Calculate footer row position
            let total_count = self.current_list_len();
            let end = (self.scroll_offset + self.visible_height as usize).min(total_count);
            let visible_count = (end - self.scroll_offset) as u16;
            let footer_row = header_lines + visible_count;

            // Move from cursor row to footer
            let down_to_footer = footer_row.saturating_sub(cursor_item_row);
            execute!(stderr, MoveDown(down_to_footer))?;

            // Update footer
            self.render_footer_inline(stderr)?;

            // Move back to filter line
            let up_to_filter = footer_row.saturating_sub(filter_row);
            execute!(stderr, MoveUp(up_to_filter))?;
        } else {
            // Move back to filter line
            let up_to_filter = cursor_item_row.saturating_sub(filter_row);
            execute!(stderr, MoveUp(up_to_filter))?;
        }

        // Position cursor within filter text
        self.position_fuzzy_cursor(stderr)?;

        // Update state
        self.prev_cursor = self.cursor;
        self.toggled_item = None;

        execute!(stderr, EndSynchronizedUpdate)?;
        stderr.flush()
    }

    /// Multi mode: only update the checkbox for the toggled item
    fn render_multi_toggle_only(&mut self, stderr: &mut Stderr) -> io::Result<()> {
        let toggled = self.toggled_item.expect("toggled_item must be Some");
        execute!(stderr, BeginSynchronizedUpdate)?;

        let mut header_lines: u16 = if self.prompt.is_some() { 1 } else { 0 };
        if self.is_table_mode() {
            header_lines += 2; // table header + header separator line
        }

        // Calculate display position of toggled item relative to scroll
        let display_row = (toggled - self.scroll_offset) as u16;

        // Current position is at end of rendered content
        let items_rendered = self.rendered_lines - header_lines as usize;

        // Move to the toggled row
        // Cursor is at end of last content line, so subtract 1 from items_rendered
        let lines_up = (items_rendered as u16)
            .saturating_sub(1)
            .saturating_sub(display_row);
        execute!(stderr, MoveUp(lines_up))?;

        // Move to checkbox column (after "> " or "  ")
        execute!(stderr, MoveToColumn(2))?;

        // Write new checkbox state
        let checkbox = if self.selected.contains(&toggled) {
            "[x]"
        } else {
            "[ ]"
        };
        execute!(stderr, Print(checkbox))?;

        // Move back to end position (footer line if shown, else last item line)
        execute!(stderr, MoveDown(lines_up))?;

        // Update footer to reflect new selection count
        if self.has_footer() {
            self.render_footer_inline(stderr)?;
        }

        // Reset toggle tracking
        self.toggled_item = None;

        execute!(stderr, EndSynchronizedUpdate)?;
        stderr.flush()
    }

    /// Multi mode: update all visible checkboxes (toggle all with 'a')
    fn render_multi_toggle_all(&mut self, stderr: &mut Stderr) -> io::Result<()> {
        execute!(stderr, BeginSynchronizedUpdate)?;

        let mut header_lines: u16 = if self.prompt.is_some() { 1 } else { 0 };
        if self.is_table_mode() {
            header_lines += 2; // table header + header separator line
        }

        // Current position is at end of rendered content
        let items_rendered = self.rendered_lines - header_lines as usize;

        // Calculate visible range
        let visible_end = (self.scroll_offset + self.visible_height as usize).min(self.items.len());
        let visible_count = visible_end - self.scroll_offset;

        // Move to first item row
        // Cursor is at end of last content line, so subtract 1 to get to first item
        execute!(stderr, MoveUp((items_rendered as u16).saturating_sub(1)))?;

        // Update each visible item's checkbox
        for i in 0..visible_count {
            let item_idx = self.scroll_offset + i;
            let checkbox = if self.selected.contains(&item_idx) {
                "[x]"
            } else {
                "[ ]"
            };
            // Move to checkbox column and update
            execute!(stderr, MoveToColumn(2), Print(checkbox))?;
            if i + 1 < visible_count {
                execute!(stderr, MoveDown(1))?;
            }
        }

        // Move back to end position (footer line if shown, else last item line)
        let remaining = items_rendered as u16 - visible_count as u16;
        if remaining > 0 {
            execute!(stderr, MoveDown(remaining))?;
        }

        // Update footer to reflect new selection count
        if self.has_footer() {
            self.render_footer_inline(stderr)?;
        }

        // Reset toggle tracking
        self.toggled_all = false;

        execute!(stderr, EndSynchronizedUpdate)?;
        stderr.flush()
    }

    /// FuzzyMulti mode: update all visible rows (toggle all with Alt+A)
    fn render_fuzzy_multi_toggle_all_update(&mut self, stderr: &mut Stderr) -> io::Result<()> {
        execute!(stderr, BeginSynchronizedUpdate)?;

        // Calculate header lines (prompt + filter + separator + table header)
        let header_lines = self.fuzzy_header_lines();

        let total_count = self.current_list_len();
        let end = (self.scroll_offset + self.visible_height as usize).min(total_count);
        let visible_count = end.saturating_sub(self.scroll_offset);

        // We're at the filter line
        let filter_row = self.fuzzy_filter_row();

        // Move to first item row
        let down_to_first = header_lines.saturating_sub(filter_row);
        execute!(stderr, MoveDown(down_to_first), MoveToColumn(0))?;

        for (i, idx) in (self.scroll_offset..end).enumerate() {
            let real_idx = self.filtered_indices[idx];
            let item = &self.items[real_idx];
            let checked = self.selected.contains(&real_idx);
            let active = idx == self.cursor;

            if self.is_table_mode() {
                self.render_table_row_fuzzy_multi(stderr, item, checked, active)?;
            } else {
                self.render_fuzzy_multi_item_inline(stderr, &item.name, checked, active)?;
            }

            if i + 1 < visible_count {
                execute!(stderr, MoveDown(1), MoveToColumn(0))?;
            }
        }

        // Move to footer (if present) and update it
        if self.has_footer() {
            let footer_row = header_lines + visible_count as u16;
            let last_item_row = header_lines + visible_count.saturating_sub(1) as u16;
            let down_to_footer = footer_row.saturating_sub(last_item_row);
            execute!(stderr, MoveDown(down_to_footer))?;
            self.render_footer_inline(stderr)?;
            let up_to_filter = footer_row.saturating_sub(filter_row);
            execute!(stderr, MoveUp(up_to_filter))?;
        } else {
            let up_to_filter =
                (header_lines + visible_count.saturating_sub(1) as u16).saturating_sub(filter_row);
            execute!(stderr, MoveUp(up_to_filter))?;
        }

        // Position cursor within filter text
        self.position_fuzzy_cursor(stderr)?;

        // Reset toggle tracking
        self.toggled_all = false;

        execute!(stderr, EndSynchronizedUpdate)?;
        stderr.flush()
    }

    #[allow(clippy::collapsible_if)]
    fn render(&mut self, stderr: &mut Stderr) -> io::Result<()> {
        // Keep streamed rows live-updating even when the user is not scrolling. This only drains
        // values already delivered by the background reader, so rendering stays responsive for
        // slow or infinite inputs.
        let loaded_stream_items = self.load_more_items_for(STREAM_DRAIN_TIME_BUDGET);
        if loaded_stream_items && self.follow_stream_to_end {
            self.cursor = self.current_list_len().saturating_sub(1);
            self.adjust_scroll_down();
        }
        self.update_stream_footer();

        // Check for fuzzy multi mode toggle-all optimization
        if self.can_do_fuzzy_multi_toggle_all_update() {
            return self.render_fuzzy_multi_toggle_all_update(stderr);
        }

        // Check for multi mode toggle-all optimization
        if self.can_do_multi_toggle_all_update() {
            return self.render_multi_toggle_all(stderr);
        }

        // Check for multi mode toggle-only optimization
        if self.can_do_multi_toggle_only_update() {
            return self.render_multi_toggle_only(stderr);
        }

        // Check for fuzzy multi mode toggle+move optimization
        if self.can_do_fuzzy_multi_toggle_update() {
            return self.render_fuzzy_multi_toggle_update(stderr);
        }

        // The old cursor-only navigation optimizations were removed because
        // they were brittle and caused wrapping bugs.  We now always perform a
        // full redraw for simple cursor moves; other optimizations (toggle
        // updates) are still available above.

        // If nothing changed (e.g., PageDown at bottom of list), skip render entirely
        if !self.first_render
            && !self.width_changed
            && self.cursor == self.prev_cursor
            && self.scroll_offset == self.prev_scroll_offset
            && !loaded_stream_items
            && !self.results_changed
            && !self.filter_text_changed
            && !self.horizontal_scroll_changed
            && !self.table_layout_changed
            && !self.settings_changed
            && !self.toggled_all
        {
            return Ok(());
        }

        execute!(stderr, BeginSynchronizedUpdate)?;

        // Calculate how many lines we'll render
        let total_count = self.current_list_len();
        let end = (self.scroll_offset + self.visible_height as usize).min(total_count);
        // Show footer in fuzzy modes (for settings), multi modes (for selection count), or when scrolling is needed
        let has_scroll_indicator = self.has_footer();
        let items_to_render = end - self.scroll_offset;

        // Calculate total lines needed for this render
        let mut lines_needed: usize = 0;
        if self.prompt.is_some() {
            lines_needed += 1;
        }
        if self.mode == SelectMode::Fuzzy || self.mode == SelectMode::FuzzyMulti {
            lines_needed += 1; // filter line
            if self.config.show_separator {
                lines_needed += 1;
            }
        }
        if self.is_table_mode() {
            lines_needed += 2; // table header + header separator
        }
        lines_needed += items_to_render;
        if has_scroll_indicator {
            lines_needed += 1;
        }

        // On first render, claim vertical space by printing newlines (causes scroll if needed)
        if self.first_render && lines_needed > 1 {
            for _ in 0..(lines_needed - 1) {
                execute!(stderr, Print("\n"))?;
            }
            execute!(stderr, MoveUp((lines_needed - 1) as u16))?;
        }

        // In fuzzy mode, cursor may be at filter line; move to last content line first
        if self.fuzzy_cursor_offset > 0 {
            execute!(stderr, MoveDown(self.fuzzy_cursor_offset as u16))?;
            self.fuzzy_cursor_offset = 0;
        }

        // If streaming added enough rows to grow the rendered area, claim the extra lines before
        // moving back to the top. Otherwise the terminal may scroll underneath the existing
        // header/footer and leave stale rows on screen.
        if !self.first_render && lines_needed > self.rendered_lines {
            let lines_to_add = lines_needed - self.rendered_lines;
            for _ in 0..lines_to_add {
                execute!(stderr, Print("\n"))?;
            }
            execute!(stderr, MoveUp(lines_to_add as u16))?;
        }

        // Move to start of our render area (first line, column 0)
        // Cursor is on last content line, move up to first line
        if self.rendered_lines > 1 {
            execute!(stderr, MoveUp((self.rendered_lines - 1) as u16))?;
        }
        execute!(stderr, MoveToColumn(0))?;

        let mut lines_rendered: usize = 0;

        // Render prompt (only on first render, it doesn't change)
        if self.first_render {
            if let Some(prompt) = self.prompt {
                execute!(stderr, Print(prompt), Clear(ClearType::UntilNewLine))?;
            }
        }
        if self.prompt.is_some() {
            lines_rendered += 1;
            if lines_rendered < lines_needed {
                execute!(stderr, MoveDown(1), MoveToColumn(0))?;
            }
        }

        // Render filter line for fuzzy modes
        if self.mode == SelectMode::Fuzzy || self.mode == SelectMode::FuzzyMulti {
            execute!(
                stderr,
                Print(self.prompt_marker()),
                Print(&self.filter_text),
                Clear(ClearType::UntilNewLine),
            )?;
            lines_rendered += 1;
            if lines_rendered < lines_needed {
                execute!(stderr, MoveDown(1), MoveToColumn(0))?;
            }

            // Render separator line
            if self.config.show_separator {
                execute!(
                    stderr,
                    Print(self.config.separator.paint(&self.separator_line)),
                    Clear(ClearType::UntilNewLine),
                )?;
                lines_rendered += 1;
                if lines_rendered < lines_needed {
                    execute!(stderr, MoveDown(1), MoveToColumn(0))?;
                }
            }
        }

        // Render table header and separator if in table mode.
        // Redraw when column positioning or widths changed.
        if self.is_table_mode() {
            let need_header_redraw =
                self.first_render || self.horizontal_scroll_changed || self.table_layout_changed;
            if need_header_redraw {
                self.render_table_header(stderr)?;
            }
            lines_rendered += 1;
            if lines_rendered < lines_needed {
                execute!(stderr, MoveDown(1), MoveToColumn(0))?;
            }
            if need_header_redraw {
                self.render_table_header_separator(stderr)?;
            }
            lines_rendered += 1;
            if lines_rendered < lines_needed {
                execute!(stderr, MoveDown(1), MoveToColumn(0))?;
            }
        }

        // Render items
        for idx in self.scroll_offset..end {
            let is_active = idx == self.cursor;
            let is_last_line = lines_rendered + 1 == lines_needed;

            if self.is_table_mode() {
                // Table mode rendering
                match self.mode {
                    SelectMode::Single => {
                        let item = &self.items[idx];
                        self.render_table_row_single(stderr, item, is_active)?;
                    }
                    SelectMode::Multi => {
                        let real_idx = if self.refined {
                            self.filtered_indices[idx]
                        } else {
                            idx
                        };
                        let item = &self.items[real_idx];
                        let is_checked = self.selected.contains(&real_idx);
                        self.render_table_row_multi(stderr, item, is_checked, is_active)?;
                    }
                    SelectMode::Fuzzy => {
                        let real_idx = self.filtered_indices[idx];
                        let item = &self.items[real_idx];
                        self.render_table_row_fuzzy(stderr, item, is_active)?;
                    }
                    SelectMode::FuzzyMulti => {
                        let real_idx = self.filtered_indices[idx];
                        let item = &self.items[real_idx];
                        let is_checked = self.selected.contains(&real_idx);
                        self.render_table_row_fuzzy_multi(stderr, item, is_checked, is_active)?;
                    }
                }
            } else {
                // Single-line mode rendering
                match self.mode {
                    SelectMode::Single => {
                        let item = &self.items[idx];
                        self.render_single_item_inline(stderr, &item.name, is_active)?;
                    }
                    SelectMode::Multi => {
                        let real_idx = if self.refined {
                            self.filtered_indices[idx]
                        } else {
                            idx
                        };
                        let item = &self.items[real_idx];
                        let is_checked = self.selected.contains(&real_idx);
                        self.render_multi_item_inline(stderr, &item.name, is_checked, is_active)?;
                    }
                    SelectMode::Fuzzy => {
                        let real_idx = self.filtered_indices[idx];
                        let item = &self.items[real_idx];
                        self.render_fuzzy_item_inline(stderr, &item.name, is_active)?;
                    }
                    SelectMode::FuzzyMulti => {
                        let real_idx = self.filtered_indices[idx];
                        let item = &self.items[real_idx];
                        let is_checked = self.selected.contains(&real_idx);
                        self.render_fuzzy_multi_item_inline(
                            stderr, &item.name, is_checked, is_active,
                        )?;
                    }
                }
            }
            lines_rendered += 1;
            if !is_last_line {
                execute!(stderr, MoveDown(1), MoveToColumn(0))?;
            }
        }

        // Show scroll indicator if needed
        if has_scroll_indicator {
            let indicator = self.generate_footer();
            execute!(
                stderr,
                Print(self.config.footer.paint(&indicator)),
                Clear(ClearType::UntilNewLine),
            )?;
            lines_rendered += 1;
        }

        // Clear any extra lines from previous render
        // Cursor is on last rendered line
        if lines_rendered < self.rendered_lines {
            let extra_lines = self.rendered_lines - lines_rendered;
            for _ in 0..extra_lines {
                execute!(
                    stderr,
                    MoveDown(1),
                    MoveToColumn(0),
                    Clear(ClearType::CurrentLine)
                )?;
            }
            // Move back to last content line
            execute!(stderr, MoveUp(extra_lines as u16))?;
        }

        // Update state
        self.rendered_lines = lines_rendered;
        self.prev_cursor = self.cursor;
        self.prev_scroll_offset = self.scroll_offset;
        self.first_render = false;
        self.filter_text_changed = false;
        self.results_changed = false;
        self.horizontal_scroll_changed = false;
        self.width_changed = false;
        self.table_layout_changed = false;
        self.toggled_item = None;
        self.toggled_all = false;
        self.settings_changed = false;

        // In fuzzy modes, position cursor within filter text
        if self.mode == SelectMode::Fuzzy || self.mode == SelectMode::FuzzyMulti {
            // Cursor is on last content line, move up to filter line
            let filter_row = self.fuzzy_filter_row() as usize;
            self.fuzzy_cursor_offset = lines_rendered.saturating_sub(filter_row + 1);
            if self.fuzzy_cursor_offset > 0 {
                execute!(stderr, MoveUp(self.fuzzy_cursor_offset as u16))?;
            }
            // Position cursor after prompt marker + text up to filter_cursor
            self.position_fuzzy_cursor(stderr)?;
        }

        execute!(stderr, EndSynchronizedUpdate)?;
        stderr.flush()
    }

    fn render_single_item_inline(
        &self,
        stderr: &mut Stderr,
        text: &str,
        active: bool,
    ) -> io::Result<()> {
        let prefix = if active { self.selected_marker() } else { "  " };
        let prefix_width = 2;

        execute!(stderr, Print(prefix))?;
        self.render_truncated_text(stderr, text, prefix_width)?;
        execute!(stderr, Print(RESET), Clear(ClearType::UntilNewLine))?;
        Ok(())
    }

    fn render_multi_item_inline(
        &self,
        stderr: &mut Stderr,
        text: &str,
        checked: bool,
        active: bool,
    ) -> io::Result<()> {
        let cursor = if active { self.selected_marker() } else { "  " };
        let checkbox = if checked { "[x] " } else { "[ ] " };
        let prefix_width = 6; // "> [x] " or "  [ ] "

        execute!(stderr, Print(cursor), Print(checkbox))?;
        self.render_truncated_text(stderr, text, prefix_width)?;
        execute!(stderr, Print(RESET), Clear(ClearType::UntilNewLine))?;
        Ok(())
    }

    fn render_fuzzy_item_inline(
        &self,
        stderr: &mut Stderr,
        text: &str,
        active: bool,
    ) -> io::Result<()> {
        let prefix = if active { self.selected_marker() } else { "  " };
        let prefix_width = 2;
        execute!(stderr, Print(prefix))?;

        if self.filter_text.is_empty() {
            self.render_truncated_text(stderr, text, prefix_width)?;
        } else if let Some(indices) = self.fuzzy_match_indices(text) {
            self.render_truncated_fuzzy_text(stderr, text, &indices, prefix_width)?;
        } else {
            self.render_truncated_text(stderr, text, prefix_width)?;
        }
        execute!(stderr, Print(RESET), Clear(ClearType::UntilNewLine))?;
        Ok(())
    }

    fn render_fuzzy_multi_item_inline(
        &self,
        stderr: &mut Stderr,
        text: &str,
        checked: bool,
        active: bool,
    ) -> io::Result<()> {
        let cursor = if active { self.selected_marker() } else { "  " };
        let checkbox = if checked { "[x] " } else { "[ ] " };
        let prefix_width = 6; // "> [x] " or "  [ ] "
        execute!(stderr, Print(cursor), Print(checkbox))?;

        if self.filter_text.is_empty() {
            self.render_truncated_text(stderr, text, prefix_width)?;
        } else if let Some(indices) = self.fuzzy_match_indices(text) {
            self.render_truncated_fuzzy_text(stderr, text, &indices, prefix_width)?;
        } else {
            self.render_truncated_text(stderr, text, prefix_width)?;
        }
        execute!(stderr, Print(RESET), Clear(ClearType::UntilNewLine))?;
        Ok(())
    }

    /// Render text, truncating with ellipsis if it exceeds available width.
    fn item_text_width(&self, prefix_width: usize) -> usize {
        // Keep one printable cell free so drawing a full-width item does not leave the terminal in
        // a wrap-pending state before the footer or next row is rendered.
        self.term_width
            .saturating_sub(prefix_width as u16)
            .saturating_sub(1) as usize
    }

    fn render_truncated_text(
        &self,
        stderr: &mut Stderr,
        text: &str,
        prefix_width: usize,
    ) -> io::Result<()> {
        let available_width = self.item_text_width(prefix_width);
        let text = truncate_ansi_aware_text_at(text, available_width, prefix_width);
        execute!(stderr, Print(text.as_ref()))?;
        Ok(())
    }

    fn render_display_segments(
        &self,
        stderr: &mut Stderr,
        sanitized: &SanitizedText,
        match_indices: Option<&[usize]>,
        base_style: Option<Style>,
    ) -> io::Result<()> {
        let mut match_iter = match_indices.map(|indices| indices.iter().peekable());

        for segment in &sanitized.segments {
            let is_match = if let (Some(source_index), Some(match_iter)) =
                (segment.source_index, match_iter.as_mut())
            {
                while match_iter.peek().is_some_and(|&&idx| idx < source_index) {
                    match_iter.next();
                }
                match_iter.peek().is_some_and(|&&idx| idx == source_index)
            } else {
                false
            };

            if is_match {
                execute!(stderr, Print(self.config.match_text.paint(&segment.text)))?;
            } else if let Some(style) = base_style {
                execute!(stderr, Print(style.paint(&segment.text)))?;
            } else {
                execute!(stderr, Print(&segment.text))?;
            }
        }

        Ok(())
    }

    /// Render fuzzy-highlighted text, truncating with ellipsis if needed.
    /// The ellipsis is highlighted if any matches fall in the truncated portion.
    fn render_truncated_fuzzy_text(
        &self,
        stderr: &mut Stderr,
        text: &str,
        match_indices: &[usize],
        prefix_width: usize,
    ) -> io::Result<()> {
        let available_width = self.item_text_width(prefix_width);

        if available_width <= 1 {
            // Only room for ellipsis
            let has_any_matches = !match_indices.is_empty();
            if has_any_matches {
                execute!(stderr, Print(self.config.match_text.paint("…")))?;
            } else {
                execute!(stderr, Print("…"))?;
            }
            return Ok(());
        }

        let sanitized = sanitize_text_for_display(text, available_width, prefix_width);
        if !sanitized.truncated {
            self.render_display_segments(stderr, &sanitized, Some(match_indices), None)?;
            return Ok(());
        }

        let sanitized = sanitize_text_for_display(text, available_width - 1, prefix_width);
        self.render_display_segments(stderr, &sanitized, Some(match_indices), None)?;

        let has_hidden_matches = match_indices
            .iter()
            .any(|&idx| idx >= sanitized.source_chars);
        if has_hidden_matches {
            execute!(stderr, Print(self.config.match_text.paint("…")))?;
        } else {
            execute!(stderr, Print("…"))?;
        }
        Ok(())
    }

    /// Render the table header row
    fn render_table_header(&self, stderr: &mut Stderr) -> io::Result<()> {
        let Some(layout) = &self.table_layout else {
            return Ok(());
        };

        let prefix_width = self.row_prefix_width();
        let (cols_visible, has_more_right) = self.calculate_visible_columns();
        let has_more_left = self.horizontal_offset > 0;

        // Render prefix space (no marker for header)
        execute!(stderr, Print(" ".repeat(prefix_width)))?;

        // Left scroll indicator (ellipsis + column separator)
        if has_more_left {
            let sep = self.table_column_separator();
            execute!(
                stderr,
                Print(self.config.table_separator.paint("…")),
                Print(self.config.table_separator.paint(&sep))
            )?;
        }

        // Render visible column headers
        let visible_range = self.horizontal_offset..(self.horizontal_offset + cols_visible);
        for (i, col_idx) in visible_range.enumerate() {
            if col_idx >= layout.columns.len() {
                break;
            }

            // Separator between columns
            if i > 0 {
                let sep = self.table_column_separator();
                execute!(stderr, Print(self.config.table_separator.paint(&sep)))?;
            }

            // Render column header, center-aligned to column width
            let header = &layout.columns[col_idx];
            let col_width = layout.col_widths[col_idx];
            let header_width = header.width();
            let padding = col_width.saturating_sub(header_width);
            let left_pad = padding / 2;
            let right_pad = padding - left_pad;
            let header_padded = format!(
                "{}{}{}",
                " ".repeat(left_pad),
                header,
                " ".repeat(right_pad)
            );
            execute!(
                stderr,
                Print(self.config.table_header.paint(&header_padded))
            )?;
        }

        // Right scroll indicator (column separator + ellipsis)
        if has_more_right {
            let sep = self.table_column_separator();
            execute!(
                stderr,
                Print(self.config.table_separator.paint(&sep)),
                Print(self.config.table_separator.paint("…"))
            )?;
        }

        execute!(stderr, Clear(ClearType::UntilNewLine))?;
        Ok(())
    }

    /// Render the separator line between table header and data rows
    fn render_table_header_separator(&self, stderr: &mut Stderr) -> io::Result<()> {
        let Some(layout) = &self.table_layout else {
            return Ok(());
        };

        let prefix_width = self.row_prefix_width();
        let (cols_visible, has_more_right) = self.calculate_visible_columns();
        let has_more_left = self.horizontal_offset > 0;

        let h_char = self.config.table_header_separator;
        let int_char = self.config.table_header_intersection;

        // Render prefix as horizontal line
        let prefix_line: String = std::iter::repeat_n(h_char, prefix_width).collect();
        execute!(
            stderr,
            Print(self.config.table_separator.paint(&prefix_line))
        )?;

        // Left scroll indicator (as horizontal continuation with intersection)
        // Width matches "… │ " = 1 + separator_width
        if has_more_left {
            let left_indicator = format!("{}{}{}{}", h_char, h_char, int_char, h_char);
            execute!(
                stderr,
                Print(self.config.table_separator.paint(&left_indicator))
            )?;
        }

        // Render horizontal lines for visible columns with intersections
        let visible_range = self.horizontal_offset..(self.horizontal_offset + cols_visible);
        for (i, col_idx) in visible_range.enumerate() {
            if col_idx >= layout.col_widths.len() {
                break;
            }

            // Intersection between columns (must match width of column separator " │ ")
            if i > 0 {
                let intersection = format!("{}{}{}", h_char, int_char, h_char);
                execute!(
                    stderr,
                    Print(self.config.table_separator.paint(&intersection))
                )?;
            }

            // Horizontal line for this column's width
            let col_width = layout.col_widths[col_idx];
            let line: String = std::iter::repeat_n(h_char, col_width).collect();
            execute!(stderr, Print(self.config.table_separator.paint(&line)))?;
        }

        // Right scroll indicator (as horizontal continuation with intersection)
        // Width matches " │ …" = separator_width + 1
        if has_more_right {
            let right_indicator = format!("{}{}{}{}", h_char, int_char, h_char, h_char);
            execute!(
                stderr,
                Print(self.config.table_separator.paint(&right_indicator))
            )?;
        }

        execute!(stderr, Clear(ClearType::UntilNewLine))?;
        Ok(())
    }

    /// Render a table row in single-select mode
    fn render_table_row_single(
        &self,
        stderr: &mut Stderr,
        item: &SelectItem,
        active: bool,
    ) -> io::Result<()> {
        let prefix = if active { self.selected_marker() } else { "  " };
        execute!(stderr, Print(prefix))?;
        self.render_table_cells(stderr, item, None)?;
        execute!(stderr, Print(RESET), Clear(ClearType::UntilNewLine))?;
        Ok(())
    }

    /// Render a table row in multi-select mode
    fn render_table_row_multi(
        &self,
        stderr: &mut Stderr,
        item: &SelectItem,
        checked: bool,
        active: bool,
    ) -> io::Result<()> {
        let cursor = if active { self.selected_marker() } else { "  " };
        let checkbox = if checked { "[x] " } else { "[ ] " };
        execute!(stderr, Print(cursor), Print(checkbox))?;
        self.render_table_cells(stderr, item, None)?;
        execute!(stderr, Print(RESET), Clear(ClearType::UntilNewLine))?;
        Ok(())
    }

    /// Render a table row in fuzzy mode with match highlighting
    fn render_table_row_fuzzy(
        &self,
        stderr: &mut Stderr,
        item: &SelectItem,
        active: bool,
    ) -> io::Result<()> {
        let prefix = if active { self.selected_marker() } else { "  " };
        execute!(stderr, Print(prefix))?;

        // Get match indices for highlighting (skip if per_column - handled in render_table_cells)
        let match_indices = if !self.filter_text.is_empty() && !self.per_column {
            self.fuzzy_match_indices(&item.name)
        } else {
            None
        };

        self.render_table_cells(stderr, item, match_indices.as_deref())?;
        execute!(stderr, Print(RESET), Clear(ClearType::UntilNewLine))?;
        Ok(())
    }

    /// Render a table row in fuzzy-multi mode with match highlighting and checkbox
    fn render_table_row_fuzzy_multi(
        &self,
        stderr: &mut Stderr,
        item: &SelectItem,
        checked: bool,
        active: bool,
    ) -> io::Result<()> {
        let cursor = if active { self.selected_marker() } else { "  " };
        let checkbox = if checked { "[x] " } else { "[ ] " };
        execute!(stderr, Print(cursor), Print(checkbox))?;

        // Get match indices for highlighting (skip if per_column - handled in render_table_cells)
        let match_indices = if !self.filter_text.is_empty() && !self.per_column {
            self.fuzzy_match_indices(&item.name)
        } else {
            None
        };

        self.render_table_cells(stderr, item, match_indices.as_deref())?;
        execute!(stderr, Print(RESET), Clear(ClearType::UntilNewLine))?;
        Ok(())
    }

    /// Render table cells with proper alignment and optional fuzzy highlighting
    fn render_table_cells(
        &self,
        stderr: &mut Stderr,
        item: &SelectItem,
        match_indices: Option<&[usize]>,
    ) -> io::Result<()> {
        let Some(layout) = &self.table_layout else {
            return Ok(());
        };
        let Some(cells) = &item.cells else {
            return Ok(());
        };

        let (cols_visible, has_more_right) = self.calculate_visible_columns();
        let has_more_left = self.horizontal_offset > 0;

        // Track if there are matches in hidden columns (for scroll indicator highlighting)
        let mut matches_in_hidden_left = false;
        let mut matches_in_hidden_right = false;

        // For per-column mode, pre-compute match indices for each cell
        let per_column_matches: Vec<Option<Vec<usize>>> =
            if self.per_column && !self.filter_text.is_empty() {
                cells
                    .iter()
                    .map(|(cell_text, _)| self.fuzzy_match_indices(cell_text))
                    .collect()
            } else {
                vec![]
            };

        // Calculate character offset for each cell to map match indices (for non-per-column mode)
        // The search text (item.name) is space-separated cells, so we need to track offsets
        let cell_offsets: Vec<usize> = if match_indices.is_some() {
            let mut offsets = Vec::with_capacity(cells.len());
            let mut offset = 0;
            for (i, (cell_text, _)) in cells.iter().enumerate() {
                offsets.push(offset);
                offset += cell_text.chars().count();
                if i + 1 < cells.len() {
                    offset += 1; // For the space separator
                }
            }
            offsets
        } else {
            vec![]
        };

        // Check for matches in hidden left columns
        if self.per_column && !self.filter_text.is_empty() {
            for col_idx in 0..self.horizontal_offset {
                if col_idx < per_column_matches.len() && per_column_matches[col_idx].is_some() {
                    matches_in_hidden_left = true;
                    break;
                }
            }
        } else if let Some(indices) = match_indices {
            for col_idx in 0..self.horizontal_offset {
                if col_idx < cell_offsets.len() && col_idx + 1 < cell_offsets.len() {
                    let cell_start = cell_offsets[col_idx];
                    let cell_end = cell_offsets[col_idx + 1].saturating_sub(1); // -1 for space
                    if indices.iter().any(|&i| i >= cell_start && i < cell_end) {
                        matches_in_hidden_left = true;
                        break;
                    }
                }
            }
        }

        // Left scroll indicator (ellipsis + column separator)
        if has_more_left {
            let sep = self.table_column_separator();
            if matches_in_hidden_left {
                execute!(
                    stderr,
                    Print(self.config.match_text.paint("…")),
                    Print(self.config.table_separator.paint(&sep))
                )?;
            } else {
                execute!(
                    stderr,
                    Print(self.config.table_separator.paint("…")),
                    Print(self.config.table_separator.paint(&sep))
                )?;
            }
        }

        // Render visible cells
        let visible_range = self.horizontal_offset..(self.horizontal_offset + cols_visible);
        for (i, col_idx) in visible_range.enumerate() {
            if col_idx >= cells.len() {
                break;
            }

            // Separator between columns
            if i > 0 {
                let sep = self.table_column_separator();
                execute!(stderr, Print(self.config.table_separator.paint(&sep)))?;
            }

            let (cell_text, cell_style) = &cells[col_idx];
            let col_width = layout.col_widths[col_idx];

            // Get match indices for this cell
            let cell_matches: Option<Vec<usize>> =
                if self.per_column && !self.filter_text.is_empty() {
                    // Per-column mode: use pre-computed per-cell indices
                    per_column_matches.get(col_idx).cloned().flatten()
                } else if let Some(indices) = match_indices {
                    // Standard mode: map global indices to cell-relative
                    if col_idx < cell_offsets.len() {
                        let cell_start = cell_offsets[col_idx];
                        // Filter indices that fall within this cell and adjust to cell-relative
                        let cell_char_count = cell_text.chars().count();
                        let relative_indices: Vec<usize> = indices
                            .iter()
                            .filter_map(|&idx| {
                                if idx >= cell_start && idx < cell_start + cell_char_count {
                                    Some(idx - cell_start)
                                } else {
                                    None
                                }
                            })
                            .collect();
                        if relative_indices.is_empty() {
                            None
                        } else {
                            Some(relative_indices)
                        }
                    } else {
                        None
                    }
                } else {
                    None
                };

            // Render cell with padding and type-based styling
            self.render_table_cell(
                stderr,
                cell_text,
                cell_style,
                col_width,
                cell_matches.as_deref(),
            )?;
        }

        // Check for matches in hidden right columns
        if self.per_column && !self.filter_text.is_empty() {
            for col_idx in (self.horizontal_offset + cols_visible)..cells.len() {
                if col_idx < per_column_matches.len() && per_column_matches[col_idx].is_some() {
                    matches_in_hidden_right = true;
                    break;
                }
            }
        } else if let Some(indices) = match_indices {
            for col_idx in (self.horizontal_offset + cols_visible)..cells.len() {
                if col_idx < cell_offsets.len() {
                    let cell_start = cell_offsets[col_idx];
                    let cell_end = if col_idx + 1 < cell_offsets.len() {
                        cell_offsets[col_idx + 1].saturating_sub(1)
                    } else {
                        item.name.chars().count()
                    };
                    if indices.iter().any(|&i| i >= cell_start && i < cell_end) {
                        matches_in_hidden_right = true;
                        break;
                    }
                }
            }
        }

        // Right scroll indicator (column separator + ellipsis)
        if has_more_right {
            let sep = self.table_column_separator();
            if matches_in_hidden_right {
                execute!(
                    stderr,
                    Print(self.config.table_separator.paint(&sep)),
                    Print(self.config.match_text.paint("…"))
                )?;
            } else {
                execute!(
                    stderr,
                    Print(self.config.table_separator.paint(&sep)),
                    Print(self.config.table_separator.paint("…"))
                )?;
            }
        }

        Ok(())
    }

    /// Render a single table cell with padding, type-based styling, alignment, and optional match highlighting
    fn render_table_cell(
        &self,
        stderr: &mut Stderr,
        cell: &str,
        cell_style: &TextStyle,
        col_width: usize,
        match_indices: Option<&[usize]>,
    ) -> io::Result<()> {
        let cell_width = terminal_text_width_from(cell, 0);
        let padding_needed = col_width.saturating_sub(cell_width);

        // Calculate left and right padding based on alignment from TextStyle
        let (left_pad, right_pad) = match cell_style.alignment {
            Alignment::Left => (0, padding_needed),
            Alignment::Right => (padding_needed, 0),
            Alignment::Center => {
                let left = padding_needed / 2;
                (left, padding_needed - left)
            }
        };

        // Add left padding
        if left_pad > 0 {
            execute!(stderr, Print(" ".repeat(left_pad)))?;
        }

        // Keep tab expansion cell-relative so the rendered content width matches the width used
        // for table layout and padding.
        let sanitized = sanitize_text_for_display(cell, cell_width, 0);
        self.render_display_segments(stderr, &sanitized, match_indices, cell_style.color_style)?;

        // Add right padding
        if right_pad > 0 {
            execute!(stderr, Print(" ".repeat(right_pad)))?;
        }

        Ok(())
    }

    fn clear_display(&mut self, stderr: &mut Stderr) -> io::Result<()> {
        // In fuzzy mode, cursor may be at filter line; move back to end first
        if self.fuzzy_cursor_offset > 0 {
            execute!(stderr, MoveDown(self.fuzzy_cursor_offset as u16))?;
            self.fuzzy_cursor_offset = 0;
        }

        if self.rendered_lines > 0 {
            // Clear each line by moving up from current position and clearing.
            // This doesn't assume we know exactly where the cursor is.
            // First, move to column 0 and clear current line.
            execute!(stderr, MoveToColumn(0), Clear(ClearType::CurrentLine))?;
            // Then move up and clear each remaining line
            for _ in 1..self.rendered_lines {
                execute!(
                    stderr,
                    MoveUp(1),
                    MoveToColumn(0),
                    Clear(ClearType::CurrentLine)
                )?;
            }
            // Now we're at the first rendered line, which is where output should go
        }
        self.rendered_lines = 0;
        stderr.flush()
    }
}

enum KeyAction {
    Continue,
    Cancel,
    Confirm,
}

#[cfg(test)]
mod test {
    use super::*;

    fn make_widget(items: &[&str]) -> SelectWidget<'static> {
        let options: Vec<SelectItem> = items
            .iter()
            .map(|s| SelectItem {
                name: s.to_string(),
                cells: None,
                value: nu_protocol::Value::nothing(nu_protocol::Span::test_data()),
            })
            .collect();

        SelectWidget::new(
            SelectMode::Single,
            None,
            options,
            InputListConfig::default(),
            None,
            false,
            StreamState {
                stream_reader: None,
                item_generator: None,
            },
        )
    }

    #[test]
    fn wrap_up_and_down_cycles() {
        let mut w = make_widget(&["A", "B", "C"]);
        // navigate up three times, expect proper cycling
        w.navigate_up();
        assert_eq!(w.cursor, 2);
        w.navigate_up();
        assert_eq!(w.cursor, 1);
        w.navigate_up();
        assert_eq!(w.cursor, 0);

        // navigate down three times, expect cycling as well
        w.navigate_down();
        assert_eq!(w.cursor, 1);
        w.navigate_down();
        assert_eq!(w.cursor, 2);
        w.navigate_down();
        assert_eq!(w.cursor, 0);
    }

    #[test]
    fn down_navigation_cycles_with_full_redraw() -> io::Result<()> {
        let mut w = make_widget(&["Banana", "Kiwi", "Pear"]);
        w.first_render = false;
        w.prev_cursor = 0;
        w.prev_scroll_offset = 0;
        w.cursor = 0;
        w.scroll_offset = 0;

        let mut stderr = io::stderr();

        for _ in 0..7 {
            w.navigate_down();
            w.render(&mut stderr)?;
            assert_eq!(w.scroll_offset, 0);
        }

        Ok(())
    }

    #[test]
    fn up_arrow_sequence_state_and_render() -> io::Result<()> {
        let mut w = make_widget(&["Banana", "Kiwi", "Pear"]);
        w.first_render = false;
        w.prev_cursor = 0;
        w.prev_scroll_offset = 0;
        w.cursor = 0;
        w.scroll_offset = 0;

        let mut stderr = io::stderr();

        w.render(&mut stderr)?;
        assert_eq!(w.cursor, 0);

        w.navigate_up();
        w.render(&mut stderr)?;
        assert_eq!(w.cursor, 2);

        w.navigate_up();
        w.render(&mut stderr)?;
        assert_eq!(w.cursor, 1);

        Ok(())
    }

    #[test]
    fn ansi_styled_text_that_visibly_fits_is_not_truncated() {
        let text = "\u{1b}[1;37mabcdef\u{1b}[0m";

        let rendered = truncate_ansi_aware_text(text, 6);

        assert_eq!(
            nu_utils::strip_ansi_unlikely(rendered.as_ref()).as_ref(),
            "abcdef"
        );
        assert!(!rendered.contains('…'));
    }

    #[test]
    fn ansi_styled_text_truncates_by_visible_width() {
        let text = "\u{1b}[1;37mabcdef\u{1b}[0m";

        let rendered = truncate_ansi_aware_text(text, 4);

        assert_eq!(
            nu_utils::strip_ansi_unlikely(rendered.as_ref()).as_ref(),
            "abc…"
        );
    }

    #[test]
    fn tabbed_text_truncates_by_terminal_width() {
        let rendered = truncate_ansi_aware_text("ab\tcdef", 6);

        assert_eq!(rendered.as_ref(), "ab…");
    }

    #[test]
    fn tabbed_text_truncates_from_prefixed_column() {
        let rendered = truncate_ansi_aware_text_at("\t--hostname-bin", 6, 2);

        assert_eq!(rendered.as_ref(), "…");
    }

    #[test]
    fn tabbed_text_expands_when_not_truncated() {
        let rendered = truncate_ansi_aware_text_at("\t--hostname-bin", 32, 2);

        assert_eq!(rendered.as_ref(), "      --hostname-bin");
    }

    #[test]
    fn sanitizer_tracks_source_indices_after_expanding_tabs() {
        let sanitized = sanitize_text_for_display("a\tb\u{7}c", 16, 0);

        assert_eq!(sanitized.text, "a       bc");
        assert_eq!(
            sanitized
                .segments
                .iter()
                .filter_map(|segment| segment.source_index)
                .collect::<Vec<_>>(),
            vec![0, 1, 2, 4]
        );
    }

    #[test]
    fn item_text_width_reserves_prefix() {
        let mut w = make_widget(&[""]);
        w.term_width = 129;

        let available_width = w.item_text_width(2);
        let rendered = truncate_ansi_aware_text_at(
            "\t--hostname-bin # Run a program to get this system's hostname",
            available_width,
            2,
        );

        assert_eq!(available_width, 126);
        assert!(terminal_text_width_from(rendered.as_ref(), 2) <= available_width);
    }

    #[test]
    fn table_layout_uses_sanitized_terminal_width() {
        let span = nu_protocol::Span::test_data();
        let columns = vec!["name".to_string()];
        let items = vec![SelectItem {
            name: "\tname".to_string(),
            cells: Some(vec![("\tname".to_string(), TextStyle::default())]),
            value: Value::nothing(span),
        }];

        let layout = InputList::calculate_table_layout(&columns, &items);

        assert_eq!(layout.col_widths, vec![12]);
    }

    #[test]
    fn fuzzy_filter_does_not_drain_pending_stream() {
        let span = nu_protocol::Span::test_data();
        let mut w = make_widget(&["needle"]);
        w.mode = SelectMode::Fuzzy;
        w.filter_text = "needle".to_string();
        w.filter_cursor = w.filter_text.len();
        w.stream_reader = Some(StreamReader::new(ListStream::new(
            (0..10_000).map(move |i| Value::string(format!("row-{i}"), span)),
            span,
            nu_protocol::Signals::empty(),
        )));

        w.update_filter();

        assert_eq!(w.items.len(), 1);
        assert!(w.stream_is_pending());
    }

    #[test]
    fn initial_read_collects_fast_finite_stream() {
        let span = nu_protocol::Span::test_data();
        let stream = ListStream::new(
            (0..5).map(move |i| Value::int(i, span)),
            span,
            nu_protocol::Signals::empty(),
        );

        let (values, pending_stream) = InputList::read_initial_stream_values(stream);

        assert_eq!(values.len(), 5);
        assert!(pending_stream.is_none());
    }

    #[test]
    fn initial_read_stops_before_exhausting_unbounded_stream() {
        let span = nu_protocol::Span::test_data();
        let stream = ListStream::new(
            (0..).map(move |i| Value::int(i, span)),
            span,
            nu_protocol::Signals::empty(),
        );

        let (values, pending_stream) = InputList::read_initial_stream_values(stream);

        assert!(!values.is_empty());
        assert!(values.len() <= INITIAL_STREAM_MAX_ITEMS);
        assert!(pending_stream.is_some());
    }

    #[test]
    fn initial_read_timeout_does_not_block_on_slow_stream() {
        let span = nu_protocol::Span::test_data();
        let (sender, receiver) = std::sync::mpsc::channel::<Value>();
        let stream = ListStream::new(receiver.into_iter(), span, nu_protocol::Signals::empty());
        let start = nu_utils::time::Instant::now();

        let (values, pending_stream) = InputList::read_initial_stream_values(stream);

        assert!(values.is_empty());
        assert!(pending_stream.is_some());
        assert!(start.elapsed() < INITIAL_STREAM_COLLECT_TIMEOUT * 2);

        drop(sender);
    }

    #[test]
    fn materialized_list_input_is_not_streamed() {
        let span = nu_protocol::Span::test_data();
        let values: Vec<Value> = (0..=INITIAL_STREAM_MAX_ITEMS)
            .map(|i| Value::int(i as i64, span))
            .collect();
        let input = Value::list(values, span).into_pipeline_data();

        let (values, pending_stream) =
            InputList::initial_values_from_input(input, span, nu_protocol::Signals::empty())
                .expect("materialized list input should be accepted");

        assert_eq!(values.len(), INITIAL_STREAM_MAX_ITEMS + 1);
        assert!(pending_stream.is_none());
    }

    #[test]
    fn footer_marks_pending_stream() {
        let span = nu_protocol::Span::test_data();
        let mut w = make_widget(&["one", "two"]);
        w.term_width = 80;
        w.stream_reader = Some(StreamReader::new(ListStream::new(
            (0..100).map(move |i| Value::string(format!("row-{i}"), span)),
            span,
            nu_protocol::Signals::empty(),
        )));

        assert_eq!(w.generate_footer(), "[1-2 of 2 -]");
    }

    #[test]
    fn streaming_footer_updates_at_slower_interval() {
        let span = nu_protocol::Span::test_data();
        let mut w = make_widget(&["one"]);
        let (_sender, receiver) = mpsc::sync_channel(1);
        w.stream_reader = Some(StreamReader {
            receiver,
            finished: false,
        });
        w.items.push(SelectItem {
            name: "two".to_string(),
            cells: None,
            value: Value::string("two", span),
        });
        w.settings_changed = false;

        w.update_stream_footer();

        assert_eq!(w.stream_spinner_frame, 0);
        assert_eq!(w.stream_footer_item_count, 1);
        assert!(!w.settings_changed);

        w.last_stream_footer_update =
            nu_utils::time::Instant::now() - STREAM_FOOTER_UPDATE_INTERVAL;

        w.update_stream_footer();

        assert_eq!(w.stream_spinner_frame, 1);
        assert_eq!(w.stream_footer_item_count, 2);
        assert!(w.settings_changed);
    }

    #[test]
    fn footer_shows_when_items_fill_reserved_area() {
        let mut w = make_widget(&["one", "two"]);
        w.term_width = 80;
        w.visible_height = 2;

        assert!(w.has_footer());
        assert_eq!(w.generate_footer(), "[1-2 of 2]");
    }

    #[test]
    fn final_stream_drain_marks_footer_dirty() {
        let span = nu_protocol::Span::test_data();
        let mut w = make_widget(&["one"]);
        w.term_width = 80;
        let (sender, receiver) = mpsc::sync_channel(8);
        for i in 0..2 {
            sender
                .send(StreamMessage::Item(Value::string(format!("row-{i}"), span)))
                .expect("test stream receiver should be open");
        }
        drop(sender);
        w.stream_reader = Some(StreamReader {
            receiver,
            finished: false,
        });
        w.settings_changed = false;

        assert!(w.load_more_items(STREAM_LOAD_BATCH));

        assert!(w.stream_reader.is_none());
        assert!(w.settings_changed);
        assert_eq!(w.generate_footer(), "[1-3 of 3]");
    }

    #[test]
    fn streamed_table_width_growth_marks_header_dirty() {
        let span = nu_protocol::Span::test_data();
        let columns = vec!["name".to_string()];
        let items = vec![SelectItem {
            name: "sh".to_string(),
            cells: Some(vec![("sh".to_string(), TextStyle::default())]),
            value: Value::nothing(span),
        }];
        let table_layout = InputList::calculate_table_layout(&columns, &items);
        let (sender, receiver) = mpsc::sync_channel(1);
        sender
            .send(StreamMessage::Item(Value::string(
                "long-streamed-name",
                span,
            )))
            .expect("test stream receiver should be open");
        drop(sender);

        let mut w = SelectWidget::new(
            SelectMode::Single,
            None,
            items,
            InputListConfig::default(),
            Some(table_layout),
            false,
            StreamState {
                stream_reader: Some(StreamReader {
                    receiver,
                    finished: false,
                }),
                item_generator: Some(Box::new(move |value| SelectItem {
                    name: "long-streamed-name".to_string(),
                    cells: Some(vec![(
                        "long-streamed-name".to_string(),
                        TextStyle::default(),
                    )]),
                    value,
                })),
            },
        );

        assert!(w.load_more_items(STREAM_LOAD_BATCH));
        assert!(w.table_layout_changed);
    }

    #[test]
    fn end_navigation_uses_available_streamed_rows() {
        let span = nu_protocol::Span::test_data();
        let mut w = make_widget(&["initial"]);
        let (sender, receiver) = mpsc::sync_channel(8);
        for i in 0..5 {
            sender
                .send(StreamMessage::Item(Value::string(format!("row-{i}"), span)))
                .expect("test stream receiver should be open");
        }
        drop(sender);
        w.stream_reader = Some(StreamReader {
            receiver,
            finished: false,
        });

        w.navigate_end();

        assert_eq!(w.items.len(), 6);
        assert_eq!(w.cursor, 5);
        assert!(w.follow_stream_to_end);
    }

    #[test]
    fn test_examples() -> nu_test_support::Result {
        nu_test_support::test().examples(InputList)
    }
}
