use std::{
    cmp::{max, min},
    collections::HashMap,
    io,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};

use crossterm::{
    event::{poll, read, Event, KeyCode, KeyEvent},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ErrorKind,
};
use nu_ansi_term::{Color as NuColor, Style as NuStyle};
use nu_color_config::{get_color_config, style_primitive};
use nu_protocol::{ast::PathMember, Config, ShellError, Span as NuSpan, Value};
use nu_table::{string_width, Alignment, TextStyle};
use reedline::KeyModifiers;
use tui::{
    backend::{Backend, CrosstermBackend},
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    text::Span,
    widgets::{Block, Borders, StatefulWidget, Widget},
    Terminal,
};

pub fn handler(
    cols: &[String],
    data: &[Value],
    config: &nu_protocol::Config,
    ctrlc: Option<Arc<AtomicBool>>,
    show_index: bool,
    show_head: bool,
    reverse: bool,
) {
    // setup terminal
    enable_raw_mode().unwrap();
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen).unwrap();

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).unwrap();

    let color_hm = get_color_config(config);
    let mut state = UIState::new(cols, data, config, &color_hm, show_head, show_index);
    if reverse {
        state_reverse_data(&mut state, &terminal)
    }

    main_loop(&mut terminal, ctrlc, state);

    // restore terminal
    disable_raw_mode().unwrap();
    execute!(io::stdout(), LeaveAlternateScreen).unwrap();
}

fn main_loop<B>(terminal: &mut Terminal<B>, ctrlc: Option<Arc<AtomicBool>>, mut state: UIState<'_>)
where
    B: Backend,
{
    let events = UIEvents::new();

    loop {
        // handle CTRLC event
        if let Some(ctrlc) = ctrlc.clone() {
            if ctrlc.load(Ordering::SeqCst) {
                return;
            }
        }

        let mut event = events.next().unwrap();
        if let Some(key) = &event {
            let exited = handle_key_event(key, &mut state);
            if exited {
                break;
            }
        }

        terminal
            .draw(|f| f.render_stateful_widget(state, f.size(), &mut event))
            .unwrap();
    }
}

fn state_reverse_data<W>(state: &mut UIState<'_>, term: &Terminal<CrosstermBackend<W>>)
where
    W: io::Write,
{
    if let Ok(size) = term.size() {
        let height = estimate_available_height(size, state.show_header);
        if state.data.len() > height as usize {
            state.row_index = state.data.len() - height as usize;
        }
    }
}

fn handle_key_event(key: &KeyEvent, state: &mut UIState<'_>) -> bool {
    match key {
        KeyEvent {
            code: KeyCode::Char('d'),
            modifiers: KeyModifiers::CONTROL,
        } => return true,
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
            _ => {}
        },
    }

    false
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

#[derive(Debug, Clone, Copy)]
struct UIState<'a> {
    columns: &'a [String],
    data: &'a [Value],
    config: &'a nu_protocol::Config,
    color_hm: &'a HashMap<String, nu_ansi_term::Style>,
    column_index: usize,
    row_index: usize,
    show_index: bool,
    show_header: bool,
}

impl<'a> UIState<'a> {
    fn new(
        columns: &'a [String],
        data: &'a [Value],
        config: &'a nu_protocol::Config,
        color_hm: &'a HashMap<String, nu_ansi_term::Style>,
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
        }
    }

    fn count_rows(&self) -> usize {
        self.data.len()
    }

    fn count_columns(&self) -> usize {
        self.columns.len()
    }
}

impl StatefulWidget for UIState<'_> {
    type State = Option<KeyEvent>;

    fn render(
        self,
        area: tui::layout::Rect,
        buf: &mut tui::buffer::Buffer,
        _state: &mut Self::State,
    ) {
        const CELL_PADDING_LEFT: u16 = 2;
        const CELL_PADDING_RIGHT: u16 = 2;
        const CELL_MIN_WIDTH: u16 = 3;

        if area.width == 0 || area.height < 3 {
            return;
        }

        let show_index = self.show_index;

        let show_head = self.show_header;
        let has_head = !self.columns.is_empty();

        let mut available_height = area.height;
        let status_bar_offset = 3;
        available_height -= status_bar_offset;

        let mut head = "";
        let mut head_width = 0;
        let mut head_offset = 0;

        if show_head {
            head_offset = 3;
            available_height -= 3;
        }

        let mut used_width = 0;

        let mut rows = &self.data[self.row_index..];
        if rows.len() > available_height as usize {
            rows = &rows[..available_height as usize];
        }

        // header lines
        if show_head {
            render_header_borders(buf, area, 0, 1);
        }

        // status_bar
        let message = create_length_message(&self, available_height);
        render_status_bar(buf, area, &message);

        if show_index {
            used_width = render_column_index(
                buf,
                available_height,
                self.row_index,
                show_head,
                head_offset,
                self.color_hm,
            );

            render_vertical(buf, used_width, head_offset, available_height, show_head);
            used_width += 1;
        }

        for col in self.column_index..self.columns.len() {
            let column = if has_head {
                head = &self.columns[col];
                head_width = string_width(head);

                create_column(self.config, self.color_hm, NuSpan::unknown(), head, rows)
            } else {
                rows.iter()
                    .map(|item| value_to_string(item.clone(), self.config, self.color_hm))
                    .collect()
            };

            let available_space = area.width - used_width;

            let column_width = calculate_column_width(&column);
            let data_space = max(head_width as u16, column_width as u16);
            let use_space = min(available_space, data_space);

            let is_last_col = col + 1 == self.columns.len();
            let taking_space = use_space + used_width + CELL_PADDING_LEFT + CELL_PADDING_RIGHT;
            let is_enough_space = area.width >= taking_space;
            let is_space_for_next = is_enough_space
                && area.width - taking_space
                    > CELL_PADDING_LEFT + CELL_PADDING_RIGHT + CELL_MIN_WIDTH;
            if !is_enough_space || (!is_last_col && !is_space_for_next) {
                break;
            }

            used_width += CELL_PADDING_LEFT;

            render_column(buf, used_width, head_offset, use_space, &column);

            if show_head {
                let header_data = &[head_row_text(head, self.color_hm)];
                render_column(buf, used_width, 1, use_space, header_data);
            }

            used_width += use_space;

            render_column_space(buf, used_width, 1, 1, CELL_PADDING_RIGHT);
            render_column_space(buf, used_width, 3, available_height, CELL_PADDING_RIGHT);

            used_width += CELL_PADDING_RIGHT;
        }

        render_vertical(buf, used_width, head_offset, available_height, show_head);
    }
}

fn create_length_message(state: &UIState<'_>, height: u16) -> String {
    let seen = state.row_index + height as usize;
    let is_last_row_reached = seen >= state.data.len();
    if is_last_row_reached {
        String::from("[END]")
    } else {
        format!("[{}/{}]", seen, state.data.len())
    }
}

fn render_status_bar(buf: &mut Buffer, area: Rect, message: &str) {
    render_header_borders(buf, area, area.height - 3, 1);

    let style = Style::default().fg(Color::Rgb(128, 128, 128));
    let span = Span::styled(message, style);
    buf.set_span(area.x, area.bottom().saturating_sub(2), &span, area.width);
}

fn estimate_available_height(area: Rect, show_head: bool) -> u16 {
    let mut available_height = area.height;
    available_height -= 3; // status_bar

    if show_head {
        available_height -= 3; // head
    }

    available_height
}

const VERTICAL_LINE_COLOR: nu_ansi_term::Color = NuColor::Rgb(64, 64, 64);

fn head_row_text(head: &str, color_hm: &HashMap<String, NuStyle>) -> (String, TextStyle) {
    (
        String::from(head),
        TextStyle {
            alignment: Alignment::Center,
            color_style: Some(color_hm["header"]),
        },
    )
}

fn render_column_index(
    buf: &mut Buffer,
    height: u16,
    starts_at: usize,
    show_header: bool,
    header_offset: u16,
    color_hm: &HashMap<String, NuStyle>,
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

fn render_vertical(buf: &mut Buffer, x: u16, y: u16, height: u16, show_header: bool) {
    render_vertical_split(buf, x, y, height);

    if show_header && y > 0 {
        render_top_connector(buf, x, y - 1);
    }

    render_bottom_connector(buf, x, height + y);
}

fn render_vertical_split(buf: &mut Buffer, x: u16, y: u16, height: u16) {
    let style = TextStyle {
        alignment: Alignment::Left,
        color_style: Some(NuStyle::default().fg(VERTICAL_LINE_COLOR)),
    };

    let splits = vec![(String::from('│'), style); height as usize];
    render_column(buf, x, y, 1, &splits);
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

fn get_index_column_name(color_hm: &HashMap<String, NuStyle>) -> (String, TextStyle) {
    make_styled_string(String::from("index"), "string", 0, true, color_hm, 0)
}

fn create_column_index(i: usize, color_hm: &HashMap<String, NuStyle>) -> (String, TextStyle) {
    make_styled_string(i.to_string(), "string", 0, true, color_hm, 0)
}

fn render_column_space(buf: &mut Buffer, x: u16, y: u16, height: u16, padding: u16) {
    let splits = vec![(str::repeat(" ", padding as usize), TextStyle::default()); height as usize];
    render_column(buf, x, y, padding, &splits);
}

fn value_to_string(
    value: Value,
    config: &Config,
    color_hm: &HashMap<String, NuStyle>,
) -> (String, TextStyle) {
    let text = value.into_abbreviated_string(config);
    let text_type = value.get_type().to_string();
    let precision = config.float_precision as usize;
    make_styled_string(text, &text_type, 0, false, color_hm, precision)
}

fn calculate_column_width(column: &[(String, TextStyle)]) -> usize {
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
    rows: &[(String, TextStyle)],
) {
    for (row, (text, style)) in rows.iter().enumerate() {
        let mut text = String::from_utf8(strip_ansi_escapes::strip(text).unwrap()).unwrap();
        if string_width(&text) > available_width as usize {
            text = nu_table::string_truncate(&text, available_width as usize);
        }

        let style = text_style_to_tui_style(*style);
        let span = Span::styled(text, style);
        buf.set_span(x_offset, y_offset + row as u16, &span, available_width);
    }
}

fn create_column(
    config: &nu_protocol::Config,
    color_hm: &HashMap<String, NuStyle>,
    span: nu_protocol::Span,
    header: &str,
    items: &[Value],
) -> Vec<(String, TextStyle)> {
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

fn nu_ansi_color_to_tui_color(clr: nu_ansi_term::Color) -> Option<tui::style::Color> {
    use nu_ansi_term::Color;
    use tui::style::Color as TColor;

    let clr = match clr {
        Color::Black => TColor::Black,
        Color::DarkGray => TColor::DarkGray,
        Color::Red => TColor::Red,
        Color::LightRed => TColor::LightRed,
        Color::Green => TColor::Green,
        Color::LightGreen => TColor::LightGreen,
        Color::Yellow => TColor::Yellow,
        Color::LightYellow => TColor::LightYellow,
        Color::Blue => TColor::Blue,
        Color::LightBlue => TColor::LightBlue,
        Color::Magenta => TColor::Magenta,
        Color::LightMagenta => TColor::LightMagenta,
        Color::Cyan => TColor::Cyan,
        Color::LightCyan => TColor::LightCyan,
        Color::White => TColor::White,
        Color::Fixed(i) => tui::style::Color::Indexed(i),
        Color::Rgb(r, g, b) => tui::style::Color::Rgb(r, g, b),
        Color::LightGray => TColor::Gray, // todo: make a PR to add the color
        Color::LightPurple => TColor::Blue, // todo: make a PR to add the color,
        Color::Purple => TColor::Blue,    // todo: make a PR to add the color,
        Color::Default => return None,
    };

    Some(clr)
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

    pub fn next(&self) -> Result<Option<KeyEvent>, ErrorKind> {
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
            Err(_) => todo!(),
        }
    }
}

fn make_styled_string(
    text: String,
    text_type: &str,
    col: usize,
    with_index: bool,
    color_hm: &HashMap<String, nu_ansi_term::Style>,
    float_precision: usize,
) -> (String, TextStyle) {
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

fn convert_with_precision(val: &str, precision: usize) -> Result<String, ShellError> {
    // vall will always be a f64 so convert it with precision formatting
    let val_float = match val.trim().parse::<f64>() {
        Ok(f) => f,
        Err(e) => {
            return Err(ShellError::GenericError(
                format!("error converting string [{}] to f64", &val),
                "".to_string(),
                None,
                Some(e.to_string()),
                Vec::new(),
            ));
        }
    };
    Ok(format!("{:.prec$}", val_float, prec = precision))
}
