use std::{
    cmp::{max, min},
    collections::HashMap,
    io::{self, Write},
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
    ErrorKind,
};
use nu_color_config::{get_color_config, style_primitive};
use nu_protocol::{ast::PathMember, Config, ShellError, TableIndexMode, Value};
use nu_table::{Alignment, Alignments, NuTable, Table, TableTheme, TextStyle};
use num_traits::CheckedSub;
use reedline::KeyModifiers;
use strip_ansi_escapes::strip;
use tui::{
    backend::{Backend, CrosstermBackend},
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    text::Span,
    widgets::{Block, Borders, StatefulWidget, Widget},
    Terminal,
};

pub struct UITable<'a> {
    columns: &'a [String],
    data: &'a [Value],
    ctrlc: Option<Arc<AtomicBool>>,
    config: &'a nu_protocol::Config,
    show_index: bool,
    show_header: bool,
}

impl<'a> UITable<'a> {
    pub fn new(
        columns: &'a [String],
        data: &'a [Value],
        config: &'a nu_protocol::Config,
        ctrlc: Option<Arc<AtomicBool>>,
        show_index: bool,
        show_header: bool,
    ) -> Self {
        Self {
            columns,
            data,
            ctrlc,
            config,
            show_header,
            show_index,
        }
    }

    pub fn handle(&self) -> Result<(), ShellError> {
        run(
            self.columns,
            self.data,
            self.config,
            self.ctrlc.clone(),
            self.show_index,
            self.show_header,
        );

        Ok(())
    }
}

fn run(
    cols: &[String],
    data: &[Value],
    config: &nu_protocol::Config,
    ctrlc: Option<Arc<AtomicBool>>,
    show_index: bool,
    show_header: bool,
) {
    // setup terminal
    enable_raw_mode().unwrap();
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen).unwrap();

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).unwrap();

    main_loop(
        &mut terminal,
        cols,
        data,
        config,
        ctrlc,
        show_index,
        show_header,
    );

    // restore terminal
    disable_raw_mode().unwrap();

    execute!(io::stdout(), LeaveAlternateScreen).unwrap();
}

fn main_loop<B>(
    terminal: &mut Terminal<B>,
    cols: &[String],
    data: &[Value],
    config: &nu_protocol::Config,
    ctrlc: Option<Arc<AtomicBool>>,
    show_index: bool,
    show_header: bool,
) where
    B: Backend,
{
    let color_hm = get_color_config(config);
    let events = UIEvents::new();
    let mut row_index: usize = 0;
    let mut column_index: usize = 0;
    loop {
        // handle CTRLC event
        if let Some(ctrlc) = ctrlc.clone() {
            if ctrlc.load(Ordering::SeqCst) {
                return;
            }
        }

        let mut event = events.next().unwrap();
        if let Some(key) = &event {
            match key {
                KeyEvent {
                    code: KeyCode::Down,
                    ..
                } => {
                    let max_index = data.len().saturating_sub(1);
                    row_index = min(row_index + 1, max_index);
                }
                KeyEvent {
                    code: KeyCode::Up, ..
                } => {
                    row_index = row_index.saturating_sub(1);
                }
                KeyEvent {
                    code: KeyCode::Left,
                    ..
                } => {
                    column_index = column_index.saturating_sub(1);
                }
                KeyEvent {
                    code: KeyCode::Right,
                    ..
                } => {
                    let max_index = cols.len().saturating_sub(1);
                    column_index = min(column_index + 1, max_index);
                }
                KeyEvent {
                    code: KeyCode::Char('d'),
                    modifiers: KeyModifiers::CONTROL,
                } => {
                    return;
                }
                _ => {}
            }
        }

        terminal
            .draw(|f| {
                let renderer = TableUIIII::new(
                    cols,
                    data,
                    config,
                    &color_hm,
                    column_index,
                    row_index,
                    show_header,
                    show_index,
                );

                f.render_stateful_widget(renderer, f.size(), &mut event);
            })
            .unwrap();
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

struct TableUIIII<'a> {
    cols: &'a [String],
    data: &'a [Value],
    config: &'a nu_protocol::Config,
    color_hm: &'a HashMap<String, nu_ansi_term::Style>,
    column_index: usize,
    row_index: usize,
    show_index: bool,
    show_header: bool,
}

impl<'a> TableUIIII<'a> {
    fn new(
        cols: &'a [String],
        data: &'a [Value],
        config: &'a nu_protocol::Config,
        color_hm: &'a HashMap<String, nu_ansi_term::Style>,
        column_index: usize,
        row_index: usize,
        show_header: bool,
        show_index: bool,
    ) -> Self {
        Self {
            cols,
            data,
            config,
            color_hm,
            column_index,
            row_index,
            show_header,
            show_index,
        }
    }
}

impl StatefulWidget for TableUIIII<'_> {
    type State = Option<KeyEvent>;

    fn render(
        self,
        area: tui::layout::Rect,
        buf: &mut tui::buffer::Buffer,
        state: &mut Self::State,
    ) {
        const CELL_PADDING_LEFT: u16 = 2;
        const CELL_PADDING_RIGHT: u16 = 2;
        const CELL_MIN_WIDTH: u16 = 3;

        if area.width == 0 || area.height < 3 {
            return;
        }

        let show_index = self.show_index;

        let show_header = self.show_header;
        let has_header = !self.cols.is_empty();

        let status_bar_offset = 3;
        let mut header_offset = 0;

        let mut available_height = area.height;
        available_height -= status_bar_offset;

        if show_header {
            available_height -= 3;
            header_offset = 3;
        }

        let mut header = "";
        let mut header_width = 0;

        let mut used_width = 0;

        let mut rows = &self.data[self.row_index..];
        if rows.len() > available_height as usize {
            rows = &rows[..available_height as usize];
        }

        {
            // status_bar
            {
                let message = match state {
                    Some(key) => format!("{:?}", key),
                    None => "TICK ".to_string() + &area.width.to_string(),
                };
                let style = Style::default().fg(Color::Rgb(128, 128, 128));
                let span = Span::styled(message, style);
                buf.set_span(area.x, area.bottom().saturating_sub(2), &span, area.width);

                render_header_borders(buf, area, area.height - 3, 1);
            }

            // header lines
            if show_header {
                render_header_borders(buf, area, 0, 1);
            }
        }

        if show_index {
            let mut head = (String::new(), TextStyle::default());
            let mut head_width = 0;
            if show_header {
                head = get_index_column_name(self.color_hm);
                head_width = nu_table::string_width(&head.0) as u16;
            }

            let index = (0..rows.len())
                .map(|i| i + self.row_index)
                .map(|i| create_column_index(i, self.color_hm))
                .collect::<Vec<_>>();

            let index_col_width = index
                .last()
                .map(|(s, _)| nu_table::string_width(s) as u16)
                .unwrap_or(0);

            let index_width = max(head_width, index_col_width);

            render_column(
                buf,
                0,
                header_offset,
                index_width + CELL_PADDING_LEFT,
                &index,
            );

            if show_header {
                render_column(buf, used_width + CELL_PADDING_LEFT, 1, index_width, &[head]);
            }

            used_width += index_width + CELL_PADDING_LEFT;
            used_width += CELL_PADDING_RIGHT;

            let splits = vec![
                (
                    String::from('│'),
                    TextStyle {
                        alignment: Alignment::Left,
                        color_style: Some(
                            nu_ansi_term::Style::default().fg(nu_ansi_term::Color::Rgb(64, 64, 64))
                        ),
                    },
                );
                available_height as usize
            ];

            render_column(buf, used_width, header_offset, 1, &splits);

            // set split symbols for status bar and header
            {
                if show_header {
                    let style = Style::default().fg(Color::Rgb(64, 64, 64));
                    let span = Span::styled("┬", style);
                    buf.set_span(used_width, area.top() + 2, &span, 1);
                }

                let style = Style::default().fg(Color::Rgb(64, 64, 64));
                let span = Span::styled("┴", style);
                buf.set_span(used_width, area.bottom() - 3, &span, 1);
            }

            used_width += 1;
        }

        for col in self.column_index..self.cols.len() {
            let column = if has_header {
                header = &self.cols[col];
                header_width = nu_table::string_width(header);

                create_column(
                    self.config,
                    self.color_hm,
                    nu_protocol::Span::new(0, 0),
                    header,
                    rows,
                )
            } else {
                rows.iter()
                    .map(|item| value_to_string(item.clone(), self.config, self.color_hm))
                    .collect()
            };

            let available_space = area.width - used_width;

            let column_width = calculate_column_width(&column);
            let data_space = max(header_width as u16, column_width as u16);
            let use_space = min(available_space, data_space);

            let is_last_col = col + 1 == self.cols.len();
            let taking_space = use_space + used_width + CELL_PADDING_LEFT + CELL_PADDING_RIGHT;
            let is_enough_space = area.width >= taking_space;
            let is_space_for_next = is_enough_space
                && area.width - taking_space
                    > CELL_PADDING_LEFT + CELL_PADDING_RIGHT + CELL_MIN_WIDTH;
            if !is_enough_space || (!is_last_col && !is_space_for_next) {
                break;
            }

            render_column(
                buf,
                used_width + CELL_PADDING_LEFT,
                header_offset,
                use_space,
                &column,
            );

            if show_header {
                let header_data = &[(
                    String::from(header),
                    TextStyle {
                        alignment: Alignment::Center,
                        color_style: Some(self.color_hm["header"]),
                    },
                )];

                render_column(
                    buf,
                    used_width + CELL_PADDING_LEFT,
                    1,
                    use_space,
                    header_data,
                );
            }

            used_width += CELL_PADDING_LEFT;
            used_width += use_space;

            render_column_space(buf, used_width, 1, 1, CELL_PADDING_RIGHT);
            render_column_space(buf, used_width, 3, available_height, CELL_PADDING_RIGHT);

            used_width += CELL_PADDING_RIGHT;
        }

        let splits = vec![
            (
                String::from('│'),
                TextStyle {
                    alignment: Alignment::Left,
                    color_style: Some(
                        nu_ansi_term::Style::default().fg(nu_ansi_term::Color::Rgb(64, 64, 64))
                    ),
                },
            );
            available_height as usize
        ];
        render_column(buf, used_width, header_offset, 1, &splits);

        // set split symbols for status bar and header
        {
            if show_header {
                let style = Style::default().fg(Color::Rgb(64, 64, 64));
                let span = Span::styled("┬", style);
                buf.set_span(used_width, area.top() + 2, &span, 1);
            }

            let style = Style::default().fg(Color::Rgb(64, 64, 64));
            let span = Span::styled("┴", style);
            buf.set_span(used_width, area.bottom() - 3, &span, 1);
        }
    }
}

fn get_index_column_name(color_hm: &HashMap<String, nu_ansi_term::Style>) -> (String, TextStyle) {
    make_styled_string(String::from("index"), "string", 0, true, color_hm, 0)
}

fn create_column_index(
    i: usize,
    color_hm: &HashMap<String, nu_ansi_term::Style>,
) -> (String, TextStyle) {
    make_styled_string(i.to_string(), "string", 0, true, color_hm, 0)
}

fn render_column_space(buf: &mut Buffer, x: u16, y: u16, height: u16, padding: u16) {
    let splits = vec![(str::repeat(" ", padding as usize), TextStyle::default()); height as usize];
    render_column(buf, x, y, padding, &splits);
}

fn value_to_string(
    value: Value,
    config: &Config,
    color_hm: &HashMap<String, nu_ansi_term::Style>,
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
        .map(|text| nu_table::string_width(text))
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
        if nu_table::string_width(&text) > available_width as usize {
            text = nu_table::string_truncate(&text, available_width as usize);
        }

        let style = text_style_to_tui_style(*style);
        let span = Span::styled(text, style);
        buf.set_span(x_offset, y_offset + row as u16, &span, available_width);
    }
}

fn create_column(
    config: &nu_protocol::Config,
    color_hm: &HashMap<String, nu_ansi_term::Style>,
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

#[derive(Debug, Clone, Copy)]
pub struct Cfg {
    pub exit_key: KeyCode,
    pub tick_rate: Duration,
}

impl Default for Cfg {
    fn default() -> Cfg {
        Cfg {
            exit_key: KeyCode::Char('q'),
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

pub fn convert_to_table33(
    mut headers: Vec<String>,
    input: Vec<Value>,
    row_offset: usize,
    ctrlc: Option<Arc<AtomicBool>>,
    config: &Config,
    head: nu_protocol::Span,
    termwidth: usize,
    color_hm: &HashMap<String, nu_ansi_term::Style>,
) -> Result<Option<Table>, ShellError> {
    let mut input = input.iter().peekable();
    let float_precision = config.float_precision as usize;
    let with_index = match config.table_index_mode {
        TableIndexMode::Always => true,
        TableIndexMode::Never => false,
        TableIndexMode::Auto => headers.iter().any(|header| header == INDEX_COLUMN_NAME),
    };

    const INDEX_COLUMN_NAME: &str = "index";

    if input.peek().is_none() {
        return Ok(None);
    }

    if !headers.is_empty() && with_index {
        headers.insert(0, "#".into());
    }

    // The header with the INDEX is removed from the table headers since
    // it is added to the natural table index
    let headers: Vec<_> = headers
        .into_iter()
        .filter(|header| header != INDEX_COLUMN_NAME)
        .map(|text| {
            nu_table::Table::create_cell(
                text,
                TextStyle {
                    alignment: Alignment::Center,
                    color_style: Some(color_hm["header"]),
                },
            )
        })
        .collect();

    let with_header = !headers.is_empty();
    let mut count_columns = headers.len();

    let mut data: Vec<Vec<_>> = if headers.is_empty() {
        Vec::new()
    } else {
        vec![headers]
    };

    for (row_num, item) in input.enumerate() {
        if let Some(ctrlc) = &ctrlc {
            if ctrlc.load(Ordering::SeqCst) {
                return Ok(None);
            }
        }

        if let Value::Error { error } = item {
            return Err(error.clone());
        }

        let mut row = vec![];
        if with_index {
            let text = match &item {
                Value::Record { .. } => item
                    .get_data_by_key(INDEX_COLUMN_NAME)
                    .map(|value| value.into_string("", config)),
                _ => None,
            }
            .unwrap_or_else(|| (row_num + row_offset).to_string());

            let value =
                make_styled_string(text, "string", 0, with_index, color_hm, float_precision);
            let value = Table::create_cell(value.0, value.1);

            row.push(value);
        }

        if !with_header {
            let text = item.into_abbreviated_string(config);
            let text_type = item.get_type().to_string();
            let col = if with_index { 1 } else { 0 };
            let value =
                make_styled_string(text, &text_type, col, with_index, color_hm, float_precision);
            let value = Table::create_cell(value.0, value.1);

            row.push(value);
        } else {
            let skip_num = if with_index { 1 } else { 0 };
            for (col, header) in data[0].iter().enumerate().skip(skip_num) {
                let result = match item {
                    Value::Record { .. } => item.clone().follow_cell_path(
                        &[PathMember::String {
                            val: header.as_ref().to_owned(),
                            span: head,
                        }],
                        false,
                    ),
                    _ => Ok(item.clone()),
                };

                let value = match result {
                    Ok(value) => make_styled_string(
                        value.into_abbreviated_string(config),
                        &value.get_type().to_string(),
                        col,
                        with_index,
                        color_hm,
                        float_precision,
                    ),
                    Err(_) => make_styled_string(
                        String::from("❎"),
                        "empty",
                        col,
                        with_index,
                        color_hm,
                        float_precision,
                    ),
                };

                let value = Table::create_cell(value.0, value.1);
                row.push(value);
            }
        }

        count_columns = max(count_columns, row.len());

        data.push(row);
    }

    let count_rows = data.len();
    let table = Table::new(
        data,
        (count_rows, count_columns),
        termwidth,
        with_header,
        with_index,
    );

    Ok(Some(table))
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
