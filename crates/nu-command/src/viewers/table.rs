// todo: (refactoring) limit get_config() usage to 1 call
//        overall reduce the redundant calls to StyleComputer etc.
//        the goal is to configure it once...

use crossterm::terminal::size;
use lscolors::{LsColors, Style};
use nu_color_config::{color_from_hex, StyleComputer, TextStyle};
use nu_engine::{command_prelude::*, env_to_string};
use nu_path::form::Absolute;
use nu_pretty_hex::HexConfig;
use nu_protocol::{
    ByteStream, Config, DataSource, ListStream, PipelineMetadata, Signals, TableMode, ValueIterator,
};
use nu_table::{
    common::create_nu_table_config, CollapsedTable, ExpandedTable, JustTable, NuTable, NuTableCell,
    StringResult, TableOpts, TableOutput,
};
use nu_utils::get_ls_colors;
use std::{
    collections::VecDeque,
    io::{IsTerminal, Read},
    path::PathBuf,
    str::FromStr,
    time::Instant,
};
use url::Url;

const STREAM_PAGE_SIZE: usize = 1000;

fn get_width_param(width_param: Option<i64>) -> usize {
    if let Some(col) = width_param {
        col as usize
    } else if let Ok((w, _h)) = size() {
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

    fn description(&self) -> &str {
        "Render the table."
    }

    fn extra_description(&self) -> &str {
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
        let cwd = engine_state.cwd(Some(stack))?;
        let cfg = parse_table_config(call, engine_state, stack)?;
        let input = CmdInput::new(engine_state, stack, call, input);

        // reset vt processing, aka ansi because illbehaved externals can break it
        #[cfg(windows)]
        {
            let _ = nu_utils::enable_vt_processing();
        }

        handle_table_command(input, cfg, cwd)
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
                        "b" =>  Value::test_list(vec![
                            Value::test_int(4),
                            Value::test_int(4),
                        ])
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
                        "b" =>  Value::test_list(vec![
                            Value::test_int(4),
                            Value::test_int(4),
                        ])
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
        .or_else(|| stack.get_config(state).table.abbreviated_row_count);
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
        get_theme_flag(call, state, stack)?.unwrap_or_else(|| stack.get_config(state).table.mode);
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
    call: &'a Call<'a>,
    data: PipelineData,
}

impl<'a> CmdInput<'a> {
    fn new(
        engine_state: &'a EngineState,
        stack: &'a mut Stack,
        call: &'a Call<'a>,
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
    cwd: nu_path::PathBuf<Absolute>,
) -> Result<PipelineData, ShellError> {
    let span = input.data.span().unwrap_or(input.call.head);
    match input.data {
        // Binary streams should behave as if they really are `binary` data, and printed as hex
        PipelineData::ByteStream(stream, _) if stream.type_() == ByteStreamType::Binary => Ok(
            PipelineData::ByteStream(pretty_hex_stream(stream, input.call.head), None),
        ),
        PipelineData::ByteStream(..) => Ok(input.data),
        PipelineData::Value(Value::Binary { val, .. }, ..) => {
            let signals = input.engine_state.signals().clone();
            let stream = ByteStream::read_binary(val, input.call.head, signals);
            Ok(PipelineData::ByteStream(
                pretty_hex_stream(stream, input.call.head),
                None,
            ))
        }
        // None of these two receive a StyleComputer because handle_row_stream() can produce it by itself using engine_state and stack.
        PipelineData::Value(Value::List { vals, .. }, metadata) => {
            let signals = input.engine_state.signals().clone();
            let stream = ListStream::new(vals.into_iter(), span, signals);
            input.data = PipelineData::Empty;

            handle_row_stream(input, cfg, stream, metadata, cwd)
        }
        PipelineData::ListStream(stream, metadata) => {
            input.data = PipelineData::Empty;
            handle_row_stream(input, cfg, stream, metadata, cwd)
        }
        PipelineData::Value(Value::Record { val, .. }, ..) => {
            input.data = PipelineData::Empty;
            handle_record(input, cfg, val.into_owned())
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
            let signals = input.engine_state.signals().clone();
            let stream =
                ListStream::new(val.into_range_iter(span, Signals::empty()), span, signals);
            input.data = PipelineData::Empty;
            handle_row_stream(input, cfg, stream, metadata, cwd)
        }
        x => Ok(x),
    }
}

fn pretty_hex_stream(stream: ByteStream, span: Span) -> ByteStream {
    let mut cfg = HexConfig {
        // We are going to render the title manually first
        title: true,
        // If building on 32-bit, the stream size might be bigger than a usize
        length: stream.known_size().and_then(|sz| sz.try_into().ok()),
        ..HexConfig::default()
    };

    // This won't really work for us
    debug_assert!(cfg.width > 0, "the default hex config width was zero");

    let mut read_buf = Vec::with_capacity(cfg.width);

    let mut reader = if let Some(reader) = stream.reader() {
        reader
    } else {
        // No stream to read from
        return ByteStream::read_string("".into(), span, Signals::empty());
    };

    ByteStream::from_fn(
        span,
        Signals::empty(),
        ByteStreamType::String,
        move |buffer| {
            // Turn the buffer into a String we can write to
            let mut write_buf = std::mem::take(buffer);
            write_buf.clear();
            // SAFETY: we just truncated it empty
            let mut write_buf = unsafe { String::from_utf8_unchecked(write_buf) };

            // Write the title at the beginning
            if cfg.title {
                nu_pretty_hex::write_title(&mut write_buf, cfg, true).expect("format error");
                cfg.title = false;

                // Put the write_buf back into buffer
                *buffer = write_buf.into_bytes();

                Ok(true)
            } else {
                // Read up to `cfg.width` bytes
                read_buf.clear();
                (&mut reader)
                    .take(cfg.width as u64)
                    .read_to_end(&mut read_buf)
                    .err_span(span)?;

                if !read_buf.is_empty() {
                    nu_pretty_hex::hex_write(&mut write_buf, &read_buf, cfg, Some(true))
                        .expect("format error");
                    write_buf.push('\n');

                    // Advance the address offset for next time
                    cfg.address_offset += read_buf.len();

                    // Put the write_buf back into buffer
                    *buffer = write_buf.into_bytes();

                    Ok(true)
                } else {
                    Ok(false)
                }
            }
        },
    )
}

fn handle_record(
    input: CmdInput,
    cfg: TableConfig,
    mut record: Record,
) -> Result<PipelineData, ShellError> {
    let config = {
        let state = input.engine_state;
        let stack: &Stack = input.stack;
        stack.get_config(state)
    };
    let span = input.data.span().unwrap_or(input.call.head);
    let styles = &StyleComputer::from_config(input.engine_state, input.stack);

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

    let indent = (config.table.padding.left, config.table.padding.right);
    let opts = TableOpts::new(
        &config,
        styles,
        input.engine_state.signals(),
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
        None => report_unsuccessful_output(input.engine_state.signals(), cfg.term_width),
    };

    let val = Value::string(result, span);

    Ok(val.into_pipeline_data())
}

fn report_unsuccessful_output(signals: &Signals, term_width: usize) -> String {
    if signals.interrupted() {
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
    cwd: nu_path::PathBuf<Absolute>,
) -> Result<PipelineData, ShellError> {
    let stream = match metadata.as_ref() {
        // First, `ls` sources:
        Some(PipelineMetadata {
            data_source: DataSource::Ls,
            ..
        }) => {
            let config = {
                let state = input.engine_state;
                let stack: &Stack = input.stack;
                stack.get_config(state)
            };
            let ls_colors_env_str = match input.stack.get_env_var(input.engine_state, "LS_COLORS") {
                Some(v) => Some(env_to_string(
                    "LS_COLORS",
                    v,
                    input.engine_state,
                    input.stack,
                )?),
                None => None,
            };
            let ls_colors = get_ls_colors(ls_colors_env_str);

            stream.map(move |mut value| {
                if let Value::Record { val: record, .. } = &mut value {
                    // Only the name column gets special colors, for now
                    if let Some(value) = record.to_mut().get_mut("name") {
                        let span = value.span();
                        if let Value::String { val, .. } = value {
                            if let Some(val) =
                                render_path_name(val, &config, &ls_colors, cwd.clone(), span)
                            {
                                *value = val;
                            }
                        }
                    }
                }
                value
            })
        }
        // Next, `to html -l` sources:
        Some(PipelineMetadata {
            data_source: DataSource::HtmlThemes,
            ..
        }) => {
            stream.map(|mut value| {
                if let Value::Record { val: record, .. } = &mut value {
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
                }
                value
            })
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
        cfg,
    );
    let stream = ByteStream::from_result_iter(
        paginator,
        input.call.head,
        Signals::empty(),
        ByteStreamType::String,
    );
    Ok(PipelineData::ByteStream(stream, None))
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
    stream: ValueIterator,
    engine_state: EngineState,
    stack: Stack,
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
        cfg: TableConfig,
    ) -> Self {
        PagingTableCreator {
            head,
            stream: stream.into_inner(),
            engine_state,
            stack,
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

        let cfg = {
            let state = &self.engine_state;
            let stack = &self.stack;
            stack.get_config(state)
        };
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

        let cfg = {
            let state = &self.engine_state;
            let stack = &self.stack;
            stack.get_config(state)
        };
        let style_comp = StyleComputer::from_config(&self.engine_state, &self.stack);
        let opts = self.create_table_opts(&cfg, &style_comp);

        build_table_batch(batch, TableView::Collapsed, opts, self.head)
    }

    fn build_general(&mut self, batch: Vec<Value>) -> StringResult {
        let cfg = {
            let state = &self.engine_state;
            let stack = &self.stack;
            stack.get_config(state)
        };
        let style_comp = StyleComputer::from_config(&self.engine_state, &self.stack);
        let opts = self.create_table_opts(&cfg, &style_comp);

        build_table_batch(batch, TableView::General, opts, self.head)
    }

    fn create_table_opts<'a>(
        &'a self,
        cfg: &'a Config,
        style_comp: &'a StyleComputer<'a>,
    ) -> TableOpts<'a> {
        TableOpts::new(
            cfg,
            style_comp,
            self.engine_state.signals(),
            self.head,
            self.cfg.term_width,
            (cfg.table.padding.left, cfg.table.padding.right),
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
                    stream_collect_abbriviated(&mut self.stream, abbr, self.engine_state.signals());
            }
            None => {
                // Pull from stream until time runs out or we have enough items
                (batch, end) = stream_collect(
                    &mut self.stream,
                    STREAM_PAGE_SIZE,
                    self.engine_state.signals(),
                );
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

        let config = {
            let state = &self.engine_state;
            let stack = &self.stack;
            stack.get_config(state)
        };
        convert_table_to_output(
            table,
            &config,
            self.engine_state.signals(),
            self.cfg.term_width,
        )
    }
}

fn stream_collect(
    stream: impl Iterator<Item = Value>,
    size: usize,
    signals: &Signals,
) -> (Vec<Value>, bool) {
    let start_time = Instant::now();
    let mut end = true;

    let mut batch = Vec::with_capacity(size);
    for (i, item) in stream.enumerate() {
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

        if signals.interrupted() {
            break;
        }
    }

    (batch, end)
}

fn stream_collect_abbriviated(
    stream: impl Iterator<Item = Value>,
    size: usize,
    signals: &Signals,
) -> (Vec<Value>, usize, bool) {
    let mut end = true;
    let mut read = 0;
    let mut head = Vec::with_capacity(size);
    let mut tail = VecDeque::with_capacity(size);

    if size == 0 {
        return (vec![], 0, false);
    }

    for item in stream {
        read += 1;

        if read <= size {
            head.push(item);
        } else if tail.len() < size {
            tail.push_back(item);
        } else {
            let _ = tail.pop_front();
            tail.push_back(item);
        }

        if signals.interrupted() {
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
    cwd: nu_path::PathBuf<Absolute>,
    span: Span,
) -> Option<Value> {
    if !config.ls.use_ls_colors {
        return None;
    }

    let fullpath = cwd.join(path);
    let stripped_path = nu_utils::strip_ansi_unlikely(path);
    let metadata = std::fs::symlink_metadata(fullpath);
    let has_metadata = metadata.is_ok();
    let style =
        ls_colors.style_for_path_with_metadata(stripped_path.as_ref(), metadata.ok().as_ref());

    // clickable links don't work in remote SSH sessions
    let in_ssh_session = std::env::var("SSH_CLIENT").is_ok();
    //TODO: Deprecated show_clickable_links_in_ls in favor of shell_integration_osc8
    let show_clickable_links = config.ls.clickable_links
        && !in_ssh_session
        && has_metadata
        && config.shell_integration.osc8;

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
    let config = stack.get_config(engine_state);
    if !config.table.show_empty {
        return String::new();
    }

    let cell = NuTableCell::new(format!("empty {}", value_type_name));
    let data = vec![vec![cell]];
    let mut table = NuTable::from(data);
    table.set_data_style(TextStyle::default().dimmed());
    let out = TableOutput::new(table, false, false, 1);

    let style_computer = &StyleComputer::from_config(engine_state, stack);
    let config = create_nu_table_config(&config, style_computer, &out, false, TableMode::default());

    out.table
        .draw(config, termwidth)
        .expect("Could not create empty table placeholder")
}

fn convert_table_to_output(
    table: Result<Option<String>, ShellError>,
    config: &Config,
    signals: &Signals,
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
            let msg = if signals.interrupted() {
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
