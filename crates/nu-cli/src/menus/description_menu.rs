use {
    nu_ansi_term::{ansi::RESET, Style},
    reedline::{
        menu_functions::string_difference, Completer, Editor, Menu, MenuEvent, MenuTextStyle,
        Painter, Suggestion, UndoBehavior,
    },
};

/// Default values used as reference for the menu. These values are set during
/// the initial declaration of the menu and are always kept as reference for the
/// changeable [`WorkingDetails`]
struct DefaultMenuDetails {
    /// Number of columns that the menu will have
    pub columns: u16,
    /// Column width
    pub col_width: Option<usize>,
    /// Column padding
    pub col_padding: usize,
    /// Number of rows for commands
    pub selection_rows: u16,
    /// Number of rows allowed to display the description
    pub description_rows: usize,
}

impl Default for DefaultMenuDetails {
    fn default() -> Self {
        Self {
            columns: 4,
            col_width: None,
            col_padding: 2,
            selection_rows: 4,
            description_rows: 10,
        }
    }
}

/// Represents the actual column conditions of the menu. These conditions change
/// since they need to accommodate possible different line sizes for the column values
#[derive(Default)]
struct WorkingDetails {
    /// Number of columns that the menu will have
    pub columns: u16,
    /// Column width
    pub col_width: usize,
    /// Number of rows for description
    pub description_rows: usize,
}

/// Completion menu definition
pub struct DescriptionMenu {
    /// Menu name
    name: String,
    /// Menu status
    active: bool,
    /// Menu coloring
    color: MenuTextStyle,
    /// Default column details that are set when creating the menu
    /// These values are the reference for the working details
    default_details: DefaultMenuDetails,
    /// Number of minimum rows that are displayed when
    /// the required lines is larger than the available lines
    min_rows: u16,
    /// Working column details keep changing based on the collected values
    working_details: WorkingDetails,
    /// Menu cached values
    values: Vec<Suggestion>,
    /// column position of the cursor. Starts from 0
    col_pos: u16,
    /// row position in the menu. Starts from 0
    row_pos: u16,
    /// Menu marker when active
    marker: String,
    /// Event sent to the menu
    event: Option<MenuEvent>,
    /// String collected after the menu is activated
    input: Option<String>,
    /// Examples to select
    examples: Vec<String>,
    /// Example index
    example_index: Option<usize>,
    /// Examples may not be shown if there is not enough space in the screen
    show_examples: bool,
    /// Skipped description rows
    skipped_rows: usize,
    /// Calls the completer using only the line buffer difference difference
    /// after the menu was activated
    only_buffer_difference: bool,
}

impl Default for DescriptionMenu {
    fn default() -> Self {
        Self {
            name: "description_menu".to_string(),
            active: false,
            color: MenuTextStyle::default(),
            default_details: DefaultMenuDetails::default(),
            min_rows: 3,
            working_details: WorkingDetails::default(),
            values: Vec::new(),
            col_pos: 0,
            row_pos: 0,
            marker: "? ".to_string(),
            event: None,
            input: None,
            examples: Vec::new(),
            example_index: None,
            show_examples: true,
            skipped_rows: 0,
            only_buffer_difference: true,
        }
    }
}

// Menu configuration
impl DescriptionMenu {
    /// Menu builder with new name
    pub fn with_name(mut self, name: &str) -> Self {
        self.name = name.into();
        self
    }

    /// Menu builder with new value for text style
    pub fn with_text_style(mut self, text_style: Style) -> Self {
        self.color.text_style = text_style;
        self
    }

    /// Menu builder with new value for text style
    pub fn with_selected_text_style(mut self, selected_text_style: Style) -> Self {
        self.color.selected_text_style = selected_text_style;
        self
    }

    /// Menu builder with new value for text style
    pub fn with_description_text_style(mut self, description_text_style: Style) -> Self {
        self.color.description_style = description_text_style;
        self
    }

    /// Menu builder with new columns value
    pub fn with_columns(mut self, columns: u16) -> Self {
        self.default_details.columns = columns;
        self
    }

    /// Menu builder with new column width value
    pub fn with_column_width(mut self, col_width: Option<usize>) -> Self {
        self.default_details.col_width = col_width;
        self
    }

    /// Menu builder with new column width value
    pub fn with_column_padding(mut self, col_padding: usize) -> Self {
        self.default_details.col_padding = col_padding;
        self
    }

    /// Menu builder with new selection rows value
    pub fn with_selection_rows(mut self, selection_rows: u16) -> Self {
        self.default_details.selection_rows = selection_rows;
        self
    }

    /// Menu builder with new description rows value
    pub fn with_description_rows(mut self, description_rows: usize) -> Self {
        self.default_details.description_rows = description_rows;
        self
    }

    /// Menu builder with marker
    pub fn with_marker(mut self, marker: String) -> Self {
        self.marker = marker;
        self
    }

    /// Menu builder with new only buffer difference
    pub fn with_only_buffer_difference(mut self, only_buffer_difference: bool) -> Self {
        self.only_buffer_difference = only_buffer_difference;
        self
    }
}

// Menu functionality
impl DescriptionMenu {
    /// Move menu cursor to the next element
    fn move_next(&mut self) {
        let mut new_col = self.col_pos + 1;
        let mut new_row = self.row_pos;

        if new_col >= self.get_cols() {
            new_row += 1;
            new_col = 0;
        }

        if new_row >= self.get_rows() {
            new_row = 0;
            new_col = 0;
        }

        let position = new_row * self.get_cols() + new_col;
        if position >= self.get_values().len() as u16 {
            self.reset_position();
        } else {
            self.col_pos = new_col;
            self.row_pos = new_row;
        }
    }

    /// Move menu cursor to the previous element
    fn move_previous(&mut self) {
        let new_col = self.col_pos.checked_sub(1);

        let (new_col, new_row) = match new_col {
            Some(col) => (col, self.row_pos),
            None => match self.row_pos.checked_sub(1) {
                Some(row) => (self.get_cols().saturating_sub(1), row),
                None => (
                    self.get_cols().saturating_sub(1),
                    self.get_rows().saturating_sub(1),
                ),
            },
        };

        let position = new_row * self.get_cols() + new_col;
        if position >= self.get_values().len() as u16 {
            self.col_pos = (self.get_values().len() as u16 % self.get_cols()).saturating_sub(1);
            self.row_pos = self.get_rows().saturating_sub(1);
        } else {
            self.col_pos = new_col;
            self.row_pos = new_row;
        }
    }

    /// Menu index based on column and row position
    fn index(&self) -> usize {
        let index = self.row_pos * self.get_cols() + self.col_pos;
        index as usize
    }

    /// Get selected value from the menu
    fn get_value(&self) -> Option<Suggestion> {
        self.get_values().get(self.index()).cloned()
    }

    /// Calculates how many rows the Menu will use
    fn get_rows(&self) -> u16 {
        let values = self.get_values().len() as u16;

        if values == 0 {
            // When the values are empty the no_records_msg is shown, taking 1 line
            return 1;
        }

        let rows = values / self.get_cols();
        if values % self.get_cols() != 0 {
            rows + 1
        } else {
            rows
        }
    }

    /// Returns working details col width
    fn get_width(&self) -> usize {
        self.working_details.col_width
    }

    /// Reset menu position
    fn reset_position(&mut self) {
        self.col_pos = 0;
        self.row_pos = 0;
        self.skipped_rows = 0;
    }

    fn no_records_msg(&self, use_ansi_coloring: bool) -> String {
        let msg = "TYPE TO START SEARCH";
        if use_ansi_coloring {
            format!(
                "{}{}{}",
                self.color.selected_text_style.prefix(),
                msg,
                RESET
            )
        } else {
            msg.to_string()
        }
    }

    /// Returns working details columns
    fn get_cols(&self) -> u16 {
        self.working_details.columns.max(1)
    }

    /// End of line for menu
    fn end_of_line(&self, column: u16, index: usize) -> &str {
        let is_last = index == self.values.len().saturating_sub(1);
        if column == self.get_cols().saturating_sub(1) || is_last {
            "\r\n"
        } else {
            ""
        }
    }

    /// Update list of examples from the actual value
    fn update_examples(&mut self) {
        self.examples = self
            .get_value()
            .and_then(|suggestion| suggestion.extra)
            .unwrap_or_default();

        self.example_index = None;
    }

    /// Creates default string that represents one suggestion from the menu
    fn create_entry_string(
        &self,
        suggestion: &Suggestion,
        index: usize,
        column: u16,
        empty_space: usize,
        use_ansi_coloring: bool,
    ) -> String {
        if use_ansi_coloring {
            if index == self.index() {
                format!(
                    "{}{}{}{:>empty$}{}",
                    self.color.selected_text_style.prefix(),
                    &suggestion.value,
                    RESET,
                    "",
                    self.end_of_line(column, index),
                    empty = empty_space,
                )
            } else {
                format!(
                    "{}{}{}{:>empty$}{}",
                    self.color.text_style.prefix(),
                    &suggestion.value,
                    RESET,
                    "",
                    self.end_of_line(column, index),
                    empty = empty_space,
                )
            }
        } else {
            // If no ansi coloring is found, then the selection word is
            // the line in uppercase
            let (marker, empty_space) = if index == self.index() {
                (">", empty_space.saturating_sub(1))
            } else {
                ("", empty_space)
            };

            let line = format!(
                "{}{}{:>empty$}{}",
                marker,
                &suggestion.value,
                "",
                self.end_of_line(column, index),
                empty = empty_space,
            );

            if index == self.index() {
                line.to_uppercase()
            } else {
                line
            }
        }
    }

    /// Description string with color
    fn create_description_string(&self, use_ansi_coloring: bool) -> String {
        let description = self
            .get_value()
            .and_then(|suggestion| suggestion.description)
            .unwrap_or_default()
            .lines()
            .skip(self.skipped_rows)
            .take(self.working_details.description_rows)
            .collect::<Vec<&str>>()
            .join("\r\n");

        if use_ansi_coloring && !description.is_empty() {
            format!(
                "{}{}{}",
                self.color.description_style.prefix(),
                description,
                RESET,
            )
        } else {
            description
        }
    }

    /// Selectable list of examples from the actual value
    fn create_example_string(&self, use_ansi_coloring: bool) -> String {
        if !self.show_examples {
            return "".into();
        }

        let examples: String = self
            .examples
            .iter()
            .enumerate()
            .map(|(index, example)| {
                if let Some(example_index) = self.example_index {
                    if index == example_index {
                        format!(
                            "  {}{}{}\r\n",
                            self.color.selected_text_style.prefix(),
                            example,
                            RESET
                        )
                    } else {
                        format!("  {}\r\n", example)
                    }
                } else {
                    format!("  {}\r\n", example)
                }
            })
            .collect();

        if examples.is_empty() {
            "".into()
        } else if use_ansi_coloring {
            format!(
                "{}\r\n\r\nExamples:\r\n{}{}",
                self.color.description_style.prefix(),
                RESET,
                examples,
            )
        } else {
            format!("\r\n\r\nExamples:\r\n{}", examples,)
        }
    }
}

impl Menu for DescriptionMenu {
    /// Menu name
    fn name(&self) -> &str {
        self.name.as_str()
    }

    /// Menu indicator
    fn indicator(&self) -> &str {
        self.marker.as_str()
    }

    /// Deactivates context menu
    fn is_active(&self) -> bool {
        self.active
    }

    /// The menu stays active even with one record
    fn can_quick_complete(&self) -> bool {
        false
    }

    /// The menu does not need to partially complete
    fn can_partially_complete(
        &mut self,
        _values_updated: bool,
        _editor: &mut Editor,
        _completer: &mut dyn Completer,
    ) -> bool {
        false
    }

    /// Selects what type of event happened with the menu
    fn menu_event(&mut self, event: MenuEvent) {
        match &event {
            MenuEvent::Activate(_) => self.active = true,
            MenuEvent::Deactivate => {
                self.active = false;
                self.input = None;
                self.values = Vec::new();
            }
            _ => {}
        };

        self.event = Some(event);
    }

    /// Updates menu values
    fn update_values(&mut self, editor: &mut Editor, completer: &mut dyn Completer) {
        if self.only_buffer_difference {
            if let Some(old_string) = &self.input {
                let (start, input) = string_difference(editor.get_buffer(), old_string);
                if !input.is_empty() {
                    self.reset_position();
                    self.values = completer.complete(input, start);
                }
            }
        } else {
            let trimmed_buffer = editor.get_buffer().replace('\n', " ");
            self.values = completer.complete(
                trimmed_buffer.as_str(),
                editor.line_buffer().insertion_point(),
            );
            self.reset_position();
        }
    }

    /// The working details for the menu changes based on the size of the lines
    /// collected from the completer
    fn update_working_details(
        &mut self,
        editor: &mut Editor,
        completer: &mut dyn Completer,
        painter: &Painter,
    ) {
        if let Some(event) = self.event.take() {
            // Updating all working parameters from the menu before executing any of the
            // possible event
            let max_width = self.get_values().iter().fold(0, |acc, suggestion| {
                let str_len = suggestion.value.len() + self.default_details.col_padding;
                if str_len > acc {
                    str_len
                } else {
                    acc
                }
            });

            // If no default width is found, then the total screen width is used to estimate
            // the column width based on the default number of columns
            let default_width = if let Some(col_width) = self.default_details.col_width {
                col_width
            } else {
                let col_width = painter.screen_width() / self.default_details.columns;
                col_width as usize
            };

            // Adjusting the working width of the column based the max line width found
            // in the menu values
            if max_width > default_width {
                self.working_details.col_width = max_width;
            } else {
                self.working_details.col_width = default_width;
            };

            // The working columns is adjusted based on possible number of columns
            // that could be fitted in the screen with the calculated column width
            let possible_cols = painter.screen_width() / self.working_details.col_width as u16;
            if possible_cols > self.default_details.columns {
                self.working_details.columns = self.default_details.columns.max(1);
            } else {
                self.working_details.columns = possible_cols;
            }

            // Updating the working rows to display the description
            if self.menu_required_lines(painter.screen_width()) <= painter.remaining_lines() {
                self.working_details.description_rows = self.default_details.description_rows;
                self.show_examples = true;
            } else {
                self.working_details.description_rows = painter
                    .remaining_lines()
                    .saturating_sub(self.default_details.selection_rows + 1)
                    as usize;

                self.show_examples = false;
            }

            match event {
                MenuEvent::Activate(_) => {
                    self.reset_position();
                    self.input = Some(editor.get_buffer().to_string());
                    self.update_values(editor, completer);
                }
                MenuEvent::Deactivate => self.active = false,
                MenuEvent::Edit(_) => {
                    self.reset_position();
                    self.update_values(editor, completer);
                    self.update_examples()
                }
                MenuEvent::NextElement => {
                    self.skipped_rows = 0;
                    self.move_next();
                    self.update_examples();
                }
                MenuEvent::PreviousElement => {
                    self.skipped_rows = 0;
                    self.move_previous();
                    self.update_examples();
                }
                MenuEvent::MoveUp => {
                    if let Some(example_index) = self.example_index {
                        if let Some(index) = example_index.checked_sub(1) {
                            self.example_index = Some(index);
                        } else {
                            self.example_index = Some(self.examples.len().saturating_sub(1));
                        }
                    } else if !self.examples.is_empty() {
                        self.example_index = Some(0);
                    }
                }
                MenuEvent::MoveDown => {
                    if let Some(example_index) = self.example_index {
                        let index = example_index + 1;
                        if index < self.examples.len() {
                            self.example_index = Some(index);
                        } else {
                            self.example_index = Some(0);
                        }
                    } else if !self.examples.is_empty() {
                        self.example_index = Some(0);
                    }
                }
                MenuEvent::MoveLeft => self.skipped_rows = self.skipped_rows.saturating_sub(1),
                MenuEvent::MoveRight => {
                    let skipped = self.skipped_rows + 1;
                    let description_rows = self
                        .get_value()
                        .and_then(|suggestion| suggestion.description)
                        .unwrap_or_default()
                        .lines()
                        .count();

                    let allowed_skips =
                        description_rows.saturating_sub(self.working_details.description_rows);

                    if skipped < allowed_skips {
                        self.skipped_rows = skipped;
                    } else {
                        self.skipped_rows = allowed_skips;
                    }
                }
                MenuEvent::PreviousPage | MenuEvent::NextPage => {}
            }
        }
    }

    /// The buffer gets replaced in the Span location
    fn replace_in_buffer(&self, editor: &mut Editor) {
        if let Some(Suggestion { value, span, .. }) = self.get_value() {
            let start = span.start.min(editor.line_buffer().len());
            let end = span.end.min(editor.line_buffer().len());

            let replacement = if let Some(example_index) = self.example_index {
                self.examples
                    .get(example_index)
                    .expect("the example index is always checked")
            } else {
                &value
            };

            editor.edit_buffer(
                |lb| {
                    lb.replace_range(start..end, replacement);
                    let mut offset = lb.insertion_point();
                    offset += lb.len().saturating_sub(end.saturating_sub(start));
                    lb.set_insertion_point(offset);
                },
                UndoBehavior::CreateUndoPoint,
            );
        }
    }

    /// Minimum rows that should be displayed by the menu
    fn min_rows(&self) -> u16 {
        self.get_rows().min(self.min_rows)
    }

    /// Gets values from filler that will be displayed in the menu
    fn get_values(&self) -> &[Suggestion] {
        &self.values
    }

    fn menu_required_lines(&self, _terminal_columns: u16) -> u16 {
        let example_lines = self
            .examples
            .iter()
            .fold(0, |acc, example| example.lines().count() + acc);

        self.default_details.selection_rows
            + self.default_details.description_rows as u16
            + example_lines as u16
            + 3
    }

    fn menu_string(&self, _available_lines: u16, use_ansi_coloring: bool) -> String {
        if self.get_values().is_empty() {
            self.no_records_msg(use_ansi_coloring)
        } else {
            // The skip values represent the number of lines that should be skipped
            // while printing the menu
            let available_lines = self.default_details.selection_rows;
            let skip_values = if self.row_pos >= available_lines {
                let skip_lines = self.row_pos.saturating_sub(available_lines) + 1;
                (skip_lines * self.get_cols()) as usize
            } else {
                0
            };

            // It seems that crossterm prefers to have a complete string ready to be printed
            // rather than looping through the values and printing multiple things
            // This reduces the flickering when printing the menu
            let available_values = (available_lines * self.get_cols()) as usize;
            let selection_values: String = self
                .get_values()
                .iter()
                .skip(skip_values)
                .take(available_values)
                .enumerate()
                .map(|(index, suggestion)| {
                    // Correcting the enumerate index based on the number of skipped values
                    let index = index + skip_values;
                    let column = index as u16 % self.get_cols();
                    let empty_space = self.get_width().saturating_sub(suggestion.value.len());

                    self.create_entry_string(
                        suggestion,
                        index,
                        column,
                        empty_space,
                        use_ansi_coloring,
                    )
                })
                .collect();

            format!(
                "{}{}{}",
                selection_values,
                self.create_description_string(use_ansi_coloring),
                self.create_example_string(use_ansi_coloring)
            )
        }
    }
}
