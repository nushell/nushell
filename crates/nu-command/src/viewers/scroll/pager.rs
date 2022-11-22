use std::{
    borrow::Cow,
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
use nu_cli::eval_source2;
use nu_color_config::style_primitive;
use nu_protocol::{
    engine::{EngineState, Stack},
    Config as NuConfig, PipelineData, ShellError, Span as NuSpan, Value,
};
use nu_table::{string_width, Alignment, TextStyle};
use reedline::KeyModifiers;
use tui::{
    backend::{Backend, CrosstermBackend},
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::Span,
    widgets::{Block, Borders, Paragraph, StatefulWidget, Widget},
    Frame, Terminal,
};

use super::collect_pipeline;

type NuText = (String, TextStyle);

type CtrlC = Option<Arc<AtomicBool>>;

type NuStyleTable = HashMap<String, NuStyle>;

pub trait View {
    type State;

    fn draw<B>(
        &self,
        f: &mut Frame<B>,
        area: Rect,
        cfg: &ViewConfig,
        layout: &mut Layout<Self::State>,
    ) where
        B: Backend;

    fn handle_input(
        &mut self,
        layout: &Layout<Self::State>,
        info: &mut ViewInfo,
        key: KeyEvent,
    ) -> Option<Transition>;

    fn show_data(&mut self, _: usize) -> bool {
        false
    }

    fn collect_data(&self) -> Vec<NuText> {
        Vec::new()
    }

    fn exit(&mut self) -> Option<Value> {
        None
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

#[derive(Debug, Clone)]
struct RecordView<'a> {
    layer_stack: Vec<RecordLayer<'a>>,
    mode: UIMode,
    cfg: TableConfig,
    cursor: Position,
}

#[derive(Debug, Clone)]
pub struct TableTheme {
    pub splitline: NuStyle,
}

#[derive(Debug, Default)]
struct RecordViewState {
    count_rows: usize,
    count_columns: usize,
    data_index: HashMap<(usize, usize), ElementInfo>,
}

impl<'a> RecordView<'a> {
    fn new(
        columns: impl Into<Cow<'a, [String]>>,
        records: impl Into<Cow<'a, [Vec<Value>]>>,
        table_cfg: TableConfig,
    ) -> Self {
        Self {
            layer_stack: vec![RecordLayer::new(columns, records)],
            mode: UIMode::View,
            cursor: Position::new(0, 0),
            cfg: table_cfg,
        }
    }

    // todo: rename to get_layer
    fn get_layer_last(&self) -> &RecordLayer<'a> {
        self.layer_stack.last().unwrap()
    }

    fn get_layer_last_mut(&mut self) -> &mut RecordLayer<'a> {
        self.layer_stack.last_mut().unwrap()
    }

    fn create_tablew<'b>(&self, layer: &'b RecordLayer, view_cfg: &'b ViewConfig) -> TableW<'b> {
        let data = convert_records_to_string(&layer.records, view_cfg.config, view_cfg.color_hm);

        TableW::new(
            layer.columns.as_ref(),
            data,
            self.cfg.show_index,
            self.cfg.show_head,
            view_cfg.theme.split_line,
            view_cfg.color_hm,
            layer.index_row,
            layer.index_column,
        )
    }
}

impl View for RecordView<'_> {
    type State = RecordViewState;

    fn draw<B>(
        &self,
        f: &mut Frame<B>,
        area: Rect,
        cfg: &ViewConfig,
        layout: &mut Layout<Self::State>,
    ) where
        B: Backend,
    {
        let layer = self.get_layer_last();
        let table = self.create_tablew(layer, cfg);

        let mut table_layout = Layout::default();
        f.render_stateful_widget(table, area, &mut table_layout);

        layout.data = table_layout.data;
        layout.state = RecordViewState {
            count_rows: table_layout.state.count_rows,
            count_columns: table_layout.state.count_columns,
            data_index: table_layout.state.data_index,
        };

        if self.mode == UIMode::Cursor {
            let cursor = get_cursor(self, layout);
            highlight_cell(f, layout, area, cursor, cfg.theme);
        }
    }

    fn handle_input(
        &mut self,
        layout: &Layout<Self::State>,
        info: &mut ViewInfo,
        key: KeyEvent,
    ) -> Option<Transition> {
        let result = match self.mode {
            UIMode::View => handle_key_event_view_mode(self, layout, &key),
            UIMode::Cursor => {
                // we handle a situation where we got resized and the old cursor is no longer valid
                self.cursor = get_cursor(self, layout);

                handle_key_event_cursor_mode(self, layout, &key)
            }
        };

        if matches!(&result, Some(Transition::Ok) | Some(Transition::Cmd(..))) {
            // update status bar
            let report =
                create_records_report(self.get_layer_last(), self.mode, self.cursor, layout);
            info.status = Some(report);
        }

        result
    }

    fn collect_data(&self) -> Vec<NuText> {
        let data = convert_records_to_string(
            &self.get_layer_last().records,
            &NuConfig::default(),
            &HashMap::default(),
        );

        data.iter().flatten().cloned().collect()
    }

    fn show_data(&mut self, pos: usize) -> bool {
        let data = &self.get_layer_last().records;

        let mut i = 0;
        for (row, cells) in data.iter().enumerate() {
            if pos > i + cells.len() {
                i += cells.len();
                continue;
            }

            for (column, _) in cells.iter().enumerate() {
                if i == pos {
                    let layer = self.get_layer_last_mut();
                    layer.index_column = column;
                    layer.index_row = row;

                    return true;
                }

                i += 1;
            }
        }

        false
    }

    fn exit(&mut self) -> Option<Value> {
        Some(build_last_value(self))
    }
}

fn create_records_report(
    layer: &RecordLayer,
    mode: UIMode,
    cursor: Position,
    layout: &Layout<RecordViewState>,
) -> Report {
    let seen_rows = layer.index_row + layout.state.count_rows;
    let seen_rows = min(seen_rows, layer.count_rows());
    let percent_rows = get_percentage(seen_rows, layer.count_rows());
    let covered_percent = match percent_rows {
        100 => String::from("All"),
        _ if layer.index_row == 0 => String::from("Top"),
        value => format!("{}%", value),
    };
    let title = if let Some(name) = &layer.name {
        name.clone()
    } else {
        String::new()
    };
    let cursor = {
        if mode == UIMode::Cursor {
            let row = layer.index_row + cursor.y as usize;
            let column = layer.index_column + cursor.x as usize;
            format!("{},{}", row, column)
        } else {
            format!("{},{}", layer.index_row, layer.index_column)
        }
    };

    Report {
        message: title,
        context: covered_percent,
        context2: cursor,
        level: Severentity::Info,
    }
}

fn build_last_value(v: &RecordView) -> Value {
    if v.mode == UIMode::Cursor {
        peak_current_value(v)
    } else if v.get_layer_last().count_rows() < 2 {
        build_table_as_record(v)
    } else {
        build_table_as_list(v)
    }
}

fn peak_current_value(v: &RecordView) -> Value {
    let layer = v.get_layer_last();
    let Position { x: column, y: row } = v.cursor;
    let row = row as usize + layer.index_row;
    let column = column as usize + layer.index_column;
    let value = &layer.records[row][column];
    value.clone()
}

fn build_table_as_list(v: &RecordView) -> Value {
    let layer = v.get_layer_last();

    let headers = layer.columns.to_vec();
    let vals = layer
        .records
        .iter()
        .cloned()
        .map(|vals| Value::Record {
            cols: headers.clone(),
            vals,
            span: NuSpan::unknown(),
        })
        .collect();

    Value::List {
        vals,
        span: NuSpan::unknown(),
    }
}

fn build_table_as_record(v: &RecordView) -> Value {
    let layer = v.get_layer_last();

    let cols = layer.columns.to_vec();
    let vals = layer.records.get(0).map_or(Vec::new(), |row| row.clone());

    Value::Record {
        cols,
        vals,
        span: NuSpan::unknown(),
    }
}

#[derive(Debug, Clone)]
struct RecordLayer<'a> {
    columns: Cow<'a, [String]>,
    records: Cow<'a, [Vec<Value>]>,
    index_row: usize,
    index_column: usize,
    name: Option<String>,
}

impl<'a> RecordLayer<'a> {
    fn new(
        columns: impl Into<Cow<'a, [String]>>,
        records: impl Into<Cow<'a, [Vec<Value>]>>,
    ) -> Self {
        Self {
            columns: columns.into(),
            records: records.into(),
            index_row: 0,
            index_column: 0,
            name: None,
        }
    }

    fn set_name(&mut self, name: impl Into<String>) {
        self.name = Some(name.into());
    }

    fn count_rows(&self) -> usize {
        self.records.len()
    }

    fn count_columns(&self) -> usize {
        self.columns.len()
    }

    fn get_current_value(&self, Position { x, y }: Position) -> Value {
        let current_row = y as usize + self.index_row;
        let current_column = x as usize + self.index_column;

        let row = self.records[current_row].clone();
        row[current_column].clone()
    }

    fn get_current_header(&self, Position { x, .. }: Position) -> Option<String> {
        let col = x as usize + self.index_column;

        self.columns.get(col).map(|header| header.to_string())
    }
}

fn convert_records_to_string(
    records: &[Vec<Value>],
    cfg: &NuConfig,
    color_hm: &NuStyleTable,
) -> Vec<Vec<NuText>> {
    records
        .iter()
        .map(|row| {
            row.iter()
                .map(|value| {
                    let text = value.clone().into_abbreviated_string(cfg);
                    let tp = value.get_type().to_string();
                    let float_precision = cfg.float_precision as usize;

                    make_styled_string(text, &tp, 0, false, color_hm, float_precision)
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>()
}

fn handle_key_event_view_mode(
    view: &mut RecordView,
    layout: &Layout<RecordViewState>,
    key: &KeyEvent,
) -> Option<Transition> {
    match key.code {
        KeyCode::Esc => {
            if view.layer_stack.len() > 1 {
                view.layer_stack.pop();
            }

            Some(Transition::Ok)
        }
        KeyCode::Char('i') => {
            view.mode = UIMode::Cursor;
            view.cursor = Position::default();

            Some(Transition::Ok)
        }
        KeyCode::Up => {
            let layer = view.get_layer_last_mut();
            layer.index_row = layer.index_row.saturating_sub(1);

            Some(Transition::Ok)
        }
        KeyCode::Down => {
            let layer = view.get_layer_last_mut();
            let max_index = layer.count_rows().saturating_sub(1);
            layer.index_row = min(layer.index_row + 1, max_index);

            Some(Transition::Ok)
        }
        KeyCode::Left => {
            let layer = view.get_layer_last_mut();
            layer.index_column = layer.index_column.saturating_sub(1);

            Some(Transition::Ok)
        }
        KeyCode::Right => {
            let layer = view.get_layer_last_mut();
            let max_index = layer.count_columns().saturating_sub(1);
            layer.index_column = min(layer.index_column + 1, max_index);

            Some(Transition::Ok)
        }
        KeyCode::PageUp => {
            let layer = view.get_layer_last_mut();
            let count_rows = layout.state.count_rows;
            layer.index_row = layer.index_row.saturating_sub(count_rows as usize);

            Some(Transition::Ok)
        }
        KeyCode::PageDown => {
            let layer = view.get_layer_last_mut();
            let count_rows = layout.state.count_rows;
            let max_index = layer.count_rows().saturating_sub(1);
            layer.index_row = min(layer.index_row + count_rows as usize, max_index);

            Some(Transition::Ok)
        }
        _ => None,
    }
}

fn handle_key_event_cursor_mode(
    view: &mut RecordView,
    layout: &Layout<RecordViewState>,
    key: &KeyEvent,
) -> Option<Transition> {
    match key.code {
        KeyCode::Esc => {
            view.mode = UIMode::View;
            view.cursor = Position::default();

            Some(Transition::Ok)
        }
        KeyCode::Up => {
            if view.cursor.y == 0 {
                let layer = view.get_layer_last_mut();
                layer.index_row = layer.index_row.saturating_sub(1);
            } else {
                view.cursor.y -= 1
            }

            Some(Transition::Ok)
        }
        KeyCode::Down => {
            let cursor = view.cursor;
            let layer = view.get_layer_last_mut();

            let showed_rows = layout.state.count_rows;
            let total_rows = layer.count_rows();
            let row_index = layer.index_row + cursor.y as usize + 1;

            if row_index < total_rows {
                if cursor.y as usize + 1 == showed_rows {
                    layer.index_row += 1;
                } else {
                    view.cursor.y += 1;
                }
            }

            Some(Transition::Ok)
        }
        KeyCode::Left => {
            let cursor = view.cursor;
            let layer = view.get_layer_last_mut();

            if cursor.x == 0 {
                layer.index_column = layer.index_column.saturating_sub(1);
            } else {
                view.cursor.x -= 1
            }

            Some(Transition::Ok)
        }
        KeyCode::Right => {
            let cursor = view.cursor;
            let layer = view.get_layer_last_mut();

            let showed_columns = layout.state.count_columns;
            let total_columns = layer.count_columns();
            let column_index = layer.index_column + cursor.x as usize + 1;

            if column_index < total_columns {
                if cursor.x as usize + 1 == showed_columns {
                    layer.index_column += 1;
                } else {
                    view.cursor.x += 1;
                }
            }

            Some(Transition::Ok)
        }
        KeyCode::Enter => {
            push_current_value_to_layer(view);
            Some(Transition::Ok)
        }
        _ => None,
    }
}

fn push_current_value_to_layer(view: &mut RecordView) {
    let layer = view.get_layer_last();

    let value = layer.get_current_value(view.cursor);
    let header = layer.get_current_header(view.cursor);

    let (columns, values) = super::collect_input(value);

    let mut next_layer = RecordLayer::new(columns, values);
    if let Some(header) = header {
        next_layer.set_name(header);
    }

    view.layer_stack.push(next_layer);

    view.mode = UIMode::View;
    view.cursor = Position::default();
}

fn push_values_to_layer(view: &mut RecordView, columns: Vec<String>, values: Vec<Vec<Value>>) {
    let next_layer = RecordLayer::new(columns, values);
    view.layer_stack.push(next_layer);
    view.mode = UIMode::View;
    view.cursor = Position::default();
}

#[derive(Debug, Default, Clone)]
pub struct TableConfig {
    pub(crate) show_index: bool,
    pub(crate) show_head: bool,
    pub(crate) reverse: bool,
    pub(crate) peek_value: bool,
}

pub fn run_pager(
    pager: &mut Pager,
    engine_state: &EngineState,
    stack: &mut Stack,
    ctrlc: CtrlC,
) -> Result<Option<Value>> {
    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, Clear(ClearType::All))?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // todo: find a better place for it
    if pager.table_cfg.reverse {
        if let Some(view) = &mut pager.records_view {
            if let Ok(size) = terminal.size() {
                let size = estimate_page_size(size, pager.table_cfg.show_head);
                state_reverse_data(view, size as usize);
            }
        }
    }

    let result = render_ui(&mut terminal, ctrlc, engine_state, stack, pager);

    // restore terminal
    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen)?;

    result
}

fn render_ui<B>(
    terminal: &mut Terminal<B>,
    ctrlc: CtrlC,
    engine_state: &EngineState,
    stack: &mut Stack,
    pager: &mut Pager<'_>,
) -> Result<Option<Value>>
where
    B: Backend,
{
    let events = UIEvents::new();

    let mut info = ViewInfo {
        status: Some(Report::default()),
        ..Default::default()
    };

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
            terminal.draw(|f| {
                let area = f.size();

                // todo: delete it?
                // f.render_widget(tui::widgets::Clear, area);

                // if let Some(view) = &mut command_view {}

                if let Some(view) = &mut pager.records_view {
                    let available_area =
                        Rect::new(area.x, area.y, area.width, area.height.saturating_sub(2));
                    view.draw(f, available_area, &pager.view_cfg, &mut layout);
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

                {
                    // set a cursor to cmd bar
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
            })?;
        }

        let exited = handle_events(
            &events,
            &layout,
            &mut info,
            &mut pager.search_buf,
            &mut pager.cmd_buf,
            pager.records_view.as_mut(),
        );
        if exited {
            let val = if pager.table_cfg.peek_value {
                pager.records_view.as_mut().and_then(|v| v.exit())
            } else {
                None
            };

            break Ok(val);
        }

        if pager.cmd_buf.run_cmd {
            let cmd = pager.cmd_buf.buf_cmd2.clone();
            pager.cmd_buf.run_cmd = false;
            pager.cmd_buf.buf_cmd2 = String::new();

            run_command(engine_state, stack, pager, &mut info, &cmd);
        }
    }
}

fn run_command(
    engine_state: &EngineState,
    stack: &mut Stack,
    pager: &mut Pager,
    info: &mut ViewInfo,
    cmd: &str,
) -> bool {
    match cmd {
        _ if cmd.starts_with("nu") => {
            let cmd = cmd.strip_prefix("nu").unwrap();

            let value = if let Some(view) = &pager.records_view {
                build_last_value(view)
            } else {
                Value::default()
            };

            let pipeline = PipelineData::Value(value, None);

            let pipeline = run_nu_command(engine_state, stack, cmd, pipeline);

            #[allow(clippy::single_match)]
            match pipeline {
                Ok(pipeline_data) => {
                    let (columns, values) = collect_pipeline(pipeline_data);

                    match &mut pager.records_view {
                        Some(view) => {
                            push_values_to_layer(view, columns, values);
                        }
                        None => {
                            pager.set_records(columns, values);
                        }
                    }
                }
                Err(err) => {
                    info.report = Some(Report::new(
                        format!("Error: {}", err),
                        Severentity::Err,
                        String::new(),
                        String::new(),
                    ))
                }
            }
        }
        "help" => {
            let (headers, data) = help_frame_data();
            match &mut pager.records_view {
                Some(view) => {
                    push_values_to_layer(view, headers, data);
                }
                None => {
                    pager.set_records(headers, data);
                }
            }
        }
        "q" => return true,
        command => {
            info.report = Some(Report::new(
                format!("Error: A command {:?} was not recognized", command),
                Severentity::Err,
                String::new(),
                String::new(),
            ));
        }
    }

    false
}

fn help_frame_data() -> (Vec<String>, Vec<Vec<Value>>) {
    macro_rules! null {
        () => {
            Value::Nothing {
                span: NuSpan::unknown(),
            }
        };
    }

    macro_rules! nu_str {
        ($text:expr) => {
            Value::String {
                val: $text.to_string(),
                span: NuSpan::unknown(),
            }
        };
    }

    let commands_headers = [String::from("name"), String::from("description")];

    #[rustfmt::skip]
    let supported_commands = [
        ("nu",   "Run a custom `nu` command with showed table as an input"),
        ("help", "Print a help menu")
    ];

    let commands = Value::List {
        vals: supported_commands
            .iter()
            .map(|(name, description)| Value::Record {
                cols: commands_headers.to_vec(),
                vals: vec![nu_str!(name), nu_str!(description)],
                span: NuSpan::unknown(),
            })
            .collect(),
        span: NuSpan::unknown(),
    };

    let headers = vec!["name", "mode", "information", "description"];

    #[rustfmt::skip]
    let shortcuts = [
        ("i",      "view",    null!(),   "Turn on a cursor mode so you can inspect values"),
        (":",      "view",    commands,  "Run a command"),
        ("/",      "view",    null!(),   "Search via pattern"),
        ("?",      "view",    null!(),   "Search via pattern but results will be reversed when you press <n>"),
        ("n",      "view",    null!(),   "Gets to the next found element in search"),
        ("Up",     "",        null!(),   "Moves to an element above"),
        ("Down",   "",        null!(),   "Moves to an element bellow"),
        ("Left",   "",        null!(),   "Moves to an element to the left"),
        ("Right",  "",        null!(),   "Moves to an element to the right"),
        ("PgDown", "view",    null!(),   "Moves to an a bunch of elements bellow"),
        ("PgUp",   "view",    null!(),   "Moves to an a bunch of elements above"),
        ("Esc",    "cursor",  null!(),   "Exits a cursor mode. Exists an expected element."),
        ("Enter",  "cursor",  null!(),   "Inspect a chosen element"),
    ];

    let headers = headers.iter().map(|s| s.to_string()).collect();
    let data = shortcuts
        .iter()
        .map(|(name, mode, info, desc)| {
            vec![nu_str!(name), nu_str!(mode), info.clone(), nu_str!(desc)]
        })
        .collect();

    (headers, data)
}

fn run_nu_command(
    engine_state: &EngineState,
    stack: &mut Stack,
    cmd: &str,
    current: PipelineData,
) -> std::result::Result<PipelineData, ShellError> {
    let mut engine_state = engine_state.clone();
    eval_source2(&mut engine_state, stack, cmd.as_bytes(), "", current)
}

fn render_status_bar<B>(f: &mut Frame<B>, area: Rect, report: Report, theme: &StyleConfig)
where
    B: Backend,
{
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

fn render_cmd_bar<B>(
    f: &mut Frame<B>,
    area: Rect,
    pager: &Pager,
    report: Option<Report>,
    theme: &StyleConfig,
) where
    B: Backend,
{
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

fn render_cmd_bar_search<B>(f: &mut Frame<B>, area: Rect, pager: &Pager<'_>, theme: &StyleConfig)
where
    B: Backend,
{
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

fn render_cmd_bar_cmd<B>(f: &mut Frame<B>, area: Rect, pager: &Pager, theme: &StyleConfig)
where
    B: Backend,
{
    let prefix = ':';
    let text = format!("{}{}", prefix, pager.cmd_buf.buf_cmd2);
    f.render_widget(CmdBar::new(&text, "", theme.cmd_bar), area);
}

fn highlight_search_results<B, S>(
    f: &mut Frame<B>,
    pager: &Pager,
    layout: &Layout<S>,
    style: NuStyle,
) where
    B: Backend,
{
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

fn highlight_cell<B>(
    f: &mut Frame<B>,
    layout: &Layout<RecordViewState>,
    area: Rect,
    cursor: Position,
    theme: &StyleConfig,
) where
    B: Backend,
{
    let Position { x: column, y: row } = cursor;

    let info = layout
        .state
        .data_index
        .get(&(row as usize, column as usize));

    if let Some(info) = info {
        if let Some(style) = theme.selected_column {
            let hightlight_block = Block::default().style(nu_style_to_tui(style));
            let area = Rect::new(info.area.x, area.y, info.area.width, area.height);
            f.render_widget(hightlight_block.clone(), area);
        }

        if let Some(style) = theme.selected_row {
            let hightlight_block = Block::default().style(nu_style_to_tui(style));
            let area = Rect::new(area.x, info.area.y, area.width, 1);
            f.render_widget(hightlight_block.clone(), area);
        }

        if let Some(style) = theme.selected_cell {
            let hightlight_block = Block::default().style(nu_style_to_tui(style));
            let area = Rect::new(info.area.x, info.area.y, info.area.width, 1);
            f.render_widget(hightlight_block.clone(), area);
        }

        if theme.show_cursow {
            f.set_cursor(info.area.x, info.area.y);
        }
    }
}

fn get_cursor(state: &RecordView<'_>, layout: &Layout<RecordViewState>) -> Position {
    let count_rows = layout.state.count_rows as u16;
    let count_columns = layout.state.count_columns as u16;

    let mut cursor = state.cursor;
    cursor.y = min(cursor.y, count_rows.saturating_sub(1) as u16);
    cursor.x = min(cursor.x, count_columns.saturating_sub(1) as u16);

    cursor
}

fn handle_events<V>(
    events: &UIEvents,
    layout: &Layout<V::State>,
    info: &mut ViewInfo,
    search: &mut SearchBuf,
    command: &mut CommandBuf,
    mut view: Option<&mut V>,
) -> bool
where
    V: View,
{
    let key = match events.next() {
        Ok(Some(key)) => key,
        _ => return false,
    };

    if handle_exit_key_event(&key) {
        return true;
    }

    if handle_general_key_events1(&key, search, command, view.as_deref_mut()) {
        return false;
    }

    if let Some(view) = &mut view {
        let t = view.handle_input(layout, info, key);
        match t {
            Some(Transition::Exit) => return true,
            Some(Transition::Cmd(..)) => {
                // todo: handle it
                return false;
            }
            Some(Transition::Ok) => return false,
            None => {}
        }
    }

    // was not handled so we must check our default controlls

    handle_general_key_events2(&key, search, command, view, info);

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
    records_view: Option<RecordView<'a>>,
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
}

impl<'a> Pager<'a> {
    pub fn new(table_cfg: TableConfig, view_cfg: ViewConfig<'a>) -> Self {
        Self {
            records_view: None,
            cmd_buf: CommandBuf::default(),
            search_buf: SearchBuf::default(),
            table_cfg,
            view_cfg,
        }
    }

    pub fn set_records(
        &mut self,
        columns: impl Into<Cow<'a, [String]>>,
        records: impl Into<Cow<'a, [Vec<Value>]>>,
    ) {
        let view = RecordView::new(columns, records, self.table_cfg.clone());
        self.records_view = Some(view);
    }

    pub fn run(
        &mut self,
        engine_state: &EngineState,
        stack: &mut Stack,
        ctrlc: CtrlC,
    ) -> Result<Option<Value>> {
        run_pager(self, engine_state, stack, ctrlc)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum UIMode {
    Cursor,
    View,
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

struct TableW<'a> {
    columns: Cow<'a, [String]>,
    data: Cow<'a, [Vec<NuText>]>,
    show_index: bool,
    show_header: bool,
    index_row: usize,
    index_column: usize,
    splitline_style: NuStyle,
    color_hm: &'a NuStyleTable,
}

impl<'a> TableW<'a> {
    #[allow(clippy::too_many_arguments)]
    fn new(
        columns: impl Into<Cow<'a, [String]>>,
        data: impl Into<Cow<'a, [Vec<NuText>]>>,
        show_index: bool,
        show_header: bool,
        splitline_style: NuStyle,
        color_hm: &'a NuStyleTable,
        index_row: usize,
        index_column: usize,
    ) -> Self {
        Self {
            columns: columns.into(),
            data: data.into(),
            color_hm,
            show_index,
            show_header,
            splitline_style,
            index_row,
            index_column,
        }
    }
}

#[derive(Debug, Default)]
struct TableWState {
    count_rows: usize,
    count_columns: usize,
    data_index: HashMap<(usize, usize), ElementInfo>,
}

impl StatefulWidget for TableW<'_> {
    type State = Layout<TableWState>;

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

        let mut data_y = area.y;
        if show_head {
            data_y += 3;
        }

        let head_y = area.y + 1;

        if area.width == 0 || area.height == 0 {
            return;
        }

        let mut data_height = area.height;
        if show_head {
            data_height -= 3;
        }

        let mut width = area.x;

        let mut data = &self.data[self.index_row..];
        if data.len() > data_height as usize {
            data = &data[..data_height as usize];
        }

        // header lines
        if show_head {
            render_header_borders(buf, area, 0, 1);
        }

        if show_index {
            let area = Rect::new(width, data_y, area.width, data_height);
            width += render_index(buf, area, self.color_hm, self.index_row);
            width += render_vertical(
                buf,
                width,
                data_y,
                data_height,
                show_head,
                self.splitline_style,
            );
        }

        let mut do_render_split_line = true;
        let mut do_render_shift_column = false;

        state.state.count_rows = data.len();
        state.state.count_columns = 0;

        for (i, col) in (self.index_column..self.columns.len()).enumerate() {
            let mut head = String::from(&self.columns[col]);

            let mut column = create_column(data, col);

            let column_width = calculate_column_width(&column);
            let mut use_space = column_width as u16;

            if show_head {
                let head_width = string_width(&head);
                use_space = max(head_width as u16, use_space);
            }

            {
                let available_space = area.width - width;
                let head = show_head.then(|| &mut head);
                let control = truncate_column(
                    &mut column,
                    head,
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
                let header = &[head_row_text(&head, self.color_hm)];

                let mut w = width;
                w += render_space(buf, w, head_y, 1, CELL_PADDING_LEFT);
                w += render_column(buf, w, head_y, use_space, header);
                render_space(buf, w, head_y, 1, CELL_PADDING_RIGHT);

                let x = w - CELL_PADDING_RIGHT - use_space;
                state.push(&header[0].0, x, head_y, use_space, 1);

                // it would be nice to add it so it would be available on search
                // state.state.data_index.insert((i, col), ElementInfo::new(text, x, data_y, use_space, 1));
            }

            width += render_space(buf, width, data_y, data_height, CELL_PADDING_LEFT);
            width += render_column(buf, width, data_y, use_space, &column);
            width += render_space(buf, width, data_y, data_height, CELL_PADDING_RIGHT);

            for (row, (text, _)) in column.iter().enumerate() {
                let x = width - CELL_PADDING_RIGHT - use_space;
                let y = data_y + row as u16;
                state.push(text, x, y, use_space, 1);

                let e = ElementInfo::new(text, x, y, use_space, 1);
                state.state.data_index.insert((row, i), e);
            }

            state.state.count_columns += 1;

            if do_render_shift_column {
                break;
            }
        }

        if do_render_shift_column {
            // we actually want to show a shift only in header.
            //
            // render_shift_column(buf, used_width, head_offset, available_height);

            if show_head {
                width += render_space(buf, width, data_y, data_height, CELL_PADDING_LEFT);
                width += render_shift_column(buf, width, head_y, 1, self.splitline_style);
                width += render_space(buf, width, data_y, data_height, CELL_PADDING_RIGHT);
            }
        }

        if do_render_split_line {
            width += render_vertical(
                buf,
                width,
                data_y,
                data_height,
                show_head,
                self.splitline_style,
            );
        }

        // we try out best to cleanup the rest of the space cause it could be meassed.
        let rest = area.width.saturating_sub(width);
        if rest > 0 {
            render_space(buf, width, data_y, data_height, rest);
            if show_head {
                render_space(buf, width, head_y, 1, rest);
            }
        }
    }
}

struct IndexColumn<'a> {
    color_hm: &'a NuStyleTable,
    start: usize,
}

impl<'a> IndexColumn<'a> {
    fn new(color_hm: &'a NuStyleTable, start: usize) -> Self {
        Self { color_hm, start }
    }

    fn estimate_width(&self, height: u16) -> usize {
        let last_row = self.start + height as usize;
        last_row.to_string().len()
    }
}

impl Widget for IndexColumn<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let style = nu_style_to_tui(self.color_hm["row_index"]);

        for row in 0..area.height {
            let i = 1 + row as usize + self.start;
            let text = i.to_string();

            let p = Paragraph::new(text)
                .style(style)
                .alignment(tui::layout::Alignment::Right);
            let area = Rect::new(area.x, area.y + row, area.width, 1);

            p.render(area, buf);
        }
    }
}

fn nu_style_to_tui(style: NuStyle) -> tui::style::Style {
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

fn render_index(buf: &mut Buffer, area: Rect, color_hm: &NuStyleTable, start_index: usize) -> u16 {
    const PADDING_LEFT: u16 = 2;
    const PADDING_RIGHT: u16 = 1;

    let mut width = render_space(buf, area.x, area.y, area.height, PADDING_LEFT);

    let index = IndexColumn::new(color_hm, start_index);
    let w = index.estimate_width(area.height) as u16;
    let area = Rect::new(area.x + width, area.y, w, area.height);

    index.render(area, buf);

    width += w;
    width += render_space(buf, area.x + width, area.y, area.height, PADDING_RIGHT);

    width
}

// todo: Change layout so it's not dependent on 2x2 grid structure
#[derive(Debug, Default)]
pub struct Layout<S> {
    data: Vec<ElementInfo>,
    state: S,
}

#[derive(Debug, Default, Clone)]
pub struct ViewInfo {
    #[allow(dead_code)]
    cursor: Option<Position>,
    status: Option<Report>,
    report: Option<Report>,
}

#[derive(Debug, Clone)]
struct Report {
    message: String,
    level: Severentity,
    context: String,
    context2: String,
}

impl Report {
    fn new(message: String, level: Severentity, context: String, context2: String) -> Self {
        Self {
            message,
            level,
            context,
            context2,
        }
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
enum Severentity {
    Info,
    #[allow(dead_code)]
    Warn,
    Err,
}

impl<S> Layout<S> {
    fn push(&mut self, text: &str, x: u16, y: u16, width: u16, height: u16) {
        self.data.push(ElementInfo::new(text, x, y, width, height));
    }
}

#[allow(dead_code)]
#[derive(Debug, Default, Clone)]
struct ElementInfo {
    // todo: make it a Cow
    text: String,
    area: Rect,
}

impl ElementInfo {
    fn new(text: impl Into<String>, x: u16, y: u16, width: u16, height: u16) -> Self {
        Self {
            text: text.into(),
            area: Rect::new(x, y, width, height),
        }
    }
}

#[derive(Debug, Default, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash)]
struct Position {
    x: u16,
    y: u16,
}

impl Position {
    fn new(x: u16, y: u16) -> Self {
        Self { x, y }
    }
}

fn state_reverse_data(state: &mut RecordView<'_>, page_size: usize) {
    let layer = state.get_layer_last_mut();
    let count_rows = layer.records.len();
    if count_rows > page_size as usize {
        layer.index_row = count_rows - page_size as usize;
    }
}

fn render_header_borders(buf: &mut Buffer, area: Rect, y: u16, span: u16) -> (u16, u16) {
    let block = Block::default()
        .borders(Borders::TOP | Borders::BOTTOM)
        .border_style(Style::default().fg(Color::Rgb(64, 64, 64)));
    let height = 2 + span;
    let area = Rect::new(area.x, area.y + y, area.width, height);
    block.render(area, buf);
    // y pos of header text and next line
    (height.saturating_sub(2), height)
}

fn create_column(data: &[Vec<NuText>], col: usize) -> Vec<NuText> {
    let mut column = vec![NuText::default(); data.len()];
    for (row, values) in data.iter().enumerate() {
        if values.is_empty() {
            debug_assert!(false, "must never happen?");
            continue;
        }

        let value = &values[col];
        column[row] = value.clone();
    }

    column
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

fn get_percentage(value: usize, max: usize) -> usize {
    debug_assert!(value <= max, "{:?} {:?}", value, max);

    ((value as f32 / max as f32) * 100.0).floor() as usize
}

fn estimate_page_size(area: Rect, show_head: bool) -> u16 {
    let mut available_height = area.height;
    available_height -= 3; // status_bar

    if show_head {
        available_height -= 3; // head
    }

    available_height
}

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

fn render_shift_column(buf: &mut Buffer, x: u16, y: u16, height: u16, style: NuStyle) -> u16 {
    let style = TextStyle {
        alignment: Alignment::Left,
        color_style: Some(style),
    };

    repeat_vertical(buf, x, y, 1, height, '…', style);

    1
}

fn render_vertical(
    buf: &mut Buffer,
    x: u16,
    y: u16,
    height: u16,
    show_header: bool,
    style: NuStyle,
) -> u16 {
    render_vertical_split(buf, x, y, height, style);

    if show_header && y > 0 {
        render_top_connector(buf, x, y - 1, style);
    }

    // render_bottom_connector(buf, x, height + y);

    1
}

fn render_vertical_split(buf: &mut Buffer, x: u16, y: u16, height: u16, style: NuStyle) {
    let style = TextStyle {
        alignment: Alignment::Left,
        color_style: Some(style),
    };

    repeat_vertical(buf, x, y, 1, height, '│', style);
}

fn render_top_connector(buf: &mut Buffer, x: u16, y: u16, style: NuStyle) {
    let style = nu_style_to_tui(style);
    let span = Span::styled("┬", style);
    buf.set_span(x, y, &span, 1);
}

fn render_space(buf: &mut Buffer, x: u16, y: u16, height: u16, padding: u16) -> u16 {
    repeat_vertical(buf, x, y, padding, height, ' ', TextStyle::default());
    padding
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
    x: u16,
    y: u16,
    available_width: u16,
    rows: &[NuText],
) -> u16 {
    for (row, (text, style)) in rows.iter().enumerate() {
        let text = strip_string(text);
        let style = text_style_to_tui_style(*style);
        let span = Span::styled(text, style);
        buf.set_span(x, y + row as u16, &span, available_width);
    }

    available_width
}

fn strip_string(text: &str) -> String {
    strip_ansi_escapes::strip(text)
        .ok()
        .and_then(|s| String::from_utf8(s).ok())
        .unwrap_or_else(|| text.to_owned())
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
