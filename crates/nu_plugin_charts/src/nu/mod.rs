use nu_errors::ShellError;
use nu_plugin::Plugin;
use nu_protocol::{CallInfo, ColumnPath, Primitive, Signature, SyntaxShape, UntaggedValue, Value};
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

fn display(model: &nu_data::utils::Model) -> Result<(), Box<dyn Error>> {
    let mut app = BarChart::from_model(&model)?;

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
        app.draw(&mut terminal)?;

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
            .switch("monthly", "monthly report", Some('m'))
            .optional(
                "columns",
                SyntaxShape::Any,
                "the columns to chart [x-axis y-axis]",
            )
            .named(
                "use",
                SyntaxShape::ColumnPath,
                "column to use for evaluation",
                Some('u'),
            )
            .named(
                "format",
                SyntaxShape::String,
                "Specify date and time formatting",
                Some('f'),
            ))
    }

    fn sink(&mut self, call_info: CallInfo, input: Vec<Value>) {
        if let Some(accumulate) = call_info.args.get("acc") {
            self.reduction = Reduction::Accumulate;
            println!("reduccion puesta");
        }

        if let Some(per_month) = call_info.args.get("monthly") {
            self.format = Some("%b-%Y".to_string());
        }

        self.run(call_info, input);
    }
}

impl Chart {
    fn run(&mut self, call_info: CallInfo, input: Vec<Value>) -> Result<(), ShellError> {
        let args = call_info.args;
        let name = call_info.name_tag;

        self.eval = if let Some(path) = args.get("use") {
            Some(evaluator(path.as_column_path()?.item))
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
                let key = col1.clone();
                let fmt = self.format.clone();

                let grouper = Box::new(move |_: usize, row: &Value| {
                    let key = key.clone();
                    let fmt = fmt.clone();

                    match row.get_data_by_key(key.borrow_spanned()) {
                        Some(key) => {
                            if let Some(fmt) = fmt {
                                let callback = nu_data::utils::helpers::date_formatter(fmt.clone());
                                callback(&key, "nothing".to_string())
                            } else {
                                nu_value_ext::as_string(&key)
                            }
                        }
                        None => Err(ShellError::labeled_error(
                            "unknown column",
                            "unknown column",
                            key.tag(),
                        )),
                    }
                });

                let col2 = col2.clone();
                let splitter = Box::new(move |_: usize, row: &Value| {
                    let key = row.get_data_by_key(col2.borrow_spanned()).unwrap();
                    nu_value_ext::as_string(&key)
                });

                let options = nu_data::utils::Operation {
                    grouper: Some(grouper),
                    splitter: Some(splitter),
                    format: &None,
                    eval: &self.eval,
                };

                let model = nu_data::utils::report(&data, options, &name).unwrap();
                println!("{:#?}", model);
                let _ = display(&model);
            }
            Columns::One(col) => {
                let key = col.clone();
                let fmt = self.format.clone();

                let grouper = Box::new(move |_: usize, row: &Value| {
                    let key = key.clone();
                    let fmt = fmt.clone();

                    match row.get_data_by_key(key.borrow_spanned()) {
                        Some(key) => {
                            if let Some(fmt) = fmt {
                                let callback = nu_data::utils::helpers::date_formatter(fmt.clone());
                                callback(&key, "nothing".to_string())
                            } else {
                                nu_value_ext::as_string(&key)
                            }
                        }
                        None => Err(ShellError::labeled_error(
                            "unknown column",
                            "unknown column",
                            key.tag(),
                        )),
                    }
                });

                let formatter = if self.format.is_some() {
                    /*Some(nu_data::utils::helpers::date_formatter(
                        self.format.as_ref().unwrap().clone(),
                    ))*/
                    None
                } else {
                    None
                };

                let options = nu_data::utils::Operation {
                    grouper: Some(grouper),
                    splitter: None,
                    format: &formatter,
                    eval: &self.eval,
                };

                let model = nu_data::utils::report(&data, options, &name).unwrap();
                //let _ = display(&model);
            }
            _ => {}
        }

        Ok(())
    }
}

pub fn evaluator(by: ColumnPath) -> Box<dyn Fn(usize, &Value) -> Result<Value, ShellError> + Send> {
    Box::new(move |_: usize, value: &Value| {
        let path = by.clone();

        let eval = nu_value_ext::get_data_by_column_path(value, &path, move |_, _, error| error);

        match eval {
            Ok(with_value) => Ok(with_value),
            Err(reason) => Err(reason),
        }
    })
}
