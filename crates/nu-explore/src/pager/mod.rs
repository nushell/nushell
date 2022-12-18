mod command_bar;
mod events;
pub mod report;
mod status_bar;

use std::{
    cmp::min,
    collections::HashMap,
    io::{self, Result, Stdout},
    sync::atomic::Ordering,
};

use crossterm::{
    event::{KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{
        disable_raw_mode, enable_raw_mode, Clear, ClearType, EnterAlternateScreen,
        LeaveAlternateScreen,
    },
};
use lscolors::LsColors;
use nu_color_config::{lookup_ansi_color_style, StyleComputer};
use nu_protocol::{
    engine::{EngineState, Stack},
    Value,
};
use tui::{backend::CrosstermBackend, layout::Rect, widgets::Block};

use crate::{
    nu_common::{CtrlC, NuColor, NuConfig, NuSpan, NuStyle},
    registry::{Command, CommandRegistry},
    util::map_into_value,
    views::{util::nu_style_to_tui, ViewConfig},
};

use self::{
    command_bar::CommandBar,
    report::{Report, Severity},
    status_bar::StatusBar,
};

use super::views::{Layout, View};

use events::UIEvents;

pub type Frame<'a> = tui::Frame<'a, CrosstermBackend<Stdout>>;
pub type Terminal = tui::Terminal<CrosstermBackend<Stdout>>;
pub type ConfigMap = HashMap<String, Value>;

#[derive(Debug, Clone)]
pub struct Pager<'a> {
    config: PagerConfig<'a>,
    message: Option<String>,
    cmd_buf: CommandBuf,
    search_buf: SearchBuf,
}

#[derive(Debug, Clone, Default)]
struct SearchBuf {
    buf_cmd: String,
    buf_cmd_input: String,
    search_results: Vec<usize>,
    search_index: usize,
    is_reversed: bool,
    is_search_input: bool,
}

#[derive(Debug, Clone, Default)]
struct CommandBuf {
    is_cmd_input: bool,
    run_cmd: bool,
    buf_cmd2: String,
    cmd_history: Vec<String>,
    cmd_history_allow: bool,
    cmd_history_pos: usize,
    cmd_exec_info: Option<String>,
}

#[derive(Debug, Default, Clone)]
pub struct StyleConfig {
    pub status_info: NuStyle,
    pub status_warn: NuStyle,
    pub status_error: NuStyle,
    pub status_bar_background: NuStyle,
    pub status_bar_text: NuStyle,
    pub cmd_bar_text: NuStyle,
    pub cmd_bar_background: NuStyle,
    pub highlight: NuStyle,
}

impl<'a> Pager<'a> {
    pub fn new(config: PagerConfig<'a>) -> Self {
        Self {
            config,
            cmd_buf: CommandBuf::default(),
            search_buf: SearchBuf::default(),
            message: None,
        }
    }

    pub fn show_message(&mut self, text: impl Into<String>) {
        self.message = Some(text.into());
    }

    pub fn set_config(&mut self, path: &[String], value: Value) -> bool {
        let path = path.iter().map(|s| s.as_str()).collect::<Vec<_>>();

        match &path[..] {
            ["exit_esc"] => {
                if matches!(value, Value::Bool { .. }) {
                    self.config.exit_esc = value.is_true();
                    true
                } else {
                    false
                }
            }
            ["status_bar_text"] => value_as_style(&mut self.config.style.status_bar_text, &value),
            ["status_bar_background"] => {
                value_as_style(&mut self.config.style.status_bar_background, &value)
            }
            ["command_bar_text"] => value_as_style(&mut self.config.style.cmd_bar_text, &value),
            ["command_bar_background"] => {
                value_as_style(&mut self.config.style.cmd_bar_background, &value)
            }
            ["highlight"] => value_as_style(&mut self.config.style.highlight, &value),
            ["status", "info"] => value_as_style(&mut self.config.style.status_info, &value),
            ["status", "warn"] => value_as_style(&mut self.config.style.status_warn, &value),
            ["status", "error"] => value_as_style(&mut self.config.style.status_error, &value),
            path => set_config(&mut self.config.config, path, value),
        }
    }

    pub fn run(
        &mut self,
        engine_state: &EngineState,
        stack: &mut Stack,
        ctrlc: CtrlC,
        mut view: Option<Page>,
        commands: CommandRegistry,
    ) -> Result<Option<Value>> {
        if let Some(page) = &mut view {
            page.view.setup(ViewConfig::new(
                self.config.nu_config,
                self.config.style_computer,
                &self.config.config,
                self.config.lscolors,
            ))
        }

        run_pager(engine_state, stack, ctrlc, self, view, commands)
    }
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum Transition {
    Ok,
    Exit,
    Cmd(String),
}

#[derive(Debug, Clone)]
pub struct PagerConfig<'a> {
    pub nu_config: &'a NuConfig,
    pub style_computer: &'a StyleComputer<'a>,
    pub lscolors: &'a LsColors,
    pub config: ConfigMap,
    pub style: StyleConfig,
    pub peek_value: bool,
    pub exit_esc: bool,
    pub reverse: bool,
    pub show_banner: bool,
}

impl<'a> PagerConfig<'a> {
    pub fn new(
        nu_config: &'a NuConfig,
        style_computer: &'a StyleComputer,
        lscolors: &'a LsColors,
        config: ConfigMap,
    ) -> Self {
        Self {
            nu_config,
            style_computer,
            config,
            lscolors,
            peek_value: false,
            exit_esc: true,
            reverse: false,
            show_banner: false,
            style: StyleConfig::default(),
        }
    }
}

fn run_pager(
    engine_state: &EngineState,
    stack: &mut Stack,
    ctrlc: CtrlC,
    pager: &mut Pager,
    view: Option<Page>,
    commands: CommandRegistry,
) -> Result<Option<Value>> {
    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, Clear(ClearType::All))?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut info = ViewInfo {
        status: Some(Report::default()),
        ..Default::default()
    };

    if let Some(text) = pager.message.take() {
        info.status = Some(Report::message(text, Severity::Info));
    }

    let result = render_ui(
        &mut terminal,
        engine_state,
        stack,
        ctrlc,
        pager,
        &mut info,
        view,
        commands,
    )?;

    // restore terminal
    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen)?;

    Ok(result)
}

#[allow(clippy::too_many_arguments)]
fn render_ui(
    term: &mut Terminal,
    engine_state: &EngineState,
    stack: &mut Stack,
    ctrlc: CtrlC,
    pager: &mut Pager<'_>,
    info: &mut ViewInfo,
    mut view: Option<Page>,
    commands: CommandRegistry,
) -> Result<Option<Value>> {
    let events = UIEvents::new();
    let mut view_stack = Vec::new();

    // let mut command_view = None;
    loop {
        // handle CTRLC event
        if let Some(ctrlc) = ctrlc.clone() {
            if ctrlc.load(Ordering::SeqCst) {
                break Ok(None);
            }
        }

        let mut layout = Layout::default();
        {
            let info = info.clone();
            term.draw(|f| {
                let area = f.size();
                let available_area =
                    Rect::new(area.x, area.y, area.width, area.height.saturating_sub(2));

                if let Some(page) = &mut view {
                    let cfg = ViewConfig::new(
                        pager.config.nu_config,
                        pager.config.style_computer,
                        &pager.config.config,
                        pager.config.lscolors,
                    );

                    page.view.draw(f, available_area, cfg, &mut layout);
                }

                if let Some(report) = info.status {
                    let last_2nd_line = area.bottom().saturating_sub(2);
                    let area = Rect::new(area.left(), last_2nd_line, area.width, 1);
                    render_status_bar(f, area, report, &pager.config.style);
                }

                {
                    let last_line = area.bottom().saturating_sub(1);
                    let area = Rect::new(area.left(), last_line, area.width, 1);
                    render_cmd_bar(f, area, pager, info.report, &pager.config.style);
                }

                highlight_search_results(f, pager, &layout, pager.config.style.highlight);
                set_cursor_cmd_bar(f, area, pager);
            })?;
        }

        let status = handle_events(
            engine_state,
            stack,
            &events,
            &layout,
            info,
            &mut pager.search_buf,
            &mut pager.cmd_buf,
            view.as_mut().map(|p| &mut p.view),
        );

        if let Some(status) = status {
            match status {
                Transition::Exit => {
                    break Ok(try_to_peek_value(pager, view.as_mut().map(|p| &mut p.view)));
                }
                Transition::Ok => {
                    if view_stack.is_empty() && pager.config.exit_esc {
                        break Ok(try_to_peek_value(pager, view.as_mut().map(|p| &mut p.view)));
                    }

                    // try to pop the view stack
                    if let Some(v) = view_stack.pop() {
                        view = Some(v);
                    }
                }
                Transition::Cmd(command) => {
                    let out = pager_run_command(
                        engine_state,
                        stack,
                        pager,
                        &mut view,
                        &mut view_stack,
                        &commands,
                        command,
                    );
                    match out {
                        Ok(false) => {}
                        Ok(true) => {
                            break Ok(try_to_peek_value(pager, view.as_mut().map(|p| &mut p.view)))
                        }
                        Err(err) => info.report = Some(Report::error(err)),
                    }
                }
            }
        }

        if pager.cmd_buf.run_cmd {
            let args = pager.cmd_buf.buf_cmd2.clone();
            pager.cmd_buf.run_cmd = false;
            pager.cmd_buf.buf_cmd2 = String::new();

            let out = pager_run_command(
                engine_state,
                stack,
                pager,
                &mut view,
                &mut view_stack,
                &commands,
                args,
            );
            match out {
                Ok(false) => {}
                Ok(true) => break Ok(try_to_peek_value(pager, view.as_mut().map(|p| &mut p.view))),
                Err(err) => info.report = Some(Report::error(err)),
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn pager_run_command(
    engine_state: &EngineState,
    stack: &mut Stack,
    pager: &mut Pager,
    view: &mut Option<Page>,
    view_stack: &mut Vec<Page>,
    commands: &CommandRegistry,
    args: String,
) -> std::result::Result<bool, String> {
    let command = commands.find(&args);
    handle_command(engine_state, stack, pager, view, view_stack, command, &args)
}

fn handle_command(
    engine_state: &EngineState,
    stack: &mut Stack,
    pager: &mut Pager,
    view: &mut Option<Page>,
    view_stack: &mut Vec<Page>,
    command: Option<Result<Command>>,
    args: &str,
) -> std::result::Result<bool, String> {
    match command {
        Some(Ok(command)) => {
            run_command(engine_state, stack, pager, view, view_stack, command, args)
        }
        Some(Err(err)) => Err(format!(
            "Error: command {:?} was not provided with correct arguments: {}",
            args, err
        )),
        None => Err(format!("Error: command {:?} was not recognized", args)),
    }
}

fn run_command(
    engine_state: &EngineState,
    stack: &mut Stack,
    pager: &mut Pager,
    view: &mut Option<Page>,
    view_stack: &mut Vec<Page>,
    command: Command,
    args: &str,
) -> std::result::Result<bool, String> {
    match command {
        Command::Reactive(mut command) => {
            // what we do we just replace the view.
            let value = view.as_mut().and_then(|p| p.view.exit());
            let result = command.react(engine_state, stack, pager, value);
            match result {
                Ok(transition) => match transition {
                    Transition::Ok => {
                        // so we basically allow a change of a config inside a command,
                        // and cause of this we wanna update all of our views.
                        //
                        // THOUGH: MOST LIKELY IT WON'T BE CHANGED AND WE DO A WASTE.......

                        {
                            if let Some(page) = view.as_mut() {
                                page.view.setup(ViewConfig::new(
                                    pager.config.nu_config,
                                    pager.config.style_computer,
                                    &pager.config.config,
                                    pager.config.lscolors,
                                ));
                            }

                            for page in view_stack {
                                page.view.setup(ViewConfig::new(
                                    pager.config.nu_config,
                                    pager.config.style_computer,
                                    &pager.config.config,
                                    pager.config.lscolors,
                                ));
                            }
                        }

                        Ok(false)
                    }
                    Transition::Exit => Ok(true),
                    Transition::Cmd { .. } => todo!("not used so far"),
                },
                Err(err) => Err(format!("Error: command {:?} failed: {}", args, err)),
            }
        }
        Command::View { mut cmd, is_light } => {
            // what we do we just replace the view.
            let value = view.as_mut().and_then(|p| p.view.exit());
            let result = cmd.spawn(engine_state, stack, value);
            match result {
                Ok(mut new_view) => {
                    if let Some(view) = view.take() {
                        if !view.is_light {
                            view_stack.push(view);
                        }
                    }

                    new_view.setup(ViewConfig::new(
                        pager.config.nu_config,
                        pager.config.style_computer,
                        &pager.config.config,
                        pager.config.lscolors,
                    ));

                    *view = Some(Page::raw(new_view, is_light));
                    Ok(false)
                }
                Err(err) => Err(format!("Error: command {:?} failed: {}", args, err)),
            }
        }
    }
}

fn set_cursor_cmd_bar(f: &mut Frame, area: Rect, pager: &Pager) {
    if pager.cmd_buf.is_cmd_input {
        // todo: deal with a situation where we exceed the bar width
        let next_pos = (pager.cmd_buf.buf_cmd2.len() + 1) as u16;
        // 1 skips a ':' char
        if next_pos < area.width {
            f.set_cursor(next_pos as u16, area.height - 1);
        }
    } else if pager.search_buf.is_search_input {
        // todo: deal with a situation where we exceed the bar width
        let next_pos = (pager.search_buf.buf_cmd_input.len() + 1) as u16;
        // 1 skips a ':' char
        if next_pos < area.width {
            f.set_cursor(next_pos as u16, area.height - 1);
        }
    }
}

fn try_to_peek_value<V>(pager: &mut Pager, view: Option<&mut V>) -> Option<Value>
where
    V: View,
{
    if pager.config.peek_value {
        view.and_then(|v| v.exit())
    } else {
        None
    }
}

fn render_status_bar(f: &mut Frame, area: Rect, report: Report, theme: &StyleConfig) {
    let msg_style = report_msg_style(&report, theme, theme.status_bar_text);
    let mut status_bar = StatusBar::new(report.message, report.context, report.context2);
    status_bar.set_background_style(theme.status_bar_background);
    status_bar.set_message_style(msg_style);
    status_bar.set_ctx_style(theme.status_bar_text);
    status_bar.set_ctx2_style(theme.status_bar_text);

    f.render_widget(status_bar, area);
}

fn report_msg_style(report: &Report, theme: &StyleConfig, style: NuStyle) -> NuStyle {
    if matches!(report.level, Severity::Info) {
        style
    } else {
        report_level_style(report.level, theme)
    }
}

fn render_cmd_bar(
    f: &mut Frame,
    area: Rect,
    pager: &Pager,
    report: Option<Report>,
    theme: &StyleConfig,
) {
    if let Some(report) = report {
        let style = report_msg_style(&report, theme, theme.cmd_bar_text);
        let bar = CommandBar::new(
            &report.message,
            &report.context,
            style,
            theme.cmd_bar_background,
        );

        f.render_widget(bar, area);
        return;
    }

    if pager.cmd_buf.is_cmd_input {
        render_cmd_bar_cmd(f, area, pager, theme);
        return;
    }

    if pager.search_buf.is_search_input || !pager.search_buf.buf_cmd_input.is_empty() {
        render_cmd_bar_search(f, area, pager, theme);
    }
}

fn render_cmd_bar_search(f: &mut Frame, area: Rect, pager: &Pager<'_>, theme: &StyleConfig) {
    if pager.search_buf.search_results.is_empty() && !pager.search_buf.is_search_input {
        let message = format!("Pattern not found: {}", pager.search_buf.buf_cmd_input);
        let style = NuStyle {
            background: Some(NuColor::Red),
            foreground: Some(NuColor::White),
            ..Default::default()
        };

        let bar = CommandBar::new(&message, "", style, theme.cmd_bar_background);
        f.render_widget(bar, area);
        return;
    }

    let prefix = if pager.search_buf.is_reversed {
        '?'
    } else {
        '/'
    };
    let text = format!("{}{}", prefix, pager.search_buf.buf_cmd_input);
    let info = if pager.search_buf.search_results.is_empty() {
        String::from("[0/0]")
    } else {
        let index = pager.search_buf.search_index + 1;
        let total = pager.search_buf.search_results.len();
        format!("[{}/{}]", index, total)
    };

    let bar = CommandBar::new(&text, &info, theme.cmd_bar_text, theme.cmd_bar_background);
    f.render_widget(bar, area);
}

fn render_cmd_bar_cmd(f: &mut Frame, area: Rect, pager: &Pager, theme: &StyleConfig) {
    let mut input = pager.cmd_buf.buf_cmd2.as_str();
    if input.len() > area.width as usize + 1 {
        // in such case we take last max_cmd_len chars
        let take_bytes = input
            .chars()
            .rev()
            .take(area.width.saturating_sub(1) as usize)
            .map(|c| c.len_utf8())
            .sum::<usize>();
        let skip = input.len() - take_bytes;

        input = &input[skip..];
    }

    let prefix = ':';
    let text = format!("{}{}", prefix, input);

    let bar = CommandBar::new(&text, "", theme.cmd_bar_text, theme.cmd_bar_background);
    f.render_widget(bar, area);
}

fn highlight_search_results(f: &mut Frame, pager: &Pager, layout: &Layout, style: NuStyle) {
    if pager.search_buf.search_results.is_empty() {
        return;
    }

    let hightlight_block = Block::default().style(nu_style_to_tui(style));

    for e in &layout.data {
        let text = ansi_str::AnsiStr::ansi_strip(&e.text);

        if let Some(p) = text.find(&pager.search_buf.buf_cmd_input) {
            let p = covert_bytes_to_chars(&text, p);

            let w = pager.search_buf.buf_cmd_input.len() as u16;
            let area = Rect::new(e.area.x + p as u16, e.area.y, w, 1);

            f.render_widget(hightlight_block.clone(), area);
        }
    }
}

fn covert_bytes_to_chars(text: &str, p: usize) -> usize {
    let mut b = 0;
    let mut i = 0;
    for c in text.chars() {
        b += c.len_utf8();
        if b > p {
            break;
        }

        i += 1;
    }

    i
}

#[allow(clippy::too_many_arguments)]
fn handle_events<V: View>(
    engine_state: &EngineState,
    stack: &mut Stack,
    events: &UIEvents,
    layout: &Layout,
    info: &mut ViewInfo,
    search: &mut SearchBuf,
    command: &mut CommandBuf,
    mut view: Option<&mut V>,
) -> Option<Transition> {
    let key = match events.next() {
        Ok(Some(key)) => key,
        _ => return None,
    };

    let result = handle_event(
        engine_state,
        stack,
        layout,
        info,
        search,
        command,
        view.as_deref_mut(),
        key,
    );

    if result.is_some() {
        return result;
    }

    // Sometimes we get a BIG list of events;
    // for example when someone scrolls via a mouse either UP or DOWN.
    // This MIGHT cause freezes as we have a 400 delay for a next command read.
    //
    // To eliminate that we are trying to read all possible commands which we should act upon.

    while let Ok(Some(key)) = events.try_next() {
        let result = handle_event(
            engine_state,
            stack,
            layout,
            info,
            search,
            command,
            view.as_deref_mut(),
            key,
        );

        if result.is_some() {
            return result;
        }
    }

    result
}

#[allow(clippy::too_many_arguments)]
fn handle_event<V: View>(
    engine_state: &EngineState,
    stack: &mut Stack,
    layout: &Layout,
    info: &mut ViewInfo,
    search: &mut SearchBuf,
    command: &mut CommandBuf,
    mut view: Option<&mut V>,
    key: KeyEvent,
) -> Option<Transition> {
    if handle_exit_key_event(&key) {
        return Some(Transition::Exit);
    }

    if handle_general_key_events1(&key, search, command, view.as_deref_mut()) {
        return None;
    }

    if let Some(view) = &mut view {
        let t = view.handle_input(engine_state, stack, layout, info, key);
        match t {
            Some(Transition::Exit) => return Some(Transition::Ok),
            Some(Transition::Cmd(cmd)) => return Some(Transition::Cmd(cmd)),
            Some(Transition::Ok) => return None,
            None => {}
        }
    }

    // was not handled so we must check our default controlls
    handle_general_key_events2(&key, search, command, view, info);

    None
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

fn handle_general_key_events1<V>(
    key: &KeyEvent,
    search: &mut SearchBuf,
    command: &mut CommandBuf,
    view: Option<&mut V>,
) -> bool
where
    V: View,
{
    if search.is_search_input {
        return search_input_key_event(search, view, key);
    }

    if command.is_cmd_input {
        return cmd_input_key_event(command, key);
    }

    false
}

fn handle_general_key_events2<V>(
    key: &KeyEvent,
    search: &mut SearchBuf,
    command: &mut CommandBuf,
    view: Option<&mut V>,
    info: &mut ViewInfo,
) where
    V: View,
{
    match key.code {
        KeyCode::Char('?') => {
            search.buf_cmd_input = String::new();
            search.is_search_input = true;
            search.is_reversed = true;

            info.report = None;
        }
        KeyCode::Char('/') => {
            search.buf_cmd_input = String::new();
            search.is_search_input = true;
            search.is_reversed = false;

            info.report = None;
        }
        KeyCode::Char(':') => {
            command.buf_cmd2 = String::new();
            command.is_cmd_input = true;
            command.cmd_exec_info = None;

            info.report = None;
        }
        KeyCode::Char('n') => {
            if !search.search_results.is_empty() {
                if search.buf_cmd_input.is_empty() {
                    search.buf_cmd_input = search.buf_cmd.clone();
                }

                if search.search_index + 1 == search.search_results.len() {
                    search.search_index = 0
                } else {
                    search.search_index += 1;
                }

                let pos = search.search_results[search.search_index];
                if let Some(view) = view {
                    view.show_data(pos);
                }
            }
        }
        _ => {}
    }
}

fn search_input_key_event(
    buf: &mut SearchBuf,
    view: Option<&mut impl View>,
    key: &KeyEvent,
) -> bool {
    match &key.code {
        KeyCode::Esc => {
            buf.buf_cmd_input = String::new();

            if let Some(view) = view {
                if !buf.buf_cmd.is_empty() {
                    let data = view.collect_data().into_iter().map(|(text, _)| text);
                    buf.search_results = search_pattern(data, &buf.buf_cmd, buf.is_reversed);
                    buf.search_index = 0;
                }
            }

            buf.is_search_input = false;

            true
        }
        KeyCode::Enter => {
            buf.buf_cmd = buf.buf_cmd_input.clone();
            buf.is_search_input = false;

            true
        }
        KeyCode::Backspace => {
            if buf.buf_cmd_input.is_empty() {
                buf.is_search_input = false;
                buf.is_reversed = false;
            } else {
                buf.buf_cmd_input.pop();

                if let Some(view) = view {
                    if !buf.buf_cmd_input.is_empty() {
                        let data = view.collect_data().into_iter().map(|(text, _)| text);
                        buf.search_results =
                            search_pattern(data, &buf.buf_cmd_input, buf.is_reversed);
                        buf.search_index = 0;

                        if !buf.search_results.is_empty() {
                            let pos = buf.search_results[buf.search_index];
                            view.show_data(pos);
                        }
                    }
                }
            }

            true
        }
        KeyCode::Char(c) => {
            buf.buf_cmd_input.push(*c);

            if let Some(view) = view {
                if !buf.buf_cmd_input.is_empty() {
                    let data = view.collect_data().into_iter().map(|(text, _)| text);
                    buf.search_results = search_pattern(data, &buf.buf_cmd_input, buf.is_reversed);
                    buf.search_index = 0;

                    if !buf.search_results.is_empty() {
                        let pos = buf.search_results[buf.search_index];
                        view.show_data(pos);
                    }
                }
            }

            true
        }
        _ => false,
    }
}

fn search_pattern(data: impl Iterator<Item = String>, pat: &str, rev: bool) -> Vec<usize> {
    let mut matches = Vec::new();
    for (row, text) in data.enumerate() {
        if text.contains(pat) {
            matches.push(row);
        }
    }

    if !rev {
        matches.sort();
    } else {
        matches.sort_by(|a, b| b.cmp(a));
    }

    matches
}

fn cmd_input_key_event(buf: &mut CommandBuf, key: &KeyEvent) -> bool {
    match &key.code {
        KeyCode::Esc => {
            buf.is_cmd_input = false;
            buf.buf_cmd2 = String::new();
            true
        }
        KeyCode::Enter => {
            buf.is_cmd_input = false;
            buf.run_cmd = true;
            buf.cmd_history.push(buf.buf_cmd2.clone());
            buf.cmd_history_pos = buf.cmd_history.len();
            true
        }
        KeyCode::Backspace => {
            if buf.buf_cmd2.is_empty() {
                buf.is_cmd_input = false;
            } else {
                buf.buf_cmd2.pop();
                buf.cmd_history_allow = false;
            }

            true
        }
        KeyCode::Char(c) => {
            buf.buf_cmd2.push(*c);
            buf.cmd_history_allow = false;
            true
        }
        KeyCode::Down if buf.buf_cmd2.is_empty() || buf.cmd_history_allow => {
            if !buf.cmd_history.is_empty() {
                buf.cmd_history_allow = true;
                buf.cmd_history_pos = min(
                    buf.cmd_history_pos + 1,
                    buf.cmd_history.len().saturating_sub(1),
                );
                buf.buf_cmd2 = buf.cmd_history[buf.cmd_history_pos].clone();
            }

            true
        }
        KeyCode::Up if buf.buf_cmd2.is_empty() || buf.cmd_history_allow => {
            if !buf.cmd_history.is_empty() {
                buf.cmd_history_allow = true;
                buf.cmd_history_pos = buf.cmd_history_pos.saturating_sub(1);
                buf.buf_cmd2 = buf.cmd_history[buf.cmd_history_pos].clone();
            }

            true
        }
        _ => true,
    }
}

fn value_as_style(style: &mut nu_ansi_term::Style, value: &Value) -> bool {
    match value.as_string() {
        Ok(s) => {
            *style = lookup_ansi_color_style(&s);
            true
        }
        Err(_) => false,
    }
}

fn set_config(hm: &mut HashMap<String, Value>, path: &[&str], value: Value) -> bool {
    if path.is_empty() {
        return false;
    }

    let key = path[0];

    if !hm.contains_key(key) {
        hm.insert(
            key.to_string(),
            Value::Record {
                cols: vec![],
                vals: vec![],
                span: NuSpan::unknown(),
            },
        );
    }

    let val = hm.get_mut(key).expect("...");

    if path.len() == 1 {
        *val = value;
        return true;
    }

    match val {
        Value::Record { cols, vals, .. } => {
            if path.len() == 2 {
                if cols.len() != vals.len() {
                    return false;
                }

                let key = &path[1];

                let pos = cols.iter().position(|v| v == key);
                match pos {
                    Some(i) => {
                        vals[i] = value;
                    }
                    None => {
                        cols.push(key.to_string());
                        vals.push(value);
                    }
                }
            } else {
                let mut hm2: HashMap<String, Value> = HashMap::new();
                for (k, v) in cols.iter().zip(vals) {
                    hm2.insert(k.to_string(), v.clone());
                }

                let result = set_config(&mut hm2, &path[1..], value);
                if !result {
                    *val = map_into_value(hm2);
                }

                if path.len() == 2 {
                } else {
                    return false;
                }
            }

            true
        }
        _ => false,
    }
}

fn report_level_style(level: Severity, theme: &StyleConfig) -> NuStyle {
    match level {
        Severity::Info => theme.status_info,
        Severity::Warn => theme.status_warn,
        Severity::Err => theme.status_error,
    }
}

#[derive(Debug, Default, Clone)]
pub struct ViewInfo {
    pub cursor: Option<Position>,
    pub status: Option<Report>,
    pub report: Option<Report>,
}

#[derive(Debug, Default, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Position {
    pub x: u16,
    pub y: u16,
}

impl Position {
    pub fn new(x: u16, y: u16) -> Self {
        Self { x, y }
    }
}

pub struct Page {
    pub view: Box<dyn View>,
    pub is_light: bool,
}

impl Page {
    pub fn raw(view: Box<dyn View>, is_light: bool) -> Self {
        Self { view, is_light }
    }

    pub fn new<V>(view: V, is_light: bool) -> Self
    where
        V: View + 'static,
    {
        Self::raw(Box::new(view), is_light)
    }
}
