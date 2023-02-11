use lscolors::{LsColors, Style};
use nu_color_config::color_from_hex;
use nu_color_config::{Alignment, StyleComputer, TextStyle};
use nu_engine::{column::get_columns, env_to_string, CallExt};
use nu_protocol::TrimStrategy;
use nu_protocol::{
    ast::{Call, PathMember},
    engine::{Command, EngineState, Stack},
    Category, Config, DataSource, Example, FooterMode, IntoPipelineData, ListStream, PipelineData,
    PipelineMetadata, RawStream, ShellError, Signature, Span, SyntaxShape, TableIndexMode, Type,
    Value,
};
use nu_table::{string_width, Table as NuTable, TableConfig, TableTheme};
use nu_utils::get_ls_colors;
use rayon::prelude::*;
use std::sync::Arc;
use std::time::Instant;
use std::{cmp::max, path::PathBuf, sync::atomic::AtomicBool};
use terminal_size::{Height, Width};
use url::Url;

const STREAM_PAGE_SIZE: usize = 1000;
const INDEX_COLUMN_NAME: &str = "index";

type NuText = (String, TextStyle);

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
        "If the table contains a column called 'index', this column is used as the table index instead of the usual continuous index"
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["display", "render"]
    }

    fn signature(&self) -> nu_protocol::Signature {
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
        let term_width = get_width_param(width_param);

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
            term_width,
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
                        Value::Record {
                            cols: vec!["a".to_string(), "b".to_string()],
                            vals: vec![Value::test_int(1), Value::test_int(2)],
                            span,
                        },
                        Value::Record {
                            cols: vec!["a".to_string(), "b".to_string()],
                            vals: vec![Value::test_int(3), Value::test_int(4)],
                            span,
                        },
                    ],
                    span,
                }),
            },
            Example {
                description: "Render data in table view (expanded)",
                example: r#"[[a b]; [1 2] [2 [4 4]]] | table --expand"#,
                result: Some(Value::List {
                    vals: vec![
                        Value::Record {
                            cols: vec!["a".to_string(), "b".to_string()],
                            vals: vec![Value::test_int(1), Value::test_int(2)],
                            span,
                        },
                        Value::Record {
                            cols: vec!["a".to_string(), "b".to_string()],
                            vals: vec![Value::test_int(3), Value::test_int(4)],
                            span,
                        },
                    ],
                    span,
                }),
            },
            Example {
                description: "Render data in table view (collapsed)",
                example: r#"[[a b]; [1 2] [2 [4 4]]] | table --collapse"#,
                result: Some(Value::List {
                    vals: vec![
                        Value::Record {
                            cols: vec!["a".to_string(), "b".to_string()],
                            vals: vec![Value::test_int(1), Value::test_int(2)],
                            span,
                        },
                        Value::Record {
                            cols: vec!["a".to_string(), "b".to_string()],
                            vals: vec![Value::test_int(3), Value::test_int(4)],
                            span,
                        },
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
    term_width: usize,
) -> Result<PipelineData, ShellError> {
    let ctrlc = engine_state.ctrlc.clone();
    let config = engine_state.get_config();

    match input {
        PipelineData::ExternalStream { .. } => Ok(input),
        PipelineData::Value(Value::Binary { val, .. }, ..) => Ok(PipelineData::ExternalStream {
            stdout: Some(RawStream::new(
                Box::new(
                    vec![Ok(format!("{}\n", nu_pretty_hex::pretty_hex(&val))
                        .as_bytes()
                        .to_vec())]
                    .into_iter(),
                ),
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
        PipelineData::Value(Value::Record { cols, vals, span }, ..) => {
            // Create a StyleComputer to compute styles for each value in the table.
            let style_computer = &StyleComputer::from_config(engine_state, stack);
            let result = match table_view {
                TableView::General => build_general_table2(
                    style_computer,
                    cols,
                    vals,
                    ctrlc.clone(),
                    config,
                    term_width,
                ),
                TableView::Expanded {
                    limit,
                    flatten,
                    flatten_separator,
                } => {
                    let sep = flatten_separator.as_deref().unwrap_or(" ");
                    build_expanded_table(
                        cols,
                        vals,
                        span,
                        ctrlc.clone(),
                        config,
                        style_computer,
                        term_width,
                        limit,
                        flatten,
                        sep,
                    )
                }
                TableView::Collapsed => {
                    build_collapsed_table(style_computer, cols, vals, config, term_width)
                }
            }?;

            let result = strip_output_color(result, config);

            let result = result.unwrap_or_else(|| {
                if nu_utils::ctrl_c::was_pressed(&ctrlc) {
                    "".into()
                } else {
                    // assume this failed because the table was too wide
                    // TODO: more robust error classification
                    format!("Couldn't fit table into {term_width} columns!")
                }
            });

            let val = Value::String {
                val: result,
                span: call.head,
            };

            Ok(val.into_pipeline_data())
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
            Err(error)
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

fn build_collapsed_table(
    style_computer: &StyleComputer,
    cols: Vec<String>,
    vals: Vec<Value>,
    config: &Config,
    term_width: usize,
) -> Result<Option<String>, ShellError> {
    let value = Value::Record {
        cols,
        vals,
        span: Span::new(0, 0),
    };

    let theme = load_theme_from_config(config);
    let table = nu_table::NuTable::new(
        value,
        true,
        term_width,
        config,
        style_computer,
        &theme,
        false,
    );

    let table = table.draw();

    Ok(table)
}

fn build_general_table2(
    style_computer: &StyleComputer,
    cols: Vec<String>,
    vals: Vec<Value>,
    ctrlc: Option<Arc<AtomicBool>>,
    config: &Config,
    term_width: usize,
) -> Result<Option<String>, ShellError> {
    let mut data = Vec::with_capacity(vals.len());
    for (column, value) in cols.into_iter().zip(vals.into_iter()) {
        if nu_utils::ctrl_c::was_pressed(&ctrlc) {
            return Ok(None);
        }

        let row = vec![
            NuTable::create_cell(column, TextStyle::default_field()),
            NuTable::create_cell(value.into_abbreviated_string(config), TextStyle::default()),
        ];

        data.push(row);
    }

    let data_len = data.len();
    let table_config = create_table_config(config, style_computer, data_len, false, false, false);

    let table = NuTable::new(data, (data_len, 2));

    let table = table.draw(table_config, term_width);

    Ok(table)
}

// The table produced by `table -e`
#[allow(clippy::too_many_arguments)]
fn build_expanded_table(
    cols: Vec<String>,
    vals: Vec<Value>,
    span: Span,
    ctrlc: Option<Arc<AtomicBool>>,
    config: &Config,
    style_computer: &StyleComputer,
    term_width: usize,
    expand_limit: Option<usize>,
    flatten: bool,
    flatten_sep: &str,
) -> Result<Option<String>, ShellError> {
    let theme = load_theme_from_config(config);

    let key_width = cols.iter().map(|col| string_width(col)).max().unwrap_or(0);

    let count_borders =
        theme.has_inner() as usize + theme.has_right() as usize + theme.has_left() as usize;
    let padding = 2;
    if key_width + count_borders + padding + padding > term_width {
        return Ok(None);
    }

    let value_width = term_width - key_width - count_borders - padding - padding;

    let mut data = Vec::with_capacity(cols.len());
    for (key, value) in cols.into_iter().zip(vals) {
        if nu_utils::ctrl_c::was_pressed(&ctrlc) {
            return Ok(None);
        }

        let is_limited = matches!(expand_limit, Some(0));
        let mut is_expanded = false;
        let value = if is_limited {
            value_to_styled_string(&value, config, style_computer).0
        } else {
            let deep = expand_limit.map(|i| i - 1);

            match value {
                Value::List { vals, .. } => {
                    let table = convert_to_table2(
                        0,
                        vals.iter(),
                        ctrlc.clone(),
                        config,
                        span,
                        style_computer,
                        deep,
                        flatten,
                        flatten_sep,
                        value_width,
                    )?;

                    match table {
                        Some((table, with_header, with_index)) => {
                            is_expanded = true;

                            let table_config = create_table_config(
                                config,
                                style_computer,
                                table.count_rows(),
                                with_header,
                                with_index,
                                false,
                            );

                            let val = table.draw(table_config, value_width);
                            match val {
                                Some(result) => result,
                                None => return Ok(None),
                            }
                        }
                        None => {
                            // it means that the list is empty
                            let value = Value::List { vals, span };
                            let text = value_to_styled_string(&value, config, style_computer).0;
                            wrap_text(&text, value_width, config)
                        }
                    }
                }
                Value::Record { cols, vals, span } => {
                    let result = build_expanded_table(
                        cols.clone(),
                        vals.clone(),
                        span,
                        ctrlc.clone(),
                        config,
                        style_computer,
                        value_width,
                        deep,
                        flatten,
                        flatten_sep,
                    )?;

                    match result {
                        Some(result) => {
                            is_expanded = true;
                            result
                        }
                        None => {
                            let failed_value = value_to_styled_string(
                                &Value::Record { cols, vals, span },
                                config,
                                style_computer,
                            );

                            wrap_text(&failed_value.0, value_width, config)
                        }
                    }
                }
                val => {
                    let text = value_to_styled_string(&val, config, style_computer).0;
                    wrap_text(&text, value_width, config)
                }
            }
        };

        // we want to have a key being aligned to 2nd line,
        // we could use Padding for it but,
        // the easiest way to do so is just push a new_line char before
        let mut key = key;
        if !key.is_empty() && is_expanded && theme.has_top_line() {
            key.insert(0, '\n');
        }

        let key = NuTable::create_cell(key, TextStyle::default_field());
        let val = NuTable::create_cell(value, TextStyle::default());

        let row = vec![key, val];
        data.push(row);
    }

    let data_len = data.len();
    let table_config = create_table_config(config, style_computer, data_len, false, false, false);
    let table = NuTable::new(data, (data_len, 2));

    let table_s = table.clone().draw(table_config.clone(), term_width);

    let table = match table_s {
        Some(s) => {
            // check whether we need to expand table or not,
            // todo: we can make it more effitient

            const EXPAND_TREASHHOLD: f32 = 0.80;

            let width = string_width(&s);
            let used_percent = width as f32 / term_width as f32;

            if width < term_width && used_percent > EXPAND_TREASHHOLD {
                let table_config = table_config.expand();
                table.draw(table_config, term_width)
            } else {
                Some(s)
            }
        }
        None => None,
    };

    Ok(table)
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
            let config = engine_state.config.clone();
            let ctrlc = ctrlc.clone();
            let ls_colors_env_str = match stack.get_env_var(engine_state, "LS_COLORS") {
                Some(v) => Some(env_to_string("LS_COLORS", &v, engine_state, stack)?),
                None => None,
            };
            let ls_colors = get_ls_colors(ls_colors_env_str);

            ListStream::from_stream(
                stream.map(move |mut x| match &mut x {
                    Value::Record { cols, vals, .. } => {
                        let mut idx = 0;

                        while idx < cols.len() {
                            // Only the name column gets special colors, for now
                            if cols[idx] == "name" {
                                if let Some(Value::String { val, span }) = vals.get(idx) {
                                    let val = render_path_name(val, &config, &ls_colors, *span);
                                    if let Some(val) = val {
                                        vals[idx] = val;
                                    }
                                }
                            }

                            idx += 1;
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
                    Value::Record { cols, vals, .. } => {
                        let mut idx = 0;
                        // Every column in the HTML theme table except 'name' is colored
                        while idx < cols.len() {
                            if cols[idx] != "name" {
                                // Simple routine to grab the hex code, convert to a style,
                                // then place it in a new Value::String.
                                if let Some(Value::String { val, span }) = vals.get(idx) {
                                    let s = match color_from_hex(val) {
                                        Ok(c) => match c {
                                            // .normal() just sets the text foreground color.
                                            Some(c) => c.normal(),
                                            None => nu_ansi_term::Style::default(),
                                        },
                                        Err(_) => nu_ansi_term::Style::default(),
                                    };
                                    vals[idx] = Value::String {
                                        // Apply the style (ANSI codes) to the string
                                        val: s.paint(val).to_string(),
                                        span: *span,
                                    };
                                }
                            }
                            idx += 1;
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
            Box::new(PagingTableCreator {
                row_offset,
                // These are passed in as a way to have PagingTable create StyleComputers
                // for the values it outputs. Because engine_state is passed in, config doesn't need to.
                engine_state: engine_state.clone(),
                stack: stack.clone(),
                ctrlc: ctrlc.clone(),
                head,
                stream,
                width_param,
                view: table_view,
            }),
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

// convert_to_table() defers all its style computations so that they can be run in parallel using par_extend().
// This structure holds the intermediate computations.
// Currently, the other table forms don't use this.
// Because of how table-specific this is, I don't think this can be pushed into StyleComputer itself.
enum DeferredStyleComputation {
    Value { value: Value },
    Header { text: String },
    RowIndex { text: String },
    Empty {},
}

impl DeferredStyleComputation {
    // This is only run inside a par_extend().
    fn compute(&self, config: &Config, style_computer: &StyleComputer) -> NuText {
        match self {
            DeferredStyleComputation::Value { value } => {
                match value {
                    // Float precision is required here.
                    Value::Float { val, .. } => (
                        format!("{:.prec$}", val, prec = config.float_precision as usize),
                        style_computer.style_primitive(value),
                    ),
                    _ => (
                        value.into_abbreviated_string(config),
                        style_computer.style_primitive(value),
                    ),
                }
            }
            DeferredStyleComputation::Header { text } => (
                text.clone(),
                TextStyle::with_style(
                    Alignment::Center,
                    style_computer
                        .compute("header", &Value::string(text.as_str(), Span::unknown())),
                ),
            ),
            DeferredStyleComputation::RowIndex { text } => (
                text.clone(),
                TextStyle::with_style(
                    Alignment::Right,
                    style_computer
                        .compute("row_index", &Value::string(text.as_str(), Span::unknown())),
                ),
            ),
            DeferredStyleComputation::Empty {} => (
                "❎".into(),
                TextStyle::with_style(
                    Alignment::Right,
                    style_computer.compute("empty", &Value::nothing(Span::unknown())),
                ),
            ),
        }
    }
}

fn convert_to_table(
    row_offset: usize,
    input: &[Value],
    ctrlc: Option<Arc<AtomicBool>>,
    config: &Config,
    head: Span,
    style_computer: &StyleComputer,
) -> Result<Option<(NuTable, bool, bool)>, ShellError> {
    let mut headers = get_columns(input);
    let mut input = input.iter().peekable();
    let with_index = match config.table_index_mode {
        TableIndexMode::Always => true,
        TableIndexMode::Never => false,
        TableIndexMode::Auto => headers.iter().any(|header| header == INDEX_COLUMN_NAME),
    };

    if input.peek().is_none() {
        return Ok(None);
    }

    let with_header = !headers.is_empty();

    if with_header && with_index {
        headers.insert(0, "#".into());
    }

    // The header with the INDEX is removed from the table headers since
    // it is added to the natural table index
    let headers: Vec<_> = headers
        .into_iter()
        .filter(|header| header != INDEX_COLUMN_NAME)
        .map(|text| DeferredStyleComputation::Header { text })
        .collect();

    let mut count_columns = headers.len();

    let mut data: Vec<Vec<_>> = if !with_header {
        Vec::new()
    } else {
        vec![headers]
    };

    // Turn each item of each row into a DeferredStyleComputation for that item.
    for (row_num, item) in input.enumerate() {
        if nu_utils::ctrl_c::was_pressed(&ctrlc) {
            return Ok(None);
        }

        if let Value::Error { error } = item {
            return Err(error.clone());
        }

        let mut row = vec![];
        if with_index {
            let text = match &item {
                Value::Record { .. } => item
                    .get_data_by_key(INDEX_COLUMN_NAME)
                    .map(|value| value.into_string("", config)),
                _ => None,
            }
            .unwrap_or_else(|| (row_num + row_offset).to_string());

            row.push(DeferredStyleComputation::RowIndex { text });
        }

        if !with_header {
            row.push(DeferredStyleComputation::Value {
                value: item.clone(),
            });
        } else {
            let skip_num = usize::from(with_index);
            // data[0] is used here because headers (the direct reference to it) has been moved.
            for header in data[0].iter().skip(skip_num) {
                if let DeferredStyleComputation::Header { text } = header {
                    row.push(match item {
                        Value::Record { .. } => {
                            let path = PathMember::String {
                                val: text.clone(),
                                span: head,
                            };
                            let val = item.clone().follow_cell_path(&[path], false, false);

                            match val {
                                Ok(val) => DeferredStyleComputation::Value { value: val },
                                Err(_) => DeferredStyleComputation::Empty {},
                            }
                        }
                        _ => DeferredStyleComputation::Value {
                            value: item.clone(),
                        },
                    });
                }
            }
        }

        count_columns = max(count_columns, row.len());

        data.push(row);
    }

    // All the computations are parallelised here.
    // NOTE: It's currently not possible to Ctrl-C out of this...
    let mut cells: Vec<Vec<_>> = Vec::with_capacity(data.len());
    data.into_par_iter()
        .map(|row| {
            let mut new_row = Vec::with_capacity(row.len());
            row.into_par_iter()
                .map(|deferred| {
                    let pair = deferred.compute(config, style_computer);

                    NuTable::create_cell(pair.0, pair.1)
                })
                .collect_into_vec(&mut new_row);
            new_row
        })
        .collect_into_vec(&mut cells);

    let count_rows = cells.len();
    let table = NuTable::new(cells, (count_rows, count_columns));

    Ok(Some((table, with_header, with_index)))
}

#[allow(clippy::too_many_arguments)]
#[allow(clippy::into_iter_on_ref)]
fn convert_to_table2<'a>(
    row_offset: usize,
    input: impl Iterator<Item = &'a Value> + ExactSizeIterator + Clone,
    ctrlc: Option<Arc<AtomicBool>>,
    config: &Config,
    head: Span,
    style_computer: &StyleComputer,
    deep: Option<usize>,
    flatten: bool,
    flatten_sep: &str,
    available_width: usize,
) -> Result<Option<(NuTable, bool, bool)>, ShellError> {
    const PADDING_SPACE: usize = 2;
    const SPLIT_LINE_SPACE: usize = 1;
    const ADDITIONAL_CELL_SPACE: usize = PADDING_SPACE + SPLIT_LINE_SPACE;
    const MIN_CELL_CONTENT_WIDTH: usize = 1;
    const TRUNCATE_CONTENT_WIDTH: usize = 3;
    const TRUNCATE_CELL_WIDTH: usize = TRUNCATE_CONTENT_WIDTH + PADDING_SPACE;

    if input.len() == 0 {
        return Ok(None);
    }

    // 2 - split lines
    let mut available_width = available_width.saturating_sub(SPLIT_LINE_SPACE + SPLIT_LINE_SPACE);
    if available_width < MIN_CELL_CONTENT_WIDTH {
        return Ok(None);
    }

    let headers = get_columns(input.clone());

    let with_index = match config.table_index_mode {
        TableIndexMode::Always => true,
        TableIndexMode::Never => false,
        TableIndexMode::Auto => headers.iter().any(|header| header == INDEX_COLUMN_NAME),
    };

    // The header with the INDEX is removed from the table headers since
    // it is added to the natural table index
    let headers: Vec<_> = headers
        .into_iter()
        .filter(|header| header != INDEX_COLUMN_NAME)
        .collect();

    let with_header = !headers.is_empty();

    let mut data = vec![vec![]; input.len()];
    if !headers.is_empty() {
        data.push(vec![]);
    };

    if with_index {
        if with_header {
            data[0].push(NuTable::create_cell(
                "#",
                header_style(style_computer, String::from("#")),
            ));
        }

        let mut last_index = 0;
        for (row, item) in input.clone().enumerate() {
            if nu_utils::ctrl_c::was_pressed(&ctrlc) {
                return Ok(None);
            }

            if let Value::Error { error } = item {
                return Err(error.clone());
            }

            let index = row + row_offset;
            let text = matches!(item, Value::Record { .. })
                .then(|| lookup_index_value(item, config).unwrap_or_else(|| index.to_string()))
                .unwrap_or_else(|| index.to_string());
            let value = make_index_string(text, style_computer);

            let value = NuTable::create_cell(value.0, value.1);

            let row = if with_header { row + 1 } else { row };
            data[row].push(value);

            last_index = index;
        }

        let column_width = string_width(&last_index.to_string());

        if column_width + ADDITIONAL_CELL_SPACE > available_width {
            available_width = 0;
        } else {
            available_width -= column_width + ADDITIONAL_CELL_SPACE;
        }
    }

    if !with_header {
        if available_width >= ADDITIONAL_CELL_SPACE {
            available_width -= PADDING_SPACE;
        }

        for (row, item) in input.into_iter().enumerate() {
            if nu_utils::ctrl_c::was_pressed(&ctrlc) {
                return Ok(None);
            }

            if let Value::Error { error } = item {
                return Err(error.clone());
            }

            let value = convert_to_table2_entry(
                item,
                config,
                &ctrlc,
                style_computer,
                deep,
                flatten,
                flatten_sep,
                available_width,
            );

            let value = NuTable::create_cell(value.0, value.1);
            data[row].push(value);
        }

        let count_columns = if with_index { 2 } else { 1 };
        let size = (data.len(), count_columns);

        let table = NuTable::new(data, size);

        return Ok(Some((table, with_header, with_index)));
    }

    if !headers.is_empty() {
        let mut pad_space = PADDING_SPACE;
        if headers.len() > 1 {
            pad_space += SPLIT_LINE_SPACE;
        }

        if available_width < pad_space {
            // there's no space for actual data so we don't return index if it's present.
            // (also see the comment after the loop)

            return Ok(None);
        }
    }

    let count_columns = headers.len();
    let mut widths = Vec::new();
    let mut truncate = false;
    let mut rendered_column = 0;
    for (col, header) in headers.into_iter().enumerate() {
        let is_last_column = col + 1 == count_columns;

        let mut pad_space = PADDING_SPACE;
        if !is_last_column {
            pad_space += SPLIT_LINE_SPACE;
        }

        let mut available = available_width - pad_space;

        let mut column_width = string_width(&header);

        if !is_last_column {
            // we need to make sure that we have a space for a next column if we use available width
            // so we might need to decrease a bit it.

            // we consider a header width be a minimum width
            let pad_space = PADDING_SPACE + TRUNCATE_CONTENT_WIDTH;

            if available > pad_space {
                // In we have no space for a next column,
                // We consider showing something better then nothing,
                // So we try to decrease the width to show at least a truncution column

                available -= pad_space;
            } else {
                truncate = true;
                break;
            }

            if available < column_width {
                truncate = true;
                break;
            }
        }

        let head_cell =
            NuTable::create_cell(header.clone(), header_style(style_computer, header.clone()));
        data[0].push(head_cell);

        for (row, item) in input.clone().enumerate() {
            if nu_utils::ctrl_c::was_pressed(&ctrlc) {
                return Ok(None);
            }

            if let Value::Error { error } = item {
                return Err(error.clone());
            }

            let mut value = create_table2_entry(
                item,
                header.as_str(),
                head,
                config,
                &ctrlc,
                style_computer,
                deep,
                flatten,
                flatten_sep,
                available,
            );

            let mut value_width = string_width(&value.0);

            if value_width > available {
                // it must only happen when a string is produced, so we can safely wrap it.
                // (it might be string table representation as well)

                value.0 = wrap_text(&value.0, available, config);
                value_width = available;
            }

            column_width = max(column_width, value_width);

            let value = NuTable::create_cell(value.0, value.1);

            data[row + 1].push(value);
        }

        if column_width > available {
            // remove the column we just inserted
            for row in &mut data {
                row.pop();
            }

            truncate = true;
            break;
        }

        widths.push(column_width);

        available_width -= pad_space + column_width;
        rendered_column += 1;
    }

    if truncate && rendered_column == 0 {
        // it means that no actual data was rendered, there might be only index present,
        // so there's no point in rendering the table.
        //
        // It's actually quite important in case it's called recursively,
        // cause we will back up to the basic table view as a string e.g. '[table 123 columns]'.
        //
        // But potentially if its reached as a 1st called function we might would love to see the index.

        return Ok(None);
    }

    if truncate {
        if available_width < TRUNCATE_CELL_WIDTH {
            // back up by removing last column.
            // it's LIKELY that removing only 1 column will leave us enough space for a shift column.

            while let Some(width) = widths.pop() {
                for row in &mut data {
                    row.pop();
                }

                available_width += width + PADDING_SPACE;
                if !widths.is_empty() {
                    available_width += SPLIT_LINE_SPACE;
                }

                if available_width > TRUNCATE_CELL_WIDTH {
                    break;
                }
            }
        }

        // this must be a RARE case or even NEVER happen,
        // but we do check it just in case.
        if available_width < TRUNCATE_CELL_WIDTH {
            return Ok(None);
        }

        let is_last_column = widths.len() == count_columns;
        if !is_last_column {
            let shift = NuTable::create_cell(String::from("..."), TextStyle::default());
            for row in &mut data {
                row.push(shift.clone());
            }

            widths.push(3);
        }
    }

    let count_columns = widths.len() + with_index as usize;
    let count_rows = data.len();
    let size = (count_rows, count_columns);

    let table = NuTable::new(data, size);

    Ok(Some((table, with_header, with_index)))
}

fn lookup_index_value(item: &Value, config: &Config) -> Option<String> {
    item.get_data_by_key(INDEX_COLUMN_NAME)
        .map(|value| value.into_string("", config))
}

fn header_style(style_computer: &StyleComputer, header: String) -> TextStyle {
    let style = style_computer.compute("header", &Value::string(header.as_str(), Span::unknown()));
    TextStyle {
        alignment: Alignment::Center,
        color_style: Some(style),
    }
}

#[allow(clippy::too_many_arguments)]
fn create_table2_entry(
    item: &Value,
    header: &str,
    head: Span,
    config: &Config,
    ctrlc: &Option<Arc<AtomicBool>>,
    style_computer: &StyleComputer,
    deep: Option<usize>,
    flatten: bool,
    flatten_sep: &str,
    width: usize,
) -> NuText {
    match item {
        Value::Record { .. } => {
            let val = header.to_owned();
            let path = PathMember::String { val, span: head };
            let val = item.clone().follow_cell_path(&[path], false, false);

            match val {
                Ok(val) => convert_to_table2_entry(
                    &val,
                    config,
                    ctrlc,
                    style_computer,
                    deep,
                    flatten,
                    flatten_sep,
                    width,
                ),
                Err(_) => error_sign(style_computer),
            }
        }
        _ => convert_to_table2_entry(
            item,
            config,
            ctrlc,
            style_computer,
            deep,
            flatten,
            flatten_sep,
            width,
        ),
    }
}

fn error_sign(style_computer: &StyleComputer) -> (String, TextStyle) {
    make_styled_string(style_computer, String::from("❎"), None, 0)
}

fn wrap_text(text: &str, width: usize, config: &Config) -> String {
    nu_table::string_wrap(text, width, is_cfg_trim_keep_words(config))
}

#[allow(clippy::too_many_arguments)]
fn convert_to_table2_entry(
    item: &Value,
    config: &Config,
    ctrlc: &Option<Arc<AtomicBool>>,
    // This is passed in, even though it could be retrieved from config,
    // to save reallocation (because it's presumably being used upstream).
    style_computer: &StyleComputer,
    deep: Option<usize>,
    flatten: bool,
    flatten_sep: &str,
    width: usize,
) -> NuText {
    let is_limit_reached = matches!(deep, Some(0));
    if is_limit_reached {
        return value_to_styled_string(item, config, style_computer);
    }

    match &item {
        Value::Record { span, cols, vals } => {
            if cols.is_empty() && vals.is_empty() {
                return value_to_styled_string(item, config, style_computer);
            }

            // we verify what is the structure of a Record cause it might represent

            let table = build_expanded_table(
                cols.clone(),
                vals.clone(),
                *span,
                ctrlc.clone(),
                config,
                style_computer,
                width,
                deep.map(|i| i - 1),
                flatten,
                flatten_sep,
            );

            match table {
                Ok(Some(table)) => (table, TextStyle::default()),
                _ => value_to_styled_string(item, config, style_computer),
            }
        }
        Value::List { vals, span } => {
            if flatten {
                let is_simple_list = vals
                    .iter()
                    .all(|v| !matches!(v, Value::Record { .. } | Value::List { .. }));

                if is_simple_list {
                    return convert_value_list_to_string(vals, config, style_computer, flatten_sep);
                }
            }

            let table = convert_to_table2(
                0,
                vals.iter(),
                ctrlc.clone(),
                config,
                *span,
                style_computer,
                deep.map(|i| i - 1),
                flatten,
                flatten_sep,
                width,
            );

            let (table, whead, windex) = match table {
                Ok(Some(out)) => out,
                _ => return value_to_styled_string(item, config, style_computer),
            };

            let count_rows = table.count_rows();
            let table_config =
                create_table_config(config, style_computer, count_rows, whead, windex, false);

            let table = table.draw(table_config, usize::MAX);
            match table {
                Some(table) => (table, TextStyle::default()),
                None => value_to_styled_string(item, config, style_computer),
            }
        }
        _ => value_to_styled_string(item, config, style_computer), // unknown type.
    }
}

fn convert_value_list_to_string(
    vals: &[Value],
    config: &Config,
    // This is passed in, even though it could be retrieved from config,
    // to save reallocation (because it's presumably being used upstream).
    style_computer: &StyleComputer,
    flatten_sep: &str,
) -> NuText {
    let mut buf = Vec::new();
    for value in vals {
        let (text, _) = value_to_styled_string(value, config, style_computer);

        buf.push(text);
    }
    let text = buf.join(flatten_sep);
    (text, TextStyle::default())
}

fn value_to_styled_string(
    value: &Value,
    config: &Config,
    // This is passed in, even though it could be retrieved from config,
    // to save reallocation (because it's presumably being used upstream).
    style_computer: &StyleComputer,
) -> NuText {
    let float_precision = config.float_precision as usize;
    make_styled_string(
        style_computer,
        value.into_abbreviated_string(config),
        Some(value),
        float_precision,
    )
}

fn make_styled_string(
    style_computer: &StyleComputer,
    text: String,
    value: Option<&Value>, // None represents table holes.
    float_precision: usize,
) -> NuText {
    match value {
        Some(value) => {
            match value {
                Value::Float { .. } => {
                    // set dynamic precision from config
                    let precise_number = match convert_with_precision(&text, float_precision) {
                        Ok(num) => num,
                        Err(e) => e.to_string(),
                    };
                    (precise_number, style_computer.style_primitive(value))
                }
                _ => (text, style_computer.style_primitive(value)),
            }
        }
        None => {
            // Though holes are not the same as null, the closure for "empty" is passed a null anyway.
            (
                text,
                TextStyle::with_style(
                    Alignment::Center,
                    style_computer.compute("empty", &Value::nothing(Span::unknown())),
                ),
            )
        }
    }
}

fn make_index_string(text: String, style_computer: &StyleComputer) -> NuText {
    let style = style_computer.compute("row_index", &Value::string(text.as_str(), Span::unknown()));
    (text, TextStyle::with_style(Alignment::Right, style))
}

fn convert_with_precision(val: &str, precision: usize) -> Result<String, ShellError> {
    // vall will always be a f64 so convert it with precision formatting
    let val_float = match val.trim().parse::<f64>() {
        Ok(f) => f,
        Err(e) => {
            return Err(ShellError::GenericError(
                format!("error converting string [{}] to f64", &val),
                "".to_string(),
                None,
                Some(e.to_string()),
                Vec::new(),
            ));
        }
    };
    Ok(format!("{val_float:.precision$}"))
}

fn is_cfg_trim_keep_words(config: &Config) -> bool {
    matches!(
        config.trim_strategy,
        TrimStrategy::Wrap {
            try_to_keep_words: true
        }
    )
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
}

impl PagingTableCreator {
    fn build_extended(
        &mut self,
        batch: &[Value],
        limit: Option<usize>,
        flatten: bool,
        flatten_separator: Option<String>,
    ) -> Result<Option<String>, ShellError> {
        if batch.is_empty() {
            return Ok(None);
        }

        let config = self.engine_state.get_config();
        let style_computer = StyleComputer::from_config(&self.engine_state, &self.stack);
        let term_width = get_width_param(self.width_param);

        let table = convert_to_table2(
            self.row_offset,
            batch.iter(),
            self.ctrlc.clone(),
            config,
            self.head,
            &style_computer,
            limit,
            flatten,
            flatten_separator.as_deref().unwrap_or(" "),
            term_width,
        )?;

        let (table, with_header, with_index) = match table {
            Some(table) => table,
            None => return Ok(None),
        };

        let table_config = create_table_config(
            config,
            &style_computer,
            table.count_rows(),
            with_header,
            with_index,
            false,
        );

        let table_s = table.clone().draw(table_config.clone(), term_width);

        let table = match table_s {
            Some(s) => {
                // check whether we need to expand table or not,
                // todo: we can make it more efficient

                const EXPAND_THRESHOLD: f32 = 0.80;

                let width = string_width(&s);
                let used_percent = width as f32 / term_width as f32;

                if width < term_width && used_percent > EXPAND_THRESHOLD {
                    let table_config = table_config.expand();
                    table.draw(table_config, term_width)
                } else {
                    Some(s)
                }
            }
            None => None,
        };

        Ok(table)
    }

    fn build_collapsed(&mut self, batch: Vec<Value>) -> Result<Option<String>, ShellError> {
        if batch.is_empty() {
            return Ok(None);
        }

        let config = self.engine_state.get_config();
        let style_computer = StyleComputer::from_config(&self.engine_state, &self.stack);
        let theme = load_theme_from_config(config);
        let term_width = get_width_param(self.width_param);
        let need_footer = matches!(config.footer_mode, FooterMode::RowCount(limit) if batch.len() as u64 > limit)
            || matches!(config.footer_mode, FooterMode::Always);
        let value = Value::List {
            vals: batch,
            span: Span::new(0, 0),
        };

        let table = nu_table::NuTable::new(
            value,
            true,
            term_width,
            config,
            &style_computer,
            &theme,
            need_footer,
        );

        Ok(table.draw())
    }

    fn build_general(&mut self, batch: &[Value]) -> Result<Option<String>, ShellError> {
        let term_width = get_width_param(self.width_param);
        let config = &self.engine_state.get_config();
        let style_computer = StyleComputer::from_config(&self.engine_state, &self.stack);
        let table = convert_to_table(
            self.row_offset,
            batch,
            self.ctrlc.clone(),
            config,
            self.head,
            &style_computer,
        )?;

        let (table, with_header, with_index) = match table {
            Some(table) => table,
            None => return Ok(None),
        };

        let table_config = create_table_config(
            config,
            &style_computer,
            table.count_rows(),
            with_header,
            with_index,
            false,
        );

        let table = table.draw(table_config, term_width);

        Ok(table)
    }
}

impl Iterator for PagingTableCreator {
    type Item = Result<Vec<u8>, ShellError>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut batch = vec![];

        let start_time = Instant::now();

        let mut idx = 0;

        // Pull from stream until time runs out or we have enough items
        for item in self.stream.by_ref() {
            batch.push(item);
            idx += 1;

            // If we've been buffering over a second, go ahead and send out what we have so far
            if (Instant::now() - start_time).as_secs() >= 1 {
                break;
            }

            if idx == STREAM_PAGE_SIZE {
                break;
            }

            if nu_utils::ctrl_c::was_pressed(&self.ctrlc) {
                break;
            }
        }

        if batch.is_empty() {
            return None;
        }

        let table = match &self.view {
            TableView::General => self.build_general(&batch),
            TableView::Collapsed => self.build_collapsed(batch),
            TableView::Expanded {
                limit,
                flatten,
                flatten_separator,
            } => self.build_extended(&batch, *limit, *flatten, flatten_separator.clone()),
        };

        self.row_offset += idx;

        match table {
            Ok(Some(table)) => {
                let table = strip_output_color(Some(table), self.engine_state.get_config())
                    .expect("must never happen");

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
        "basic" => nu_table::TableTheme::basic(),
        "thin" => nu_table::TableTheme::thin(),
        "light" => nu_table::TableTheme::light(),
        "compact" => nu_table::TableTheme::compact(),
        "with_love" => nu_table::TableTheme::with_love(),
        "compact_double" => nu_table::TableTheme::compact_double(),
        "rounded" => nu_table::TableTheme::rounded(),
        "reinforced" => nu_table::TableTheme::reinforced(),
        "heavy" => nu_table::TableTheme::heavy(),
        "none" => nu_table::TableTheme::none(),
        _ => nu_table::TableTheme::rounded(),
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

    let ansi_style = style
        .map(Style::to_crossterm_style)
        // .map(ToNuAnsiStyle::to_nu_ansi_style)
        .unwrap_or_default();

    let full_path = PathBuf::from(stripped_path.as_ref())
        .canonicalize()
        .unwrap_or_else(|_| PathBuf::from(stripped_path.as_ref()));

    let full_path_link = make_clickable_link(
        full_path.display().to_string(),
        Some(path),
        show_clickable_links,
    );

    let val = ansi_style.apply(full_path_link).to_string();
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
fn strip_output_color(output: Option<String>, config: &Config) -> Option<String> {
    match output {
        Some(output) => {
            // the atty is for when people do ls from vim, there should be no coloring there
            if !config.use_ansi_coloring || !atty::is(atty::Stream::Stdout) {
                // Draw the table without ansi colors
                Some(nu_utils::strip_ansi_string_likely(output))
            } else {
                // Draw the table with ansi colors
                Some(output)
            }
        }
        None => None,
    }
}

fn create_table_config(
    config: &Config,
    style_computer: &StyleComputer,
    count_records: usize,
    with_header: bool,
    with_index: bool,
    expand: bool,
) -> TableConfig {
    let theme = load_theme_from_config(config);
    let append_footer = with_footer(config, with_header, count_records);

    let mut table_cfg = TableConfig::new(theme, with_header, with_index, append_footer);

    table_cfg = table_cfg.splitline_style(lookup_separator_color(style_computer));

    if expand {
        table_cfg = table_cfg.expand();
    }

    table_cfg.trim(config.trim_strategy.clone())
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
