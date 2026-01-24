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
use unicode_width::UnicodeWidthStr;

#[derive(Debug, Clone)]
struct InputListConfig {
    match_text: Style,     // For fuzzy match highlighting
    footer: Style,         // For footer "[1-5 of 10]"
    separator_style: Style, // For separator line
    show_footer: bool,     // Whether to show the footer
    separator: String,     // Character(s) for separator line between search and results
    show_separator: bool,  // Whether to show the separator line
}

impl Default for InputListConfig {
    fn default() -> Self {
        Self {
            match_text: Style::new().italic().underline(),
            footer: Style::new().fg(nu_ansi_term::Color::DarkGray),
            separator_style: Style::new(),
            show_footer: true,
            separator: "─".to_string(),
            show_separator: true,
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
        if let Some(s) = colors.get("separator_style") {
            ret.separator_style = *s;
        }
        if let Some(val) = config.input_list.get("separator") {
            if let Ok(s) = val.as_str() {
                ret.separator = s.to_string();
            }
        }
        ret
    }
}

enum InteractMode {
    Single(Option<usize>),
    Multi(Option<Vec<usize>>),
}

struct Options {
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
                "display",
                SyntaxShape::CellPath,
                "Field to use as display value",
                Some('d'),
            )
            .allow_variants_without_examples(true)
            .category(Category::Platform)
    }

    fn description(&self) -> &str {
        "Interactive list selection."
    }

    fn extra_description(&self) -> &str {
        r#"Keybindings:
- Single/Multi mode: up/down or j/k to navigate, enter to confirm, esc or q to cancel
- Multi mode: space to toggle selection, a to toggle all
- Fuzzy mode: type to filter, up/down or ctrl+p/n to navigate results, enter to confirm, esc to cancel
  Readline-style editing: ctrl+a/e (home/end), ctrl+b/f (char left/right), alt+b/f (word left/right),
  ctrl+u/k (delete to start/end), ctrl+w (delete word back), ctrl+d (delete char), ctrl+t (transpose)
- All modes: home/end for first/last item, pageup/pagedown for pagination

Configuration (in $config.input_list):
- match_text: style for fuzzy match highlighting (default: italic underline)
- footer: style for the footer text (default: dark gray)
- separator_style: style for the separator line (default: none)
- separator: character(s) for the separator line (default: "─")"#
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["prompt", "ask", "menu"]
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
        let config = stack.get_config(engine_state);
        let mut input_list_config = InputListConfig::from_nu_config(&config);
        if no_footer {
            input_list_config.show_footer = false;
        }
        if no_separator {
            input_list_config.show_separator = false;
        }

        let options: Vec<Options> = match input {
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
                    Ok(Options {
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
                err_message: "expected a list or table, it can also be a problem with the an inner type of your list.".to_string(),
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
                description: "Select from a list with a minimal UI (no footer or separator)",
                example: r#"[1 2 3] | input list --no-footer --no-separator"#,
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
    items: &'a [Options],
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
        items: &'a [Options],
        config: InputListConfig,
    ) -> Self {
        let filtered_indices: Vec<usize> = (0..items.len()).collect();
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
            matcher: SkimMatcherV2::default(),
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
        let old_width = self.term_width;
        self.term_width = width;

        // Regenerate separator line if width changed
        if old_width != width && self.config.show_separator {
            self.generate_separator_line();
        }

        // Recalculate visible height
        let mut reserved: u16 = 1; // prompt
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
                self.move_cursor_up();
                KeyAction::Continue
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.move_cursor_down();
                KeyAction::Continue
            }
            KeyCode::Home => {
                self.cursor = 0;
                self.scroll_offset = 0;
                KeyAction::Continue
            }
            KeyCode::End => {
                self.cursor = self.items.len().saturating_sub(1);
                self.adjust_scroll_down();
                KeyAction::Continue
            }
            KeyCode::PageUp => {
                self.cursor = self.cursor.saturating_sub(self.visible_height as usize);
                self.adjust_scroll_down();
                KeyAction::Continue
            }
            KeyCode::PageDown => {
                self.cursor = (self.cursor + self.visible_height as usize)
                    .min(self.items.len().saturating_sub(1));
                self.adjust_scroll_down();
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
                self.move_cursor_up();
                KeyAction::Continue
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.move_cursor_down();
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
                self.cursor = 0;
                self.scroll_offset = 0;
                KeyAction::Continue
            }
            KeyCode::End => {
                self.cursor = self.items.len().saturating_sub(1);
                self.adjust_scroll_down();
                KeyAction::Continue
            }
            KeyCode::PageUp => {
                self.cursor = self.cursor.saturating_sub(self.visible_height as usize);
                self.adjust_scroll_down();
                KeyAction::Continue
            }
            KeyCode::PageDown => {
                self.cursor = (self.cursor + self.visible_height as usize)
                    .min(self.items.len().saturating_sub(1));
                self.adjust_scroll_down();
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
                self.move_fuzzy_cursor_up();
                KeyAction::Continue
            }
            KeyCode::Down | KeyCode::Char('n' | 'N') if ctrl => {
                self.move_fuzzy_cursor_down();
                KeyAction::Continue
            }
            KeyCode::Up => {
                self.move_fuzzy_cursor_up();
                KeyAction::Continue
            }
            KeyCode::Down => {
                self.move_fuzzy_cursor_down();
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
                // Delete character before cursor
                if self.filter_cursor > 0 {
                    self.filter_cursor -= 1;
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
                self.filter_cursor += 1;
                self.update_filter();
                KeyAction::Continue
            }

            // List navigation with Home/End/PageUp/PageDown
            KeyCode::Home => {
                self.cursor = 0;
                self.scroll_offset = 0;
                KeyAction::Continue
            }
            KeyCode::End => {
                self.cursor = self.filtered_indices.len().saturating_sub(1);
                self.adjust_scroll_down();
                KeyAction::Continue
            }
            KeyCode::PageUp => {
                // Go to top of current page, or previous page if already at top
                let page_top = self.scroll_offset;
                if self.cursor == page_top {
                    // Already at top of page, go to previous page
                    self.cursor = self.cursor.saturating_sub(self.visible_height as usize);
                    self.adjust_scroll_up();
                } else {
                    // Go to top of current page
                    self.cursor = page_top;
                }
                KeyAction::Continue
            }
            KeyCode::PageDown => {
                // Go to bottom of current page, or next page if already at bottom
                let list_len = self.filtered_indices.len();
                let page_bottom =
                    (self.scroll_offset + self.visible_height as usize - 1).min(list_len - 1);
                if self.cursor == page_bottom {
                    // Already at bottom of page, go to next page
                    self.cursor = (self.cursor + self.visible_height as usize)
                        .min(list_len.saturating_sub(1));
                    self.adjust_scroll_down();
                } else {
                    // Go to bottom of current page
                    self.cursor = page_bottom;
                }
                KeyAction::Continue
            }
            _ => KeyAction::Continue,
        }
    }

    fn move_cursor_up(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
            if self.cursor < self.scroll_offset {
                self.scroll_offset = self.cursor;
            }
        } else if !self.items.is_empty() {
            // Wrap to bottom
            self.cursor = self.items.len() - 1;
            self.adjust_scroll_down();
        }
    }

    fn move_cursor_down(&mut self) {
        if self.cursor + 1 < self.items.len() {
            self.cursor += 1;
            self.adjust_scroll_down();
        } else {
            // Wrap to top
            self.cursor = 0;
            self.scroll_offset = 0;
        }
    }

    fn move_fuzzy_cursor_up(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
            if self.cursor < self.scroll_offset {
                self.scroll_offset = self.cursor;
            }
        } else if !self.filtered_indices.is_empty() {
            // Wrap to bottom
            self.cursor = self.filtered_indices.len() - 1;
            self.adjust_scroll_down();
        }
    }

    fn move_fuzzy_cursor_down(&mut self) {
        if self.cursor + 1 < self.filtered_indices.len() {
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

        // Lines to move up from current position (end of rendered content)
        // Current position is after all rendered lines
        let items_rendered = self.rendered_lines - header_lines as usize;

        // Move to previous cursor row and clear the '>'
        let lines_up_to_prev = items_rendered as u16 - prev_display_row;
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

        // Move back to end position
        let lines_down_to_end = items_rendered as u16 - curr_display_row;
        execute!(stderr, MoveDown(lines_down_to_end))?;

        // Update state
        self.prev_cursor = self.cursor;

        execute!(stderr, EndSynchronizedUpdate)?;
        stderr.flush()
    }

    /// Fuzzy mode: cursor-only update (just navigating the list)
    fn render_fuzzy_cursor_update(&mut self, stderr: &mut Stderr) -> io::Result<()> {
        execute!(stderr, BeginSynchronizedUpdate)?;

        // Calculate header offset (prompt + filter line + separator)
        let mut header_lines: u16 = if self.prompt.is_some() { 2 } else { 1 };
        if self.config.show_separator {
            header_lines += 1;
        }

        // Calculate display positions relative to scroll
        let prev_display_row = (self.prev_cursor - self.scroll_offset) as u16;
        let curr_display_row = (self.cursor - self.scroll_offset) as u16;

        // We're at the filter line; need to move down to items area first
        // Items start after filter line (which is 1 line below prompt or at line 1)
        // fuzzy_cursor_offset tells us how many lines up we are from the end

        // Move down from filter line to end of rendered area
        execute!(stderr, MoveDown(self.fuzzy_cursor_offset as u16))?;

        // Now at end of rendered content; figure out lines to items
        let items_rendered = self.rendered_lines - header_lines as usize;

        // Move to previous cursor row and clear the '>'
        let lines_up_to_prev = items_rendered as u16 - prev_display_row;
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

        // Move back to end position, then up to filter line
        let lines_down_to_end = items_rendered as u16 - curr_display_row;
        execute!(stderr, MoveDown(lines_down_to_end))?;

        // Now move back up to filter line
        execute!(stderr, MoveUp(self.fuzzy_cursor_offset as u16))?;

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
        let lines_up = items_rendered as u16 - display_row;
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
        execute!(stderr, MoveUp(items_rendered as u16))?;

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

        // Move back to end position
        let remaining = items_rendered as u16 - visible_count as u16;
        if remaining > 0 {
            execute!(stderr, MoveDown(remaining))?;
        }
        // Move down one more to get past the last item line
        execute!(stderr, MoveDown(1), MoveToColumn(0))?;

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

        // In fuzzy mode, cursor may be at filter line; move back to end first
        if self.fuzzy_cursor_offset > 0 {
            execute!(stderr, MoveDown(self.fuzzy_cursor_offset as u16))?;
            self.fuzzy_cursor_offset = 0;
        }

        // Move to start of our render area
        if self.rendered_lines > 0 {
            execute!(stderr, MoveUp(self.rendered_lines as u16), MoveToColumn(0))?;
        }

        let mut lines_rendered = 0;

        // Render prompt (only on first render, it doesn't change)
        if self.first_render {
            if let Some(prompt) = self.prompt {
                execute!(
                    stderr,
                    Print(prompt),
                    Clear(ClearType::UntilNewLine),
                    Print("\r\n")
                )?;
            }
        } else if self.prompt.is_some() {
            // Skip past prompt line
            execute!(stderr, MoveDown(1))?;
        }
        if self.prompt.is_some() {
            lines_rendered += 1;
        }

        // Render filter line for fuzzy mode
        if self.mode == SelectMode::Fuzzy {
            execute!(
                stderr,
                Print("> "),
                Print(&self.filter_text),
                Clear(ClearType::UntilNewLine),
                Print("\r\n")
            )?;
            lines_rendered += 1;

            // Render separator line (uses cached separator_line, regenerated on resize)
            if self.config.show_separator {
                execute!(
                    stderr,
                    Print(self.config.separator_style.paint(&self.separator_line)),
                    Clear(ClearType::UntilNewLine),
                    Print("\r\n")
                )?;
                lines_rendered += 1;
            }
        }

        // Calculate which items to show
        let (display_items, total_count) = match self.mode {
            SelectMode::Fuzzy => {
                let count = self.filtered_indices.len();
                let end = (self.scroll_offset + self.visible_height as usize).min(count);
                let indices: Vec<usize> = (self.scroll_offset..end).collect();
                (indices, count)
            }
            _ => {
                let count = self.items.len();
                let end = (self.scroll_offset + self.visible_height as usize).min(count);
                let indices: Vec<usize> = (self.scroll_offset..end).collect();
                (indices, count)
            }
        };

        // Render items with clear to end of line
        for (display_idx, idx) in display_items.iter().enumerate() {
            let actual_cursor_pos = self.scroll_offset + display_idx;
            let is_active = actual_cursor_pos == self.cursor;

            match self.mode {
                SelectMode::Single => {
                    let item = &self.items[*idx];
                    self.render_single_item_inline(stderr, &item.name, is_active)?;
                }
                SelectMode::Multi => {
                    let item = &self.items[*idx];
                    let is_checked = self.selected.contains(idx);
                    self.render_multi_item_inline(stderr, &item.name, is_checked, is_active)?;
                }
                SelectMode::Fuzzy => {
                    let real_idx = self.filtered_indices[*idx];
                    let item = &self.items[real_idx];
                    self.render_fuzzy_item_inline(stderr, &item.name, is_active)?;
                }
            }
            lines_rendered += 1;
        }

        // Show scroll indicator if needed and footer is enabled
        let has_scroll_indicator =
            self.config.show_footer && total_count > self.visible_height as usize;
        if has_scroll_indicator {
            let indicator = format!(
                "[{}-{} of {}]",
                self.scroll_offset + 1,
                (self.scroll_offset + display_items.len()).min(total_count),
                total_count
            );
            execute!(
                stderr,
                Print(self.config.footer.paint(&indicator)),
                Clear(ClearType::UntilNewLine),
                Print("\r\n")
            )?;
            lines_rendered += 1;
        }

        // Clear any extra lines from previous render
        if lines_rendered < self.rendered_lines {
            for _ in 0..(self.rendered_lines - lines_rendered) {
                execute!(stderr, Clear(ClearType::CurrentLine), Print("\r\n"))?;
            }
            // Move back up to end of actual content
            execute!(
                stderr,
                MoveUp((self.rendered_lines - lines_rendered) as u16)
            )?;
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
            // Cursor is currently at line (lines_rendered + 1) relative to start
            // Filter line is at position (prompt_lines + 1)
            // Move up: (lines_rendered + 1) - (prompt_lines + 1) = lines_rendered - prompt_lines
            let prompt_lines = if self.prompt.is_some() { 1usize } else { 0 };
            self.fuzzy_cursor_offset = lines_rendered - prompt_lines;
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
        execute!(
            stderr,
            Print(prefix),
            Print(text),
            Print(RESET),
            Clear(ClearType::UntilNewLine),
            Print("\r\n")
        )
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
        execute!(
            stderr,
            Print(cursor),
            Print(checkbox),
            Print(text),
            Print(RESET),
            Clear(ClearType::UntilNewLine),
            Print("\r\n")
        )
    }

    fn render_fuzzy_item_inline(
        &self,
        stderr: &mut Stderr,
        text: &str,
        active: bool,
    ) -> io::Result<()> {
        let prefix = if active { "> " } else { "  " };
        execute!(stderr, Print(prefix))?;

        if self.filter_text.is_empty() {
            execute!(
                stderr,
                Print(text),
                Print(RESET),
                Clear(ClearType::UntilNewLine),
                Print("\r\n")
            )
        } else if let Some((_score, indices)) = self.matcher.fuzzy_indices(text, &self.filter_text)
        {
            // Highlight matching characters using the configured style
            for (idx, c) in text.chars().enumerate() {
                if indices.contains(&idx) {
                    execute!(
                        stderr,
                        Print(self.config.match_text.paint(c.to_string()))
                    )?;
                } else {
                    execute!(stderr, Print(c))?;
                }
            }
            execute!(
                stderr,
                Print(RESET),
                Clear(ClearType::UntilNewLine),
                Print("\r\n")
            )
        } else {
            execute!(
                stderr,
                Print(text),
                Print(RESET),
                Clear(ClearType::UntilNewLine),
                Print("\r\n")
            )
        }
    }

    fn clear_display(&mut self, stderr: &mut Stderr) -> io::Result<()> {
        // In fuzzy mode, cursor may be at filter line; move back to end first
        if self.fuzzy_cursor_offset > 0 {
            execute!(stderr, MoveDown(self.fuzzy_cursor_offset as u16))?;
            self.fuzzy_cursor_offset = 0;
        }

        if self.rendered_lines > 0 {
            execute!(stderr, MoveUp(self.rendered_lines as u16), MoveToColumn(0))?;
            for _ in 0..self.rendered_lines {
                execute!(stderr, Clear(ClearType::CurrentLine), MoveDown(1))?;
            }
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
