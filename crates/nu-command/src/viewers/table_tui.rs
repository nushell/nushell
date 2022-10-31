use std::{
    cmp::max,
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
    terminal::{
        disable_raw_mode, enable_raw_mode, Clear, ClearType, EnterAlternateScreen,
        LeaveAlternateScreen,
    },
    ErrorKind,
};
use nu_color_config::{style_primitive, get_color_config};
use nu_protocol::{ast::PathMember, Config, ShellError, TableIndexMode, Value};
use nu_table::{Alignment, NuTable, Table, TableTheme, TextStyle, Alignments};
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
    columns: Vec<String>,
    data: Vec<Value>,
    ctrlc: Option<Arc<AtomicBool>>,
    config: &'a nu_protocol::Config,
    term_width: usize,
}

impl<'a> UITable<'a> {
    pub fn new(
        columns: Vec<String>,
        data: Vec<Value>,
        config: &'a nu_protocol::Config,
        ctrlc: Option<Arc<AtomicBool>>,
        term_width: usize,
    ) -> Self {
        Self {
            columns,
            data,
            ctrlc,
            config,
            term_width,
        }
    }

    pub fn handle(&self) -> Result<(), ShellError> {
        run(
            &self.columns,
            &self.data,
            self.config,
            self.ctrlc.clone(),
            self.term_width,
        );
        Ok(())
    }
}

fn run(
    cols: &[String],
    data: &[Value],
    config: &nu_protocol::Config,
    ctrlc: Option<Arc<AtomicBool>>,
    term_width: usize,
) {
    // setup terminal
    let mut stdout = io::stdout();

    execute!(stdout, EnterAlternateScreen).unwrap();
    enable_raw_mode().unwrap();

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).unwrap();

    main_loop(&mut terminal, cols, data, config, ctrlc, term_width);

    // restore terminal
    disable_raw_mode().unwrap();

    execute!(io::stdout(), LeaveAlternateScreen).unwrap();
}

fn main_loop<B>(
    terminal: &mut Terminal<B>,
    mut cols: &[String],
    mut data: &[Value],
    config: &nu_protocol::Config,
    ctrlc: Option<Arc<AtomicBool>>,
    term_width: usize,
) where
    B: Backend,
{
    let events = UIEvents::new();
    let mut state_skip = 0;
    loop {
        // handle CTRLC event
        if let Some(ctrlc) = ctrlc.clone() {
            if ctrlc.load(Ordering::SeqCst) {
                return;
            }
        }

        let mut event = events.next().unwrap();
        match &event {
            Some(KeyEvent {code: KeyCode::Down, ..}) => {
                state_skip += 1;
                state_skip = state_skip.max(cols.len());
            },
            Some(KeyEvent {code: KeyCode::Up, ..}) => {
                if state_skip > 0 {
                    state_skip -= 1;
                } 
            },
            Some(KeyEvent {code: KeyCode::Left, ..}) => {
            },
            Some(KeyEvent {code: KeyCode::Right, ..}) => {
                if !cols.is_empty() {
                    cols = &cols[1..];
                    data = &data[1..];
                }
            },
            _ => {}
        }

        terminal
            .draw(|f| {
                f.render_stateful_widget(
                    TableUIIII {
                        cols,
                        data,
                        config,
                        ctrlc: ctrlc.clone(),
                        term_width,
                        state_skip,
                    },
                    f.size(),
                    &mut event,
                );
            })
            .unwrap();
    }
}

fn render_header_borders(buf: &mut Buffer, area: Rect, y: u16) -> (u16, u16) {
    let block = Block::default()
        .borders(Borders::TOP | Borders::BOTTOM)
        .border_style(Style::default().fg(Color::Rgb(64, 64, 64)));
    let height = 3;
    let area = Rect::new(0, y, area.width, height);
    block.render(area, buf);
    // y pos of header text and next line
    (height.saturating_sub(2), height)
}

struct TableUIIII<'a> {
    cols: &'a [String],
    data: &'a [Value],
    config: &'a nu_protocol::Config,
    state_skip: usize,
    ctrlc: Option<Arc<AtomicBool>>,
    term_width: usize,
}

impl StatefulWidget for TableUIIII<'_> {
    type State = Option<KeyEvent>;

    fn render(
        self,
        area: tui::layout::Rect,
        buf: &mut tui::buffer::Buffer,
        state: &mut Self::State,
    ) {
        // render_header_borders(buf, area, 0);

        let color_hm = get_color_config(&self.config);
        let data = convert_to_table33(
            self.cols.to_vec(),
            self.data.to_vec(),
            0,
            self.ctrlc.clone(),
            self.config,
            nu_protocol::Span::new(0, 0),
            area.width as usize,
            &color_hm,
        )
        .unwrap()
        .unwrap();
        let data = data
            .draw_table(
                self.config,
                &color_hm,
                Alignments::default(),
                &TableTheme::rounded(),
                area.width as usize,
            )
            .unwrap();
        let data = String::from_utf8(strip(data).unwrap()).unwrap();

        for (i, line) in data.lines().skip(self.state_skip).take(area.height as usize - 0 - 3).enumerate() {
            let style = Style::default();
            let span = Span::styled(line, style);
            buf.set_string(0, 0 + i as u16, line, style);
        }

        // let style = Style::default().fg(Color::Rgb(128, 128, 128));
        // let span = Span::styled(data.lines().count().to_string(), style);
        // buf.set_span(0, 3, &span, area.width);

        let style = Style::default().fg(Color::Rgb(128, 128, 128));
        let message = match state {
            Some(key) => format!("{:?}", key),
            None => "TICK".to_string(),
        };
        let span = Span::styled(message, style);
        buf.set_span(area.x, area.bottom().saturating_sub(2), &span, area.width);

        render_header_borders(buf, area, area.height - 3);
    }
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
