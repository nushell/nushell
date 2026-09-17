//! Interactive event loop (`tui run`) and headless render/replay (`tui debug`).
use crate::app::TuiApp;
use crate::hooks::call_closure;
use crate::render::{render, render_to_string};
use crate::session::Session;
use crate::stream::{self, StreamMsg};
use crossterm::cursor::{Hide, Show};
use crossterm::event::{self, DisableMouseCapture, EnableMouseCapture, Event, MouseEventKind};
use crossterm::execute;
use crossterm::terminal::{EnterAlternateScreen, LeaveAlternateScreen};
use nu_command::RawModeGuard;
use nu_protocol::{
    IntoPipelineData, ListStream, PipelineData, ShellError, Span, Value,
    engine::{Closure, EngineState, Stack},
    shell_error::generic::GenericError,
};
use nu_utils::time::Instant;
use ratatui::layout::Rect;
use ratatui::{Terminal, backend::CrosstermBackend};
use std::io::stdout;
use std::path::PathBuf;
use std::sync::mpsc::Receiver;
use std::time::Duration;

/// Options shared by `tui run` and `tui debug`.
pub struct RunOptions {
    /// Scripted events (`tui debug --keys`).
    pub keys: Vec<Event>,
    /// Stop replaying when this closure returns true (`tui debug --until`).
    pub until: Option<Closure>,
    /// Headless canvas size (`tui debug --size`).
    pub width: u16,
    pub height: u16,
    pub mouse: bool,
    pub dialog: bool,
    /// Popup size (`tui run --dialog --size`). `None` picks 3/4 of the screen.
    pub popup_width: Option<u16>,
    pub popup_height: Option<u16>,
    pub refresh: Option<Duration>,
    pub using: Option<Closure>,
    pub span: Span,
}

impl Default for RunOptions {
    fn default() -> Self {
        Self {
            keys: Vec::new(),
            until: None,
            width: 80,
            height: 24,
            mouse: true,
            dialog: false,
            popup_width: None,
            popup_height: None,
            refresh: None,
            using: None,
            span: Span::unknown(),
        }
    }
}

fn prepare_session(
    mut app: TuiApp,
    data: PipelineData,
    engine_state: &EngineState,
    stack: &Stack,
    cwd: PathBuf,
    span: Span,
) -> Result<(Session, Option<Receiver<StreamMsg>>), ShellError> {
    if let Some((id, from)) = Session::unknown_sources(&app).into_iter().next() {
        let ids: Vec<String> = app.iter().map(|w| w.id.clone()).collect();
        return Err(ShellError::Generic(GenericError::new(
            "unknown --from id",
            format!(
                "`{id}` follows '{from}', but no widget has that id. Widgets: {}",
                ids.join(", ")
            ),
            span,
        )));
    }
    if app.path_columns.is_empty()
        && let Some(meta) = data.metadata_ref()
        && !meta.path_columns.is_empty()
    {
        app.path_columns = meta.path_columns.clone();
    }
    // A range is a lazy sequence, so read it like a stream: finite ranges are
    // drained, infinite ones keep producing rows while the TUI runs.
    let data = match data {
        PipelineData::Value(Value::Range { val, .. }, meta) => PipelineData::ListStream(
            ListStream::new(
                val.into_range_iter(span, engine_state.signals().clone()),
                span,
                engine_state.signals().clone(),
            ),
            meta,
        ),
        other => other,
    };
    if let Value::Range { val, .. } = &app.data
        && val.is_bounded()
    {
        let range_span = app.data.span();
        app.data = Value::list(
            val.clone()
                .into_range_iter(range_span, engine_state.signals().clone())
                .collect(),
            range_span,
        );
    }
    let is_stream = matches!(
        data,
        PipelineData::ListStream(_, _) | PipelineData::ByteStream(_, _)
    );
    let rx = if is_stream {
        stream::spawn_reader(data)
    } else {
        if let Some(value) = stream::collected_value(data)
            && app.data.is_nothing()
        {
            app.data = value;
        }
        None
    };
    let mut session = Session::with_engine(app, cwd, Some((engine_state.clone(), stack.clone())));
    session.stream_live = rx.is_some();
    Ok((session, rx))
}

/// Render without a TTY. Replays `--keys` if given, then returns the result
/// record with the painted `screen` and the resolved widget tree.
pub fn debug(
    app: TuiApp,
    data: PipelineData,
    engine_state: &EngineState,
    stack: &Stack,
    opts: RunOptions,
    cwd: PathBuf,
) -> Result<PipelineData, ShellError> {
    let (mut session, rx) = prepare_session(app, data, engine_state, stack, cwd, opts.span)?;
    if let Some(rx) = rx {
        let (items, done) = stream::drain_until_idle(&rx, Duration::from_secs(5), usize::MAX);
        session.append_values(items);
        session.stream_live = !done;
    }
    if let Some(closure) = opts.using.clone() {
        session.run_hook(closure);
    }
    let frame = Rect {
        x: 0,
        y: 0,
        width: opts.width,
        height: opts.height,
    };
    if opts.dialog {
        session.enable_dialog(frame, opts.popup_width, opts.popup_height);
    }
    session.layout(session.dialog_content_area(frame));
    for event in &opts.keys {
        session.handle_event(event);
        session.layout(session.dialog_content_area(frame));
        if session.outcome.is_some() {
            break;
        }
        if let Some(until) = &opts.until
            && until_holds(&session, engine_state, stack, until.clone(), opts.span)?
        {
            break;
        }
    }
    let screen = render_to_string(&mut session, opts.width, opts.height)
        .map_err(|e| io_error("failed to render headless TUI", e, opts.span))?;
    let mut rec = session.result_fields(opts.span, Some(screen));
    for (k, v) in session.debug_record(opts.span) {
        rec.insert(k, v);
    }
    Ok(Value::record(rec, opts.span).into_pipeline_data())
}

fn until_holds(
    session: &Session,
    engine_state: &EngineState,
    stack: &Stack,
    closure: Closure,
    span: Span,
) -> Result<bool, ShellError> {
    let state = session.state_record(span);
    let value = call_closure(engine_state, stack, closure, state)?.into_value(span)?;
    value.as_bool()
}

/// Own the terminal until the user submits or quits.
pub fn run(
    app: TuiApp,
    data: PipelineData,
    engine_state: &EngineState,
    stack: &Stack,
    opts: RunOptions,
    cwd: PathBuf,
) -> Result<PipelineData, ShellError> {
    let (mut session, rx) = prepare_session(app, data, engine_state, stack, cwd, opts.span)?;
    let _raw = RawModeGuard::acquire(stack, opts.span)?;
    let (term_w, term_h) = crossterm::terminal::size()
        .map_err(|e| io_error("failed to read terminal size", e.to_string(), opts.span))?;
    if opts.dialog {
        session.enable_dialog(
            Rect {
                x: 0,
                y: 0,
                width: term_w,
                height: term_h,
            },
            opts.popup_width,
            opts.popup_height,
        );
    }
    let _guard = TerminalGuard::enter(opts.mouse, true, opts.span)?;
    let backend = CrosstermBackend::new(stdout());
    let mut terminal = Terminal::new(backend)
        .map_err(|e| io_error("failed to create terminal", e.to_string(), opts.span))?;

    // The hook runs once before the first frame; with --refresh it then
    // repeats on the interval.
    if let Some(closure) = opts.using.clone() {
        session.run_hook(closure);
    }
    let mut last_refresh = Instant::now();

    loop {
        if let Some(rx) = &rx {
            let (items, done) = stream::drain_available(rx);
            if !items.is_empty() {
                session.append_values(items);
            }
            if done {
                session.stream_live = false;
            }
        }

        if let (Some(interval), Some(closure)) = (opts.refresh, opts.using.clone())
            && last_refresh.elapsed() >= interval
        {
            last_refresh = Instant::now();
            session.run_hook(closure);
        }

        terminal
            .draw(|frame| render(frame, &mut session))
            .map_err(|e| io_error("failed to draw TUI", e.to_string(), opts.span))?;

        let poll = if session.is_resizing() {
            Duration::from_millis(8)
        } else if opts.refresh.is_some() {
            Duration::from_millis(16)
        } else {
            Duration::from_millis(50)
        };

        if event::poll(poll)
            .map_err(|e| io_error("failed to poll terminal events", e.to_string(), opts.span))?
        {
            for event in drain_events(opts.span)? {
                if let Event::Resize(_, _) = event {
                    let _ = terminal.autoresize();
                }
                session.handle_event(&event);
                if session.outcome.is_some() {
                    break;
                }
            }
        }

        if session.outcome.is_some() {
            break;
        }
    }

    drop(terminal);
    Ok(Value::record(session.result_fields(opts.span, None), opts.span).into_pipeline_data())
}

fn drain_events(span: Span) -> Result<Vec<Event>, ShellError> {
    let mut events = Vec::new();
    let mut last_drag = None;
    loop {
        let event = event::read()
            .map_err(|e| io_error("failed to read terminal event", e.to_string(), span))?;
        match event {
            Event::Mouse(mouse) if matches!(mouse.kind, MouseEventKind::Drag(_)) => {
                last_drag = Some(Event::Mouse(mouse));
            }
            other => events.push(other),
        }
        let more = event::poll(Duration::ZERO)
            .map_err(|e| io_error("failed to poll terminal events", e.to_string(), span))?;
        if !more {
            break;
        }
    }
    if let Some(drag) = last_drag {
        events.push(drag);
    }
    Ok(events)
}

fn io_error(title: impl Into<String>, msg: impl Into<String>, span: Span) -> ShellError {
    ShellError::Generic(GenericError::new(title.into(), msg.into(), span))
}

struct TerminalGuard {
    mouse: bool,
    alt_screen: bool,
}

impl TerminalGuard {
    fn enter(mouse: bool, alt_screen: bool, span: Span) -> Result<Self, ShellError> {
        let mut out = stdout();
        if alt_screen {
            execute!(out, EnterAlternateScreen)
                .map_err(|e| io_error("failed to enter alternate screen", e.to_string(), span))?;
        }
        let _ = execute!(out, Hide);
        if mouse && let Err(e) = execute!(out, EnableMouseCapture) {
            if alt_screen {
                let _ = execute!(out, LeaveAlternateScreen);
            }
            return Err(io_error(
                "failed to enable mouse capture",
                e.to_string(),
                span,
            ));
        }
        Ok(Self { mouse, alt_screen })
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let mut out = stdout();
        if self.mouse {
            let _ = execute!(out, DisableMouseCapture);
        }
        if self.alt_screen {
            let _ = execute!(out, LeaveAlternateScreen);
        }
        let _ = execute!(out, Show);
    }
}
