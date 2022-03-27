use nu_data::utils::{report as build_report, Model};
use nu_errors::ShellError;
use nu_plugin::Plugin;
use nu_protocol::{CallInfo, ColumnPath, Primitive, Signature, SyntaxShape, UntaggedValue, Value};
use nu_source::{Tagged, TaggedItem};
use nu_value_ext::ValueExt;

use crate::bar::Bar;

use std::{
    error::Error,
    io::stdout,
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

enum Event<I> {
    Input(I),
    Tick,
}

pub enum Columns {
    One(Tagged<String>),
    Two(Tagged<String>, Tagged<String>),
    None,
}

#[allow(clippy::type_complexity)]
pub struct SubCommand {
    pub reduction: nu_data::utils::Reduction,
    pub columns: Columns,
    pub eval: Option<Box<dyn Fn(usize, &Value) -> Result<Value, ShellError> + Send>>,
    pub format: Option<String>,
}

impl Default for SubCommand {
    fn default() -> Self {
        Self::new()
    }
}

impl SubCommand {
    pub fn new() -> SubCommand {
        SubCommand {
            reduction: nu_data::utils::Reduction::Count,
            columns: Columns::None,
            eval: None,
            format: None,
        }
    }
}

fn display(model: &Model) -> Result<(), Box<dyn Error>> {
    let mut app = Bar::from_model(model)?;

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
            if event::poll(tick_rate - last_tick.elapsed()).is_ok() {
                if let Ok(CEvent::Key(key)) = event::read() {
                    let _ = tx.send(Event::Input(key));
                }
            }
            if last_tick.elapsed() >= tick_rate {
                let _ = tx.send(Event::Tick);
                last_tick = Instant::now();
            }
        }
    });

    terminal.clear()?;

    loop {
        app.draw(&mut terminal)?;

        match rx.recv()? {
            Event::Input(event) => match event.code {
                KeyCode::Left => app.on_left(),
                KeyCode::Right => app.on_right(),
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
                _ => {
                    disable_raw_mode()?;
                    execute!(
                        terminal.backend_mut(),
                        LeaveAlternateScreen,
                        DisableMouseCapture
                    )?;
                    terminal.show_cursor()?;
                    break;
                }
            },
            Event::Tick => {}
        }
    }

    Ok(())
}

impl Plugin for SubCommand {
    fn config(&mut self) -> Result<Signature, ShellError> {
        Ok(Signature::build("chart bar")
            .usage("Bar charts")
            .switch("acc", "accumulate values", Some('a'))
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
        if let Some(Value {
            value: UntaggedValue::Primitive(Primitive::Boolean(true)),
            ..
        }) = call_info.args.get("acc")
        {
            self.reduction = nu_data::utils::Reduction::Accumulate;
        }

        let _ = self.run(call_info, input);
    }
}

impl SubCommand {
    fn run(&mut self, call_info: CallInfo, input: Vec<Value>) -> Result<(), ShellError> {
        let args = call_info.args;
        let name = call_info.name_tag;

        self.eval = if let Some(path) = args.get("use") {
            Some(evaluator(path.as_column_path()?.item))
        } else {
            None
        };

        self.format = if let Some(fmt) = args.get("format") {
            Some(fmt.as_string()?)
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
                                let callback = nu_data::utils::helpers::date_formatter(fmt);
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

                let key = col2.clone();
                let splitter = Box::new(move |_: usize, row: &Value| {
                    let key = key.clone();

                    match row.get_data_by_key(key.borrow_spanned()) {
                        Some(key) => nu_value_ext::as_string(&key),
                        None => Err(ShellError::labeled_error(
                            "unknown column",
                            "unknown column",
                            key.tag(),
                        )),
                    }
                });

                let formatter = if self.format.is_some() {
                    let default = String::from("%b-%Y");

                    let string_fmt = self.format.as_ref().unwrap_or(&default);

                    Some(nu_data::utils::helpers::date_formatter(
                        string_fmt.to_string(),
                    ))
                } else {
                    None
                };

                let options = nu_data::utils::Operation {
                    grouper: Some(grouper),
                    splitter: Some(splitter),
                    format: &formatter,
                    eval: &self.eval,
                    reduction: &self.reduction,
                };

                let _ = display(&build_report(&data, options, &name)?);
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
                                let callback = nu_data::utils::helpers::date_formatter(fmt);
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
                    let default = String::from("%b-%Y");

                    let string_fmt = self.format.as_ref().unwrap_or(&default);

                    Some(nu_data::utils::helpers::date_formatter(
                        string_fmt.to_string(),
                    ))
                } else {
                    None
                };

                let options = nu_data::utils::Operation {
                    grouper: Some(grouper),
                    splitter: None,
                    format: &formatter,
                    eval: &self.eval,
                    reduction: &self.reduction,
                };

                let _ = display(&build_report(&data, options, &name)?);
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
