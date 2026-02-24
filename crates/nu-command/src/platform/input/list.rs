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
use fuzzy_matcher::{FuzzyMatcher, skim::SkimMatcherV2};
use nu_ansi_term::{Style, ansi::RESET};
use nu_color_config::{Alignment, StyleComputer, TextStyle};
use nu_engine::{ClosureEval, command_prelude::*, get_columns};
use nu_protocol::engine::Closure;
use nu_protocol::{TableMode, shell_error::io::IoError};
use nu_table::common::nu_value_to_string;
use std::{
    collections::HashSet,
    io::{self, Stderr, Write},
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

/// Maps TableMode to the appropriate vertical separator character
fn table_mode_to_separator(mode: TableMode) -> char {
    match mode {
        // ASCII-based themes
        TableMode::Basic | TableMode::BasicCompact | TableMode::Psql | TableMode::Markdown => '|',
        TableMode::AsciiRounded => '|',
        // Modern unicode (single line)
        TableMode::Thin | TableMode::Rounded | TableMode::Single | TableMode::Compact => '│',
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
        TableMode::Thin | TableMode::Rounded | TableMode::Single | TableMode::Compact => ('─', '┼'),
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
    fn from_nu_config(config: &nu_protocol::Config, style_computer: &StyleComputer) -> Self {
        let mut ret = Self::default();

        // Get styles from color_config (same as regular table command and find)
        let color_config_header =
            style_computer.compute("header", &Value::string("", Span::unknown()));
        let color_config_separator =
            style_computer.compute("separator", &Value::nothing(Span::unknown()));
        let color_config_search_result =
            style_computer.compute("search_result", &Value::string("", Span::unknown()));
        let color_config_hints = style_computer.compute("hints", &Value::nothing(Span::unknown()));
        let color_config_row_index =
            style_computer.compute("row_index", &Value::string("", Span::unknown()));

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
- Up/Down, j/k: Navigate items
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
        let mut input_list_config = InputListConfig::from_nu_config(&config, &style_computer);
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

        // Collect all values first for table detection
        let values: Vec<Value> = match input {
            PipelineData::Value(Value::Range { .. }, ..)
            | PipelineData::Value(Value::List { .. }, ..)
            | PipelineData::ListStream { .. } => input.into_iter().collect(),
            _ => {
                return Err(ShellError::TypeMismatch {
                    err_message: "expected a list, a table, or a range".to_string(),
                    span: head,
                });
            }
        };

        // Detect table mode: enable if we have columns AND --display is not provided AND --no-table is not set
        let columns = if display_flag.is_none() && !no_table {
            get_columns(&values)
        } else {
            vec![]
        };
        let is_table_mode = !columns.is_empty();

        // Create SelectItems, with cells for table mode
        // Use nu_value_to_string to get consistent formatting and styling with regular tables
        let options: Vec<SelectItem> = if is_table_mode {
            values
                .into_iter()
                .map(|val| {
                    let cells: Vec<(String, TextStyle)> = columns
                        .iter()
                        .map(|col| {
                            if let Value::Record { val: record, .. } = &val {
                                record
                                    .get(col)
                                    .map(|v| nu_value_to_string(v, &config, &style_computer))
                                    .unwrap_or_else(|| (String::new(), TextStyle::default()))
                            } else {
                                (String::new(), TextStyle::default())
                            }
                        })
                        .collect();
                    // Search text is space-separated concatenation of all cell strings
                    let name = cells
                        .iter()
                        .map(|(s, _)| s.as_str())
                        .collect::<Vec<_>>()
                        .join(" ");
                    SelectItem {
                        name,
                        cells: Some(cells),
                        value: val,
                    }
                })
                .collect()
        } else {
            // Handle --display flag: can be CellPath or Closure
            match &display_flag {
                Some(Value::CellPath { val: cellpath, .. }) => values
                    .into_iter()
                    .map(|val| {
                        let display_value = val
                            .follow_cell_path(&cellpath.members)
                            .map(|v| v.to_expanded_string(", ", &config))
                            .unwrap_or_else(|_| val.to_expanded_string(", ", &config));
                        SelectItem {
                            name: display_value,
                            cells: None,
                            value: val,
                        }
                    })
                    .collect(),
                Some(Value::Closure { val: closure, .. }) => {
                    let mut closure_eval =
                        ClosureEval::new(engine_state, stack, Closure::clone(closure));
                    let mut options = Vec::with_capacity(values.len());
                    for val in values {
                        let display_value = closure_eval
                            .run_with_value(val.clone())
                            .and_then(|data| data.into_value(head))
                            .map(|v| v.to_expanded_string(", ", &config))
                            .unwrap_or_else(|_| val.to_expanded_string(", ", &config));
                        options.push(SelectItem {
                            name: display_value,
                            cells: None,
                            value: val,
                        });
                    }
                    options
                }
                None => values
                    .into_iter()
                    .map(|val| {
                        let display_value = val.to_expanded_string(", ", &config);
                        SelectItem {
                            name: display_value,
                            cells: None,
                            value: val,
                        }
                    })
                    .collect(),
                _ => {
                    return Err(ShellError::TypeMismatch {
                        err_message: "expected a cell path or closure for --display".to_string(),
                        span: display_flag.as_ref().map(|v| v.span()).unwrap_or(head),
                    });
                }
            }
        };

        // Calculate table layout if in table mode
        let table_layout = if is_table_mode {
            Some(Self::calculate_table_layout(&columns, &options))
        } else {
            None
        };

        if options.is_empty() {
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

        let mut widget = SelectWidget::new(
            mode,
            prompt.as_deref(),
            &options,
            input_list_config,
            table_layout,
            per_column,
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
                            opts.iter().map(|s| options[*s].value.clone()).collect(),
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
                        Some(opt) => options[opt].value.clone(),
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
                example: r#"[1 2 3 4 5] | input list 'Rate it'"#,
                result: None,
            },
            Example {
                description: "Return multiple values from a list.",
                example: r#"[Banana Kiwi Pear Peach Strawberry] | input list --multi 'Add fruits to the basket'"#,
                result: None,
            },
            Example {
                description: "Return a single record from a table with fuzzy search.",
                example: r#"ls | input list --fuzzy 'Select the target'"#,
                result: None,
            },
            Example {
                description: "Choose an item from a range.",
                example: r#"1..10 | input list"#,
                result: None,
            },
            Example {
                description: "Return the index of a selected item.",
                example: r#"[Banana Kiwi Pear Peach Strawberry] | input list --index"#,
                result: None,
            },
            Example {
                description: "Choose an item from a table using a column as display value.",
                example: r#"[[name price]; [Banana 12] [Kiwi 4] [Pear 7]] | input list -d name"#,
                result: None,
            },
            Example {
                description: "Choose an item using a closure to generate display text",
                example: r#"[[name price]; [Banana 12] [Kiwi 4] [Pear 7]] | input list -d {|it| $"($it.name): $($it.price)"}"#,
                result: None,
            },
            Example {
                description: "Fuzzy search with case-sensitive matching",
                example: r#"[abc ABC aBc] | input list --fuzzy --case-sensitive true"#,
                result: None,
            },
            Example {
                description: "Fuzzy search without the footer showing item count",
                example: r#"ls | input list --fuzzy --no-footer"#,
                result: None,
            },
            Example {
                description: "Fuzzy search without the separator line",
                example: r#"ls | input list --fuzzy --no-separator"#,
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
                example: r#"ls | input list --no-table"#,
                result: None,
            },
            Example {
                description: "Fuzzy search with multiple selection (use Tab to toggle)",
                example: r#"ls | input list --fuzzy --multi"#,
                result: None,
            },
        ]
    }
}

impl InputList {
    /// Calculate column widths for table rendering
    fn calculate_table_layout(columns: &[String], options: &[SelectItem]) -> TableLayout {
        let mut col_widths: Vec<usize> = columns.iter().map(|c| c.width()).collect();

        // Find max width for each column from all rows
        for item in options {
            if let Some(cells) = &item.cells {
                for (i, (cell_text, _)) in cells.iter().enumerate() {
                    if i < col_widths.len() {
                        col_widths[i] = col_widths[i].max(cell_text.width());
                    }
                }
            }
        }

        TableLayout {
            columns: columns.to_vec(),
            col_widths,
            truncated_cols: 0, // Will be calculated when terminal width is known
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SelectMode {
    Single,
    Multi,
    Fuzzy,
    FuzzyMulti,
}

struct SelectWidget<'a> {
    mode: SelectMode,
    prompt: Option<&'a str>,
    items: &'a [SelectItem],
    cursor: usize,
    selected: HashSet<usize>,
    filter_text: String,
    filtered_indices: Vec<usize>,
    scroll_offset: usize,
    visible_height: u16,
    matcher: SkimMatcherV2,
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
    /// Whether the list has been refined to only show selected items (Multi/FuzzyMulti)
    refined: bool,
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
    fn new(
        mode: SelectMode,
        prompt: Option<&'a str>,
        items: &'a [SelectItem],
        config: InputListConfig,
        table_layout: Option<TableLayout>,
        per_column: bool,
    ) -> Self {
        let filtered_indices: Vec<usize> = (0..items.len()).collect();
        let matcher = match config.case_sensitivity {
            CaseSensitivity::Smart => SkimMatcherV2::default().smart_case(),
            CaseSensitivity::CaseSensitive => SkimMatcherV2::default().respect_case(),
            CaseSensitivity::CaseInsensitive => SkimMatcherV2::default().ignore_case(),
        };
        // Pre-compute the selected marker string (doesn't change at runtime)
        let selected_marker_cached = format!(
            "{} ",
            config
                .selected_marker
                .paint(config.selected_marker_char.to_string())
        );
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
            refined: false,
            refined_base_indices: Vec::new(),
            per_column,
            settings_changed: false,
            selected_marker_cached,
            visible_columns_cache: None,
        }
    }

    /// Generate the separator line based on current terminal width
    fn generate_separator_line(&mut self) {
        let sep_width = self.config.separator_char.width();
        let repeat_count = if sep_width > 0 {
            self.term_width as usize / sep_width
        } else {
            self.term_width as usize
        };
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
                self.update_filter();
            }
            self.settings_changed = true;
        }
    }

    /// Rebuild the fuzzy matcher with current case sensitivity setting
    fn rebuild_matcher(&mut self) {
        self.matcher = match self.config.case_sensitivity {
            CaseSensitivity::Smart => SkimMatcherV2::default().smart_case(),
            CaseSensitivity::CaseSensitive => SkimMatcherV2::default().respect_case(),
            CaseSensitivity::CaseInsensitive => SkimMatcherV2::default().ignore_case(),
        };
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

    /// Generate the footer string, truncating if necessary to fit terminal width
    fn generate_footer(&self) -> String {
        let total_count = self.current_list_len();
        let end = (self.scroll_offset + self.visible_height as usize).min(total_count);
        let settings = self.settings_indicator();

        let position_part = if self.is_multi_mode() {
            format!(
                "[{}-{} of {}, {} selected]",
                self.scroll_offset + 1,
                end.min(total_count),
                total_count,
                self.selected.len()
            )
        } else {
            format!(
                "[{}-{} of {}]",
                self.scroll_offset + 1,
                end.min(total_count),
                total_count
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
    /// Footer is always shown in fuzzy modes (for settings display), multi modes (for selection count),
    /// or when the list is longer than visible height (for scroll position)
    fn has_footer(&self) -> bool {
        self.config.show_footer
            && (self.is_fuzzy_mode()
                || self.is_multi_mode()
                || self.current_list_len() > self.visible_height as usize)
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

        enable_raw_mode()?;
        scopeguard::defer! {
            let _ = disable_raw_mode();
        }

        // Only hide cursor for non-fuzzy modes (fuzzy modes need visible cursor for text input)
        if self.mode != SelectMode::Fuzzy && self.mode != SelectMode::FuzzyMulti {
            execute!(stderr, Hide)?;
        }
        scopeguard::defer! {
            let _ = execute!(io::stderr(), Show);
        }

        // Get initial terminal size and cache it
        let (term_width, term_height) = terminal::size()?;
        self.update_term_size(term_width, term_height);

        self.render(&mut stderr)?;

        loop {
            if event::poll(std::time::Duration::from_millis(100))? {
                match event::read()? {
                    Event::Key(key_event) => {
                        match self.handle_key(key_event) {
                            KeyAction::Continue => {}
                            KeyAction::Cancel => {
                                self.clear_display(&mut stderr)?;
                                return Ok(match self.mode {
                                    SelectMode::Multi => InteractMode::Multi(None),
                                    _ => InteractMode::Single(None),
                                });
                            }
                            KeyAction::Confirm => {
                                self.clear_display(&mut stderr)?;
                                return Ok(self.get_result());
                            }
                        }
                        self.render(&mut stderr)?;
                    }
                    Event::Resize(width, height) => {
                        // Clear old content first - terminal reflow may have corrupted positions
                        self.clear_display(&mut stderr)?;
                        self.update_term_size(width, height);
                        // Force full redraw on resize
                        self.first_render = true;
                        self.render(&mut stderr)?;
                    }
                    _ => {}
                }
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
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => KeyAction::Cancel,
            KeyCode::Enter => KeyAction::Confirm,
            KeyCode::Up | KeyCode::Char('k') => {
                self.navigate_up();
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
            KeyCode::Up | KeyCode::Char('k') => {
                self.navigate_up();
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
                KeyAction::Continue
            }
            KeyCode::Char('e' | 'E') if ctrl => {
                // Ctrl-E: Move to end of line
                self.filter_cursor = self.filter_text.len();
                KeyAction::Continue
            }
            KeyCode::Char('b' | 'B') if ctrl => {
                // Ctrl-B: Move back one character
                self.move_filter_cursor_left();
                KeyAction::Continue
            }
            KeyCode::Char('f' | 'F') if ctrl => {
                // Ctrl-F: Move forward one character
                self.move_filter_cursor_right();
                KeyAction::Continue
            }
            KeyCode::Char('b' | 'B') if alt => {
                // Alt-B: Move back one word
                self.move_filter_cursor_word_left();
                KeyAction::Continue
            }
            KeyCode::Char('f' | 'F') if alt => {
                // Alt-F: Move forward one word
                self.move_filter_cursor_word_right();
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
                KeyAction::Continue
            }
            KeyCode::Right if ctrl || alt => {
                // Ctrl/Alt-Right: Move forward one word
                self.move_filter_cursor_word_right();
                KeyAction::Continue
            }
            KeyCode::Left => {
                self.move_filter_cursor_left();
                KeyAction::Continue
            }
            KeyCode::Right => {
                self.move_filter_cursor_right();
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
                self.toggle_current_fuzzy();
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
                self.filter_cursor = 0;
                KeyAction::Continue
            }
            KeyCode::Char('e' | 'E') if ctrl => {
                self.filter_cursor = self.filter_text.len();
                KeyAction::Continue
            }
            KeyCode::Char('b' | 'B') if ctrl => {
                self.move_filter_cursor_left();
                KeyAction::Continue
            }
            KeyCode::Char('f' | 'F') if ctrl => {
                self.move_filter_cursor_right();
                KeyAction::Continue
            }
            KeyCode::Char('b' | 'B') if alt => {
                self.move_filter_cursor_word_left();
                KeyAction::Continue
            }
            KeyCode::Char('f' | 'F') if alt => {
                self.move_filter_cursor_word_right();
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
                KeyAction::Continue
            }
            KeyCode::Right if ctrl || alt => {
                self.move_filter_cursor_word_right();
                KeyAction::Continue
            }
            KeyCode::Left => {
                self.move_filter_cursor_left();
                KeyAction::Continue
            }
            KeyCode::Right => {
                self.move_filter_cursor_right();
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
        let list_len = self.current_list_len();
        if self.cursor > 0 {
            self.cursor -= 1;
            self.adjust_scroll_up();
        } else if list_len > 0 {
            // Wrap to bottom
            self.cursor = list_len - 1;
            self.adjust_scroll_down();
        }
    }

    /// Move cursor down with wrapping
    fn navigate_down(&mut self) {
        let list_len = self.current_list_len();
        if self.cursor + 1 < list_len {
            self.cursor += 1;
            self.adjust_scroll_down();
        } else {
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
        self.cursor = 0;
        self.scroll_offset = 0;
    }

    /// Navigate to the end of the list
    fn navigate_end(&mut self) {
        self.cursor = self.current_list_len().saturating_sub(1);
        self.adjust_scroll_down();
    }

    /// Navigate page up: go to top of current page, or previous page if already at top
    fn navigate_page_up(&mut self) {
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

    /// Score an item using per-column matching (best column wins)
    fn score_per_column(&self, item: &SelectItem) -> Option<i64> {
        item.cells.as_ref().and_then(|cells| {
            cells
                .iter()
                .filter_map(|(cell_text, _)| self.matcher.fuzzy_match(cell_text, &self.filter_text))
                .max()
        })
    }

    /// Score an item - uses per-column matching if enabled and in table mode
    fn score_item(&self, item: &SelectItem) -> Option<i64> {
        if self.per_column && item.cells.is_some() {
            self.score_per_column(item)
        } else {
            self.matcher.fuzzy_match(&item.name, &self.filter_text)
        }
    }

    fn update_filter(&mut self) {
        let old_indices = std::mem::take(&mut self.filtered_indices);

        // Determine whether to filter from refined subset or all items
        let use_refined = self.refined && !self.refined_base_indices.is_empty();

        if self.filter_text.is_empty() {
            // When empty, copy the base indices
            self.filtered_indices = if use_refined {
                self.refined_base_indices.clone()
            } else {
                (0..self.items.len()).collect()
            };
        } else {
            // When filtering, iterate without cloning the base indices
            let mut scored: Vec<(usize, i64)> = if use_refined {
                self.refined_base_indices
                    .iter()
                    .filter_map(|&i| self.score_item(&self.items[i]).map(|score| (i, score)))
                    .collect()
            } else {
                (0..self.items.len())
                    .filter_map(|i| self.score_item(&self.items[i]).map(|score| (i, score)))
                    .collect()
            };
            // Sort by score descending
            scored.sort_by(|a, b| b.1.cmp(&a.1));
            self.filtered_indices = scored.into_iter().map(|(i, _)| i).collect();
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
                if self
                    .matcher
                    .fuzzy_match(cell_text, &self.filter_text)
                    .is_some()
                {
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

                if let Some((_, indices)) =
                    self.matcher.fuzzy_indices(&item.name, &self.filter_text)
                {
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

    /// Check if we can do a cursor-only update in fuzzy mode
    /// (just navigating, no text changes, no toggles)
    fn can_do_fuzzy_cursor_only_update(&self) -> bool {
        !self.first_render
            && !self.width_changed
            && (self.mode == SelectMode::Fuzzy || self.mode == SelectMode::FuzzyMulti)
            && !self.filter_text_changed
            && !self.results_changed
            && self.scroll_offset == self.prev_scroll_offset
            && self.cursor != self.prev_cursor
            && self.toggled_item.is_none() // FuzzyMulti: no item was toggled
            && !self.toggled_all // FuzzyMulti: Alt+A toggled all items
    }

    /// Check if we can do a toggle-only update in multi mode
    /// (just toggled a single visible item, no cursor movement)
    fn can_do_multi_toggle_only_update(&self) -> bool {
        if self.first_render || self.width_changed || self.mode != SelectMode::Multi {
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
    }

    /// Check if we can do a toggle-all update in multi mode
    /// (toggled all items with 'a' key)
    fn can_do_multi_toggle_all_update(&self) -> bool {
        !self.first_render
            && !self.width_changed
            && self.mode == SelectMode::Multi
            && self.toggled_all
    }

    /// Check if we can do a cursor-only update in single/multi mode
    /// (just navigating without scrolling or horizontal scroll changes)
    fn can_do_cursor_only_update(&self) -> bool {
        !self.first_render
            && !self.width_changed
            && (self.mode == SelectMode::Single || self.mode == SelectMode::Multi)
            && self.scroll_offset == self.prev_scroll_offset
            && self.cursor != self.prev_cursor
            && !self.horizontal_scroll_changed
            && self.toggled_item.is_none() // Multi mode: no item was toggled
            && !self.toggled_all // Multi mode: 'a' wasn't pressed
    }

    /// Single/Multi mode: cursor-only update (just update the selection markers)
    fn render_cursor_only_update(&mut self, stderr: &mut Stderr) -> io::Result<()> {
        execute!(stderr, BeginSynchronizedUpdate)?;

        // Calculate header lines (prompt + table header + table header separator)
        let mut header_lines: u16 = if self.prompt.is_some() { 1 } else { 0 };
        if self.is_table_mode() {
            header_lines += 2; // table header + header separator line
        }

        // Display rows are 0-indexed within the visible items area
        let prev_display_row = (self.prev_cursor - self.scroll_offset) as u16;
        let curr_display_row = (self.cursor - self.scroll_offset) as u16;

        // Cursor is at the end of the last rendered content line
        // rendered_lines includes header + items + footer
        // We need to go from there to the previous cursor row, then to the new cursor row

        // Calculate how many item lines were rendered
        let footer_lines: u16 = if self.config.show_footer
            && (self.is_multi_mode() || self.current_list_len() > self.visible_height as usize)
        {
            1
        } else {
            0
        };
        let items_rendered = self.rendered_lines - header_lines as usize - footer_lines as usize;

        // Current position is at last rendered line. Move up to first item row.
        let last_item_display_row = (items_rendered as u16).saturating_sub(1);

        // Move from last line to prev cursor row
        // Last line = header_lines + last_item_display_row + footer_lines
        // Prev item = header_lines + prev_display_row
        let lines_up_to_prev = last_item_display_row + footer_lines - prev_display_row;
        execute!(stderr, MoveUp(lines_up_to_prev), MoveToColumn(0))?;

        // Clear the old marker
        execute!(stderr, Print("  "))?;

        // Move to new cursor row and draw marker
        let marker = self.selected_marker();
        if curr_display_row > prev_display_row {
            let lines_down = curr_display_row - prev_display_row;
            execute!(
                stderr,
                MoveDown(lines_down),
                MoveToColumn(0),
                Print(&marker)
            )?;
        } else if curr_display_row < prev_display_row {
            let lines_up = prev_display_row - curr_display_row;
            execute!(stderr, MoveUp(lines_up), MoveToColumn(0), Print(&marker))?;
        } else {
            // Same row (shouldn't happen since cursor != prev_cursor), just redraw
            execute!(stderr, MoveToColumn(0), Print(&marker))?;
        }

        // Move back to the last rendered line (where cursor should be at end of render)
        let lines_down_to_end = last_item_display_row + footer_lines - curr_display_row;
        execute!(stderr, MoveDown(lines_down_to_end))?;

        // Update state
        self.prev_cursor = self.cursor;

        execute!(stderr, EndSynchronizedUpdate)?;
        stderr.flush()
    }

    /// Fuzzy mode: cursor-only update (just navigating the list)
    fn render_fuzzy_cursor_update(&mut self, stderr: &mut Stderr) -> io::Result<()> {
        execute!(stderr, BeginSynchronizedUpdate)?;

        // Calculate header lines (prompt + filter + separator + table header + table header separator)
        let header_lines = self.fuzzy_header_lines();

        // Display rows are 0-indexed within the visible items area
        let prev_display_row = (self.prev_cursor - self.scroll_offset) as u16;
        let curr_display_row = (self.cursor - self.scroll_offset) as u16;

        // Calculate absolute row positions from the top of our render area:
        // - Row 0: prompt (if present)
        // - Row 1 (or 0): filter line
        // - Row 2 (or 1): separator (if enabled)
        // - Remaining rows: items
        // header_lines = rows before items (prompt + filter + separator as applicable)
        let prev_item_row = header_lines + prev_display_row;
        let curr_item_row = header_lines + curr_display_row;

        // We're at the filter line, which is row 1 if prompt exists, row 0 otherwise
        let filter_row = self.fuzzy_filter_row();

        // Clear old cursor: move from filter line to prev item row
        let down_to_prev = prev_item_row.saturating_sub(filter_row);
        execute!(stderr, MoveDown(down_to_prev), MoveToColumn(0), Print("  "))?;

        // Draw new cursor: move from prev item row to curr item row
        let marker = self.selected_marker();
        if curr_item_row > prev_item_row {
            let lines_down = curr_item_row - prev_item_row;
            execute!(
                stderr,
                MoveDown(lines_down),
                MoveToColumn(0),
                Print(&marker)
            )?;
        } else if curr_item_row < prev_item_row {
            let lines_up = prev_item_row - curr_item_row;
            execute!(stderr, MoveUp(lines_up), MoveToColumn(0), Print(&marker))?;
        } else {
            // Same row, just redraw
            execute!(stderr, MoveToColumn(0), Print(&marker))?;
        }

        // Move back to filter line
        let up_to_filter = curr_item_row.saturating_sub(filter_row);
        execute!(stderr, MoveUp(up_to_filter))?;

        // Position cursor within filter text
        self.position_fuzzy_cursor(stderr)?;

        // Update state
        self.prev_cursor = self.cursor;

        execute!(stderr, EndSynchronizedUpdate)?;
        stderr.flush()
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

        // Check for fuzzy mode cursor-only update (navigation without typing)
        if self.can_do_fuzzy_cursor_only_update() {
            return self.render_fuzzy_cursor_update(stderr);
        }

        // Check for single/multi mode cursor-only update (navigation without scrolling)
        if self.can_do_cursor_only_update() {
            return self.render_cursor_only_update(stderr);
        }

        // If nothing changed (e.g., PageDown at bottom of list), skip render entirely
        if !self.first_render
            && !self.width_changed
            && self.cursor == self.prev_cursor
            && self.scroll_offset == self.prev_scroll_offset
            && !self.results_changed
            && !self.filter_text_changed
            && !self.horizontal_scroll_changed
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

        // Render table header and separator if in table mode
        // Only redraw if first render or horizontal scroll changed
        if self.is_table_mode() {
            let need_header_redraw = self.first_render || self.horizontal_scroll_changed;
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
        } else if let Some((_score, indices)) = self.matcher.fuzzy_indices(text, &self.filter_text)
        {
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
        } else if let Some((_score, indices)) = self.matcher.fuzzy_indices(text, &self.filter_text)
        {
            self.render_truncated_fuzzy_text(stderr, text, &indices, prefix_width)?;
        } else {
            self.render_truncated_text(stderr, text, prefix_width)?;
        }
        execute!(stderr, Print(RESET), Clear(ClearType::UntilNewLine))?;
        Ok(())
    }

    /// Render text, truncating with ellipsis if it exceeds available width.
    fn render_truncated_text(
        &self,
        stderr: &mut Stderr,
        text: &str,
        prefix_width: usize,
    ) -> io::Result<()> {
        let available_width = (self.term_width as usize).saturating_sub(prefix_width);
        let text_width = UnicodeWidthStr::width(text);

        if text_width <= available_width {
            // Text fits, render as-is
            execute!(stderr, Print(text))?;
        } else if available_width <= 1 {
            // Only room for ellipsis
            execute!(stderr, Print("…"))?;
        } else {
            // Find the substring that fits in available_width - 1 (reserve 1 for ellipsis)
            let target_width = available_width - 1;
            let mut current_width = 0;
            let mut end_pos = 0;

            for (byte_pos, c) in text.char_indices() {
                let char_width = UnicodeWidthChar::width(c).unwrap_or(0);
                if current_width + char_width > target_width {
                    break;
                }
                end_pos = byte_pos + c.len_utf8();
                current_width += char_width;
            }
            execute!(stderr, Print(&text[..end_pos]))?;
            execute!(stderr, Print("…"))?;
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
        let available_width = (self.term_width as usize).saturating_sub(prefix_width);
        let text_width = UnicodeWidthStr::width(text);

        // Reusable single-char buffer for styled output (avoids allocation per char)
        let mut char_buf = [0u8; 4];

        if text_width <= available_width {
            // Text fits, render with highlighting.
            // match_indices is sorted, so use two-pointer approach for O(n) instead of O(n*m)
            let mut match_iter = match_indices.iter().peekable();
            for (idx, c) in text.chars().enumerate() {
                // Advance match_iter past any indices we've passed
                while match_iter.peek().is_some_and(|&&i| i < idx) {
                    match_iter.next();
                }
                let is_match = match_iter.peek().is_some_and(|&&i| i == idx);
                if is_match {
                    let s = c.encode_utf8(&mut char_buf);
                    execute!(stderr, Print(self.config.match_text.paint(&*s)))?;
                } else {
                    execute!(stderr, Print(c))?;
                }
            }
        } else if available_width <= 1 {
            // Only room for ellipsis
            let has_any_matches = !match_indices.is_empty();
            if has_any_matches {
                execute!(stderr, Print(self.config.match_text.paint("…")))?;
            } else {
                execute!(stderr, Print("…"))?;
            }
        } else {
            // Find how many chars fit in available_width - 1 (reserve 1 for ellipsis)
            let target_width = available_width - 1;
            let mut current_width = 0;
            let mut chars_to_render: usize = 0;

            for c in text.chars() {
                let char_width = UnicodeWidthChar::width(c).unwrap_or(0);
                if current_width + char_width > target_width {
                    break;
                }
                current_width += char_width;
                chars_to_render += 1;
            }

            // Render the characters that fit, using two-pointer approach for efficiency
            let mut match_iter = match_indices.iter().peekable();
            for (idx, c) in text.chars().enumerate() {
                if idx >= chars_to_render {
                    break;
                }
                while match_iter.peek().is_some_and(|&&i| i < idx) {
                    match_iter.next();
                }
                let is_match = match_iter.peek().is_some_and(|&&i| i == idx);
                if is_match {
                    let s = c.encode_utf8(&mut char_buf);
                    execute!(stderr, Print(self.config.match_text.paint(&*s)))?;
                } else {
                    execute!(stderr, Print(c))?;
                }
            }

            // Check if any matches are in the truncated portion (remaining in match_iter)
            let has_hidden_matches = match_iter.any(|&idx| idx >= chars_to_render);

            if has_hidden_matches {
                execute!(stderr, Print(self.config.match_text.paint("…")))?;
            } else {
                execute!(stderr, Print("…"))?;
            }
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
            self.matcher
                .fuzzy_indices(&item.name, &self.filter_text)
                .map(|(_, indices)| indices)
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
            self.matcher
                .fuzzy_indices(&item.name, &self.filter_text)
                .map(|(_, indices)| indices)
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
                    .map(|(cell_text, _)| {
                        self.matcher
                            .fuzzy_indices(cell_text, &self.filter_text)
                            .map(|(_, indices)| indices)
                    })
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
        let cell_width = cell.width();
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

        if let Some(indices) = match_indices {
            // Render with fuzzy highlighting (match highlighting takes priority over type styling)
            let mut char_buf = [0u8; 4];
            let mut match_iter = indices.iter().peekable();

            for (idx, c) in cell.chars().enumerate() {
                while match_iter.peek().is_some_and(|&&i| i < idx) {
                    match_iter.next();
                }
                let is_match = match_iter.peek().is_some_and(|&&i| i == idx);
                if is_match {
                    let s = c.encode_utf8(&mut char_buf);
                    execute!(stderr, Print(self.config.match_text.paint(&*s)))?;
                } else {
                    // Apply type-based style for non-match characters
                    let s = c.encode_utf8(&mut char_buf);
                    if let Some(color) = cell_style.color_style {
                        execute!(stderr, Print(color.paint(&*s)))?;
                    } else {
                        execute!(stderr, Print(&*s))?;
                    }
                }
            }
        } else {
            // Render with type-based styling
            if let Some(color) = cell_style.color_style {
                execute!(stderr, Print(color.paint(cell)))?;
            } else {
                execute!(stderr, Print(cell))?;
            }
        }

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

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(InputList {})
    }
}
