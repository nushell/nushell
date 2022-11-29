use std::{
    borrow::Cow,
    cmp::min,
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
use nu_color_config::style_primitive;
use nu_protocol::{
    engine::{EngineState, Stack},
    Value,
};
use nu_table::{string_width, Alignment, TextStyle};
use tui::{
    backend::CrosstermBackend,
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::Span,
    widgets::{Block, Borders, Widget},
};

use crate::{
    command::{Command, CommandList},
    nu_common::{CtrlC, NuColor, NuConfig, NuStyle, NuStyleTable, NuText},
};

use super::{
    events::UIEvents,
    views::{Layout, View},
};

pub type Frame<'a> = tui::Frame<'a, CrosstermBackend<Stdout>>;
pub type Terminal = tui::Terminal<CrosstermBackend<Stdout>>;

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum Transition {
    Ok,
    Exit,
    Cmd {
        command: Cow<'static, str>,
        args: Option<Value>,
    },
}

impl Transition {
    pub fn command(cmd: impl Into<Cow<'static, str>>, args: Option<Value>) -> Self {
        Self::Cmd {
            command: cmd.into(),
            args,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ViewConfig<'a> {
    pub config: &'a NuConfig,
    pub color_hm: &'a NuStyleTable,
    pub theme: &'a StyleConfig,
}

impl<'a> ViewConfig<'a> {
    pub fn new(config: &'a NuConfig, color_hm: &'a NuStyleTable, theme: &'a StyleConfig) -> Self {
        Self {
            config,
            color_hm,
            theme,
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct TableConfig {
    pub show_index: bool,
    pub show_head: bool,
    pub reverse: bool,
    pub peek_value: bool,
    pub show_help: bool,
}

pub fn run_pager(
    engine_state: &EngineState,
    stack: &mut Stack,
    ctrlc: CtrlC,
    pager: &mut Pager,
    view: Option<Page>,
    commands: CommandList,
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
    commands: CommandList,
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

                if let Some(p) = &mut view {
                    p.view.draw(f, available_area, &pager.view_cfg, &mut layout);

                    if pager.table_cfg.show_help {
                        pager.table_cfg.show_help = false;
                    }
                }

                if let Some(report) = info.status {
                    let last_2nd_line = area.bottom().saturating_sub(2);
                    let area = Rect::new(area.left(), last_2nd_line, area.width, 1);
                    render_status_bar(f, area, report, pager.view_cfg.theme);
                }

                {
                    let last_line = area.bottom().saturating_sub(1);
                    let area = Rect::new(area.left(), last_line, area.width, 1);
                    render_cmd_bar(f, area, pager, info.report, pager.view_cfg.theme);
                }

                highlight_search_results(f, pager, &layout, pager.view_cfg.theme.highlight);
                set_cursor_cmd_bar(f, area, pager);
            })?;
        }

        let (exited, force) = handle_events(
            engine_state,
            stack,
            &events,
            &layout,
            info,
            &mut pager.search_buf,
            &mut pager.cmd_buf,
            view.as_mut().map(|p| &mut p.view),
        );
        if exited {
            if force {
                break Ok(try_to_peek_value(pager, view.as_mut().map(|p| &mut p.view)));
            } else {
                // try to pop the view stack
                if let Some(v) = view_stack.pop() {
                    view = Some(v);
                }
            }
        }

        if pager.cmd_buf.run_cmd {
            let args = pager.cmd_buf.buf_cmd2.clone();
            pager.cmd_buf.run_cmd = false;
            pager.cmd_buf.buf_cmd2 = String::new();

            let command = commands.find(&args);
            let result = handle_command(
                engine_state,
                stack,
                pager,
                &mut view,
                &mut view_stack,
                command,
                &args,
            );

            match result {
                Ok(false) => {}
                Ok(true) => break Ok(try_to_peek_value(pager, view.as_mut().map(|p| &mut p.view))),
                Err(err) => info.report = Some(Report::error(err)),
            }
        }
    }
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
                    Transition::Ok => Ok(false),
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
                Ok(new_view) => {
                    if let Some(view) = view.take() {
                        if !view.is_light {
                            view_stack.push(view);
                        }
                    }

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
        // todo: deal with a situation where we exeed the bar width
        let next_pos = (pager.cmd_buf.buf_cmd2.len() + 1) as u16;
        // 1 skips a ':' char
        if next_pos < area.width {
            f.set_cursor(next_pos as u16, area.height - 1);
        }
    } else if pager.search_buf.is_search_input {
        // todo: deal with a situation where we exeed the bar width
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
    if pager.table_cfg.peek_value {
        view.and_then(|v| v.exit())
    } else {
        None
    }
}

fn render_status_bar(f: &mut Frame, area: Rect, report: Report, theme: &StyleConfig) {
    let msg_style = report_msg_style(&report, theme, theme.status_bar);
    let status_bar = StatusBar::new(report, theme.status_bar, msg_style);
    f.render_widget(status_bar, area);
}

fn report_msg_style(report: &Report, theme: &StyleConfig, style: NuStyle) -> NuStyle {
    if matches!(report.level, Severentity::Info) {
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
        let style = report_msg_style(&report, theme, theme.cmd_bar);
        f.render_widget(CmdBar::new(&report.message, &report.context, style), area);
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

        f.render_widget(CmdBar::new(&message, "", style), area);
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

    f.render_widget(CmdBar::new(&text, &info, theme.cmd_bar), area);
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
    f.render_widget(CmdBar::new(&text, "", theme.cmd_bar), area);
}

fn highlight_search_results(f: &mut Frame, pager: &Pager, layout: &Layout, style: NuStyle) {
    if pager.search_buf.search_results.is_empty() {
        return;
    }

    let hightlight_block = Block::default().style(nu_style_to_tui(style));

    for e in &layout.data {
        if let Some(p) = e.text.find(&pager.search_buf.buf_cmd_input) {
            // if p > e.width as usize {
            //     // we probably need to handle it somehow
            //     break;
            // }

            // todo: might be not UTF-8 friendly
            let w = pager.search_buf.buf_cmd_input.len() as u16;
            let area = Rect::new(e.area.x + p as u16, e.area.y, w, 1);
            f.render_widget(hightlight_block.clone(), area);
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn handle_events<V>(
    engine_state: &EngineState,
    stack: &mut Stack,
    events: &UIEvents,
    layout: &Layout,
    info: &mut ViewInfo,
    search: &mut SearchBuf,
    command: &mut CommandBuf,
    mut view: Option<&mut V>,
) -> (bool, bool)
where
    V: View,
{
    let key = match events.next() {
        Ok(Some(key)) => key,
        _ => return (false, false),
    };

    if handle_exit_key_event(&key) {
        return (true, true);
    }

    if handle_general_key_events1(&key, search, command, view.as_deref_mut()) {
        return (false, false);
    }

    if let Some(view) = &mut view {
        let t = view.handle_input(engine_state, stack, layout, info, key);
        match t {
            Some(Transition::Exit) => return (true, false),
            Some(Transition::Cmd { .. }) => {
                // todo: handle it
                return (false, false);
            }
            Some(Transition::Ok) => return (false, false),
            None => {}
        }
    }

    // was not handled so we must check our default controlls

    handle_general_key_events2(&key, search, command, view, info);

    (false, false)
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

#[derive(Debug, Clone)]
pub struct Pager<'a> {
    cmd_buf: CommandBuf,
    search_buf: SearchBuf,
    table_cfg: TableConfig,
    view_cfg: ViewConfig<'a>,
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
    pub status_bar: NuStyle,
    pub cmd_bar: NuStyle,
    pub split_line: NuStyle,
    pub highlight: NuStyle,
    pub selected_cell: Option<NuStyle>,
    pub selected_column: Option<NuStyle>,
    pub selected_row: Option<NuStyle>,
    pub show_cursow: bool,
    pub split_lines: TableSplitLines,
}

#[derive(Debug, Default, Clone)]
pub struct TableSplitLines {
    pub header_top: bool,
    pub header_bottom: bool,
    pub shift_line: bool,
    pub index_line: bool,
}

impl<'a> Pager<'a> {
    pub fn new(table_cfg: TableConfig, view_cfg: ViewConfig<'a>) -> Self {
        Self {
            cmd_buf: CommandBuf::default(),
            search_buf: SearchBuf::default(),
            table_cfg,
            view_cfg,
        }
    }

    pub fn run(
        &mut self,
        engine_state: &EngineState,
        stack: &mut Stack,
        ctrlc: CtrlC,
        view: Option<Page>,
        commands: CommandList,
    ) -> Result<Option<Value>> {
        run_pager(engine_state, stack, ctrlc, self, view, commands)
    }
}

struct StatusBar {
    report: Report,
    style: NuStyle,
    message_style: NuStyle,
}

impl StatusBar {
    fn new(report: Report, style: NuStyle, message_style: NuStyle) -> Self {
        Self {
            report,
            style,
            message_style,
        }
    }
}

impl Widget for StatusBar {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let block_style = nu_style_to_tui(self.style);
        let text_style = nu_style_to_tui(self.style).add_modifier(Modifier::BOLD);
        let message_style = nu_style_to_tui(self.message_style).add_modifier(Modifier::BOLD);

        // colorize the line
        let block = Block::default()
            .borders(Borders::empty())
            .style(block_style);
        block.render(area, buf);

        if !self.report.message.is_empty() {
            let width = area.width.saturating_sub(3 + 12 + 12 + 12);
            let name = nu_table::string_truncate(&self.report.message, width as usize);
            let span = Span::styled(name, message_style);
            buf.set_span(area.left(), area.y, &span, width);
        }

        if !self.report.context2.is_empty() {
            let span = Span::styled(&self.report.context2, text_style);
            let span_w = self.report.context2.len() as u16;
            let span_x = area.right().saturating_sub(3 + 12 + span_w);
            buf.set_span(span_x, area.y, &span, span_w);
        }

        if !self.report.context.is_empty() {
            let span = Span::styled(&self.report.context, text_style);
            let span_w = self.report.context.len() as u16;
            let span_x = area.right().saturating_sub(span_w);
            buf.set_span(span_x, area.y, &span, span_w);
        }
    }
}

fn report_level_style(level: Severentity, theme: &StyleConfig) -> NuStyle {
    match level {
        Severentity::Info => theme.status_info,
        Severentity::Warn => theme.status_warn,
        Severentity::Err => theme.status_error,
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

pub fn nu_style_to_tui(style: NuStyle) -> tui::style::Style {
    let mut out = tui::style::Style::default();
    if let Some(clr) = style.background {
        out.bg = nu_ansi_color_to_tui_color(clr);
    }

    if let Some(clr) = style.foreground {
        out.fg = nu_ansi_color_to_tui_color(clr);
    }

    if style.is_blink {
        out.add_modifier |= Modifier::SLOW_BLINK;
    }

    if style.is_bold {
        out.add_modifier |= Modifier::BOLD;
    }

    if style.is_dimmed {
        out.add_modifier |= Modifier::DIM;
    }

    if style.is_hidden {
        out.add_modifier |= Modifier::HIDDEN;
    }

    if style.is_italic {
        out.add_modifier |= Modifier::ITALIC;
    }

    if style.is_reverse {
        out.add_modifier |= Modifier::REVERSED;
    }

    if style.is_underline {
        out.add_modifier |= Modifier::UNDERLINED;
    }

    out
}

#[derive(Debug, Default, Clone)]
pub struct ViewInfo {
    pub cursor: Option<Position>,
    pub status: Option<Report>,
    pub report: Option<Report>,
}

#[derive(Debug, Clone)]
pub struct Report {
    pub message: String,
    pub level: Severentity,
    pub context: String,
    pub context2: String,
}

impl Report {
    pub fn new(message: String, level: Severentity, context: String, context2: String) -> Self {
        Self {
            message,
            level,
            context,
            context2,
        }
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self::new(
            message.into(),
            Severentity::Err,
            String::new(),
            String::new(),
        )
    }
}

impl Default for Report {
    fn default() -> Self {
        Self::new(
            String::new(),
            Severentity::Info,
            String::new(),
            String::new(),
        )
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Severentity {
    Info,
    #[allow(dead_code)]
    Warn,
    Err,
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

pub fn text_style_to_tui_style(style: TextStyle) -> tui::style::Style {
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

pub fn nu_ansi_color_to_tui_color(clr: NuColor) -> Option<tui::style::Color> {
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
        LightGray => Color::Gray,
        LightPurple => Color::LightMagenta,
        Purple => Color::Magenta,
        Default => return None,
    };

    Some(clr)
}

pub fn make_styled_string(
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
