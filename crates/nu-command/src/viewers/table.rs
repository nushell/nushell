// todo: (refactoring) limit get_config() usage to 1 call
//        overall reduce the redundant calls to StyleComputer etc.
//        the goal is to configure it once...

use lscolors::{LsColors, Style};
use nu_color_config::{color_from_hex, StyleComputer, TextStyle};
use nu_engine::{command_prelude::*, env::get_config, env_to_string};
use nu_protocol::{Config, DataSource, ListStream, PipelineMetadata, RawStream, TableMode};
use nu_table::{
    common::create_nu_table_config, CollapsedTable, ExpandedTable, JustTable, NuTable, NuTableCell,
    StringResult, TableOpts, TableOutput,
};
use nu_utils::get_ls_colors;
use std::{
    collections::VecDeque, io::IsTerminal, path::PathBuf, str::FromStr, sync::atomic::AtomicBool,
    sync::Arc, time::Instant,
};
use terminal_size::{Height, Width};
use url::Url;

const STREAM_PAGE_SIZE: usize = 1000;

fn get_width_param(width_param: Option<i64>) -> usize {
    if let Some(col) = width_param {
        col as usize
    } else if let Some((Width(w), Height(_))) = terminal_size::terminal_size() {
        w as usize
    } else {
        80
    }
}

#[derive(Clone)]
pub struct Table;

//NOTE: this is not a real implementation :D. It's just a simple one to test with until we port the real one.
impl Command for Table {
    fn name(&self) -> &str {
        "table"
    }

    fn usage(&self) -> &str {
        "Render the table."
    }

    fn extra_usage(&self) -> &str {
        "If the table contains a column called 'index', this column is used as the table index instead of the usual continuous index."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["display", "render"]
    }

    fn signature(&self) -> Signature {
        Signature::build("table")
            .input_output_types(vec![(Type::Any, Type::Any)])
            // TODO: make this more precise: what turns into string and what into raw stream
            .named(
                "theme",
                SyntaxShape::String,
                "set a table mode/theme",
                Some('t'),
            )
            .named(
                "index",
                SyntaxShape::Any,
                "enable (true) or disable (false) the #/index column or set the starting index",
                Some('i'),
            )
            .named(
                "width",
                SyntaxShape::Int,
                "number of terminal columns wide (not output columns)",
                Some('w'),
            )
            .switch(
                "expand",
                "expand the table structure in a light mode",
                Some('e'),
            )
            .named(
                "expand-deep",
                SyntaxShape::Int,
                "an expand limit of recursion which will take place, must be used with --expand",
                Some('d'),
            )
            .switch("flatten", "Flatten simple arrays", None)
            .named(
                "flatten-separator",
                SyntaxShape::String,
                "sets a separator when 'flatten' used",
                None,
            )
            .switch(
                "collapse",
                "expand the table structure in collapse mode.\nBe aware collapse mode currently doesn't support width control",
                Some('c'),
            )
            .named(
                "abbreviated",
                SyntaxShape::Int,
                "abbreviate the data in the table by truncating the middle part and only showing amount provided on top and bottom",
                Some('a'),
            )
            .switch("list", "list available table modes/themes", Some('l'))
            .category(Category::Viewers)
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let list_themes: bool = call.has_flag(engine_state, stack, "list")?;
        // if list argument is present we just need to return a list of supported table themes
        if list_themes {
            let val = Value::list(supported_table_modes(), Span::test_data());
            return Ok(val.into_pipeline_data());
        }

        let cfg = parse_table_config(call, engine_state, stack)?;
        let input = CmdInput::new(engine_state, stack, call, input);

        // reset vt processing, aka ansi because illbehaved externals can break it
        #[cfg(windows)]
        {
            let _ = nu_utils::enable_vt_processing();
        }

        handle_table_command(input, cfg)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "List the files in current directory, with indexes starting from 1",
                example: r#"ls | table --index 1"#,
                result: None,
            },
            Example {
                description: "Render data in table view",
                example: r#"[[a b]; [1 2] [3 4]] | table"#,
                result: Some(Value::test_list(vec![
                    Value::test_record(record! {
                        "a" =>  Value::test_int(1),
                        "b" =>  Value::test_int(2),
                    }),
                    Value::test_record(record! {
                        "a" =>  Value::test_int(3),
                        "b" =>  Value::test_int(4),
                    }),
                ])),
            },
            Example {
                description: "Render data in table view (expanded)",
                example: r#"[[a b]; [1 2] [2 [4 4]]] | table --expand"#,
                result: Some(Value::test_list(vec![
                    Value::test_record(record! {
                        "a" =>  Value::test_int(1),
                        "b" =>  Value::test_int(2),
                    }),
                    Value::test_record(record! {
                        "a" =>  Value::test_int(3),
                        "b" =>  Value::test_int(4),
                    }),
                ])),
            },
            Example {
                description: "Render data in table view (collapsed)",
                example: r#"[[a b]; [1 2] [2 [4 4]]] | table --collapse"#,
                result: Some(Value::test_list(vec![
                    Value::test_record(record! {
                        "a" =>  Value::test_int(1),
                        "b" =>  Value::test_int(2),
                    }),
                    Value::test_record(record! {
                        "a" =>  Value::test_int(3),
                        "b" =>  Value::test_int(4),
                    }),
                ])),
            },
            Example {
                description: "Change the table theme to the specified theme for a single run",
                example: r#"[[a b]; [1 2] [2 [4 4]]] | table --theme basic"#,
                result: None,
            },
            Example {
                description: "Force showing of the #/index column for a single run",
                example: r#"[[a b]; [1 2] [2 [4 4]]] | table -i true"#,
                result: None,
            },
            Example {
                description:
                    "Set the starting number of the #/index column to 100 for a single run",
                example: r#"[[a b]; [1 2] [2 [4 4]]] | table -i 100"#,
                result: None,
            },
            Example {
                description: "Force hiding of the #/index column for a single run",
                example: r#"[[a b]; [1 2] [2 [4 4]]] | table -i false"#,
                result: None,
            },
        ]
    }
}

#[derive(Debug, Clone)]
struct TableConfig {
    index: Option<usize>,
    table_view: TableView,
    term_width: usize,
    theme: TableMode,
    abbreviation: Option<usize>,
}

impl TableConfig {
    fn new(
        table_view: TableView,
        term_width: usize,
        theme: TableMode,
        abbreviation: Option<usize>,
        index: Option<usize>,
    ) -> Self {
        Self {
            index,
            table_view,
            term_width,
            abbreviation,
            theme,
        }
    }
}

fn parse_table_config(
    call: &Call,
    state: &EngineState,
    stack: &mut Stack,
) -> Result<TableConfig, ShellError> {
    let width_param: Option<i64> = call.get_flag(state, stack, "width")?;
    let expand: bool = call.has_flag(state, stack, "expand")?;
    let expand_limit: Option<usize> = call.get_flag(state, stack, "expand-deep")?;
    let collapse: bool = call.has_flag(state, stack, "collapse")?;
    let flatten: bool = call.has_flag(state, stack, "flatten")?;
    let flatten_separator: Option<String> = call.get_flag(state, stack, "flatten-separator")?;
    let abbrivation: Option<usize> = call
        .get_flag(state, stack, "abbreviated")?
        .or_else(|| get_config(state, stack).table_abbreviation_threshold);
    let table_view = match (expand, collapse) {
        (false, false) => TableView::General,
        (_, true) => TableView::Collapsed,
        (true, _) => TableView::Expanded {
            limit: expand_limit,
            flatten,
            flatten_separator,
        },
    };
    let theme =
        get_theme_flag(call, state, stack)?.unwrap_or_else(|| get_config(state, stack).table_mode);
    let index = get_index_flag(call, state, stack)?;

    let term_width = get_width_param(width_param);

    Ok(TableConfig::new(
        table_view,
        term_width,
        theme,
        abbrivation,
        index,
    ))
}

fn get_index_flag(
    call: &Call,
    state: &EngineState,
    stack: &mut Stack,
) -> Result<Option<usize>, ShellError> {
    let index: Option<Value> = call.get_flag(state, stack, "index")?;
    let value = match index {
        Some(value) => value,
        None => return Ok(Some(0)),
    };

    match value {
        Value::Bool { val, .. } => {
            if val {
                Ok(Some(0))
            } else {
                Ok(None)
            }
        }
        Value::Int { val, internal_span } => {
            if val < 0 {
                Err(ShellError::UnsupportedInput {
                    msg: String::from("got a negative integer"),
                    input: val.to_string(),
                    msg_span: call.span(),
                    input_span: internal_span,
                })
            } else {
                Ok(Some(val as usize))
            }
        }
        Value::Nothing { .. } => Ok(Some(0)),
        _ => Err(ShellError::CantConvert {
            to_type: String::from("index"),
            from_type: String::new(),
            span: call.span(),
            help: Some(String::from("supported values: [bool, int, nothing]")),
        }),
    }
}

fn get_theme_flag(
    call: &Call,
    state: &EngineState,
    stack: &mut Stack,
) -> Result<Option<TableMode>, ShellError> {
    call.get_flag(state, stack, "theme")?
        .map(|theme: String| {
            TableMode::from_str(&theme).map_err(|err| ShellError::CantConvert {
                to_type: String::from("theme"),
                from_type: String::from("string"),
                span: call.span(),
                help: Some(format!("{}, but found '{}'.", err, theme)),
            })
        })
        .transpose()
}

struct CmdInput<'a> {
    engine_state: &'a EngineState,
    stack: &'a mut Stack,
    call: &'a Call,
    data: PipelineData,
}

impl<'a> CmdInput<'a> {
    fn new(
        engine_state: &'a EngineState,
        stack: &'a mut Stack,
        call: &'a Call,
        data: PipelineData,
    ) -> Self {
        Self {
            engine_state,
            stack,
            call,
            data,
        }
    }
}

fn handle_table_command(
    mut input: CmdInput<'_>,
    cfg: TableConfig,
) -> Result<PipelineData, ShellError> {
    let span = input.data.span().unwrap_or(input.call.head);
    match input.data {
        PipelineData::ExternalStream { .. } => Ok(input.data),
        PipelineData::Value(Value::Binary { val, .. }, ..) => {
            let bytes = format!("{}\n", nu_pretty_hex::pretty_hex(&val)).into_bytes();
            let ctrlc = input.engine_state.ctrlc.clone();
            let stream = RawStream::new(
                Box::new([Ok(bytes)].into_iter()),
                ctrlc,
                input.call.head,
                None,
            );

            Ok(PipelineData::ExternalStream {
                stdout: Some(stream),
                stderr: None,
                exit_code: None,
                span: input.call.head,
                metadata: None,
                trim_end_newline: false,
            })
        }
        // None of these two receive a StyleComputer because handle_row_stream() can produce it by itself using engine_state and stack.
        PipelineData::Value(Value::List { vals, .. }, metadata) => {
            let ctrlc = input.engine_state.ctrlc.clone();
            let stream = ListStream::from_stream(vals.into_iter(), ctrlc);
            input.data = PipelineData::Empty;

            handle_row_stream(input, cfg, stream, metadata)
        }
        PipelineData::ListStream(stream, metadata) => {
            input.data = PipelineData::Empty;
            handle_row_stream(input, cfg, stream, metadata)
        }
        PipelineData::Value(Value::Record { val, .. }, ..) => {
            input.data = PipelineData::Empty;
            handle_record(input, cfg, val.into_owned())
        }
        PipelineData::Value(Value::LazyRecord { val, .. }, ..) => {
            input.data = val.collect()?.into_pipeline_data();
            handle_table_command(input, cfg)
        }
        PipelineData::Value(Value::Error { error, .. }, ..) => {
            // Propagate this error outward, so that it goes to stderr
            // instead of stdout.
            Err(*error)
        }
        PipelineData::Value(Value::Custom { val, .. }, ..) => {
            let base_pipeline = val.to_base_value(span)?.into_pipeline_data();
            Table.run(input.engine_state, input.stack, input.call, base_pipeline)
        }
        PipelineData::Value(Value::Range { val, .. }, metadata) => {
            let ctrlc = input.engine_state.ctrlc.clone();
            let stream = ListStream::from_stream(val.into_range_iter(span, ctrlc.clone()), ctrlc);
            input.data = PipelineData::Empty;
            handle_row_stream(input, cfg, stream, metadata)
        }
        x => Ok(x),
    }
}

fn handle_record(
    input: CmdInput,
    cfg: TableConfig,
    mut record: Record,
) -> Result<PipelineData, ShellError> {
    let config = get_config(input.engine_state, input.stack);
    let span = input.data.span().unwrap_or(input.call.head);
    let styles = &StyleComputer::from_config(input.engine_state, input.stack);
    let ctrlc = input.engine_state.ctrlc.clone();
    let ctrlc1 = ctrlc.clone();

    if record.is_empty() {
        let value =
            create_empty_placeholder("record", cfg.term_width, input.engine_state, input.stack);
        let value = Value::string(value, span);
        return Ok(value.into_pipeline_data());
    };

    if let Some(limit) = cfg.abbreviation {
        let prev_len = record.len();
        if record.len() > limit * 2 + 1 {
            // TODO: see if the following table builders would be happy with a simple iterator
            let mut record_iter = record.into_iter();
            record = Record::with_capacity(limit * 2 + 1);
            record.extend(record_iter.by_ref().take(limit));
            record.push(String::from("..."), Value::string("...", Span::unknown()));
            record.extend(record_iter.skip(prev_len - 2 * limit));
        }
    }

    let indent = (config.table_indent.left, config.table_indent.right);
    let opts = TableOpts::new(
        &config,
        styles,
        ctrlc,
        span,
        cfg.term_width,
        indent,
        cfg.theme,
        cfg.index.unwrap_or(0),
        cfg.index.is_none(),
    );
    let result = build_table_kv(record, cfg.table_view, opts, span)?;

    let result = match result {
        Some(output) => maybe_strip_color(output, &config),
        None => report_unsuccessful_output(ctrlc1, cfg.term_width),
    };

    let val = Value::string(result, span);

    Ok(val.into_pipeline_data())
}

fn report_unsuccessful_output(ctrlc1: Option<Arc<AtomicBool>>, term_width: usize) -> String {
    if nu_utils::ctrl_c::was_pressed(&ctrlc1) {
        "".into()
    } else {
        // assume this failed because the table was too wide
        // TODO: more robust error classification
        format!("Couldn't fit table into {term_width} columns!")
    }
}

fn build_table_kv(
    record: Record,
    table_view: TableView,
    opts: TableOpts<'_>,
    span: Span,
) -> StringResult {
    match table_view {
        TableView::General => JustTable::kv_table(&record, opts),
        TableView::Expanded {
            limit,
            flatten,
            flatten_separator,
        } => {
            let sep = flatten_separator.unwrap_or_else(|| String::from(' '));
            ExpandedTable::new(limit, flatten, sep).build_map(&record, opts)
        }
        TableView::Collapsed => {
            let value = Value::record(record, span);
            CollapsedTable::build(value, opts)
        }
    }
}

fn build_table_batch(
    vals: Vec<Value>,
    table_view: TableView,
    opts: TableOpts<'_>,
    span: Span,
) -> StringResult {
    match table_view {
        TableView::General => JustTable::table(&vals, opts),
        TableView::Expanded {
            limit,
            flatten,
            flatten_separator,
        } => {
            let sep = flatten_separator.unwrap_or_else(|| String::from(' '));
            ExpandedTable::new(limit, flatten, sep).build_list(&vals, opts)
        }
        TableView::Collapsed => {
            let value = Value::list(vals, span);
            CollapsedTable::build(value, opts)
        }
    }
}

fn handle_row_stream(
    input: CmdInput<'_>,
    cfg: TableConfig,
    stream: ListStream,
    metadata: Option<PipelineMetadata>,
) -> Result<PipelineData, ShellError> {
    let ctrlc = input.engine_state.ctrlc.clone();

    let stream = match metadata.as_ref() {
        // First, `ls` sources:
        Some(PipelineMetadata {
            data_source: DataSource::Ls,
        }) => {
            let config = get_config(input.engine_state, input.stack);
            let ctrlc = ctrlc.clone();
            let ls_colors_env_str = match input.stack.get_env_var(input.engine_state, "LS_COLORS") {
                Some(v) => Some(env_to_string(
                    "LS_COLORS",
                    &v,
                    input.engine_state,
                    input.stack,
                )?),
                None => None,
            };
            let ls_colors = get_ls_colors(ls_colors_env_str);

            ListStream::from_stream(
                stream.map(move |mut x| match &mut x {
                    Value::Record { val: record, .. } => {
                        // Only the name column gets special colors, for now
                        if let Some(value) = record.to_mut().get_mut("name") {
                            let span = value.span();
                            if let Value::String { val, .. } = value {
                                if let Some(val) = render_path_name(val, &config, &ls_colors, span)
                                {
                                    *value = val;
                                }
                            }
                        }

                        x
                    }
                    _ => x,
                }),
                ctrlc,
            )
        }
        // Next, `to html -l` sources:
        Some(PipelineMetadata {
            data_source: DataSource::HtmlThemes,
        }) => {
            let ctrlc = ctrlc.clone();

            ListStream::from_stream(
                stream.map(move |mut x| match &mut x {
                    Value::Record { val: record, .. } => {
                        for (rec_col, rec_val) in record.to_mut().iter_mut() {
                            // Every column in the HTML theme table except 'name' is colored
                            if rec_col != "name" {
                                continue;
                            }
                            // Simple routine to grab the hex code, convert to a style,
                            // then place it in a new Value::String.

                            let span = rec_val.span();
                            if let Value::String { val, .. } = rec_val {
                                let s = match color_from_hex(val) {
                                    Ok(c) => match c {
                                        // .normal() just sets the text foreground color.
                                        Some(c) => c.normal(),
                                        None => nu_ansi_term::Style::default(),
                                    },
                                    Err(_) => nu_ansi_term::Style::default(),
                                };
                                *rec_val = Value::string(
                                    // Apply the style (ANSI codes) to the string
                                    s.paint(&*val).to_string(),
                                    span,
                                );
                            }
                        }
                        x
                    }
                    _ => x,
                }),
                ctrlc,
            )
        }
        _ => stream,
    };

    let paginator = PagingTableCreator::new(
        input.call.head,
        stream,
        // These are passed in as a way to have PagingTable create StyleComputers
        // for the values it outputs. Because engine_state is passed in, config doesn't need to.
        input.engine_state.clone(),
        input.stack.clone(),
        ctrlc.clone(),
        cfg,
    );
    let stream = RawStream::new(Box::new(paginator), ctrlc, input.call.head, None);

    Ok(PipelineData::ExternalStream {
        stdout: Some(stream),
        stderr: None,
        exit_code: None,
        span: input.call.head,
        metadata: None,
        trim_end_newline: false,
    })
}

fn make_clickable_link(
    full_path: String,
    link_name: Option<&str>,
    show_clickable_links: bool,
) -> String {
    // uri's based on this https://gist.github.com/egmontkob/eb114294efbcd5adb1944c9f3cb5feda

    if show_clickable_links {
        format!(
            "\x1b]8;;{}\x1b\\{}\x1b]8;;\x1b\\",
            match Url::from_file_path(full_path.clone()) {
                Ok(url) => url.to_string(),
                Err(_) => full_path.clone(),
            },
            link_name.unwrap_or(full_path.as_str())
        )
    } else {
        match link_name {
            Some(link_name) => link_name.to_string(),
            None => full_path,
        }
    }
}

struct PagingTableCreator {
    head: Span,
    stream: ListStream,
    engine_state: EngineState,
    stack: Stack,
    ctrlc: Option<Arc<AtomicBool>>,
    elements_displayed: usize,
    reached_end: bool,
    cfg: TableConfig,
    row_offset: usize,
}

impl PagingTableCreator {
    fn new(
        head: Span,
        stream: ListStream,
        engine_state: EngineState,
        stack: Stack,
        ctrlc: Option<Arc<AtomicBool>>,
        cfg: TableConfig,
    ) -> Self {
        PagingTableCreator {
            head,
            stream,
            engine_state,
            stack,
            ctrlc,
            cfg,
            elements_displayed: 0,
            reached_end: false,
            row_offset: 0,
        }
    }

    fn build_extended(
        &mut self,
        batch: Vec<Value>,
        limit: Option<usize>,
        flatten: bool,
        flatten_separator: Option<String>,
    ) -> StringResult {
        if batch.is_empty() {
            return Ok(None);
        }

        let cfg = get_config(&self.engine_state, &self.stack);
        let style_comp = StyleComputer::from_config(&self.engine_state, &self.stack);
        let opts = self.create_table_opts(&cfg, &style_comp);
        let view = TableView::Expanded {
            limit,
            flatten,
            flatten_separator,
        };

        build_table_batch(batch, view, opts, self.head)
    }

    fn build_collapsed(&mut self, batch: Vec<Value>) -> StringResult {
        if batch.is_empty() {
            return Ok(None);
        }

        let cfg = get_config(&self.engine_state, &self.stack);
        let style_comp = StyleComputer::from_config(&self.engine_state, &self.stack);
        let opts = self.create_table_opts(&cfg, &style_comp);

        build_table_batch(batch, TableView::Collapsed, opts, self.head)
    }

    fn build_general(&mut self, batch: Vec<Value>) -> StringResult {
        let cfg = get_config(&self.engine_state, &self.stack);
        let style_comp = StyleComputer::from_config(&self.engine_state, &self.stack);
        let opts = self.create_table_opts(&cfg, &style_comp);

        build_table_batch(batch, TableView::General, opts, self.head)
    }

    fn create_table_opts<'a>(
        &self,
        cfg: &'a Config,
        style_comp: &'a StyleComputer<'a>,
    ) -> TableOpts<'a> {
        TableOpts::new(
            cfg,
            style_comp,
            self.ctrlc.clone(),
            self.head,
            self.cfg.term_width,
            (cfg.table_indent.left, cfg.table_indent.right),
            self.cfg.theme,
            self.cfg.index.unwrap_or(0) + self.row_offset,
            self.cfg.index.is_none(),
        )
    }

    fn build_table(&mut self, batch: Vec<Value>) -> Result<Option<String>, ShellError> {
        match &self.cfg.table_view {
            TableView::General => self.build_general(batch),
            TableView::Collapsed => self.build_collapsed(batch),
            TableView::Expanded {
                limit,
                flatten,
                flatten_separator,
            } => self.build_extended(batch, *limit, *flatten, flatten_separator.clone()),
        }
    }
}

impl Iterator for PagingTableCreator {
    type Item = Result<Vec<u8>, ShellError>;

    fn next(&mut self) -> Option<Self::Item> {
        let batch;
        let end;

        match self.cfg.abbreviation {
            Some(abbr) => {
                (batch, _, end) =
                    stream_collect_abbriviated(&mut self.stream, abbr, self.ctrlc.clone());
            }
            None => {
                // Pull from stream until time runs out or we have enough items
                (batch, end) =
                    stream_collect(&mut self.stream, STREAM_PAGE_SIZE, self.ctrlc.clone());
            }
        }

        let batch_size = batch.len();

        // Count how much elements were displayed and if end of stream was reached
        self.elements_displayed += batch_size;
        self.reached_end = self.reached_end || end;

        if batch.is_empty() {
            // If this iterator has not displayed a single entry and reached its end (no more elements
            // or interrupted by ctrl+c) display as "empty list"
            return if self.elements_displayed == 0 && self.reached_end {
                // Increase elements_displayed by one so on next iteration next branch of this
                // if else triggers and terminates stream
                self.elements_displayed = 1;
                let result = create_empty_placeholder(
                    "list",
                    self.cfg.term_width,
                    &self.engine_state,
                    &self.stack,
                );
                Some(Ok(result.into_bytes()))
            } else {
                None
            };
        }

        let table = self.build_table(batch);

        self.row_offset += batch_size;

        let config = get_config(&self.engine_state, &self.stack);
        convert_table_to_output(table, &config, &self.ctrlc, self.cfg.term_width)
    }
}

fn stream_collect(
    stream: &mut ListStream,
    size: usize,
    ctrlc: Option<Arc<AtomicBool>>,
) -> (Vec<Value>, bool) {
    let start_time = Instant::now();
    let mut end = true;

    let mut batch = Vec::with_capacity(size);
    for (i, item) in stream.by_ref().enumerate() {
        batch.push(item);

        // If we've been buffering over a second, go ahead and send out what we have so far
        if (Instant::now() - start_time).as_secs() >= 1 {
            end = false;
            break;
        }

        if i + 1 == size {
            end = false;
            break;
        }

        if nu_utils::ctrl_c::was_pressed(&ctrlc) {
            break;
        }
    }

    (batch, end)
}

fn stream_collect_abbriviated(
    stream: &mut ListStream,
    size: usize,
    ctrlc: Option<Arc<AtomicBool>>,
) -> (Vec<Value>, usize, bool) {
    let mut end = true;
    let mut read = 0;
    let mut head = Vec::with_capacity(size);
    let mut tail = VecDeque::with_capacity(size);

    if size == 0 {
        return (vec![], 0, false);
    }

    for item in stream.by_ref() {
        read += 1;

        if read <= size {
            head.push(item);
        } else if tail.len() < size {
            tail.push_back(item);
        } else {
            let _ = tail.pop_front();
            tail.push_back(item);
        }

        if nu_utils::ctrl_c::was_pressed(&ctrlc) {
            end = false;
            break;
        }
    }

    let have_filled_list = head.len() == size && tail.len() == size;
    if have_filled_list {
        let dummy = get_abbriviated_dummy(&head, &tail);
        head.insert(size, dummy)
    }

    head.extend(tail);

    (head, read, end)
}

fn get_abbriviated_dummy(head: &[Value], tail: &VecDeque<Value>) -> Value {
    let dummy = || Value::string(String::from("..."), Span::unknown());
    let is_record_list = is_record_list(head.iter()) && is_record_list(tail.iter());

    if is_record_list {
        // in case it's a record list we set a default text to each column instead of a single value.
        Value::record(
            head[0]
                .as_record()
                .expect("ok")
                .columns()
                .map(|key| (key.clone(), dummy()))
                .collect(),
            Span::unknown(),
        )
    } else {
        dummy()
    }
}

fn is_record_list<'a>(mut batch: impl ExactSizeIterator<Item = &'a Value>) -> bool {
    batch.len() > 0 && batch.all(|value| matches!(value, Value::Record { .. }))
}

fn render_path_name(
    path: &str,
    config: &Config,
    ls_colors: &LsColors,
    span: Span,
) -> Option<Value> {
    if !config.use_ls_colors {
        return None;
    }

    let stripped_path = nu_utils::strip_ansi_unlikely(path);

    let metadata = std::fs::symlink_metadata(stripped_path.as_ref());
    let has_metadata = metadata.is_ok();
    let style =
        ls_colors.style_for_path_with_metadata(stripped_path.as_ref(), metadata.ok().as_ref());

    // clickable links don't work in remote SSH sessions
    let in_ssh_session = std::env::var("SSH_CLIENT").is_ok();
    let show_clickable_links = config.show_clickable_links_in_ls && !in_ssh_session && has_metadata;

    let ansi_style = style.map(Style::to_nu_ansi_term_style).unwrap_or_default();

    let full_path = PathBuf::from(stripped_path.as_ref())
        .canonicalize()
        .unwrap_or_else(|_| PathBuf::from(stripped_path.as_ref()));

    let full_path_link = make_clickable_link(
        full_path.display().to_string(),
        Some(path),
        show_clickable_links,
    );

    let val = ansi_style.paint(full_path_link).to_string();
    Some(Value::string(val, span))
}

#[derive(Debug, Clone)]
enum TableView {
    General,
    Collapsed,
    Expanded {
        limit: Option<usize>,
        flatten: bool,
        flatten_separator: Option<String>,
    },
}

fn maybe_strip_color(output: String, config: &Config) -> String {
    // the terminal is for when people do ls from vim, there should be no coloring there
    if !config.use_ansi_coloring || !std::io::stdout().is_terminal() {
        // Draw the table without ansi colors
        nu_utils::strip_ansi_string_likely(output)
    } else {
        // Draw the table with ansi colors
        output
    }
}

fn create_empty_placeholder(
    value_type_name: &str,
    termwidth: usize,
    engine_state: &EngineState,
    stack: &Stack,
) -> String {
    let config = get_config(engine_state, stack);
    if !config.table_show_empty {
        return String::new();
    }

    let cell = NuTableCell::new(format!("empty {}", value_type_name));
    let data = vec![vec![cell]];
    let mut table = NuTable::from(data);
    table.set_data_style(TextStyle::default().dimmed());
    let out = TableOutput::new(table, false, false);

    let style_computer = &StyleComputer::from_config(engine_state, stack);
    let config = create_nu_table_config(&config, style_computer, &out, false, TableMode::default());

    out.table
        .draw(config, termwidth)
        .expect("Could not create empty table placeholder")
}

fn convert_table_to_output(
    table: Result<Option<String>, ShellError>,
    config: &Config,
    ctrlc: &Option<Arc<AtomicBool>>,
    term_width: usize,
) -> Option<Result<Vec<u8>, ShellError>> {
    match table {
        Ok(Some(table)) => {
            let table = maybe_strip_color(table, config);

            let mut bytes = table.as_bytes().to_vec();
            bytes.push(b'\n'); // nu-table tables don't come with a newline on the end

            Some(Ok(bytes))
        }
        Ok(None) => {
            let msg = if nu_utils::ctrl_c::was_pressed(ctrlc) {
                String::from("")
            } else {
                // assume this failed because the table was too wide
                // TODO: more robust error classification
                format!("Couldn't fit table into {} columns!", term_width)
            };

            Some(Ok(msg.as_bytes().to_vec()))
        }
        Err(err) => Some(Err(err)),
    }
}

fn supported_table_modes() -> Vec<Value> {
    vec![
        Value::test_string("basic"),
        Value::test_string("compact"),
        Value::test_string("compact_double"),
        Value::test_string("default"),
        Value::test_string("heavy"),
        Value::test_string("light"),
        Value::test_string("none"),
        Value::test_string("reinforced"),
        Value::test_string("rounded"),
        Value::test_string("thin"),
        Value::test_string("with_love"),
        Value::test_string("psql"),
        Value::test_string("markdown"),
        Value::test_string("dots"),
        Value::test_string("restructured"),
        Value::test_string("ascii_rounded"),
        Value::test_string("basic_compact"),
    ]
}
