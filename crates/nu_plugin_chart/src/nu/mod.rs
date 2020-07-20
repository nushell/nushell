use nu_errors::ShellError;
use nu_plugin::Plugin;
use nu_protocol::{CallInfo, Primitive, Signature, SyntaxShape, UntaggedValue, Value};
use nu_source::TaggedItem;
use nu_value_ext::ValueExt;

use crate::chart::{BarChart, Chart, Columns, Reduction};

use std::{
    error::Error,
    io::{stdout, Write},
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event as CEvent, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};

use tui::{backend::CrosstermBackend, Terminal};

use tui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    //widgets::{BarChart, Block, Borders},
    symbols,
    text::Span,
    widgets::{
        Axis, BarChart as TuiBarChart, Block, Borders, Chart as TuiChart, Dataset, GraphType,
    },
};

enum Event<I> {
    Input(I),
    Tick,
}

pub struct LineChart<'a> {
    pub title: &'a str,
    pub should_quit: bool,
    pub show_chart: bool,
    pub progress: f64,
    pub data: Vec<(f64, f64)>,
    pub enhanced_graphics: bool,
    window: [f64; 2],
}

impl<'a> LineChart<'a> {
    pub fn new(title: &'a str, enhanced_graphics: bool) -> LineChart<'a> {
        LineChart {
            title,
            should_quit: false,
            show_chart: true,
            progress: 0.0,
            data: vec![],
            enhanced_graphics,
            window: [0.0, 20.0],
        }
    }

    pub fn on_key(&mut self, c: char) {
        match c {
            'q' => {
                self.should_quit = true;
            }
            't' => {
                self.show_chart = !self.show_chart;
            }
            _ => {}
        }
    }

    pub fn on_right(&mut self) {
        let event = self.data.pop().unwrap();
        self.data.insert(0, event);
    }

    pub fn on_left(&mut self) {
        let event = self.data.remove(0);
        self.data.push(event);
    }

    pub fn on_tick(&mut self) {
        self.progress += 0.001;
        if self.progress > 1.0 {
            self.progress = 0.0;
        }
    }
}

const DATA2: [(f64, f64); 7] = [
    (0.0, 0.0),
    (10.0, 1.0),
    (20.0, 0.5),
    (30.0, 1.5),
    (40.0, 1.0),
    (50.0, 2.5),
    (60.0, 3.0),
];

fn display(model: &nu_cli::utils::data::tests::Model) -> Result<(), Box<dyn Error>> {
//    println!("{:#?}", model);

    //let mut app = BarChart::from_model(&model)?;
    let mut app = LineChart::new("chart", true);

    
    let labels = model.labels.grouped().map(|l| {
        Span::raw(l)
    }).collect::<Vec<_>>();

    app.data = DATA2.iter().cloned().collect();

    enable_raw_mode()?;

    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;

    let backend = CrosstermBackend::new(stdout);

    let mut terminal = Terminal::new(backend)?;

    let (tx, rx) = mpsc::channel();

    let tick_rate = Duration::from_millis(250);
    thread::spawn(move || {
        let mut last_tick = Instant::now();
        loop {
            // poll for tick rate duration, if no events, sent tick event.
            if event::poll(tick_rate - last_tick.elapsed()).unwrap() {
                if let CEvent::Key(key) = event::read().unwrap() {
                    tx.send(Event::Input(key)).unwrap();
                }
            }
            if last_tick.elapsed() >= tick_rate {
                tx.send(Event::Tick).unwrap();
                last_tick = Instant::now();
            }
        }
    });

    terminal.clear()?;

    loop {
        //app.draw(&mut terminal)?;

                
        terminal.draw(|f| {
            let size = f.size();
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(2)
                .constraints([Constraint::Percentage(100)].as_ref())
                .split(f.size());

            let datasets = vec![
                Dataset::default()
                    .name("data2")
                    .marker(symbols::Marker::Dot)
                    .style(Style::default().fg(Color::Cyan))
                    .data(&app.data),
                Dataset::default()
                    .name("data3")
                    .marker(symbols::Marker::Braille)
                    .style(Style::default().fg(Color::Yellow))
                    .data(&app.data),
            ];

            let chart = TuiChart::new(datasets)
                .block(
                    Block::default()
                        .title(Span::styled(
                            "Chart 1",
                            Style::default()
                                .fg(Color::Cyan)
                                .add_modifier(Modifier::BOLD),
                        ))
                        .borders(Borders::ALL),
                )
                .x_axis(
                    Axis::default()
                        .title("X Axis")
                        .style(Style::default().fg(Color::Gray))
                        .labels(labels.to_vec())
                )
                .y_axis(
                    Axis::default()
                        .title("Y Axis")
                        .style(Style::default().fg(Color::Gray))
                        .labels(vec![
                            Span::styled("-20", Style::default().add_modifier(Modifier::BOLD)),
                            Span::raw("0"),
                            Span::styled("20", Style::default().add_modifier(Modifier::BOLD)),
                        ])
                );
            f.render_widget(chart, chunks[0]);
        })?;

        match rx.recv()? {
            Event::Input(event) => match event.code {
                KeyCode::Char('q') => {
                    disable_raw_mode()?;
                    execute!(
                        terminal.backend_mut(),
                        LeaveAlternateScreen,
                        DisableMouseCapture
                    )?;
                    terminal.show_cursor()?;
                    break;
                }
                KeyCode::Left => app.on_left(),
                KeyCode::Right => app.on_right(),
                _ => {}
            },
            Event::Tick => {
                app.on_tick();
            }
        }
        if app.should_quit {
            break;
        }
    }

    Ok(())
}

impl Plugin for Chart {
    fn config(&mut self) -> Result<Signature, ShellError> {
        Ok(Signature::build("chart")
            .desc("Displays chart")
            .switch("acc", "accumuate values", Some('a'))
            .optional(
                "columns",
                SyntaxShape::Any,
                "the columns to chart [x-axis y-axis]",
            )
            .named(
                "use",
                SyntaxShape::String,
                "column to use for evaluation",
                Some('u'),
            ))
    }

    fn sink(&mut self, call_info: CallInfo, input: Vec<Value>) {
        if let Some(accumulate) = call_info.args.get("acc") {
            self.reduction = Reduction::Accumulate;
            println!("reduccion puesta");
        }

        self.run(call_info, input);
    }
}

impl Chart {
    fn run(&mut self, call_info: CallInfo, input: Vec<Value>) -> Result<(), ShellError> {
        let args = call_info.args;
        let name = call_info.name_tag;

        self.eval = if let Some(Value {
            value: UntaggedValue::Primitive(Primitive::String(s)),
            tag,
        }) = args.get("use")
        {
            let key = s.clone().tagged(tag);

            Some(Box::new(move |x: usize, value: &Value| {
                let key = key.clone();
                Ok(value.get_data_by_key(key.borrow_spanned()).unwrap())
            }))
        } else {
            None
        };

        for arg in args.positional_iter() {
            match arg {
                Value {
                    value: UntaggedValue::Primitive(Primitive::String(column)),
                    tag,
                } => {
                    let column = column.clone();
                    self.columns = Columns::One(column.tagged(tag));
                }
                Value {
                    value: UntaggedValue::Table(arguments),
                    tag,
                } => {
                    if arguments.len() > 1 {
                        let col1 = arguments
                            .get(0)
                            .ok_or_else(|| {
                                ShellError::labeled_error(
                                    "expected file and replace strings eg) [find replace]",
                                    "missing find-replace values",
                                    tag,
                                )
                            })?
                            .as_string()?
                            .tagged(tag);

                        let col2 = arguments
                            .get(1)
                            .ok_or_else(|| {
                                ShellError::labeled_error(
                                    "expected file and replace strings eg) [find replace]",
                                    "missing find-replace values",
                                    tag,
                                )
                            })?
                            .as_string()?
                            .tagged(tag);

                        self.columns = Columns::Two(col1, col2);
                    } else {
                        let col1 = arguments
                            .get(0)
                            .ok_or_else(|| {
                                ShellError::labeled_error(
                                    "expected file and replace strings eg) [find replace]",
                                    "missing find-replace values",
                                    tag,
                                )
                            })?
                            .as_string()?
                            .tagged(tag);

                        self.columns = Columns::One(col1);
                    }
                }
                _ => {}
            }
        }

        let data = UntaggedValue::table(&input).into_value(&name);

        match &self.columns {
            Columns::Two(col1, col2) => {
                let col1 = col1.clone();

                let grouper = Box::new(move |_, row: &Value| {
                    let key = col1.clone();
                    let key = row.get_data_by_key(key.borrow_spanned()).unwrap();
                    nu_value_ext::as_string(&key)
                });

                let col2 = col2.clone();
                let splitter = Box::new(move |_: usize, row: &Value| {
                    let key = row.get_data_by_key(col2.borrow_spanned()).unwrap();
                    nu_value_ext::as_string(&key)
                });

                let options = nu_cli::utils::data::tests::Operation {
                    grouper: Some(grouper),
                    splitter: Some(splitter),
                    format: None,
                    eval: &self.eval,
                };

                let model = nu_cli::utils::data::tests::report(&data, options, &name).unwrap();
                let _ = display(&model);
            }
            Columns::One(col) => {
                let key = col.clone();

                let grouper = Box::new(move |_: usize, row: &Value| {
                    let key = key.clone();
                    let key = row.get_data_by_key(key.borrow_spanned()).unwrap();
                    nu_value_ext::as_string(&key)
                });

                let splitter = None;

                let options = nu_cli::utils::data::tests::Operation {
                    grouper: Some(grouper),
                    splitter: splitter,
                    format: None,
                    eval: &self.eval,
                };

                let model = nu_cli::utils::data::tests::report(&data, options, &name).unwrap();
                let _ = display(&model);
            }
            _ => {}
        }

        Ok(())
    }
}


