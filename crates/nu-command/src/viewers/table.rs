use lscolors::{LsColors, Style};
use nu_color_config::color_from_hex;
use nu_color_config::{StyleComputer, TextStyle};
use nu_engine::{env::get_config, env_to_string, CallExt};
use nu_protocol::Record;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Config, DataSource, Example, FooterMode, IntoPipelineData, ListStream, PipelineData,
    PipelineMetadata, RawStream, ShellError, Signature, Span, SyntaxShape, Type, Value,
};
use nu_table::{
    BuildConfig, Cell, CollapsedTable, ExpandedTable, JustTable, NuTable, StringResult,
    TableConfig, TableOutput, TableTheme,
};
use nu_utils::get_ls_colors;
use std::sync::Arc;
use std::time::Instant;
use std::{path::PathBuf, sync::atomic::AtomicBool};
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
                "start-number",
                SyntaxShape::Int,
                "row number to start viewing from",
                Some('n'),
            )
            .switch("list", "list available table modes/themes", Some('l'))
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
                "an expand limit of recursion which will take place",
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
            .category(Category::Viewers)
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let start_num: Option<i64> = call.get_flag(engine_state, stack, "start-number")?;
        let row_offset = start_num.unwrap_or_default() as usize;
        let list: bool = call.has_flag("list");

        let width_param: Option<i64> = call.get_flag(engine_state, stack, "width")?;

        let expand: bool = call.has_flag("expand");
        let expand_limit: Option<usize> = call.get_flag(engine_state, stack, "expand-deep")?;
        let collapse: bool = call.has_flag("collapse");
        let flatten: bool = call.has_flag("flatten");
        let flatten_separator: Option<String> =
            call.get_flag(engine_state, stack, "flatten-separator")?;

        let table_view = match (expand, collapse) {
            (false, false) => TableView::General,
            (_, true) => TableView::Collapsed,
            (true, _) => TableView::Expanded {
                limit: expand_limit,
                flatten,
                flatten_separator,
            },
        };

        // if list argument is present we just need to return a list of supported table themes
        if list {
            let val = Value::List {
                vals: supported_table_modes(),
                span: Span::test_data(),
            };

            return Ok(val.into_pipeline_data());
        }

        // reset vt processing, aka ansi because illbehaved externals can break it
        #[cfg(windows)]
        {
            let _ = nu_utils::enable_vt_processing();
        }

        handle_table_command(
            engine_state,
            stack,
            call,
            input,
            row_offset,
            table_view,
            width_param,
        )
    }

    fn examples(&self) -> Vec<Example> {
        let span = Span::test_data();
        vec![
            Example {
                description: "List the files in current directory, with indexes starting from 1.",
                example: r#"ls | table -n 1"#,
                result: None,
            },
            Example {
                description: "Render data in table view",
                example: r#"[[a b]; [1 2] [3 4]] | table"#,
                result: Some(Value::List {
                    vals: vec![
                        Value::test_record(Record {
                            cols: vec!["a".to_string(), "b".to_string()],
                            vals: vec![Value::test_int(1), Value::test_int(2)],
                        }),
                        Value::test_record(Record {
                            cols: vec!["a".to_string(), "b".to_string()],
                            vals: vec![Value::test_int(3), Value::test_int(4)],
                        }),
                    ],
                    span,
                }),
            },
            Example {
                description: "Render data in table view (expanded)",
                example: r#"[[a b]; [1 2] [2 [4 4]]] | table --expand"#,
                result: Some(Value::List {
                    vals: vec![
                        Value::test_record(Record {
                            cols: vec!["a".to_string(), "b".to_string()],
                            vals: vec![Value::test_int(1), Value::test_int(2)],
                        }),
                        Value::test_record(Record {
                            cols: vec!["a".to_string(), "b".to_string()],
                            vals: vec![Value::test_int(3), Value::test_int(4)],
                        }),
                    ],
                    span,
                }),
            },
            Example {
                description: "Render data in table view (collapsed)",
                example: r#"[[a b]; [1 2] [2 [4 4]]] | table --collapse"#,
                result: Some(Value::List {
                    vals: vec![
                        Value::test_record(Record {
                            cols: vec!["a".to_string(), "b".to_string()],
                            vals: vec![Value::test_int(1), Value::test_int(2)],
                        }),
                        Value::test_record(Record {
                            cols: vec!["a".to_string(), "b".to_string()],
                            vals: vec![Value::test_int(3), Value::test_int(4)],
                        }),
                    ],
                    span,
                }),
            },
        ]
    }
}

fn handle_table_command(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input: PipelineData,
    row_offset: usize,
    table_view: TableView,
    term_width: Option<i64>,
) -> Result<PipelineData, ShellError> {
    let ctrlc = engine_state.ctrlc.clone();
    let config = get_config(engine_state, stack);

    match input {
        PipelineData::ExternalStream { .. } => Ok(input),
        PipelineData::Value(Value::Binary { val, .. }, ..) => Ok(PipelineData::ExternalStream {
            stdout: Some(RawStream::new(
                Box::new(if call.redirect_stdout {
                    vec![Ok(val)].into_iter()
                } else {
                    vec![Ok(format!("{}\n", nu_pretty_hex::pretty_hex(&val))
                        .as_bytes()
                        .to_vec())]
                    .into_iter()
                }),
                ctrlc,
                call.head,
                None,
            )),
            stderr: None,
            exit_code: None,
            span: call.head,
            metadata: None,
            trim_end_newline: false,
        }),
        // None of these two receive a StyleComputer because handle_row_stream() can produce it by itself using engine_state and stack.
        PipelineData::Value(Value::List { vals, .. }, metadata) => handle_row_stream(
            engine_state,
            stack,
            ListStream::from_stream(vals.into_iter(), ctrlc.clone()),
            call,
            row_offset,
            ctrlc,
            metadata,
        ),
        PipelineData::ListStream(stream, metadata) => handle_row_stream(
            engine_state,
            stack,
            stream,
            call,
            row_offset,
            ctrlc,
            metadata,
        ),
        PipelineData::Value(Value::Record { val, span }, ..) => {
            let term_width = get_width_param(term_width);

            handle_record(
                *val,
                span,
                engine_state,
                stack,
                call,
                table_view,
                term_width,
                ctrlc,
                &config,
            )
        }
        PipelineData::Value(Value::LazyRecord { val, .. }, ..) => {
            let collected = val.collect()?.into_pipeline_data();
            handle_table_command(
                engine_state,
                stack,
                call,
                collected,
                row_offset,
                table_view,
                term_width,
            )
        }
        PipelineData::Value(Value::Error { error }, ..) => {
            // Propagate this error outward, so that it goes to stderr
            // instead of stdout.
            Err(*error)
        }
        PipelineData::Value(Value::CustomValue { val, span }, ..) => {
            let base_pipeline = val.to_base_value(span)?.into_pipeline_data();
            Table.run(engine_state, stack, call, base_pipeline)
        }
        PipelineData::Value(Value::Range { val, .. }, metadata) => handle_row_stream(
            engine_state,
            stack,
            ListStream::from_stream(val.into_range_iter(ctrlc.clone())?, ctrlc.clone()),
            call,
            row_offset,
            ctrlc,
            metadata,
        ),
        x => Ok(x),
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
    ]
}

#[allow(clippy::too_many_arguments)]
fn handle_record(
    record: Record,
    span: Span,
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    table_view: TableView,
    term_width: usize,
    ctrlc: Option<Arc<AtomicBool>>,
    config: &Config,
) -> Result<PipelineData, ShellError> {
    // Create a StyleComputer to compute styles for each value in the table.
    let style_computer = &StyleComputer::from_config(engine_state, stack);
    let ctrlc1 = ctrlc.clone();

    let result = if record.is_empty() {
        create_empty_placeholder("record", term_width, engine_state, stack)
    } else {
        let opts = BuildConfig::new(ctrlc, config, style_computer, span, term_width);
        let result = build_table_kv(record, table_view, opts)?;
        match result {
            Some(output) => maybe_strip_color(output, config),
            None => report_unsuccessful_output(ctrlc1, term_width),
        }
    };

    let val = Value::String {
        val: result,
        span: call.head,
    };

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

fn build_table_kv(record: Record, table_view: TableView, opts: BuildConfig<'_>) -> StringResult {
    match table_view {
        TableView::General => JustTable::kv_table(&record.cols, &record.vals, opts),
        TableView::Expanded {
            limit,
            flatten,
            flatten_separator,
        } => {
            let sep = flatten_separator.unwrap_or_else(|| String::from(' '));
            ExpandedTable::new(limit, flatten, sep).build_map(&record, opts)
        }
        TableView::Collapsed => CollapsedTable::build(Value::record(record, opts.span()), opts),
    }
}

fn build_table_batch(
    vals: Vec<Value>,
    table_view: TableView,
    row_offset: usize,
    opts: BuildConfig<'_>,
) -> StringResult {
    match table_view {
        TableView::General => JustTable::table(&vals, row_offset, opts),
        TableView::Expanded {
            limit,
            flatten,
            flatten_separator,
        } => {
            let sep = flatten_separator.unwrap_or_else(|| String::from(' '));
            ExpandedTable::new(limit, flatten, sep).build_list(&vals, opts, row_offset)
        }
        TableView::Collapsed => {
            let span = opts.span();
            let value = Value::List { vals, span };
            CollapsedTable::build(value, opts)
        }
    }
}

fn handle_row_stream(
    engine_state: &EngineState,
    stack: &mut Stack,
    stream: ListStream,
    call: &Call,
    row_offset: usize,
    ctrlc: Option<Arc<AtomicBool>>,
    metadata: Option<Box<PipelineMetadata>>,
) -> Result<PipelineData, ShellError> {
    let stream = match metadata.as_deref() {
        // First, `ls` sources:
        Some(PipelineMetadata {
            data_source: DataSource::Ls,
        }) => {
            let config = get_config(engine_state, stack);
            let ctrlc = ctrlc.clone();
            let ls_colors_env_str = match stack.get_env_var(engine_state, "LS_COLORS") {
                Some(v) => Some(env_to_string("LS_COLORS", &v, engine_state, stack)?),
                None => None,
            };
            let ls_colors = get_ls_colors(ls_colors_env_str);

            ListStream::from_stream(
                stream.map(move |mut x| match &mut x {
                    Value::Record { val: record, .. } => {
                        for (col, val) in record.iter_mut() {
                            // Only the name column gets special colors, for now
                            if col == "name" {
                                if let Value::String { val: s, span } = val {
                                    if let Some(new_val) =
                                        render_path_name(s, &config, &ls_colors, *span)
                                    {
                                        *val = new_val;
                                    }
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
                        // Every column in the HTML theme table except 'name' is colored
                        for (col, val) in record.iter_mut() {
                            if col != "name" {
                                // Simple routine to grab the hex code, convert to a style,
                                // then place it in a new Value::String.
                                if let Value::String { val: s, span } = &*val {
                                    // .normal() just sets the text foreground color.
                                    let style = color_from_hex(s)
                                        .ok()
                                        .flatten()
                                        .map_or(nu_ansi_term::Style::default(), |c| c.normal());

                                    // Apply the style (ANSI codes) to the string
                                    *val = Value::string(style.paint(s).to_string(), *span);
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
        _ => stream,
    };

    let head = call.head;
    let width_param: Option<i64> = call.get_flag(engine_state, stack, "width")?;

    let collapse: bool = call.has_flag("collapse");

    let expand: bool = call.has_flag("expand");
    let limit: Option<usize> = call.get_flag(engine_state, stack, "expand-deep")?;
    let flatten: bool = call.has_flag("flatten");
    let flatten_separator: Option<String> =
        call.get_flag(engine_state, stack, "flatten-separator")?;

    let table_view = match (expand, collapse) {
        (_, true) => TableView::Collapsed,
        (true, _) => TableView::Expanded {
            flatten,
            flatten_separator,
            limit,
        },
        _ => TableView::General,
    };

    Ok(PipelineData::ExternalStream {
        stdout: Some(RawStream::new(
            Box::new(PagingTableCreator::new(
                head,
                stream,
                // These are passed in as a way to have PagingTable create StyleComputers
                // for the values it outputs. Because engine_state is passed in, config doesn't need to.
                engine_state.clone(),
                stack.clone(),
                ctrlc.clone(),
                row_offset,
                width_param,
                table_view,
            )),
            ctrlc,
            head,
            None,
        )),
        stderr: None,
        exit_code: None,
        span: head,
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
    row_offset: usize,
    width_param: Option<i64>,
    view: TableView,
    elements_displayed: usize,
    reached_end: bool,
}

impl PagingTableCreator {
    #[allow(clippy::too_many_arguments)]
    fn new(
        head: Span,
        stream: ListStream,
        engine_state: EngineState,
        stack: Stack,
        ctrlc: Option<Arc<AtomicBool>>,
        row_offset: usize,
        width_param: Option<i64>,
        view: TableView,
    ) -> Self {
        PagingTableCreator {
            head,
            stream,
            engine_state,
            stack,
            ctrlc,
            row_offset,
            width_param,
            view,
            elements_displayed: 0,
            reached_end: false,
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

        let config = get_config(&self.engine_state, &self.stack);
        let style_computer = StyleComputer::from_config(&self.engine_state, &self.stack);
        let term_width = get_width_param(self.width_param);

        let ctrlc = self.ctrlc.clone();
        let span = self.head;
        let opts = BuildConfig::new(ctrlc, &config, &style_computer, span, term_width);
        let view = TableView::Expanded {
            limit,
            flatten,
            flatten_separator,
        };

        build_table_batch(batch, view, self.row_offset, opts)
    }

    fn build_collapsed(&mut self, batch: Vec<Value>) -> StringResult {
        if batch.is_empty() {
            return Ok(None);
        }

        let config = get_config(&self.engine_state, &self.stack);
        let style_computer = StyleComputer::from_config(&self.engine_state, &self.stack);
        let term_width = get_width_param(self.width_param);
        let ctrlc = self.ctrlc.clone();
        let span = self.head;
        let opts = BuildConfig::new(ctrlc, &config, &style_computer, span, term_width);

        build_table_batch(batch, TableView::Collapsed, self.row_offset, opts)
    }

    fn build_general(&mut self, batch: Vec<Value>) -> StringResult {
        let term_width = get_width_param(self.width_param);
        let config = get_config(&self.engine_state, &self.stack);
        let style_computer = StyleComputer::from_config(&self.engine_state, &self.stack);
        let ctrlc = self.ctrlc.clone();
        let span = self.head;
        let row_offset = self.row_offset;
        let opts = BuildConfig::new(ctrlc, &config, &style_computer, span, term_width);

        build_table_batch(batch, TableView::General, row_offset, opts)
    }
}

impl Iterator for PagingTableCreator {
    type Item = Result<Vec<u8>, ShellError>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut batch = vec![];

        let start_time = Instant::now();

        let mut idx = 0;
        let mut reached_end = true;

        // Pull from stream until time runs out or we have enough items
        for item in self.stream.by_ref() {
            batch.push(item);
            idx += 1;

            // If we've been buffering over a second, go ahead and send out what we have so far
            if (Instant::now() - start_time).as_secs() >= 1 {
                reached_end = false;
                break;
            }

            if idx == STREAM_PAGE_SIZE {
                reached_end = false;
                break;
            }

            if nu_utils::ctrl_c::was_pressed(&self.ctrlc) {
                break;
            }
        }

        // Count how much elements were displayed and if end of stream was reached
        self.elements_displayed += idx;
        self.reached_end = self.reached_end || reached_end;

        if batch.is_empty() {
            // If this iterator has not displayed a single entry and reached its end (no more elements
            // or interrupted by ctrl+c) display as "empty list"
            return if self.elements_displayed == 0 && self.reached_end {
                // Increase elements_displayed by one so on next iteration next branch of this
                // if else triggers and terminates stream
                self.elements_displayed = 1;
                let term_width = get_width_param(self.width_param);
                let result =
                    create_empty_placeholder("list", term_width, &self.engine_state, &self.stack);
                Some(Ok(result.into_bytes()))
            } else {
                None
            };
        }

        let table = match &self.view {
            TableView::General => self.build_general(batch),
            TableView::Collapsed => self.build_collapsed(batch),
            TableView::Expanded {
                limit,
                flatten,
                flatten_separator,
            } => self.build_extended(batch, *limit, *flatten, flatten_separator.clone()),
        };

        self.row_offset += idx;

        match table {
            Ok(Some(table)) => {
                let table = maybe_strip_color(table, &get_config(&self.engine_state, &self.stack));

                let mut bytes = table.as_bytes().to_vec();
                bytes.push(b'\n'); // nu-table tables don't come with a newline on the end

                Some(Ok(bytes))
            }
            Ok(None) => {
                let msg = if nu_utils::ctrl_c::was_pressed(&self.ctrlc) {
                    "".into()
                } else {
                    // assume this failed because the table was too wide
                    // TODO: more robust error classification
                    let term_width = get_width_param(self.width_param);
                    format!("Couldn't fit table into {term_width} columns!")
                };
                Some(Ok(msg.as_bytes().to_vec()))
            }
            Err(err) => Some(Err(err)),
        }
    }
}

fn load_theme_from_config(config: &Config) -> TableTheme {
    match config.table_mode.as_str() {
        "basic" => TableTheme::basic(),
        "thin" => TableTheme::thin(),
        "light" => TableTheme::light(),
        "compact" => TableTheme::compact(),
        "with_love" => TableTheme::with_love(),
        "compact_double" => TableTheme::compact_double(),
        "rounded" => TableTheme::rounded(),
        "reinforced" => TableTheme::reinforced(),
        "heavy" => TableTheme::heavy(),
        "none" => TableTheme::none(),
        _ => TableTheme::rounded(),
    }
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

    let (style, has_metadata) = match std::fs::symlink_metadata(stripped_path.as_ref()) {
        Ok(metadata) => (
            ls_colors.style_for_path_with_metadata(stripped_path.as_ref(), Some(&metadata)),
            true,
        ),
        Err(_) => (ls_colors.style_for_path(stripped_path.as_ref()), false),
    };

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
    Some(Value::String { val, span })
}

#[derive(Debug)]
enum TableView {
    General,
    Collapsed,
    Expanded {
        limit: Option<usize>,
        flatten: bool,
        flatten_separator: Option<String>,
    },
}

#[allow(clippy::manual_filter)]
fn maybe_strip_color(output: String, config: &Config) -> String {
    // the atty is for when people do ls from vim, there should be no coloring there
    if !config.use_ansi_coloring || !atty::is(atty::Stream::Stdout) {
        // Draw the table without ansi colors
        nu_utils::strip_ansi_string_likely(output)
    } else {
        // Draw the table with ansi colors
        output
    }
}

fn create_table_config(config: &Config, comp: &StyleComputer, out: &TableOutput) -> TableConfig {
    let theme = load_theme_from_config(config);
    let footer = with_footer(config, out.with_header, out.table.count_rows());
    let line_style = lookup_separator_color(comp);
    let trim = config.trim_strategy.clone();

    TableConfig::new()
        .theme(theme)
        .with_footer(footer)
        .with_header(out.with_header)
        .with_index(out.with_index)
        .line_style(line_style)
        .trim(trim)
}

fn lookup_separator_color(style_computer: &StyleComputer) -> nu_ansi_term::Style {
    style_computer.compute("separator", &Value::nothing(Span::unknown()))
}

fn with_footer(config: &Config, with_header: bool, count_records: usize) -> bool {
    with_header && need_footer(config, count_records as u64)
}

fn need_footer(config: &Config, count_records: u64) -> bool {
    matches!(config.footer_mode, FooterMode::RowCount(limit) if count_records > limit)
        || matches!(config.footer_mode, FooterMode::Always)
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

    let cell = Cell::new(format!("empty {}", value_type_name));
    let data = vec![vec![cell]];
    let mut table = NuTable::from(data);
    table.set_cell_style((0, 0), TextStyle::default().dimmed());
    let out = TableOutput::new(table, false, false);

    let style_computer = &StyleComputer::from_config(engine_state, stack);
    let config = create_table_config(&config, style_computer, &out);

    out.table
        .draw(config, termwidth)
        .expect("Could not create empty table placeholder")
}
