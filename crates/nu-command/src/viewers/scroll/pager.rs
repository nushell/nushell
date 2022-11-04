use std::{
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
use nu_protocol::{ast::PathMember, Config, Span as NuSpan, Value};
use nu_table::{string_width, Alignment, TextStyle};
use num_traits::Saturating;
use reedline::KeyModifiers;
use tui::{
    backend::{Backend, CrosstermBackend},
    buffer::Buffer,
    layout::{Constraint, Direction, Rect},
    style::{Color, Style},
    text::Span,
    widgets::{Block, Borders, StatefulWidget, Widget},
    Frame, Terminal,
};

type NuText = (String, TextStyle);

type CtrlC = Option<Arc<AtomicBool>>;

type NuStyleTable = HashMap<String, NuStyle>;

pub fn handler(
    cols: &[String],
    data: &[Value],
    config: &Config,
    ctrlc: CtrlC,
    show_index: bool,
    show_head: bool,
    reverse: bool,
) -> Result<()> {
    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, Clear(ClearType::All))?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let color_hm = get_color_config(config);
    let mut state = UIState::new(cols, data, config, &color_hm, show_head, show_index);

    if reverse {
        if let Ok(size) = terminal.size() {
            let page_size = estimate_page_size(size, show_head);
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

    loop {
        // handle CTRLC event
        if let Some(ctrlc) = ctrlc.clone() {
            if ctrlc.load(Ordering::SeqCst) {
                break Ok(());
            }
        }

        let mut layout = Layout::default();
        terminal.draw(|f| {
            f.render_stateful_widget(state, f.size(), &mut layout);

            if state.mode == UIMode::Cursor {
                update_cursor(&mut state, &layout);
                set_cursor(f, &state, &layout);
            }

            // if state.render_inner {
            //     let block = Block::default().title("Popup").borders(Borders::ALL);
            //     let area = centered_rect(60, 20, f.size());
            //     f.render_widget(tui::widgets::Clear, area); //this clears out the background
            //     f.render_widget(block, area);

            //     let state = UIState {

            //     }

            //     f.render_stateful_widget(state, f.size(), &mut layout);
            //     return
            // }
        })?;

        let exited = handle_events(&events, &mut state, &layout, terminal);
        if exited {
            break Ok(());
        }
    }
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
    let event = events.next().unwrap();
    let key = match event {
        Some(event) => event,
        None => return false,
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
    term.show_cursor().unwrap();
}

fn end_cursor_mode<B>(term: &mut Terminal<B>)
where
    B: Backend,
{
    term.hide_cursor().unwrap();
}

fn view_mode_key_event<B>(
    key: &KeyEvent,
    state: &mut UIState<'_>,
    layout: &Layout,
    term: &mut Terminal<B>,
) where
    B: Backend,
{
    match key {
        KeyEvent {
            code: KeyCode::Char('i'),
            ..
        } => {
            init_cursor_mode(term);
            state.mode = UIMode::Cursor
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
            end_cursor_mode(term);

            state.mode = UIMode::View;
            state.cursor = Position::default();
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
                let showed_rows = layout.count_rows() as u16;
                let total_rows = state.data.len() as u16;
                let row_index = state.row_index as u16 + state.cursor.y + 1;

                if row_index < total_rows {
                    if state.cursor.y + 1 == showed_rows {
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
                let showed_columns = layout.count_rows() as u16;
                let total_columns = state.count_columns() as u16;
                let column_index = state.column_index as u16 + state.cursor.x + 1;

                if column_index < total_columns {
                    if state.cursor.x + 1 == showed_columns {
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

/// helper function to create a centered rect using up certain percentage of the available rect `r`
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = tui::layout::Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Percentage((100 - percent_y) / 2),
                Constraint::Percentage(percent_y),
                Constraint::Percentage((100 - percent_y) / 2),
            ]
            .as_ref(),
        )
        .split(r);

    tui::layout::Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            [
                Constraint::Percentage((100 - percent_x) / 2),
                Constraint::Percentage(percent_x),
                Constraint::Percentage((100 - percent_x) / 2),
            ]
            .as_ref(),
        )
        .split(popup_layout[1])[1]
}

#[derive(Debug, Clone, Copy)]
struct UIState<'a> {
    columns: &'a [String],
    data: &'a [Value],
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
}

impl<'a> UIState<'a> {
    fn new(
        columns: &'a [String],
        data: &'a [Value],
        config: &'a Config,
        color_hm: &'a NuStyleTable,
        show_header: bool,
        show_index: bool,
    ) -> Self {
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

impl StatefulWidget for UIState<'_> {
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
        let has_head = !self.columns.is_empty();

        let mut head_offset = 0;
        if show_head {
            head_offset = 3;
        }

        let status_bar_offset = 3;
        let min_data_offset = 1;

        let term_min_height = status_bar_offset + head_offset + min_data_offset;
        let term_min_width = 1;

        if area.width < term_min_width || area.height < term_min_height {
            return;
        }

        let mut height = area.height;
        height -= status_bar_offset;
        if show_head {
            height -= 3;
        }

        let mut width = 0;

        let mut rows = &self.data[self.row_index..];
        if rows.len() > height as usize {
            rows = &rows[..height as usize];
        }

        // header lines
        if show_head {
            render_header_borders(buf, area, 0, 1);
        }

        // status_bar
        render_header_borders(buf, area, area.height - 3, 1);

        if show_index {
            width = render_column_index(
                buf,
                height,
                self.row_index,
                show_head,
                head_offset,
                self.color_hm,
            );

            width += render_vertical(buf, width, head_offset, height, show_head);
        }

        let mut head: Option<String> = None;
        let mut head_width = 0;

        let mut do_render_split_line = true;
        let mut do_render_shift_column = false;

        let mut shown_columns = 0;
        for col in self.column_index..self.columns.len() {
            if has_head {
                let text = String::from(&self.columns[col]);
                head_width = string_width(&text);
                head = Some(text);
            }

            let mut column = create_column(head.as_deref(), rows, self.config, self.color_hm);

            let available_space = area.width - width;
            let column_width = calculate_column_width(&column);
            let mut use_space = max(head_width as u16, column_width as u16);

            {
                let control = truncate_column(
                    &mut column,
                    head.as_mut(),
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
                let head: &str = head.as_deref().unwrap_or_default();
                let header_data = &[head_row_text(head, self.color_hm)];

                let mut w = width;
                w += render_space(buf, w, 1, 1, CELL_PADDING_LEFT);
                w += render_column(buf, w, 1, use_space, header_data);
                render_space(buf, w, 1, 1, CELL_PADDING_RIGHT);

                state.push_head(w - CELL_PADDING_RIGHT - use_space, use_space)
            }

            width += render_space(buf, width, head_offset, height, CELL_PADDING_LEFT);
            width += render_column(buf, width, head_offset, use_space, &column);
            width += render_space(buf, width, head_offset, height, CELL_PADDING_RIGHT);

            shown_columns += 1;

            state.push_column(
                width - CELL_PADDING_RIGHT - use_space,
                head_offset,
                use_space,
                column.len() as u16,
            );

            if do_render_shift_column {
                break;
            }
        }

        // status_bar
        let message = create_length_message(&self, height, shown_columns);
        render_status_bar(buf, area, &message);

        if do_render_shift_column {
            // we actually want to show a shift only in header.
            //
            // render_shift_column(buf, used_width, head_offset, available_height);

            if show_head {
                width += render_space(buf, width, head_offset, height, CELL_PADDING_LEFT);
                width += render_shift_column(buf, width, 1, 1);
                width += render_space(buf, width, head_offset, height, CELL_PADDING_RIGHT);
            }
        }

        if do_render_split_line {
            width += render_vertical(buf, width, head_offset, height, show_head);
        }

        // we try out best to cleanup the rest of the space cause it could be meassed.
        let rest = area.width.saturating_sub(width);
        if rest > 0 {
            render_space(buf, width, head_offset, height, rest);
            if show_head {
                render_space(buf, width, 1, 1, rest);
            }
        }
    }
}

#[derive(Debug, Default)]
struct Layout {
    headers: Vec<ElementInfo>,
    data: Vec<Vec<ElementInfo>>,
}

impl Layout {
    fn count_columns(&self) -> usize {
        self.data.len()
    }

    fn count_rows(&self) -> usize {
        self.data.first().map_or(0, |col| col.len())
    }

    fn push_head(&mut self, x: u16, width: u16) {
        self.headers
            .push(ElementInfo::new(Position::new(x, 1), width, 1));
    }

    fn push_column(&mut self, x: u16, y: u16, width: u16, count_elements: u16) {
        let columns = (0..count_elements)
            .map(|i| ElementInfo::new(Position::new(x, y + i), width, 1))
            .collect();
        self.data.push(columns);
    }

    fn get(&self, row: usize, column: usize) -> Option<ElementInfo> {
        self.data.get(column).and_then(|col| col.get(row)).copied()
    }
}

#[derive(Debug, Default, Clone, Copy)]
struct ElementInfo {
    position: Position,
    width: u16,
    height: u16,
}

impl ElementInfo {
    fn new(position: Position, width: u16, height: u16) -> Self {
        Self {
            position,
            width,
            height,
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
    let area = Rect::new(0, y, area.width, height);
    block.render(area, buf);
    // y pos of header text and next line
    (height.saturating_sub(2), height)
}

fn create_column(
    head: Option<&str>,
    rows: &[Value],
    config: &Config,
    color_hm: &NuStyleTable,
) -> Vec<NuText> {
    if let Some(head) = head {
        create_column_with_head(config, color_hm, NuSpan::unknown(), head, rows)
    } else {
        create_column_raw(rows, config, color_hm)
    }
}

fn create_column_raw(rows: &[Value], config: &Config, color_hm: &NuStyleTable) -> Vec<NuText> {
    rows.iter()
        .map(|item| value_to_string(item.clone(), config, color_hm))
        .collect()
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

fn create_length_message(state: &UIState<'_>, height: u16, count_columns: usize) -> String {
    let row_status = {
        let seen = state.row_index + height as usize;
        let is_last_row_reached = seen >= state.data.len();
        if is_last_row_reached {
            String::from("[END]")
        } else {
            format!("[{}/{}]", seen, state.data.len())
        }
    };

    let mut column_status = String::new();
    if state.show_header && !state.columns.is_empty() {
        let seen = state.column_index + count_columns;
        let is_last_column_reached = seen >= state.columns.len();
        if is_last_column_reached {
            column_status = String::from("[END]")
        } else {
            column_status = format!("[{}/{}]", seen, state.columns.len())
        }
    };

    let mut message = row_status;

    if !column_status.is_empty() {
        message.push(' ');
        message.push_str(&column_status);
    }

    message
}

fn render_status_bar(buf: &mut Buffer, area: Rect, message: &str) {
    let style = Style::default().fg(Color::Rgb(128, 128, 128));
    let span = Span::styled(message, style);
    buf.set_span(area.x, area.bottom().saturating_sub(2), &span, area.width);
}

fn estimate_page_size(area: Rect, show_head: bool) -> u16 {
    let mut available_height = area.height;
    available_height -= 3; // status_bar

    if show_head {
        available_height -= 3; // head
    }

    available_height
}

const VERTICAL_LINE_COLOR: NuColor = NuColor::Rgb(64, 64, 64);

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

fn render_column_index(
    buf: &mut Buffer,
    height: u16,
    starts_at: usize,
    show_header: bool,
    header_offset: u16,
    color_hm: &NuStyleTable,
) -> u16 {
    const CELL_PADDING_LEFT: u16 = 2;
    const CELL_PADDING_RIGHT: u16 = 2;

    let mut head = (String::new(), TextStyle::default());
    let mut head_width = 0;
    if show_header {
        head = get_index_column_name(color_hm);
        head_width = string_width(&head.0) as u16;
    }

    let index = (0..height as usize)
        .map(|i| i + starts_at)
        .map(|i| create_column_index(i, color_hm))
        .collect::<Vec<_>>();

    let index_col_width = index
        .last()
        .map(|(s, _)| string_width(s) as u16)
        .unwrap_or(0);
    let index_width = max(head_width, index_col_width);

    render_column(buf, CELL_PADDING_LEFT, header_offset, index_width, &index);

    if show_header {
        render_column(buf, CELL_PADDING_LEFT, 1, index_width, &[head]);
    }

    index_width + CELL_PADDING_LEFT + CELL_PADDING_RIGHT
}

fn render_shift_column(buf: &mut Buffer, x: u16, y: u16, height: u16) -> u16 {
    let style = TextStyle {
        alignment: Alignment::Left,
        color_style: Some(NuStyle::default().fg(VERTICAL_LINE_COLOR)),
    };

    repeat_vertical(buf, x, y, 1, height, '…', style);

    1
}

fn render_vertical(buf: &mut Buffer, x: u16, y: u16, height: u16, show_header: bool) -> u16 {
    render_vertical_split(buf, x, y, height);

    if show_header && y > 0 {
        render_top_connector(buf, x, y - 1);
    }

    render_bottom_connector(buf, x, height + y);

    1
}

fn render_vertical_split(buf: &mut Buffer, x: u16, y: u16, height: u16) {
    let style = TextStyle {
        alignment: Alignment::Left,
        color_style: Some(NuStyle::default().fg(VERTICAL_LINE_COLOR)),
    };

    repeat_vertical(buf, x, y, 1, height, '│', style);
}

fn render_top_connector(buf: &mut Buffer, x: u16, y: u16) {
    let style = Style::default().fg(Color::Rgb(64, 64, 64));
    let span = Span::styled("┬", style);
    buf.set_span(x, y, &span, 1);
}

fn render_bottom_connector(buf: &mut Buffer, x: u16, y: u16) {
    let style = Style::default().fg(Color::Rgb(64, 64, 64));
    let span = Span::styled("┴", style);
    buf.set_span(x, y, &span, 1);
}

fn get_index_column_name(color_hm: &NuStyleTable) -> NuText {
    make_styled_string(String::from("index"), "string", 0, true, color_hm, 0)
}

fn create_column_index(i: usize, color_hm: &NuStyleTable) -> NuText {
    make_styled_string(i.to_string(), "string", 0, true, color_hm, 0)
}

fn render_space(buf: &mut Buffer, x: u16, y: u16, height: u16, padding: u16) -> u16 {
    repeat_vertical(buf, x, y, padding, height, ' ', TextStyle::default());
    padding
}

fn value_to_string(value: Value, config: &Config, color_hm: &NuStyleTable) -> NuText {
    let text = value.into_abbreviated_string(config);
    let text_type = value.get_type().to_string();
    let precision = config.float_precision as usize;
    make_styled_string(text, &text_type, 0, false, color_hm, precision)
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
        let text = String::from_utf8(strip_ansi_escapes::strip(text).unwrap()).unwrap();
        let style = text_style_to_tui_style(*style);
        let span = Span::styled(text, style);
        buf.set_span(x_offset, y_offset + row as u16, &span, available_width);
    }

    available_width
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

fn create_column_with_head(
    config: &Config,
    color_hm: &NuStyleTable,
    span: NuSpan,
    header: &str,
    items: &[Value],
) -> Vec<NuText> {
    let make_string = |value: String, t: &str| {
        make_styled_string(
            value,
            t,
            0,
            false,
            color_hm,
            config.float_precision as usize,
        )
    };

    let mut rows = vec![(String::new(), TextStyle::default()); items.len()];
    for (row, item) in items.iter().enumerate() {
        let (text, style) = match item {
            Value::Record { .. } => {
                let path = PathMember::String {
                    val: header.to_owned(),
                    span,
                };

                let value = item.clone().follow_cell_path(&[path], false);
                match value {
                    Ok(value) => make_string(
                        value.into_abbreviated_string(config),
                        &value.get_type().to_string(),
                    ),
                    Err(_) => make_string(String::from("❎"), "empty"),
                }
            }
            item => make_string(
                item.into_abbreviated_string(config),
                &item.get_type().to_string(),
            ),
        };

        rows[row] = (text, style);
    }

    rows
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
