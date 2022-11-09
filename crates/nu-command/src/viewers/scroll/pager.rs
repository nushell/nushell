use std::{
    borrow::Cow,
    cmp::{max, min},
    collections::HashMap,
    io::{self, Result},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};

use crossterm::{
    event::{poll, read, Event, KeyCode, KeyEvent},
    execute,
    terminal::{
        disable_raw_mode, enable_raw_mode, Clear, ClearType, EnterAlternateScreen,
        LeaveAlternateScreen,
    },
};
use nu_ansi_term::{Color as NuColor, Style as NuStyle};
use nu_color_config::{get_color_config, style_primitive};
use nu_protocol::{Config, Value};
use nu_table::{string_width, Alignment, TextStyle};
use reedline::KeyModifiers;
use tui::{
    backend::{Backend, CrosstermBackend},
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::Span,
    widgets::{Block, Borders, Paragraph, StatefulWidget, Widget},
    Frame, Terminal,
};

type NuText = (String, TextStyle);

type CtrlC = Option<Arc<AtomicBool>>;

type NuStyleTable = HashMap<String, NuStyle>;

#[derive(Debug, Default, Clone)]
pub struct TableConfig {
    pub(crate) show_index: bool,
    pub(crate) show_head: bool,
    pub(crate) reverse: bool,
}

pub fn pager(
    cols: &[String],
    data: &[Vec<Value>],
    config: &Config,
    ctrlc: CtrlC,
    table: TableConfig,
    style: StyleConfig,
) -> Result<()> {
    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, Clear(ClearType::All))?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let color_hm = get_color_config(config);
    let mut state = UIState::new(
        Cow::from(cols),
        Cow::from(data),
        config,
        &color_hm,
        table.show_head,
        table.show_index,
        style,
    );

    if table.reverse {
        if let Ok(size) = terminal.size() {
            let page_size = estimate_page_size(size, table.show_head);
            state_reverse_data(&mut state, page_size as usize)
        }
    }

    let result = render_ui(&mut terminal, ctrlc, state);

    // restore terminal
    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen)?;

    result
}

fn render_ui<B>(terminal: &mut Terminal<B>, ctrlc: CtrlC, mut state: UIState<'_>) -> Result<()>
where
    B: Backend,
{
    let events = UIEvents::new();

    let mut state_stack: Vec<UIState<'_>> = Vec::new();

    loop {
        // handle CTRLC event
        if let Some(ctrlc) = ctrlc.clone() {
            if ctrlc.load(Ordering::SeqCst) {
                break Ok(());
            }
        }

        {
            let state = state_stack.last_mut().unwrap_or(&mut state);
            let mut layout = Layout::default();

            terminal.draw(|f| {
                let area = f.size();

                f.render_widget(tui::widgets::Clear, area);

                let table = Table::from(&*state);
                let table_area =
                    Rect::new(area.x, area.y, area.width, area.height.saturating_sub(2));
                f.render_stateful_widget(table, table_area, &mut layout);

                let status_area =
                    Rect::new(area.left(), area.bottom().saturating_sub(2), area.width, 1);
                render_status_bar(f, status_area, state, &layout);

                let cmd_area =
                    Rect::new(area.left(), area.bottom().saturating_sub(1), area.width, 1);
                render_cmd_bar(f, cmd_area, state, &layout);

                highlight_search_results(f, state, &layout);

                if state.mode == UIMode::Cursor {
                    update_cursor(state, &layout);
                    highlight_cell(f, state, &layout, table_area);

                    if state.style.show_cursow {
                        set_cursor(f, state, &layout);
                    }
                }
            })?;

            let exited = handle_events(&events, state, &layout, terminal);
            if exited {
                break Ok(());
            }
        }

        {
            let state = state_stack.last().unwrap_or(&state);
            if state.render_inner {
                let current_value = get_current_value(state);
                let current_header = get_header(state);
                let (columns, values) = super::collect_input(current_value);

                let mut state = UIState::new(
                    Cow::from(columns),
                    Cow::from(values),
                    state.config,
                    state.color_hm,
                    state.show_header,
                    state.show_index,
                    state.style.clone(),
                );
                state.mode = UIMode::Cursor;
                state.section_name = current_header;

                state_stack.push(state);
            }
        }

        {
            let is_main_state = !state_stack.is_empty();
            if is_main_state && state_stack.last().unwrap_or(&state).render_close {
                state_stack.pop();

                let latest_state = state_stack.last_mut().unwrap_or(&mut state);
                latest_state.render_inner = false;
            }
        }
    }
}

fn render_status_bar<B>(f: &mut Frame<B>, area: Rect, state: &UIState<'_>, layout: &Layout)
where
    B: Backend,
{
    let seen_rows = state.row_index + layout.count_rows();
    let cursor = (state.mode == UIMode::Cursor).then(|| state.cursor);
    let status_bar = StatusBar::new(
        state.section_name.as_deref(),
        cursor,
        state.row_index,
        state.column_index,
        seen_rows,
        state.count_rows(),
        state.style.status_bar,
    );

    f.render_widget(status_bar, area);
}

fn render_cmd_bar<B>(f: &mut Frame<B>, area: Rect, _state: &UIState<'_>, _layout: &Layout)
where
    B: Backend,
{
    if _state.is_search_input || !_state.buf_cmd_input.is_empty() {
        if _state.search_results.is_empty() && !_state.is_search_input {
            let message = format!("Pattern not found: {}", _state.buf_cmd_input);
            let style = NuStyle {
                background: Some(NuColor::Red),
                foreground: Some(NuColor::White),
                ..Default::default()
            };
            f.render_widget(CmdBar::new(&message, "", style), area);

            return;
        }

        let prefix = if _state.is_search_rev { '?' } else { '/' };

        let text = format!("{}{}", prefix, _state.buf_cmd_input);
        let info = if _state.search_results.is_empty() {
            String::from("[0/0]")
        } else {
            let index = _state.search_index + 1;
            let total = _state.search_results.len();
            format!("[{}/{}]", index, total)
        };

        f.render_widget(CmdBar::new(&text, &info, _state.style.cmd_bar), area);
    }
}

fn highlight_search_results<B>(f: &mut Frame<B>, _state: &UIState<'_>, _layout: &Layout)
where
    B: Backend,
{
    let hightlight_block = Block::default().style(nu_style_to_tui(_state.style.highlight));

    if !_state.search_results.is_empty() {
        for row in 0.._layout.count_rows() {
            for column in 0.._layout.count_columns() {
                if let Some(e) = _layout.get(row, column) {
                    let pos = e.data_pos;
                    let text = &_state.data_text[pos.0][pos.1].0;
                    if let Some(p) = text.find(&_state.buf_cmd_input) {
                        if p > e.width as usize {
                            break;
                        }

                        // todo: might be not UTF-8 friendly
                        let area = Rect::new(
                            e.position.x + p as u16,
                            e.position.y,
                            _state.buf_cmd_input.len() as u16,
                            1,
                        );
                        f.render_widget(hightlight_block.clone(), area);
                    }
                }
            }
        }
    }
}

fn highlight_cell<B>(f: &mut Frame<B>, state: &UIState<'_>, layout: &Layout, area: Rect)
where
    B: Backend,
{
    let Position { x: column, y: row } = state.cursor;
    let info = layout.get(row as usize, column as usize);
    if let Some(info) = info {
        if let Some(style) = state.style.selected_column {
            let hightlight_block = Block::default().style(nu_style_to_tui(style));
            let area = Rect::new(info.position.x, area.y, info.width, area.height);
            f.render_widget(hightlight_block.clone(), area);
        }

        if let Some(style) = state.style.selected_row {
            let hightlight_block = Block::default().style(nu_style_to_tui(style));
            let area = Rect::new(area.x, info.position.y, area.width, 1);
            f.render_widget(hightlight_block.clone(), area);
        }

        if let Some(style) = state.style.selected_cell {
            let hightlight_block = Block::default().style(nu_style_to_tui(style));
            let area = Rect::new(info.position.x, info.position.y, info.width, 1);
            f.render_widget(hightlight_block.clone(), area);
        }
    }
}

fn get_current_value(state: &UIState<'_>) -> Value {
    let current_row = state.cursor.y as usize + state.row_index;
    let current_column = state.cursor.x as usize + state.column_index;

    let row = state.data[current_row].clone();
    row[current_column].clone()
}

fn get_header(state: &UIState<'_>) -> Option<String> {
    let current_column = state.cursor.x as usize + state.column_index;

    state
        .columns
        .get(current_column)
        .map(|header| header.to_string())
}

fn update_cursor(state: &mut UIState<'_>, layout: &Layout) {
    let count_rows = layout.count_rows() as u16;
    if state.cursor.y >= count_rows {
        state.cursor.y = count_rows.saturating_sub(1) as u16;
    }

    let count_columns = layout.count_columns() as u16;
    if state.cursor.x >= count_columns {
        state.cursor.x = count_columns.saturating_sub(1) as u16;
    }
}

fn set_cursor<B>(f: &mut Frame<B>, state: &UIState<'_>, layout: &Layout)
where
    B: Backend,
{
    let Position { x: column, y: row } = state.cursor;
    let info = layout.get(row as usize, column as usize);
    if let Some(info) = info {
        f.set_cursor(info.position.x, info.position.y);
    }
}

fn handle_events<B>(
    events: &UIEvents,
    state: &mut UIState,
    layout: &Layout,
    term: &mut Terminal<B>,
) -> bool
where
    B: Backend,
{
    let key = match events.next() {
        Ok(Some(key)) => key,
        _ => return false,
    };

    if handle_exit_key_event(&key) {
        return true;
    }

    match state.mode {
        UIMode::View => view_mode_key_event(&key, state, layout, term),
        UIMode::Cursor => cursor_mode_key_event(&key, state, layout, term),
    }

    false
}

fn handle_exit_key_event(key: &KeyEvent) -> bool {
    matches!(
        key,
        KeyEvent {
            code: KeyCode::Char('d'),
            modifiers: KeyModifiers::CONTROL,
        } | KeyEvent {
            code: KeyCode::Char('z'),
            modifiers: KeyModifiers::CONTROL,
        }
    )
}

fn init_cursor_mode<B>(term: &mut Terminal<B>)
where
    B: Backend,
{
    let _ = term.show_cursor();
}

fn end_cursor_mode<B>(term: &mut Terminal<B>)
where
    B: Backend,
{
    let _ = term.hide_cursor();
}

fn view_mode_key_event<B>(
    key: &KeyEvent,
    state: &mut UIState<'_>,
    layout: &Layout,
    term: &mut Terminal<B>,
) where
    B: Backend,
{
    if state.is_search_input {
        let exit = search_input_key_event(key, state, layout);
        if exit {
            return;
        }
    }

    match key {
        KeyEvent {
            code: KeyCode::Esc, ..
        } => {
            // if state.render_inner {
            state.render_close = true;
            // }
        }
        KeyEvent {
            code: KeyCode::Char('i'),
            ..
        } => {
            init_cursor_mode(term);
            state.mode = UIMode::Cursor
        }
        KeyEvent {
            code: KeyCode::Char('?'),
            ..
        } => {
            state.buf_cmd_input = String::new();
            state.is_search_input = true;
            state.is_search_rev = true;
        }
        KeyEvent {
            code: KeyCode::Char('/'),
            ..
        } => {
            state.buf_cmd_input = String::new();
            state.is_search_input = true;
        }
        KeyEvent {
            code: KeyCode::Char('n'),
            ..
        } => {
            if !state.search_results.is_empty() {
                if state.buf_cmd_input.is_empty() {
                    state.buf_cmd_input = state.buf_cmd.clone();
                }

                if state.search_index == 0 {
                    state.search_index = state.search_results.len() - 1
                } else {
                    state.search_index -= 1;
                }

                let pos = state.search_results[state.search_index];
                state.row_index = pos.0;
                state.column_index = pos.1;
            }
        }
        KeyEvent { code, .. } => match code {
            KeyCode::Up => state.row_index = state.row_index.saturating_sub(1),
            KeyCode::Down => {
                let max_index = state.count_rows().saturating_sub(1);
                state.row_index = min(state.row_index + 1, max_index);
            }
            KeyCode::Left => state.column_index = state.column_index.saturating_sub(1),
            KeyCode::Right => {
                let max_index = state.count_columns().saturating_sub(1);
                state.column_index = min(state.column_index + 1, max_index);
            }
            KeyCode::PageUp => {
                let count_rows = layout.count_rows();
                state.row_index = state.row_index.saturating_sub(count_rows as usize);
            }
            KeyCode::PageDown => {
                let count_rows = layout.count_rows();
                let max_index = state.count_rows().saturating_sub(1);
                state.row_index = min(state.row_index + count_rows as usize, max_index);
            }
            _ => {}
        },
    }
}

fn cursor_mode_key_event<B>(
    key: &KeyEvent,
    state: &mut UIState<'_>,
    layout: &Layout,
    term: &mut Terminal<B>,
) where
    B: Backend,
{
    match key {
        KeyEvent {
            code: KeyCode::Esc, ..
        } => {
            if state.render_inner {
                state.render_close = true;
            } else {
                end_cursor_mode(term);

                state.mode = UIMode::View;
                state.cursor = Position::default();
            }
        }
        KeyEvent { code, .. } => match code {
            KeyCode::Up => {
                if state.cursor.y == 0 {
                    state.row_index = state.row_index.saturating_sub(1);
                } else {
                    state.cursor.y -= 1
                }
            }
            KeyCode::Down => {
                let showed_rows = layout.count_rows();
                let total_rows = state.count_rows();
                let row_index = state.row_index + state.cursor.y as usize + 1;

                if row_index < total_rows {
                    if state.cursor.y as usize + 1 == showed_rows {
                        state.row_index += 1;
                    } else {
                        state.cursor.y += 1;
                    }
                }
            }
            KeyCode::Left => {
                if state.cursor.x == 0 {
                    state.column_index = state.column_index.saturating_sub(1);
                } else {
                    state.cursor.x -= 1
                }
            }
            KeyCode::Right => {
                let showed_columns = layout.count_columns();
                let total_columns = state.count_columns();
                let column_index = state.column_index + state.cursor.x as usize + 1;

                if column_index < total_columns {
                    if state.cursor.x as usize + 1 == showed_columns {
                        state.column_index += 1;
                    } else {
                        state.cursor.x += 1;
                    }
                }
            }
            KeyCode::Enter => {
                state.render_inner = true;
            }
            _ => {}
        },
    }
}

fn search_input_key_event(key: &KeyEvent, state: &mut UIState<'_>, _layout: &Layout) -> bool {
    match &key.code {
        KeyCode::Esc => {
            state.buf_cmd_input = String::new();
            if !state.buf_cmd.is_empty() {
                state.search_results =
                    search_pattern(&state.data_text, &state.buf_cmd, state.is_search_rev);
                state.search_index = 0;
            }

            state.is_search_input = false;
            state.is_search_rev = false;

            true
        }
        KeyCode::Enter => {
            state.is_search_input = false;
            state.buf_cmd = state.buf_cmd_input.clone();
            state.is_search_rev = false;

            true
        }
        KeyCode::Backspace => {
            if state.buf_cmd_input.is_empty() {
                state.is_search_input = false;
                state.is_search_rev = false;
            } else {
                state.buf_cmd_input.pop();

                if !state.buf_cmd_input.is_empty() {
                    state.search_results =
                        search_pattern(&state.data_text, &state.buf_cmd_input, state.is_search_rev);
                    state.search_index = 0;

                    if !state.search_results.is_empty() {
                        let pos = state.search_results[state.search_index];
                        state.row_index = pos.0;
                        state.column_index = pos.1;
                    }
                }
            }

            true
        }
        KeyCode::Char(c) => {
            state.buf_cmd_input.push(*c);

            if !state.buf_cmd_input.is_empty() {
                state.search_results =
                    search_pattern(&state.data_text, &state.buf_cmd_input, state.is_search_rev);
                state.search_index = 0;

                if !state.search_results.is_empty() {
                    let pos = state.search_results[state.search_index];
                    state.row_index = pos.0;
                    state.column_index = pos.1;
                }
            }

            true
        }
        _ => false,
    }
}

fn search_pattern(data: &[Vec<NuText>], pat: &str, rev: bool) -> Vec<(usize, usize)> {
    let mut matches = Vec::new();
    for (row, columns) in data.iter().enumerate() {
        for (col, (text, _)) in columns.iter().enumerate() {
            if text.contains(pat) {
                matches.push((row, col));
            }
        }
    }

    if !rev {
        matches.sort();
    } else {
        matches.sort_by(|a, b| b.cmp(a));
    }

    matches
}

#[derive(Debug, Clone)]
struct UIState<'a> {
    columns: Cow<'a, [String]>,
    data: Cow<'a, [Vec<Value>]>,
    data_text: Vec<Vec<NuText>>,
    config: &'a Config,
    color_hm: &'a NuStyleTable,
    column_index: usize,
    row_index: usize,
    show_index: bool,
    show_header: bool,
    mode: UIMode,
    // only applicable for CusorMode
    cursor: Position,
    // only applicable for CusorMode
    render_inner: bool,
    // only applicable for CusorMode
    render_close: bool,
    // only applicable for CusorMode
    section_name: Option<String>,
    // only applicable for SEARCH input
    is_search_input: bool,
    // only applicable for SEARCH input
    buf_cmd: String,
    // only applicable for SEARCH input
    buf_cmd_input: String,
    // only applicable for SEARCH input
    search_results: Vec<(usize, usize)>,
    search_index: usize,
    // only applicable for rev-SEARCH input
    is_search_rev: bool,
    style: StyleConfig,
}

#[derive(Debug, Default, Clone)]
pub struct StyleConfig {
    pub status_bar: NuStyle,
    pub cmd_bar: NuStyle,
    pub split_line: NuStyle,
    pub highlight: NuStyle,
    pub selected_cell: Option<NuStyle>,
    pub selected_column: Option<NuStyle>,
    pub selected_row: Option<NuStyle>,
    pub show_cursow: bool,
}

impl<'a> UIState<'a> {
    fn new(
        columns: Cow<'a, [String]>,
        data: Cow<'a, [Vec<Value>]>,
        config: &'a Config,
        color_hm: &'a NuStyleTable,
        show_header: bool,
        show_index: bool,
        style: StyleConfig,
    ) -> Self {
        let data_text = data
            .iter()
            .map(|row| {
                row.iter()
                    .map(|value| {
                        make_styled_string(
                            value.clone().into_abbreviated_string(config),
                            &value.get_type().to_string(),
                            0,
                            false,
                            color_hm,
                            config.float_precision as usize,
                        )
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();

        Self {
            columns,
            data,
            config,
            color_hm,
            column_index: 0,
            row_index: 0,
            show_header,
            show_index,
            mode: UIMode::View,
            cursor: Position::new(0, 0),
            render_inner: false,
            render_close: false,
            section_name: None,
            buf_cmd: String::new(),
            buf_cmd_input: String::new(),
            is_search_input: false,
            search_results: Vec::new(),
            search_index: 0,
            is_search_rev: false,
            data_text,
            style,
        }
    }

    fn count_rows(&self) -> usize {
        self.data.len()
    }

    fn count_columns(&self) -> usize {
        self.columns.len()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum UIMode {
    Cursor,
    View,
}

struct StatusBar<'a> {
    section: Option<&'a str>,
    cursor: Option<Position>,
    row: usize,
    column: usize,
    seen_rows: usize,
    total_rows: usize,
    style: NuStyle,
}

impl<'a> StatusBar<'a> {
    fn new(
        section: Option<&'a str>,
        cursor: Option<Position>,
        row: usize,
        column: usize,
        seen_rows: usize,
        total_rows: usize,
        style: NuStyle,
    ) -> Self {
        Self {
            section,
            cursor,
            row,
            column,
            seen_rows,
            total_rows,
            style,
        }
    }
}

impl Widget for StatusBar<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let block_style = nu_style_to_tui(self.style);
        let text_style = nu_style_to_tui(self.style).add_modifier(Modifier::BOLD);

        // colorize the line
        let block = Block::default()
            .borders(Borders::empty())
            .style(block_style);
        block.render(area, buf);

        let percent_rows = get_percentage(self.seen_rows, self.total_rows);

        let covered_percent = match percent_rows {
            100 => String::from("All"),
            _ if self.row == 0 => String::from("Top"),
            value => format!("{}%", value),
        };

        let span = Span::styled(&covered_percent, text_style);
        let covered_percent_w = covered_percent.len() as u16;
        buf.set_span(
            area.right().saturating_sub(covered_percent_w),
            area.y,
            &span,
            covered_percent_w,
        );

        if let Some(name) = self.section {
            let width = area.width.saturating_sub(3 + 12 + 12 + 12);
            let name = nu_table::string_truncate(name, width as usize);
            let span = Span::styled(name, text_style);
            buf.set_span(area.left(), area.y, &span, width);
        }

        if let Some(pos) = self.cursor {
            render_cursor_position(buf, area, text_style, pos, self.row, self.column);
        }
    }
}

#[derive(Debug)]
struct CmdBar<'a> {
    text: &'a str,
    information: &'a str,
    style: NuStyle,
}

impl<'a> CmdBar<'a> {
    fn new(text: &'a str, information: &'a str, style: NuStyle) -> Self {
        Self {
            text,
            information,
            style,
        }
    }
}

impl Widget for CmdBar<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let text_style = nu_style_to_tui(self.style).add_modifier(Modifier::BOLD);

        // colorize the line
        let block = Block::default()
            .borders(Borders::empty())
            .style(Style::default());
        block.render(area, buf);

        let span = Span::styled(self.text, text_style);
        let w = string_width(self.text);
        buf.set_span(area.x, area.y, &span, w as u16);

        let span = Span::styled(self.information, text_style);
        let w = string_width(self.information);
        buf.set_span(
            area.right().saturating_sub(12).saturating_sub(w as u16),
            area.y,
            &span,
            w as u16,
        );
    }
}

fn render_cursor_position(
    buf: &mut Buffer,
    area: Rect,
    text_style: Style,
    pos: Position,
    row: usize,
    column: usize,
) {
    let actual_row = row + pos.y as usize;
    let actual_column = column + pos.x as usize;

    let text = format!("{},{}", actual_row, actual_column);
    let width = text.len() as u16;

    let span = Span::styled(text, text_style);
    buf.set_span(
        area.right().saturating_sub(3 + 12 + width),
        area.y,
        &span,
        width,
    );
}

struct Table<'a> {
    columns: &'a [String],
    data: &'a [Vec<NuText>],
    color_hm: &'a NuStyleTable,
    column_index: usize,
    row_index: usize,
    show_index: bool,
    show_header: bool,
    splitline_style: NuStyle,
}

impl<'a> From<&'a UIState<'_>> for Table<'a> {
    fn from(state: &'a UIState<'_>) -> Self {
        Self {
            columns: &state.columns,
            data: &state.data_text,
            color_hm: state.color_hm,
            column_index: state.column_index,
            row_index: state.row_index,
            show_index: state.show_index,
            show_header: state.show_header,
            splitline_style: state.style.split_line,
        }
    }
}

impl StatefulWidget for Table<'_> {
    type State = Layout;

    fn render(
        self,
        area: tui::layout::Rect,
        buf: &mut tui::buffer::Buffer,
        state: &mut Self::State,
    ) {
        const CELL_PADDING_LEFT: u16 = 2;
        const CELL_PADDING_RIGHT: u16 = 2;

        let show_index = self.show_index;
        let show_head = self.show_header;

        let mut data_y = area.y;
        if show_head {
            data_y += 3;
        }

        let head_y = area.y + 1;

        if area.width == 0 || area.height == 0 {
            return;
        }

        let mut data_height = area.height;
        if show_head {
            data_height -= 3;
        }

        let mut width = area.x;

        let mut data = &self.data[self.row_index..];
        if data.len() > data_height as usize {
            data = &data[..data_height as usize];
        }

        // header lines
        if show_head {
            render_header_borders(buf, area, 0, 1);
        }

        if show_index {
            let area = Rect::new(width, data_y, area.width, data_height);
            width += render_index(buf, area, self.color_hm, self.row_index);
            width += render_vertical(
                buf,
                width,
                data_y,
                data_height,
                show_head,
                self.splitline_style,
            );
        }

        let mut do_render_split_line = true;
        let mut do_render_shift_column = false;

        for col in self.column_index..self.columns.len() {
            let mut head = String::from(&self.columns[col]);

            let mut column = create_column(data, col);

            let column_width = calculate_column_width(&column);
            let mut use_space = column_width as u16;

            if show_head {
                let head_width = string_width(&head);
                use_space = max(head_width as u16, use_space);
            }

            {
                let available_space = area.width - width;
                let head = show_head.then(|| &mut head);
                let control = truncate_column(
                    &mut column,
                    head,
                    available_space,
                    col + 1 == self.columns.len(),
                    PrintControl {
                        break_everything: false,
                        print_shift_column: false,
                        print_split_line: true,
                        width: use_space,
                    },
                );

                use_space = control.width;
                do_render_split_line = control.print_split_line;
                do_render_shift_column = control.print_shift_column;

                if control.break_everything {
                    break;
                }
            }

            if show_head {
                let header = &[head_row_text(&head, self.color_hm)];

                let mut w = width;
                w += render_space(buf, w, head_y, 1, CELL_PADDING_LEFT);
                w += render_column(buf, w, head_y, use_space, header);
                render_space(buf, w, head_y, 1, CELL_PADDING_RIGHT);

                state.push_head(w - CELL_PADDING_RIGHT - use_space, use_space, (0, col));
            }

            width += render_space(buf, width, data_y, data_height, CELL_PADDING_LEFT);
            width += render_column(buf, width, data_y, use_space, &column);
            width += render_space(buf, width, data_y, data_height, CELL_PADDING_RIGHT);

            state.push_column(
                width - CELL_PADDING_RIGHT - use_space,
                data_y,
                use_space,
                (0..column.len()).map(|i| (i + self.row_index, col)),
            );

            if do_render_shift_column {
                break;
            }
        }

        if do_render_shift_column {
            // we actually want to show a shift only in header.
            //
            // render_shift_column(buf, used_width, head_offset, available_height);

            if show_head {
                width += render_space(buf, width, data_y, data_height, CELL_PADDING_LEFT);
                width += render_shift_column(buf, width, head_y, 1, self.splitline_style);
                width += render_space(buf, width, data_y, data_height, CELL_PADDING_RIGHT);
            }
        }

        if do_render_split_line {
            width += render_vertical(
                buf,
                width,
                data_y,
                data_height,
                show_head,
                self.splitline_style,
            );
        }

        // we try out best to cleanup the rest of the space cause it could be meassed.
        let rest = area.width.saturating_sub(width);
        if rest > 0 {
            render_space(buf, width, data_y, data_height, rest);
            if show_head {
                render_space(buf, width, head_y, 1, rest);
            }
        }
    }
}

struct IndexColumn<'a> {
    color_hm: &'a NuStyleTable,
    start: usize,
}

impl<'a> IndexColumn<'a> {
    fn new(color_hm: &'a NuStyleTable, start: usize) -> Self {
        Self { color_hm, start }
    }

    fn estimate_width(&self, height: u16) -> usize {
        let last_row = self.start + height as usize;
        last_row.to_string().len()
    }
}

impl Widget for IndexColumn<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let style = nu_style_to_tui(self.color_hm["row_index"]);

        for row in 0..area.height {
            let i = 1 + row as usize + self.start;
            let text = i.to_string();

            let p = Paragraph::new(text)
                .style(style)
                .alignment(tui::layout::Alignment::Right);
            let area = Rect::new(area.x, area.y + row, area.width, 1);

            p.render(area, buf);
        }
    }
}

fn nu_style_to_tui(style: NuStyle) -> tui::style::Style {
    let mut out = tui::style::Style::default();
    if let Some(clr) = style.background {
        out.bg = nu_ansi_color_to_tui_color(clr);
    }

    if let Some(clr) = style.foreground {
        out.fg = nu_ansi_color_to_tui_color(clr);
    }

    out
}

fn render_index(buf: &mut Buffer, area: Rect, color_hm: &NuStyleTable, start_index: usize) -> u16 {
    const PADDING_LEFT: u16 = 2;
    const PADDING_RIGHT: u16 = 1;

    let mut width = render_space(buf, area.x, area.y, area.height, PADDING_LEFT);

    let index = IndexColumn::new(color_hm, start_index);
    let w = index.estimate_width(area.height) as u16;
    let area = Rect::new(area.x + width, area.y, w, area.height);

    index.render(area, buf);

    width += w;
    width += render_space(buf, area.x + width, area.y, area.height, PADDING_RIGHT);

    width
}

#[derive(Debug, Default)]
struct Layout {
    headers: Vec<ElementInfo>,
    data: Vec<Vec<ElementInfo>>,
    count_columns: usize,
    // todo: add Vec<width> of columns so data would contain actual width of a content
}

impl Layout {
    fn count_columns(&self) -> usize {
        self.count_columns
    }

    fn count_rows(&self) -> usize {
        self.data.first().map_or(0, |col| col.len())
    }

    fn push_head(&mut self, x: u16, width: u16, value: (usize, usize)) {
        self.headers
            .push(ElementInfo::new(Position::new(x, 1), width, 1, value));
    }

    fn push_column(
        &mut self,
        x: u16,
        y: u16,
        width: u16,
        values: impl Iterator<Item = (usize, usize)>,
    ) {
        self.count_columns += 1;

        let columns = values
            .enumerate()
            .map(|(i, value)| ElementInfo::new(Position::new(x, y + i as u16), width, 1, value))
            .collect();

        self.data.push(columns);
    }

    fn get(&self, row: usize, column: usize) -> Option<ElementInfo> {
        self.data.get(column).and_then(|col| col.get(row)).cloned()
    }
}

#[allow(dead_code)]
#[derive(Debug, Default, Clone)]
struct ElementInfo {
    data_pos: (usize, usize),
    // todo: change to area: Rect
    position: Position,
    width: u16,
    height: u16,
}

impl ElementInfo {
    fn new(position: Position, width: u16, height: u16, data_pos: (usize, usize)) -> Self {
        Self {
            position,
            width,
            height,
            data_pos,
        }
    }
}

#[derive(Debug, Default, Clone, Copy)]
struct Position {
    x: u16,
    y: u16,
}

impl Position {
    fn new(x: u16, y: u16) -> Self {
        Self { x, y }
    }
}

fn state_reverse_data(state: &mut UIState<'_>, page_size: usize) {
    if state.data.len() > page_size as usize {
        state.row_index = state.data.len() - page_size as usize;
    }
}

fn render_header_borders(buf: &mut Buffer, area: Rect, y: u16, span: u16) -> (u16, u16) {
    let block = Block::default()
        .borders(Borders::TOP | Borders::BOTTOM)
        .border_style(Style::default().fg(Color::Rgb(64, 64, 64)));
    let height = 2 + span;
    let area = Rect::new(area.x, area.y + y, area.width, height);
    block.render(area, buf);
    // y pos of header text and next line
    (height.saturating_sub(2), height)
}

fn create_column(data: &[Vec<NuText>], col: usize) -> Vec<NuText> {
    let mut column = vec![NuText::default(); data.len()];
    for (row, values) in data.iter().enumerate() {
        if values.is_empty() {
            debug_assert!(false, "must never happen?");
            continue;
        }

        let value = &values[col];
        column[row] = value.clone();
    }

    column
}

#[derive(Debug, Default, Copy, Clone)]
struct PrintControl {
    width: u16,
    break_everything: bool,
    print_split_line: bool,
    print_shift_column: bool,
}

fn truncate_column(
    column: &mut [NuText],
    head: Option<&mut String>,
    available_space: u16,
    is_column_last: bool,
    mut control: PrintControl,
) -> PrintControl {
    const CELL_PADDING_LEFT: u16 = 2;
    const CELL_PADDING_RIGHT: u16 = 2;
    const VERTICAL_LINE_WIDTH: u16 = 1;
    const CELL_MIN_WIDTH: u16 = 1;

    let min_space_cell = CELL_PADDING_LEFT + CELL_PADDING_RIGHT + CELL_MIN_WIDTH;
    let min_space = min_space_cell + VERTICAL_LINE_WIDTH;
    if available_space < min_space {
        // if there's not enough space at all just return; doing our best
        if available_space < VERTICAL_LINE_WIDTH {
            control.print_split_line = false;
        }

        control.break_everything = true;
        return control;
    }

    let column_taking_space =
        control.width + CELL_PADDING_LEFT + CELL_PADDING_RIGHT + VERTICAL_LINE_WIDTH;
    let is_enough_space = available_space > column_taking_space;
    if !is_enough_space {
        if is_column_last {
            // we can do nothing about it we need to truncate.
            // we assume that there's always at least space for padding and 1 symbol. (5 chars)

            let width = available_space
                .saturating_sub(CELL_PADDING_LEFT + CELL_PADDING_RIGHT + VERTICAL_LINE_WIDTH);
            if width == 0 {
                control.break_everything = true;
                return control;
            }

            if let Some(head) = head {
                truncate_str(head, width as usize);
            }

            truncate_list(column, width as usize);

            control.width = width;
        } else {
            let min_space_2cells = min_space + min_space_cell;
            if available_space > min_space_2cells {
                let width = available_space.saturating_sub(min_space_2cells);
                if width == 0 {
                    control.break_everything = true;
                    return control;
                }

                truncate_list(column, width as usize);

                if let Some(head) = head {
                    truncate_str(head, width as usize);
                }

                control.width = width;
                control.print_shift_column = true;
            } else {
                control.break_everything = true;
                control.print_shift_column = true;
            }
        }
    } else if !is_column_last {
        // even though we can safely render current column,
        // we need to check whether there's enough space for AT LEAST a shift column
        // (2 padding + 2 padding + 1 a char)
        let left_space = available_space - column_taking_space;
        if left_space < min_space {
            let need_space = min_space_cell - left_space;
            let min_left_width = 1;
            let is_column_big_enough = control.width > need_space + min_left_width;

            if is_column_big_enough {
                let width = control.width.saturating_sub(need_space);
                if width == 0 {
                    control.break_everything = true;
                    return control;
                }

                truncate_list(column, width as usize);

                if let Some(head) = head {
                    truncate_str(head, width as usize);
                }

                control.width = width;
                control.print_shift_column = true;
            }
        }
    }

    control
}

fn get_percentage(value: usize, max: usize) -> usize {
    debug_assert!(value <= max, "{:?} {:?}", value, max);

    ((value as f32 / max as f32) * 100.0).floor() as usize
}

fn estimate_page_size(area: Rect, show_head: bool) -> u16 {
    let mut available_height = area.height;
    available_height -= 3; // status_bar

    if show_head {
        available_height -= 3; // head
    }

    available_height
}

fn head_row_text(head: &str, color_hm: &NuStyleTable) -> NuText {
    (
        String::from(head),
        TextStyle {
            alignment: Alignment::Center,
            color_style: Some(color_hm["header"]),
        },
    )
}

fn truncate_list(list: &mut [NuText], width: usize) {
    for (text, _) in list {
        truncate_str(text, width);
    }
}

fn truncate_str(text: &mut String, width: usize) {
    if width == 0 {
        text.clear();
    } else {
        *text = nu_table::string_truncate(text, width - 1);
        text.push('…');
    }
}

fn render_shift_column(buf: &mut Buffer, x: u16, y: u16, height: u16, style: NuStyle) -> u16 {
    let style = TextStyle {
        alignment: Alignment::Left,
        color_style: Some(style),
    };

    repeat_vertical(buf, x, y, 1, height, '…', style);

    1
}

fn render_vertical(
    buf: &mut Buffer,
    x: u16,
    y: u16,
    height: u16,
    show_header: bool,
    style: NuStyle,
) -> u16 {
    render_vertical_split(buf, x, y, height, style);

    if show_header && y > 0 {
        render_top_connector(buf, x, y - 1, style);
    }

    // render_bottom_connector(buf, x, height + y);

    1
}

fn render_vertical_split(buf: &mut Buffer, x: u16, y: u16, height: u16, style: NuStyle) {
    let style = TextStyle {
        alignment: Alignment::Left,
        color_style: Some(style),
    };

    repeat_vertical(buf, x, y, 1, height, '│', style);
}

fn render_top_connector(buf: &mut Buffer, x: u16, y: u16, style: NuStyle) {
    let style = nu_style_to_tui(style);
    let span = Span::styled("┬", style);
    buf.set_span(x, y, &span, 1);
}

fn render_space(buf: &mut Buffer, x: u16, y: u16, height: u16, padding: u16) -> u16 {
    repeat_vertical(buf, x, y, padding, height, ' ', TextStyle::default());
    padding
}

fn calculate_column_width(column: &[NuText]) -> usize {
    column
        .iter()
        .map(|(text, _)| text)
        .map(|text| string_width(text))
        .max()
        .unwrap_or(0)
}

fn render_column(
    buf: &mut tui::buffer::Buffer,
    x_offset: u16,
    y_offset: u16,
    available_width: u16,
    rows: &[NuText],
) -> u16 {
    for (row, (text, style)) in rows.iter().enumerate() {
        let text = strip_string(text);
        let style = text_style_to_tui_style(*style);
        let span = Span::styled(text, style);
        buf.set_span(x_offset, y_offset + row as u16, &span, available_width);
    }

    available_width
}

fn strip_string(text: &str) -> String {
    strip_ansi_escapes::strip(text)
        .ok()
        .and_then(|s| String::from_utf8(s).ok())
        .unwrap_or_else(|| text.to_owned())
}

fn repeat_vertical(
    buf: &mut tui::buffer::Buffer,
    x_offset: u16,
    y_offset: u16,
    width: u16,
    height: u16,
    c: char,
    style: TextStyle,
) {
    let text = std::iter::repeat(c)
        .take(width as usize)
        .collect::<String>();
    let style = text_style_to_tui_style(style);
    let span = Span::styled(text, style);

    for row in 0..height {
        buf.set_span(x_offset, y_offset + row as u16, &span, width);
    }
}

fn text_style_to_tui_style(style: TextStyle) -> tui::style::Style {
    let mut out = tui::style::Style::default();
    if let Some(style) = style.color_style {
        if let Some(clr) = style.background {
            out.bg = nu_ansi_color_to_tui_color(clr);
        }

        if let Some(clr) = style.foreground {
            out.fg = nu_ansi_color_to_tui_color(clr);
        }
    }

    out
}

fn nu_ansi_color_to_tui_color(clr: NuColor) -> Option<tui::style::Color> {
    use NuColor::*;

    let clr = match clr {
        Black => Color::Black,
        DarkGray => Color::DarkGray,
        Red => Color::Red,
        LightRed => Color::LightRed,
        Green => Color::Green,
        LightGreen => Color::LightGreen,
        Yellow => Color::Yellow,
        LightYellow => Color::LightYellow,
        Blue => Color::Blue,
        LightBlue => Color::LightBlue,
        Magenta => Color::Magenta,
        LightMagenta => Color::LightMagenta,
        Cyan => Color::Cyan,
        LightCyan => Color::LightCyan,
        White => Color::White,
        Fixed(i) => Color::Indexed(i),
        Rgb(r, g, b) => tui::style::Color::Rgb(r, g, b),
        LightGray => Color::Gray,   // todo: make a PR to add the color
        LightPurple => Color::Blue, // todo: make a PR to add the color,
        Purple => Color::Blue,      // todo: make a PR to add the color,
        Default => return None,
    };

    Some(clr)
}

fn make_styled_string(
    text: String,
    text_type: &str,
    col: usize,
    with_index: bool,
    color_hm: &NuStyleTable,
    float_precision: usize,
) -> NuText {
    if col == 0 && with_index {
        (
            text,
            TextStyle {
                alignment: Alignment::Right,
                color_style: Some(color_hm["row_index"]),
            },
        )
    } else if text_type == "float" {
        // set dynamic precision from config
        let precise_number = match convert_with_precision(&text, float_precision) {
            Ok(num) => num,
            Err(e) => e.to_string(),
        };
        (precise_number, style_primitive(text_type, color_hm))
    } else {
        (text, style_primitive(text_type, color_hm))
    }
}

fn convert_with_precision(val: &str, precision: usize) -> Result<String> {
    // vall will always be a f64 so convert it with precision formatting
    match val.trim().parse::<f64>() {
        Ok(f) => Ok(format!("{:.prec$}", f, prec = precision)),
        Err(err) => {
            let message = format!("error converting string [{}] to f64; {}", &val, err);
            Err(io::Error::new(io::ErrorKind::Other, message))
        }
    }
}

pub struct UIEvents {
    tick_rate: Duration,
}

pub struct Cfg {
    pub tick_rate: Duration,
}

impl Default for Cfg {
    fn default() -> Cfg {
        Cfg {
            tick_rate: Duration::from_millis(250),
        }
    }
}

impl UIEvents {
    pub fn new() -> UIEvents {
        UIEvents::with_config(Cfg::default())
    }

    pub fn with_config(config: Cfg) -> UIEvents {
        UIEvents {
            tick_rate: config.tick_rate,
        }
    }

    pub fn next(&self) -> Result<Option<KeyEvent>> {
        let now = Instant::now();
        match poll(self.tick_rate) {
            Ok(true) => match read()? {
                Event::Key(event) => Ok(Some(event)),
                _ => {
                    let time_spent = now.elapsed();
                    let rest = self.tick_rate - time_spent;

                    Self { tick_rate: rest }.next()
                }
            },
            Ok(false) => Ok(None),
            Err(err) => Err(err),
        }
    }
}
