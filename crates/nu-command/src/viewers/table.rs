// todo: (refactoring) limit get_config() usage to 1 call
//        overall reduce the redundant calls to StyleComputer etc.
//        the goal is to configure it once...

use std::{collections::VecDeque, io::Read, path::PathBuf, str::FromStr, time::Duration};

use lscolors::{LsColors, Style};
use url::Url;
use web_time::Instant;

use nu_color_config::{StyleComputer, TextStyle, color_from_hex};
use nu_engine::{command_prelude::*, env_to_string};
use nu_path::form::Absolute;
use nu_pretty_hex::HexConfig;
use nu_protocol::{
    ByteStream, Config, DataSource, ListStream, PipelineMetadata, Signals, TableMode,
    ValueIterator, shell_error::io::IoError,
};
use nu_table::{
    CollapsedTable, ExpandedTable, JustTable, NuTable, StringResult, TableOpts, TableOutput,
    common::configure_table,
};
use nu_utils::{get_ls_colors, terminal_size};

type ShellResult<T> = Result<T, ShellError>;
type NuPathBuf = nu_path::PathBuf<Absolute>;

const DEFAULT_TABLE_WIDTH: usize = 80;

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
            .param(
                Flag::new("theme")
                    .short('t')
                    .arg(SyntaxShape::String)
                    .desc("set a table mode/theme")
                    .completion(Completion::new_list(SUPPORTED_TABLE_MODES)),
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
    ) -> ShellResult<PipelineData> {
        let list_themes: bool = call.has_flag(engine_state, stack, "list")?;
        // if list argument is present we just need to return a list of supported table themes
        if list_themes {
            let val = Value::list(supported_table_modes(), Span::test_data());
            return Ok(val.into_pipeline_data());
        }

        let input = CmdInput::parse(engine_state, stack, call, input)?;

        // reset vt processing, aka ansi because illbehaved externals can break it
        #[cfg(windows)]
        {
            let _ = nu_utils::enable_vt_processing();
        }

        handle_table_command(input)
    }

    fn examples(&self) -> Vec<Example<'_>> {
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
                example: r#"[[a b]; [1 2] [3 [4 4]]] | table --expand"#,
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
                example: r#"[[a b]; [1 2] [3 [4 4]]] | table --collapse"#,
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
                example: r#"[[a b]; [1 2] [3 [4 4]]] | table --theme basic"#,
                result: None,
            },
            Example {
                description: "Force showing of the #/index column for a single run",
                example: r#"[[a b]; [1 2] [3 [4 4]]] | table -i true"#,
                result: None,
            },
            Example {
                description: "Set the starting number of the #/index column to 100 for a single run",
                example: r#"[[a b]; [1 2] [3 [4 4]]] | table -i 100"#,
                result: None,
            },
            Example {
                description: "Force hiding of the #/index column for a single run",
                example: r#"[[a b]; [1 2] [3 [4 4]]] | table -i false"#,
                result: None,
            },
        ]
    }
}

#[derive(Debug, Clone)]
struct TableConfig {
    view: TableView,
    width: usize,
    theme: TableMode,
    abbreviation: Option<usize>,
    index: Option<usize>,
    use_ansi_coloring: bool,
}

impl TableConfig {
    fn new(
        view: TableView,
        width: usize,
        theme: TableMode,
        abbreviation: Option<usize>,
        index: Option<usize>,
        use_ansi_coloring: bool,
    ) -> Self {
        Self {
            view,
            width,
            theme,
            abbreviation,
            index,
            use_ansi_coloring,
        }
    }
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

struct CLIArgs {
    width: Option<i64>,
    abbrivation: Option<usize>,
    theme: TableMode,
    expand: bool,
    expand_limit: Option<usize>,
    expand_flatten: bool,
    expand_flatten_separator: Option<String>,
    collapse: bool,
    index: Option<usize>,
    use_ansi_coloring: bool,
}

fn parse_table_config(
    call: &Call,
    state: &EngineState,
    stack: &mut Stack,
) -> ShellResult<TableConfig> {
    let args = get_cli_args(call, state, stack)?;
    let table_view = get_table_view(&args);
    let term_width = get_table_width(args.width);

    let cfg = TableConfig::new(
        table_view,
        term_width,
        args.theme,
        args.abbrivation,
        args.index,
        args.use_ansi_coloring,
    );

    Ok(cfg)
}

fn get_table_view(args: &CLIArgs) -> TableView {
    match (args.expand, args.collapse) {
        (false, false) => TableView::General,
        (_, true) => TableView::Collapsed,
        (true, _) => TableView::Expanded {
            limit: args.expand_limit,
            flatten: args.expand_flatten,
            flatten_separator: args.expand_flatten_separator.clone(),
        },
    }
}

fn get_cli_args(call: &Call<'_>, state: &EngineState, stack: &mut Stack) -> ShellResult<CLIArgs> {
    let width: Option<i64> = call.get_flag(state, stack, "width")?;
    let expand: bool = call.has_flag(state, stack, "expand")?;
    let expand_limit: Option<usize> = call.get_flag(state, stack, "expand-deep")?;
    let expand_flatten: bool = call.has_flag(state, stack, "flatten")?;
    let expand_flatten_separator: Option<String> =
        call.get_flag(state, stack, "flatten-separator")?;
    let collapse: bool = call.has_flag(state, stack, "collapse")?;
    let abbrivation: Option<usize> = call
        .get_flag(state, stack, "abbreviated")?
        .or_else(|| stack.get_config(state).table.abbreviated_row_count);
    let theme =
        get_theme_flag(call, state, stack)?.unwrap_or_else(|| stack.get_config(state).table.mode);
    let index = get_index_flag(call, state, stack)?;

    let use_ansi_coloring = stack.get_config(state).use_ansi_coloring.get(state);

    Ok(CLIArgs {
        theme,
        abbrivation,
        collapse,
        expand,
        expand_limit,
        expand_flatten,
        expand_flatten_separator,
        width,
        index,
        use_ansi_coloring,
    })
}

fn get_index_flag(
    call: &Call,
    state: &EngineState,
    stack: &mut Stack,
) -> ShellResult<Option<usize>> {
    let index: Option<Value> = call.get_flag(state, stack, "index")?;
    let value = match index {
        Some(value) => value,
        None => return Ok(Some(0)),
    };
    let span = value.span();

    match value {
        Value::Bool { val, .. } => {
            if val {
                Ok(Some(0))
            } else {
                Ok(None)
            }
        }
        Value::Int { val, .. } => {
            if val < 0 {
                Err(ShellError::UnsupportedInput {
                    msg: String::from("got a negative integer"),
                    input: val.to_string(),
                    msg_span: call.span(),
                    input_span: span,
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
) -> ShellResult<Option<TableMode>> {
    call.get_flag(state, stack, "theme")?
        .map(|theme: String| {
            TableMode::from_str(&theme).map_err(|err| ShellError::CantConvert {
                to_type: String::from("theme"),
                from_type: String::from("string"),
                span: call.span(),
                help: Some(format!("{err}, but found '{theme}'.")),
            })
        })
        .transpose()
}

struct CmdInput<'a> {
    engine_state: &'a EngineState,
    stack: &'a mut Stack,
    call: &'a Call<'a>,
    data: PipelineData,
    cfg: TableConfig,
    cwd: Option<NuPathBuf>,
}

impl<'a> CmdInput<'a> {
    fn parse(
        engine_state: &'a EngineState,
        stack: &'a mut Stack,
        call: &'a Call<'a>,
        data: PipelineData,
    ) -> ShellResult<Self> {
        let cfg = parse_table_config(call, engine_state, stack)?;
        let cwd = get_cwd(engine_state, stack)?;

        Ok(Self {
            engine_state,
            stack,
            call,
            data,
            cfg,
            cwd,
        })
    }

    fn get_config(&self) -> std::sync::Arc<Config> {
        self.stack.get_config(self.engine_state)
    }
}

fn handle_table_command(mut input: CmdInput<'_>) -> ShellResult<PipelineData> {
    let span = input.data.span().unwrap_or(input.call.head);
    match input.data {
        // Binary streams should behave as if they really are `binary` data, and printed as hex
        PipelineData::ByteStream(stream, _) if stream.type_() == ByteStreamType::Binary => Ok(
            PipelineData::byte_stream(pretty_hex_stream(stream, input.call.head), None),
        ),
        PipelineData::ByteStream(..) => Ok(input.data),
        PipelineData::Value(Value::Binary { val, .. }, ..) => {
            let signals = input.engine_state.signals().clone();
            let stream = ByteStream::read_binary(val, input.call.head, signals);
            Ok(PipelineData::byte_stream(
                pretty_hex_stream(stream, input.call.head),
                None,
            ))
        }
        // None of these two receive a StyleComputer because handle_row_stream() can produce it by itself using engine_state and stack.
        PipelineData::Value(Value::List { vals, .. }, metadata) => {
            let signals = input.engine_state.signals().clone();
            let stream = ListStream::new(vals.into_iter(), span, signals);
            input.data = PipelineData::empty();

            handle_row_stream(input, stream, metadata)
        }
        PipelineData::ListStream(stream, metadata) => {
            input.data = PipelineData::empty();
            handle_row_stream(input, stream, metadata)
        }
        PipelineData::Value(Value::Record { val, .. }, ..) => {
            input.data = PipelineData::empty();
            handle_record(input, val.into_owned())
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
            input.data = PipelineData::empty();
            handle_row_stream(input, stream, metadata)
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
                    .map_err(|err| IoError::new(err, span, None))?;

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

fn handle_record(input: CmdInput, mut record: Record) -> ShellResult<PipelineData> {
    let span = input.data.span().unwrap_or(input.call.head);

    if record.is_empty() {
        let value = create_empty_placeholder(
            "record",
            input.cfg.width,
            input.engine_state,
            input.stack,
            input.cfg.use_ansi_coloring,
        );
        let value = Value::string(value, span);
        return Ok(value.into_pipeline_data());
    };

    if let Some(limit) = input.cfg.abbreviation {
        record = make_record_abbreviation(record, limit);
    }

    let config = input.get_config();
    let opts = create_table_opts(
        input.engine_state,
        input.stack,
        &config,
        &input.cfg,
        span,
        0,
    );
    let result = build_table_kv(record, input.cfg.view.clone(), opts, span)?;

    let result = match result {
        Some(output) => maybe_strip_color(output, input.cfg.use_ansi_coloring),
        None => report_unsuccessful_output(input.engine_state.signals(), input.cfg.width),
    };

    let val = Value::string(result, span);
    let data = val.into_pipeline_data();

    Ok(data)
}

fn make_record_abbreviation(mut record: Record, limit: usize) -> Record {
    if record.len() <= limit * 2 + 1 {
        return record;
    }

    // TODO: see if the following table builders would be happy with a simple iterator
    let prev_len = record.len();
    let mut record_iter = record.into_iter();
    record = Record::with_capacity(limit * 2 + 1);
    record.extend(record_iter.by_ref().take(limit));
    record.push(String::from("..."), Value::string("...", Span::unknown()));
    record.extend(record_iter.skip(prev_len - 2 * limit));
    record
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
        TableView::General => JustTable::kv_table(record, opts),
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
    mut vals: Vec<Value>,
    view: TableView,
    opts: TableOpts<'_>,
    span: Span,
) -> StringResult {
    // convert each custom value to its base value so it can be properly
    // displayed in a table
    for val in &mut vals {
        let span = val.span();

        if let Value::Custom { val: custom, .. } = val {
            *val = custom
                .to_base_value(span)
                .or_else(|err| Result::<_, ShellError>::Ok(Value::error(err, span)))
                .expect("error converting custom value to base value")
        }
    }

    match view {
        TableView::General => JustTable::table(vals, opts),
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
    stream: ListStream,
    metadata: Option<PipelineMetadata>,
) -> ShellResult<PipelineData> {
    let cfg = input.get_config();
    let stream = match metadata.as_ref() {
        // First, `ls` sources:
        Some(PipelineMetadata {
            data_source: DataSource::Ls,
            ..
        }) => {
            let config = cfg.clone();
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
                        if let Value::String { val, .. } = value
                            && let Some(val) =
                                render_path_name(val, &config, &ls_colors, input.cwd.clone(), span)
                        {
                            *value = val;
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
        input.cfg,
        cfg,
    );
    let stream = ByteStream::from_result_iter(
        paginator,
        input.call.head,
        Signals::empty(),
        ByteStreamType::String,
    );
    Ok(PipelineData::byte_stream(stream, None))
}

fn make_clickable_link(
    full_path: String,
    link_name: Option<&str>,
    show_clickable_links: bool,
) -> String {
    // uri's based on this https://gist.github.com/egmontkob/eb114294efbcd5adb1944c9f3cb5feda

    #[cfg(any(
        unix,
        windows,
        target_os = "redox",
        target_os = "wasi",
        target_os = "hermit"
    ))]
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

    #[cfg(not(any(
        unix,
        windows,
        target_os = "redox",
        target_os = "wasi",
        target_os = "hermit"
    )))]
    match link_name {
        Some(link_name) => link_name.to_string(),
        None => full_path,
    }
}

struct PagingTableCreator {
    head: Span,
    stream: ValueIterator,
    engine_state: EngineState,
    stack: Stack,
    elements_displayed: usize,
    reached_end: bool,
    table_config: TableConfig,
    row_offset: usize,
    config: std::sync::Arc<Config>,
}

impl PagingTableCreator {
    fn new(
        head: Span,
        stream: ListStream,
        engine_state: EngineState,
        stack: Stack,
        table_config: TableConfig,
        config: std::sync::Arc<Config>,
    ) -> Self {
        PagingTableCreator {
            head,
            stream: stream.into_inner(),
            engine_state,
            stack,
            config,
            table_config,
            elements_displayed: 0,
            reached_end: false,
            row_offset: 0,
        }
    }

    fn build_table(&mut self, batch: Vec<Value>) -> ShellResult<Option<String>> {
        if batch.is_empty() {
            return Ok(None);
        }

        let opts = self.create_table_opts();
        build_table_batch(batch, self.table_config.view.clone(), opts, self.head)
    }

    fn create_table_opts(&self) -> TableOpts<'_> {
        create_table_opts(
            &self.engine_state,
            &self.stack,
            &self.config,
            &self.table_config,
            self.head,
            self.row_offset,
        )
    }
}

impl Iterator for PagingTableCreator {
    type Item = ShellResult<Vec<u8>>;

    fn next(&mut self) -> Option<Self::Item> {
        let batch;
        let end;

        match self.table_config.abbreviation {
            Some(abbr) => {
                (batch, _, end) =
                    stream_collect_abbriviated(&mut self.stream, abbr, self.engine_state.signals());
            }
            None => {
                // Pull from stream until time runs out or we have enough items
                (batch, end) = stream_collect(
                    &mut self.stream,
                    self.config.table.stream_page_size.get() as usize,
                    self.config.table.batch_duration,
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
                    self.table_config.width,
                    &self.engine_state,
                    &self.stack,
                    self.table_config.use_ansi_coloring,
                );
                let mut bytes = result.into_bytes();
                // Add extra newline if show_empty is enabled
                if !bytes.is_empty() {
                    bytes.push(b'\n');
                }
                Some(Ok(bytes))
            } else {
                None
            };
        }

        let table = self.build_table(batch);

        self.row_offset += batch_size;

        convert_table_to_output(
            table,
            self.engine_state.signals(),
            self.table_config.width,
            self.table_config.use_ansi_coloring,
        )
    }
}

fn stream_collect(
    stream: impl Iterator<Item = Value>,
    size: usize,
    batch_duration: Duration,
    signals: &Signals,
) -> (Vec<Value>, bool) {
    let start_time = Instant::now();
    let mut end = true;

    let mut batch = Vec::with_capacity(size);
    for (i, item) in stream.enumerate() {
        batch.push(item);

        // We buffer until `$env.config.table.batch_duration`, then we send out what we have so far
        if (Instant::now() - start_time) >= batch_duration {
            end = false;
            break;
        }

        // Or until we reached `$env.config.table.stream_page_size`.
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
    cwd: Option<NuPathBuf>,
    span: Span,
) -> Option<Value> {
    if !config.ls.use_ls_colors {
        return None;
    }

    let fullpath = match cwd {
        Some(cwd) => PathBuf::from(cwd.join(path)),
        None => PathBuf::from(path),
    };

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

    // If there is no style at all set it to use 'default' foreground and background
    // colors. This prevents it being colored in tabled as string colors.
    // To test this:
    //   $env.LS_COLORS = 'fi=0'
    //   $env.config.color_config.string = 'red'
    // if a regular file without an extension is the color 'default' then it's working
    // if a regular file without an extension is the color 'red' then it's not working
    let ansi_style = style
        .map(Style::to_nu_ansi_term_style)
        .unwrap_or(nu_ansi_term::Style {
            foreground: Some(nu_ansi_term::Color::Default),
            background: Some(nu_ansi_term::Color::Default),
            is_bold: false,
            is_dimmed: false,
            is_italic: false,
            is_underline: false,
            is_blink: false,
            is_reverse: false,
            is_hidden: false,
            is_strikethrough: false,
            prefix_with_reset: false,
        });

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

fn maybe_strip_color(output: String, use_ansi_coloring: bool) -> String {
    // only use `use_ansi_coloring` here, it already includes `std::io::stdout().is_terminal()`
    // when set to "auto"
    if !use_ansi_coloring {
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
    use_ansi_coloring: bool,
) -> String {
    let config = stack.get_config(engine_state);
    if !config.table.show_empty {
        return String::new();
    }

    let cell = format!("empty {value_type_name}");
    let mut table = NuTable::new(1, 1);
    table.insert((0, 0), cell);
    table.set_data_style(TextStyle::default().dimmed());
    let mut out = TableOutput::from_table(table, false, false);

    let style_computer = &StyleComputer::from_config(engine_state, stack);
    configure_table(&mut out, &config, style_computer, TableMode::default());

    if !use_ansi_coloring {
        out.table.clear_all_colors();
    }

    out.table
        .draw(termwidth)
        .expect("Could not create empty table placeholder")
}

fn convert_table_to_output(
    table: ShellResult<Option<String>>,
    signals: &Signals,
    term_width: usize,
    use_ansi_coloring: bool,
) -> Option<ShellResult<Vec<u8>>> {
    match table {
        Ok(Some(table)) => {
            let table = maybe_strip_color(table, use_ansi_coloring);

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
                format!("Couldn't fit table into {term_width} columns!")
            };

            Some(Ok(msg.as_bytes().to_vec()))
        }
        Err(err) => Some(Err(err)),
    }
}

const SUPPORTED_TABLE_MODES: &[&str] = &[
    "basic",
    "compact",
    "compact_double",
    "default",
    "heavy",
    "light",
    "none",
    "reinforced",
    "rounded",
    "thin",
    "with_love",
    "psql",
    "markdown",
    "dots",
    "restructured",
    "ascii_rounded",
    "basic_compact",
    "single",
    "double",
];

fn supported_table_modes() -> Vec<Value> {
    SUPPORTED_TABLE_MODES
        .iter()
        .copied()
        .map(Value::test_string)
        .collect()
}

fn create_table_opts<'a>(
    engine_state: &'a EngineState,
    stack: &'a Stack,
    cfg: &'a Config,
    table_cfg: &'a TableConfig,
    span: Span,
    offset: usize,
) -> TableOpts<'a> {
    let comp = StyleComputer::from_config(engine_state, stack);
    let signals = engine_state.signals();
    let offset = table_cfg.index.unwrap_or(0) + offset;
    let index = table_cfg.index.is_none();
    let width = table_cfg.width;
    let theme = table_cfg.theme;

    TableOpts::new(cfg, comp, signals, span, width, theme, offset, index)
}

fn get_cwd(engine_state: &EngineState, stack: &mut Stack) -> ShellResult<Option<NuPathBuf>> {
    #[cfg(feature = "os")]
    let cwd = engine_state.cwd(Some(stack)).map(Some)?;

    #[cfg(not(feature = "os"))]
    let cwd = None;

    Ok(cwd)
}

fn get_table_width(width_param: Option<i64>) -> usize {
    if let Some(col) = width_param {
        col as usize
    } else if let Ok((w, _h)) = terminal_size() {
        w as usize
    } else {
        DEFAULT_TABLE_WIDTH
    }
}
