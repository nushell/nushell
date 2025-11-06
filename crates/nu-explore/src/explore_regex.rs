// Borrowed from the ut project and tweaked. Thanks!
// https://github.com/ksdme/ut
// Below is the ut license:
// MIT License
//
// Copyright (c) 2025 Kilari Teja
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

use fancy_regex::Regex;
use nu_engine::command_prelude::*;
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    crossterm::{
        event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
        execute,
        terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
    },
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span, Text},
    widgets::{Block, BorderType, Borders, Padding, Paragraph},
};
use std::io::{self, Stdout};
use tui_textarea::{CursorMove, Input, TextArea};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

/// A `regular expression explorer` program.
#[derive(Clone)]
pub struct ExploreRegex;

impl Command for ExploreRegex {
    fn name(&self) -> &str {
        "explore regex"
    }

    fn description(&self) -> &str {
        "Launch a TUI to create and explore regular expressions interactively."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("explore regex")
            .input_output_types(vec![
                (Type::Nothing, Type::String),
                (Type::String, Type::String),
            ])
            .category(Category::Viewers)
    }

    fn extra_description(&self) -> &str {
        r#"Press `Ctrl-Q` to quit and provide constructed regular expression as the output."#
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let input_span = input.span().unwrap_or(call.head);
        let (string_input, _span, _metadata) = input.collect_string_strict(input_span)?;
        let result = execute_regex_app(call, string_input);

        match result {
            Ok(Some(value)) => Ok(PipelineData::Value(value, None)),
            Ok(None) => Ok(PipelineData::empty()),
            Err(err) => Err(err),
        }
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Explore a regular expression interactively",
                example: r#"explore regex"#,
                result: None,
            },
            Example {
                description: "Explore a regular expression interactively with sample text",
                example: r#"open -r Cargo.toml | explore regex"#,
                result: None,
            },
        ]
    }
}

fn execute_regex_app(
    call: &Call,
    string_input: String,
) -> Result<Option<nu_protocol::Value>, ShellError> {
    // Setup terminal
    enable_raw_mode().map_err(|e| ShellError::GenericError {
        error: "Could not enable raw mode".into(),
        msg: format!("terminal error: {e}"),
        span: Some(call.head),
        help: None,
        inner: vec![],
    })?;

    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture).map_err(|e| {
        ShellError::GenericError {
            error: "Could not enter alternate screen".into(),
            msg: format!("terminal error: {e}"),
            span: Some(call.head),
            help: None,
            inner: vec![],
        }
    })?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).map_err(|e| ShellError::GenericError {
        error: "Could not initialize terminal".into(),
        msg: format!("terminal error: {e}"),
        span: Some(call.head),
        help: None,
        inner: vec![],
    })?;

    let mut app = App::new(Some(string_input));
    let res = run_app_loop(&mut terminal, &mut app);

    // Restore terminal
    disable_raw_mode().map_err(|e| ShellError::GenericError {
        error: "Could not disable raw mode".into(),
        msg: format!("terminal error: {e}"),
        span: Some(call.head),
        help: None,
        inner: vec![],
    })?;

    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )
    .map_err(|e| ShellError::GenericError {
        error: "Could not leave alternate screen".into(),
        msg: format!("terminal error: {e}"),
        span: Some(call.head),
        help: None,
        inner: vec![],
    })?;

    terminal
        .show_cursor()
        .map_err(|e| ShellError::GenericError {
            error: "Could not show terminal cursor".into(),
            msg: format!("terminal error: {e}"),
            span: Some(call.head),
            help: None,
            inner: vec![],
        })?;

    if let Err(err) = res {
        return Err(ShellError::GenericError {
            error: "Application error".into(),
            msg: format!("application error: {err}"),
            span: Some(call.head),
            help: None,
            inner: vec![],
        });
    }

    Ok(Some(nu_protocol::Value::string(
        app.get_regex_input(),
        call.head,
    )))
}

enum InputFocus {
    Regex,
    Sample,
}

struct App<'a> {
    input_focus: InputFocus,
    regex_textarea: TextArea<'a>,
    sample_textarea: TextArea<'a>,
    compiled_regex: Option<Regex>,
    regex_error: Option<String>,
    sample_scroll_v: u16,
    sample_scroll_h: u16,
    sample_view_height: u16,
}

impl<'a> App<'a> {
    fn new(input_string: Option<String>) -> App<'a> {
        let mut regex_textarea = TextArea::default();
        regex_textarea.set_cursor_line_style(Style::new());

        let mut sample_textarea = TextArea::default();
        sample_textarea.insert_str(input_string.unwrap_or_default());

        sample_textarea.set_cursor_line_style(Style::new());
        sample_textarea.move_cursor(CursorMove::Top);

        App {
            input_focus: InputFocus::Regex,
            regex_textarea,
            sample_textarea,
            compiled_regex: None,
            regex_error: None,
            sample_scroll_v: 0,
            sample_scroll_h: 0,
            sample_view_height: 0,
        }
    }
}

impl<'a> App<'a> {
    fn get_sample_text(&self) -> String {
        self.sample_textarea.lines().join("\n")
    }

    fn get_regex_input(&self) -> String {
        self.regex_textarea
            .lines()
            .first()
            .cloned()
            .unwrap_or_default()
    }

    fn compile_regex(&mut self) {
        self.compiled_regex = None;
        self.regex_error = None;

        let Some(regex_input) = self.regex_textarea.lines().first() else {
            return;
        };

        if regex_input.is_empty() {
            return;
        }

        match Regex::new(regex_input) {
            Ok(regex) => {
                self.compiled_regex = Some(regex);
                self.regex_error = None;
            }
            Err(e) => {
                self.compiled_regex = None;
                self.regex_error = Some(format!("Regex error: {e}"));
            }
        }
    }

    fn get_highlighted_text(&self) -> Text<'static> {
        let sample_text = self.get_sample_text();
        let Some(regex) = &self.compiled_regex else {
            return Text::from(sample_text);
        };

        let highlight_styles = &[
            Style::new().bg(Color::LightBlue).fg(Color::Black),
            Style::new().bg(Color::LightGreen).fg(Color::Black),
            Style::new().bg(Color::LightRed).fg(Color::Black),
            Style::new().bg(Color::LightYellow).fg(Color::Black),
            Style::new().bg(Color::Blue).fg(Color::Black),
            Style::new().bg(Color::Green).fg(Color::Black),
            Style::new().bg(Color::Red).fg(Color::White),
            Style::new().bg(Color::Yellow).fg(Color::Black),
            Style::new().bg(Color::Magenta).fg(Color::White),
        ];

        let mut highlights: Vec<(usize, usize, Style)> = vec![];
        for capture in regex.captures_iter(&sample_text).flatten() {
            for (group, submatch) in capture.iter().enumerate() {
                if let Some(submatch) = submatch {
                    highlights.push((
                        submatch.start(),
                        submatch.end(),
                        highlight_styles[group % highlight_styles.len()],
                    ));
                }
            }
        }

        // This is a fallback style when a span has no highlight match. Although,
        // to make sure full matches from not being highlighted, we need to make sure
        // this fallback is the last element, even after the sort later.
        highlights.push((0, sample_text.len(), Style::new()));

        // Sort the highlights by their size and start position. This lets us
        // to exit as soon as one overlapping match is found below.
        highlights.sort_by(|a, b| ((a.1 - a.0), a.0).cmp(&(b.1 - b.0, b.0)));

        // All the boundary points in the vector.
        let mut boundaries = highlights
            .iter()
            .flat_map(|(s, e, _)| vec![*s, *e])
            .collect::<Vec<usize>>();

        boundaries.sort();
        boundaries.dedup();

        // \n in Spans are ignored. Therefore, we need to construct them ourselves.
        let mut lines: Vec<Line> = vec![];
        let mut current_line = Line::from("");

        // Generate styled spans as necessary.
        // TODO: Do this in a more efficient way. You can flatten the matches using
        // a stack and last known position instead of a nested lookup here.
        for window in boundaries.windows(2) {
            if let [s, e] = window
                && let Some((_, _, style)) =
                    highlights.iter().find(|(c_s, c_e, _)| c_s <= s && c_e >= e)
            {
                let fragment = &sample_text[*s..*e];
                for (no, fragment) in fragment.split('\n').enumerate() {
                    // This works because usually, lines are terminated with a newline, therefore,
                    // we need to prefer updating the existing line for the first split item. But,
                    // starting with the second item, we know that there was an explicit newline in
                    // between.
                    if no > 0 {
                        lines.push(current_line);
                        current_line = Line::from("");
                    }

                    if !fragment.is_empty() {
                        current_line.push_span(Span::styled(fragment.to_string(), *style));
                    }
                }
            }
        }

        lines.push(current_line);
        Text::from(lines)
    }
}

fn run_app_loop(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    app: &mut App,
) -> io::Result<()> {
    loop {
        terminal.draw(|f| draw_ui(f, app))?;

        if let Event::Key(key) = event::read()?
            && key.kind == KeyEventKind::Press
        {
            // Handle Ctrl+Q to quit
            if key.code == KeyCode::Char('q')
                && key
                    .modifiers
                    .contains(ratatui::crossterm::event::KeyModifiers::CONTROL)
            {
                return Ok(());
            }

            // Handle Tab to switch focus.
            if matches!(key.code, KeyCode::Tab | KeyCode::BackTab) {
                app.input_focus = match app.input_focus {
                    InputFocus::Regex => InputFocus::Sample,
                    InputFocus::Sample => InputFocus::Regex,
                };
                continue;
            }

            // Escape will focus the Regex field back again.
            if matches!(key.code, KeyCode::Esc) {
                app.input_focus = InputFocus::Regex;
                continue;
            }

            // Intercept PageUp/PageDown in Sample pane to move by one page height
            if matches!(app.input_focus, InputFocus::Sample) {
                match key.code {
                    KeyCode::PageUp | KeyCode::PageDown => {
                        let page = std::cmp::max(app.sample_view_height, 1);
                        let (row, col) = app.sample_textarea.cursor();
                        let rows_len = app.sample_textarea.lines().len();
                        let target_row_u16 = match key.code {
                            KeyCode::PageUp => (row as u16).saturating_sub(page),
                            KeyCode::PageDown => {
                                let max_row = rows_len.saturating_sub(1) as u16;
                                let r = (row as u16).saturating_add(page);
                                if r > max_row { max_row } else { r }
                            }
                            _ => row as u16,
                        };
                        let target_col_u16 = col as u16;
                        app.sample_textarea
                            .move_cursor(CursorMove::Jump(target_row_u16, target_col_u16));
                        continue;
                    }
                    _ => {}
                }
            }

            // Convert crossterm event to tui-textarea input
            let input = Input::from(Event::Key(key));

            // Handle input based on current mode
            match app.input_focus {
                InputFocus::Regex => {
                    app.regex_textarea.input(input);
                    app.compile_regex(); // TODO: Do this in a worker thread.
                }
                InputFocus::Sample => {
                    app.sample_textarea.input(input);
                }
            }
        }
    }
}

// Draw the UI.
fn draw_ui(f: &mut ratatui::Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Spacer
            Constraint::Length(1), // Label
            Constraint::Length(1), // Regex
            Constraint::Length(1), // Spacer
            Constraint::Length(1), // Label
            Constraint::Min(8),    // Sample
            Constraint::Length(1), // Spacer
            Constraint::Length(1), // Help
        ])
        .horizontal_margin(2)
        .vertical_margin(1)
        .split(f.area());

    draw_body(f, app, (chunks[1], chunks[2], chunks[4], chunks[5]));
    draw_help(f, chunks[7]);
}

// Add a line for help text below.
fn draw_help(f: &mut ratatui::Frame, area: Rect) {
    let muted = Style::new().fg(Color::DarkGray);

    f.render_widget(
        Paragraph::new(Line::from(vec![
            "Cycle Focus ".into(),
            Span::styled("Tab", muted),
            " ".repeat(3).into(),
            "Exit ".into(),
            Span::styled("Ctrl + q", muted),
        ])),
        area,
    );
}

// Draw the application contents.
fn draw_body(f: &mut ratatui::Frame, app: &mut App, areas: (Rect, Rect, Rect, Rect)) {
    let textarea_base = Block::default()
        .borders(Borders::LEFT)
        .border_type(BorderType::Thick)
        .border_style(Style::new().fg(Color::DarkGray))
        .padding(Padding::horizontal(1));

    let textarea_active = textarea_base
        .clone()
        .border_style(Style::new().fg(Color::Blue));

    let textarea_error = textarea_active.clone().fg(Color::Red);

    let cursor_active = Style::new().bg(Color::White).fg(Color::Black);

    let mut regex_label = Paragraph::new("Regex");
    if matches!(app.input_focus, InputFocus::Regex) {
        app.regex_textarea.set_block(match app.regex_error {
            Some(_) => textarea_error,
            None => textarea_active.clone(),
        });
        app.regex_textarea.set_cursor_style(cursor_active);
    } else {
        regex_label = regex_label.fg(Color::DarkGray);
        app.regex_textarea.set_block(match app.regex_error {
            Some(_) => textarea_error,
            None => textarea_base.clone(),
        });
        app.regex_textarea.set_cursor_style(Style::new().hidden());
    }

    let mut sample_label = Paragraph::new("Test String");
    let focused_sample = matches!(app.input_focus, InputFocus::Sample);
    if focused_sample {
        app.sample_textarea.set_cursor_style(cursor_active);
    } else {
        sample_label = sample_label.fg(Color::DarkGray);
        app.sample_textarea.set_cursor_style(Style::new().hidden());
    }

    // Render the regex.
    f.render_widget(regex_label, areas.0);
    f.render_widget(&app.regex_textarea, areas.1);

    // Render the test string.
    f.render_widget(sample_label, areas.2);

    let sample_block = if focused_sample {
        textarea_active
    } else {
        textarea_base
    };

    let content_area = sample_block.inner(areas.3);
    let visible_rows = content_area.height;
    let visible_cols = content_area.width;
    app.sample_view_height = visible_rows;

    if focused_sample {
        let (cursor_row, cursor_col) = app.sample_textarea.cursor();
        let line = app.sample_textarea.lines()[cursor_row].clone();

        let cursor_display_col = line[0..cursor_col].width() as u16;

        // vertical
        let cursor_row_u16 = cursor_row as u16;
        if cursor_row_u16 < app.sample_scroll_v {
            app.sample_scroll_v = cursor_row_u16;
        } else if cursor_row_u16 >= app.sample_scroll_v + visible_rows {
            app.sample_scroll_v = cursor_row_u16 - visible_rows + 1;
        }

        // horizontal
        if cursor_display_col < app.sample_scroll_h {
            app.sample_scroll_h = cursor_display_col;
        } else if cursor_display_col >= app.sample_scroll_h + visible_cols {
            app.sample_scroll_h = cursor_display_col - visible_cols + 1;
        }
    }

    let highlighted_text = app.get_highlighted_text();
    let text_paragraph = Paragraph::new(highlighted_text)
        .scroll((app.sample_scroll_v, app.sample_scroll_h))
        .block(sample_block);

    f.render_widget(text_paragraph, areas.3);

    if focused_sample {
        let buf = f.buffer_mut();
        let (cursor_row, cursor_col) = app.sample_textarea.cursor();
        let line = &app.sample_textarea.lines()[cursor_row];
        let prefix_width = line[0..cursor_col].width() as u16;
        let relative_col = prefix_width - app.sample_scroll_h;
        let relative_row = (cursor_row as u16) - app.sample_scroll_v;
        let cursor_x = content_area.x + relative_col;
        let cursor_y = content_area.y + relative_row;
        let is_eol = cursor_col == line.len();
        let grapheme_width = if is_eol {
            1
        } else {
            line[cursor_col..]
                .graphemes(true)
                .next()
                .map(|g| g.width())
                .unwrap_or(1)
        };
        for i in 0..grapheme_width {
            let x = cursor_x + i as u16;
            let y = cursor_y;

            if let Some(cell) = buf.cell_mut((x, y)) {
                if is_eol {
                    cell.set_symbol(" ");
                }

                cell.set_style(cell.style().add_modifier(Modifier::REVERSED));
            }
        }
    }
}
