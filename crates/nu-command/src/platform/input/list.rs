use crossterm::{
    cursor::{Hide, MoveDown, MoveToColumn, MoveUp, Show},
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    style::Print,
    terminal::{
        self, BeginSynchronizedUpdate, Clear, ClearType, EndSynchronizedUpdate, disable_raw_mode,
        enable_raw_mode,
    },
};
use fuzzy_matcher::{FuzzyMatcher, skim::SkimMatcherV2};
use nu_ansi_term::{Style, ansi::RESET};
use nu_color_config::get_color_map;
use nu_engine::command_prelude::*;
use nu_protocol::shell_error::io::IoError;
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
    match_text: Style,              // For fuzzy match highlighting
    footer: Style,                  // For footer "[1-5 of 10]"
    show_footer: bool,              // Whether to show the footer
    separator: String,              // Character(s) for separator line between search and results
    show_separator: bool,           // Whether to show the separator line
    case_sensitivity: CaseSensitivity, // Fuzzy match case sensitivity
}

impl Default for InputListConfig {
    fn default() -> Self {
        Self {
            match_text: Style::new().bold().italic().underline(),
            footer: Style::new().fg(nu_ansi_term::Color::DarkGray),
            show_footer: true,
            separator: "─".to_string(),
            show_separator: true,
            case_sensitivity: CaseSensitivity::default(),
        }
    }
}

impl InputListConfig {
    fn from_nu_config(config: &nu_protocol::Config) -> Self {
        let mut ret = Self::default();
        let colors = get_color_map(&config.input_list);
        if let Some(s) = colors.get("match_text") {
            ret.match_text = *s;
        }
        if let Some(s) = colors.get("footer") {
            ret.footer = *s;
        }
        if let Some(val) = config.input_list.get("separator") {
            if let Ok(s) = val.as_str() {
                ret.separator = s.to_string();
            }
        }
        if let Some(val) = config.input_list.get("case_sensitive") {
            if let Ok(s) = val.as_str() {
                ret.case_sensitivity = match s {
                    "smart" => CaseSensitivity::Smart,
                    "true" => CaseSensitivity::CaseSensitive,
                    "false" => CaseSensitivity::CaseInsensitive,
                    _ => CaseSensitivity::Smart,
                };
            }
        }
        ret
    }
}

enum InteractMode {
    Single(Option<usize>),
    Multi(Option<Vec<usize>>),
}

struct SelectItem {
    name: String,
    value: Value,
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
                "Use multiple results, you can press a to toggle all options on/off",
                Some('m'),
            )
            .switch("fuzzy", "Use a fuzzy select.", Some('f'))
            .switch("index", "Returns list indexes.", Some('i'))
            .switch(
                "no-footer",
                "Hide the footer showing the number of items",
                Some('n'),
            )
            .switch(
                "no-separator",
                "Hide the separator line between the search box and results",
                None,
            )
            .named(
                "case-sensitive",
                SyntaxShape::String,
                "Case sensitivity for fuzzy matching: 'smart' (case-insensitive unless query has uppercase), 'true', or 'false'",
                Some('s'),
            )
            .named(
                "display",
                SyntaxShape::CellPath,
                "Field to use as display value",
                Some('d'),
            )
            .allow_variants_without_examples(true)
            .category(Category::Platform)
    }

    fn description(&self) -> &str {
        "Interactive list selection with fuzzy search support."
    }

    fn extra_description(&self) -> &str {
        r#"Presents an interactive list in the terminal for selecting items.

Three modes are available:
- Single (default): Select one item with arrow keys, confirm with Enter
- Multi (--multi): Select multiple items with Space, toggle all with 'a'
- Fuzzy (--fuzzy): Type to filter, matches are highlighted

Keyboard shortcuts:
- Up/Down, j/k: Navigate items
- Home/End: Jump to first/last item
- PageUp/PageDown: Navigate by page
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

Configuration ($env.config.input_list):
- match_text: Style for fuzzy match highlighting (default: bold italic underline)
- footer: Style for the footer text (default: dark_gray)
- separator: Character(s) for separator line (default: "─")
- case_sensitive: "smart", "true", or "false" (default: "smart")

Use --no-footer and --no-separator to hide the footer and separator line."#
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["prompt", "ask", "menu", "select", "pick", "choose", "fzf", "fuzzy"]
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
        let display_path: Option<CellPath> = call.get_flag(engine_state, stack, "display")?;
        let no_footer = call.has_flag(engine_state, stack, "no-footer")?;
        let no_separator = call.has_flag(engine_state, stack, "no-separator")?;
        let case_sensitive: Option<String> = call.get_flag(engine_state, stack, "case-sensitive")?;
        let config = stack.get_config(engine_state);
        let mut input_list_config = InputListConfig::from_nu_config(&config);
        if no_footer {
            input_list_config.show_footer = false;
        }
        if no_separator {
            input_list_config.show_separator = false;
        }
        if let Some(cs) = case_sensitive {
            input_list_config.case_sensitivity = match cs.as_str() {
                "smart" => CaseSensitivity::Smart,
                "true" => CaseSensitivity::CaseSensitive,
                "false" => CaseSensitivity::CaseInsensitive,
                _ => {
                    return Err(ShellError::InvalidValue {
                        valid: "'true', 'false', or 'smart'".to_string(),
                        actual: format!("'{cs}'"),
                        span: call.head,
                    });
                }
            };
        }

        let options: Vec<SelectItem> = match input {
            PipelineData::Value(Value::Range { .. }, ..)
            | PipelineData::Value(Value::List { .. }, ..)
            | PipelineData::ListStream { .. } => input
                .into_iter()
                .map(move |val| {
                    let display_value = if let Some(ref cellpath) = display_path {
                        val.follow_cell_path(&cellpath.members)?
                            .to_expanded_string(", ", &config)
                    } else {
                        val.to_expanded_string(", ", &config)
                    };
                    Ok(SelectItem {
                        name: display_value,
                        value: val,
                    })
                })
                .collect::<Result<Vec<_>, ShellError>>()?,

            _ => {
                return Err(ShellError::TypeMismatch {
                    err_message: "expected a list, a table, or a range".to_string(),
                    span: head,
                });
            }
        };

        if options.is_empty() {
            return Err(ShellError::TypeMismatch {
                err_message: "expected a list or table, it can also be a problem with the inner type of your list.".to_string(),
                span: head,
            });
        }

        if multi && fuzzy {
            return Err(ShellError::TypeMismatch {
                err_message: "Fuzzy search is not supported for multi select".to_string(),
                span: head,
            });
        }

        let mode = if multi {
            SelectMode::Multi
        } else if fuzzy {
            SelectMode::Fuzzy
        } else {
            SelectMode::Single
        };

        let mut widget = SelectWidget::new(mode, prompt.as_deref(), &options, input_list_config);
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
                description: "Return a single value from a list",
                example: r#"[1 2 3 4 5] | input list 'Rate it'"#,
                result: None,
            },
            Example {
                description: "Return multiple values from a list",
                example: r#"[Banana Kiwi Pear Peach Strawberry] | input list --multi 'Add fruits to the basket'"#,
                result: None,
            },
            Example {
                description: "Return a single record from a table with fuzzy search",
                example: r#"ls | input list --fuzzy 'Select the target'"#,
                result: None,
            },
            Example {
                description: "Choose an item from a range",
                example: r#"1..10 | input list"#,
                result: None,
            },
            Example {
                description: "Return the index of a selected item",
                example: r#"[Banana Kiwi Pear Peach Strawberry] | input list --index"#,
                result: None,
            },
            Example {
                description: "Choose an item from a table using a column as display value",
                example: r#"[[name price]; [Banana 12] [Kiwi 4] [Pear 7]] | input list -d name"#,
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
        ]
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SelectMode {
    Single,
    Multi,
    Fuzzy,
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
}

impl<'a> SelectWidget<'a> {
    fn new(
        mode: SelectMode,
        prompt: Option<&'a str>,
        items: &'a [SelectItem],
        config: InputListConfig,
    ) -> Self {
        let filtered_indices: Vec<usize> = (0..items.len()).collect();
        let matcher = match config.case_sensitivity {
            CaseSensitivity::Smart => SkimMatcherV2::default().smart_case(),
            CaseSensitivity::CaseSensitive => SkimMatcherV2::default().respect_case(),
            CaseSensitivity::CaseInsensitive => SkimMatcherV2::default().ignore_case(),
        };
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
        }
    }

    /// Generate the separator line based on current terminal width
    fn generate_separator_line(&mut self) {
        let sep_width = self.config.separator.width();
        let repeat_count = if sep_width > 0 {
            self.term_width as usize / sep_width
        } else {
            self.term_width as usize
        };
        self.separator_line = self.config.separator.repeat(repeat_count);
    }

    /// Update terminal dimensions and recalculate visible height
    fn update_term_size(&mut self, width: u16, height: u16) {
        // Subtract 1 to avoid issues with writing to the very last terminal column
        let new_width = width.saturating_sub(1);
        let width_changed = self.term_width != new_width;
        self.term_width = new_width;

        // Regenerate separator line if width changed
        if width_changed && self.config.show_separator {
            self.generate_separator_line();
        }

        // Recalculate visible height
        let mut reserved: u16 = if self.prompt.is_some() { 1 } else { 0 };
        if self.mode == SelectMode::Fuzzy {
            reserved += 1; // filter line
            if self.config.show_separator {
                reserved += 1; // separator line
            }
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

        // Only hide cursor for non-fuzzy modes
        if self.mode != SelectMode::Fuzzy {
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
        // Ctrl+C always cancels
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            return KeyAction::Cancel;
        }

        match self.mode {
            SelectMode::Single => self.handle_single_key(key),
            SelectMode::Multi => self.handle_multi_key(key),
            SelectMode::Fuzzy => self.handle_fuzzy_key(key),
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
                self.transpose_chars();
                self.update_filter();
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

    /// Get the current list length (filtered for fuzzy mode, full for others)
    fn current_list_len(&self) -> usize {
        match self.mode {
            SelectMode::Fuzzy => self.filtered_indices.len(),
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
            self.cursor = (self.cursor + self.visible_height as usize)
                .min(list_len.saturating_sub(1));
            self.adjust_scroll_down();
        } else {
            // Go to bottom of current page
            self.cursor = page_bottom;
        }
    }

    fn toggle_current(&mut self) {
        if self.selected.contains(&self.cursor) {
            self.selected.remove(&self.cursor);
        } else {
            self.selected.insert(self.cursor);
        }
        self.toggled_item = Some(self.cursor);
    }

    fn toggle_all(&mut self) {
        if self.selected.len() == self.items.len() {
            self.selected.clear();
        } else {
            self.selected = (0..self.items.len()).collect();
        }
        self.toggled_all = true;
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
            && pos + 1 <= len
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

    fn update_filter(&mut self) {
        let old_indices = std::mem::take(&mut self.filtered_indices);

        if self.filter_text.is_empty() {
            self.filtered_indices = (0..self.items.len()).collect();
        } else {
            let mut scored: Vec<(usize, i64)> = self
                .items
                .iter()
                .enumerate()
                .filter_map(|(i, opt)| {
                    self.matcher
                        .fuzzy_match(&opt.name, &self.filter_text)
                        .map(|score| (i, score))
                })
                .collect();
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
        }
    }

    /// Check if we can do a cursor-only update (no content change) for Single/Multi mode
    fn can_do_cursor_only_update(&self) -> bool {
        !self.first_render
            && self.mode != SelectMode::Fuzzy
            && self.scroll_offset == self.prev_scroll_offset
            && self.cursor != self.prev_cursor
    }

    /// Check if we can do a cursor-only update in fuzzy mode
    /// (just navigating, no text changes)
    fn can_do_fuzzy_cursor_only_update(&self) -> bool {
        !self.first_render
            && self.mode == SelectMode::Fuzzy
            && !self.filter_text_changed
            && !self.results_changed
            && self.scroll_offset == self.prev_scroll_offset
            && self.cursor != self.prev_cursor
    }

    /// Check if we can do a toggle-only update in multi mode
    /// (just toggled a single visible item, no cursor movement)
    fn can_do_multi_toggle_only_update(&self) -> bool {
        if self.first_render || self.mode != SelectMode::Multi {
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

    /// Check if we can do a toggle-all update in multi mode
    /// (toggled all items with 'a' key)
    fn can_do_multi_toggle_all_update(&self) -> bool {
        !self.first_render && self.mode == SelectMode::Multi && self.toggled_all
    }

    /// Efficient cursor-only update: just change the two prefix characters
    fn render_cursor_update(&mut self, stderr: &mut Stderr) -> io::Result<()> {
        execute!(stderr, BeginSynchronizedUpdate)?;

        let header_lines = if self.prompt.is_some() { 1u16 } else { 0u16 };

        // Calculate display positions relative to scroll
        let prev_display_row = (self.prev_cursor - self.scroll_offset) as u16;
        let curr_display_row = (self.cursor - self.scroll_offset) as u16;

        // Lines to move up from current position (end of last content line)
        // Cursor is at end of last line, not beginning of next line
        let items_rendered = (self.rendered_lines - header_lines as usize) as u16;

        // Move to previous cursor row and clear the '>'
        // Subtract 1 because cursor is on last line, not after it
        let lines_up_to_prev = items_rendered.saturating_sub(1).saturating_sub(prev_display_row);
        execute!(
            stderr,
            MoveUp(lines_up_to_prev),
            MoveToColumn(0),
            Print("  ")
        )?;

        // Move to new cursor row and set the '>'
        if curr_display_row > prev_display_row {
            let lines_down = curr_display_row - prev_display_row;
            execute!(stderr, MoveDown(lines_down), MoveToColumn(0), Print("> "))?;
        } else {
            let lines_up = prev_display_row - curr_display_row;
            execute!(stderr, MoveUp(lines_up), MoveToColumn(0), Print("> "))?;
        }

        // Move back to end position (last content line)
        let lines_down_to_end = items_rendered.saturating_sub(1).saturating_sub(curr_display_row);
        execute!(stderr, MoveDown(lines_down_to_end))?;

        // Update state
        self.prev_cursor = self.cursor;

        execute!(stderr, EndSynchronizedUpdate)?;
        stderr.flush()
    }

    /// Fuzzy mode: cursor-only update (just navigating the list)
    fn render_fuzzy_cursor_update(&mut self, stderr: &mut Stderr) -> io::Result<()> {
        execute!(stderr, BeginSynchronizedUpdate)?;

        // Calculate header lines (prompt + filter + separator)
        let mut header_lines: u16 = if self.prompt.is_some() { 2 } else { 1 };
        if self.config.show_separator {
            header_lines += 1;
        }

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
        let filter_row: u16 = if self.prompt.is_some() { 1 } else { 0 };

        // Clear old cursor: move from filter line to prev item row
        let down_to_prev = prev_item_row.saturating_sub(filter_row);
        execute!(
            stderr,
            MoveDown(down_to_prev),
            MoveToColumn(0),
            Print("  ")
        )?;

        // Draw new cursor: move from prev item row to curr item row
        if curr_item_row > prev_item_row {
            let lines_down = curr_item_row - prev_item_row;
            execute!(stderr, MoveDown(lines_down), MoveToColumn(0), Print("> "))?;
        } else if curr_item_row < prev_item_row {
            let lines_up = prev_item_row - curr_item_row;
            execute!(stderr, MoveUp(lines_up), MoveToColumn(0), Print("> "))?;
        } else {
            // Same row, just redraw
            execute!(stderr, MoveToColumn(0), Print("> "))?;
        }

        // Move back to filter line
        let up_to_filter = curr_item_row.saturating_sub(filter_row);
        execute!(stderr, MoveUp(up_to_filter))?;

        // Position cursor within filter text
        let text_before_cursor = &self.filter_text[..self.filter_cursor];
        let cursor_col = 2 + text_before_cursor.width() as u16;
        execute!(stderr, MoveToColumn(cursor_col))?;

        // Update state
        self.prev_cursor = self.cursor;

        execute!(stderr, EndSynchronizedUpdate)?;
        stderr.flush()
    }

    /// Multi mode: only update the checkbox for the toggled item
    fn render_multi_toggle_only(&mut self, stderr: &mut Stderr) -> io::Result<()> {
        let toggled = self.toggled_item.expect("toggled_item must be Some");
        execute!(stderr, BeginSynchronizedUpdate)?;

        let header_lines = if self.prompt.is_some() { 1u16 } else { 0u16 };

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

        // Move back to end position
        execute!(stderr, MoveDown(lines_up), MoveToColumn(0))?;

        // Reset toggle tracking
        self.toggled_item = None;

        execute!(stderr, EndSynchronizedUpdate)?;
        stderr.flush()
    }

    /// Multi mode: update all visible checkboxes (toggle all with 'a')
    fn render_multi_toggle_all(&mut self, stderr: &mut Stderr) -> io::Result<()> {
        execute!(stderr, BeginSynchronizedUpdate)?;

        let header_lines = if self.prompt.is_some() { 1u16 } else { 0u16 };

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

        // Move back to end position (last content line)
        let remaining = items_rendered as u16 - visible_count as u16;
        if remaining > 0 {
            execute!(stderr, MoveDown(remaining))?;
        }
        // Note: cursor is now at end of last content line (no extra line after)

        // Reset toggle tracking
        self.toggled_all = false;

        execute!(stderr, EndSynchronizedUpdate)?;
        stderr.flush()
    }

    fn render(&mut self, stderr: &mut Stderr) -> io::Result<()> {
        // Check if we can do an efficient cursor-only update
        if self.can_do_cursor_only_update() {
            return self.render_cursor_update(stderr);
        }

        // Check for multi mode toggle-all optimization
        if self.can_do_multi_toggle_all_update() {
            return self.render_multi_toggle_all(stderr);
        }

        // Check for multi mode toggle-only optimization
        if self.can_do_multi_toggle_only_update() {
            return self.render_multi_toggle_only(stderr);
        }

        // Check for fuzzy mode cursor-only update (navigation without typing)
        if self.can_do_fuzzy_cursor_only_update() {
            return self.render_fuzzy_cursor_update(stderr);
        }

        execute!(stderr, BeginSynchronizedUpdate)?;

        // Calculate how many lines we'll render
        let total_count = self.current_list_len();
        let end = (self.scroll_offset + self.visible_height as usize).min(total_count);
        let has_scroll_indicator =
            self.config.show_footer && total_count > self.visible_height as usize;
        let items_to_render = end - self.scroll_offset;

        // Calculate total lines needed for this render
        let mut lines_needed: usize = 0;
        if self.prompt.is_some() {
            lines_needed += 1;
        }
        if self.mode == SelectMode::Fuzzy {
            lines_needed += 1; // filter line
            if self.config.show_separator {
                lines_needed += 1;
            }
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

        // Render filter line for fuzzy mode
        if self.mode == SelectMode::Fuzzy {
            execute!(
                stderr,
                Print("> "),
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
                    Print(&self.separator_line),
                    Clear(ClearType::UntilNewLine),
                )?;
                lines_rendered += 1;
                if lines_rendered < lines_needed {
                    execute!(stderr, MoveDown(1), MoveToColumn(0))?;
                }
            }
        }

        // Render items
        for idx in self.scroll_offset..end {
            let is_active = idx == self.cursor;
            let is_last_line = lines_rendered + 1 == lines_needed;

            match self.mode {
                SelectMode::Single => {
                    let item = &self.items[idx];
                    self.render_single_item_inline(stderr, &item.name, is_active)?;
                }
                SelectMode::Multi => {
                    let item = &self.items[idx];
                    let is_checked = self.selected.contains(&idx);
                    self.render_multi_item_inline(stderr, &item.name, is_checked, is_active)?;
                }
                SelectMode::Fuzzy => {
                    let real_idx = self.filtered_indices[idx];
                    let item = &self.items[real_idx];
                    self.render_fuzzy_item_inline(stderr, &item.name, is_active)?;
                }
            }
            lines_rendered += 1;
            if !is_last_line {
                execute!(stderr, MoveDown(1), MoveToColumn(0))?;
            }
        }

        // Show scroll indicator if needed
        if has_scroll_indicator {
            let indicator = format!(
                "[{}-{} of {}]",
                self.scroll_offset + 1,
                end.min(total_count),
                total_count
            );
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
                execute!(stderr, MoveDown(1), MoveToColumn(0), Clear(ClearType::CurrentLine))?;
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
        self.toggled_item = None;
        self.toggled_all = false;

        // In fuzzy mode, position cursor within filter text
        if self.mode == SelectMode::Fuzzy {
            // Cursor is on last content line, move up to filter line
            let prompt_lines = if self.prompt.is_some() { 1usize } else { 0 };
            self.fuzzy_cursor_offset = lines_rendered.saturating_sub(prompt_lines + 1);
            if self.fuzzy_cursor_offset > 0 {
                execute!(stderr, MoveUp(self.fuzzy_cursor_offset as u16))?;
            }
            // Position cursor after "> " + text up to filter_cursor
            let text_before_cursor = &self.filter_text[..self.filter_cursor];
            let cursor_col = 2 + text_before_cursor.width() as u16;
            execute!(stderr, MoveToColumn(cursor_col))?;
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
        let prefix = if active { "> " } else { "  " };
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
        let cursor = if active { "> " } else { "  " };
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
        let prefix = if active { "> " } else { "  " };
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

        if text_width <= available_width {
            // Text fits, render with highlighting
            for (idx, c) in text.chars().enumerate() {
                if match_indices.contains(&idx) {
                    execute!(stderr, Print(self.config.match_text.paint(c.to_string())))?;
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

            // Render the characters that fit
            for (idx, c) in text.chars().enumerate() {
                if idx >= chars_to_render {
                    break;
                }
                if match_indices.contains(&idx) {
                    execute!(stderr, Print(self.config.match_text.paint(c.to_string())))?;
                } else {
                    execute!(stderr, Print(c))?;
                }
            }

            // Check if any matches are in the truncated portion
            let has_hidden_matches = match_indices.iter().any(|&idx| idx >= chars_to_render);

            if has_hidden_matches {
                execute!(stderr, Print(self.config.match_text.paint("…")))?;
            } else {
                execute!(stderr, Print("…"))?;
            }
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
            // Cursor is on the last content line, move up to first line
            if self.rendered_lines > 1 {
                execute!(stderr, MoveUp((self.rendered_lines - 1) as u16))?;
            }
            execute!(stderr, MoveToColumn(0))?;
            for _ in 0..self.rendered_lines {
                execute!(stderr, Clear(ClearType::CurrentLine), MoveDown(1))?;
            }
            // After clearing, we're one line past the end, move back to start
            execute!(stderr, MoveUp(self.rendered_lines as u16))?;
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
