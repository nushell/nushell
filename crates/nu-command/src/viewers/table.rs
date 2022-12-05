use lscolors::{LsColors, Style};
use nu_color_config::{color_from_hex, get_color_config, style_primitive};
use nu_engine::{column::get_columns, env_to_string, CallExt};
use nu_protocol::{
    ast::{Call, PathMember},
    engine::{Command, EngineState, Stack, StateWorkingSet},
    format_error, Category, Config, DataSource, Example, FooterMode, IntoPipelineData, ListStream,
    PipelineData, PipelineMetadata, RawStream, ShellError, Signature, Span, SyntaxShape,
    TableIndexMode, Value,
};
use nu_table::{string_width, Alignment, Alignments, Table as NuTable, TableTheme, TextStyle};
use nu_utils::get_ls_colors;
use std::sync::Arc;
use std::time::Instant;
use std::{
    cmp::max,
    collections::HashMap,
    path::PathBuf,
    sync::atomic::{AtomicBool, Ordering},
};
use terminal_size::{Height, Width};
use url::Url;

const STREAM_PAGE_SIZE: usize = 1000;
const STREAM_TIMEOUT_CHECK_INTERVAL: usize = 100;
const INDEX_COLUMN_NAME: &str = "index";

type NuText = (String, TextStyle);
type NuColorMap = HashMap<String, nu_ansi_term::Style>;

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
                "sets a seperator when 'flatten' used",
                None,
            )
            .switch(
                "collapse",
                "expand the table structure in colapse mode.\nBe aware collapse mode currently doesn't support width controll",
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
            )),
            stderr: None,
            exit_code: None,
            span: call.head,
            metadata: None,
            trim_end_newline: false,
        }),
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
            let result = match table_view {
                TableView::General => {
                    build_general_table2(cols, vals, ctrlc.clone(), config, term_width)
                }
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
                        term_width,
                        limit,
                        flatten,
                        sep,
                    )
                }
                TableView::Collapsed => build_collapsed_table(cols, vals, config, term_width),
            }?;

            let ctrl_c_was_triggered = || match &ctrlc {
                Some(ctrlc) => ctrlc.load(Ordering::SeqCst),
                None => false,
            };

            let result = result.unwrap_or_else(|| {
                if ctrl_c_was_triggered() {
                    "".into()
                } else {
                    // assume this failed because the table was too wide
                    // TODO: more robust error classification
                    format!("Couldn't fit table into {} columns!", term_width)
                }
            });

            let val = Value::String {
                val: result,
                span: call.head,
            };

            Ok(val.into_pipeline_data())
        }
        PipelineData::Value(Value::Error { error }, ..) => {
            let working_set = StateWorkingSet::new(engine_state);
            Ok(Value::String {
                val: format_error(&working_set, &error),
                span: call.head,
            }
            .into_pipeline_data())
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
        Value::string("basic", Span::test_data()),
        Value::string("compact", Span::test_data()),
        Value::string("compact_double", Span::test_data()),
        Value::string("default", Span::test_data()),
        Value::string("heavy", Span::test_data()),
        Value::string("light", Span::test_data()),
        Value::string("none", Span::test_data()),
        Value::string("reinforced", Span::test_data()),
        Value::string("rounded", Span::test_data()),
        Value::string("thin", Span::test_data()),
        Value::string("with_love", Span::test_data()),
    ]
}

fn build_collapsed_table(
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

    let color_hm = get_color_config(config);
    let theme = load_theme_from_config(config);
    let table = nu_table::NuTable::new(value, true, term_width, config, &color_hm, &theme, false);

    let table = table.draw();

    Ok(table)
}

fn build_general_table2(
    cols: Vec<String>,
    vals: Vec<Value>,
    ctrlc: Option<Arc<AtomicBool>>,
    config: &Config,
    term_width: usize,
) -> Result<Option<String>, ShellError> {
    let mut data = Vec::with_capacity(vals.len());
    for (column, value) in cols.into_iter().zip(vals.into_iter()) {
        // handle CTRLC event
        if let Some(ctrlc) = &ctrlc {
            if ctrlc.load(Ordering::SeqCst) {
                return Ok(None);
            }
        }

        let row = vec![
            NuTable::create_cell(column, TextStyle::default_field()),
            NuTable::create_cell(value.into_abbreviated_string(config), TextStyle::default()),
        ];

        data.push(row);
    }

    let data_len = data.len();
    let table = NuTable::new(data, (data_len, 2), term_width, false, false);

    let theme = load_theme_from_config(config);
    let color_hm = get_color_config(config);

    let table = table.draw_table(
        config,
        &color_hm,
        Alignments::default(),
        &theme,
        term_width,
        false,
    );

    Ok(table)
}

#[allow(clippy::too_many_arguments)]
fn build_expanded_table(
    cols: Vec<String>,
    vals: Vec<Value>,
    span: Span,
    ctrlc: Option<Arc<AtomicBool>>,
    config: &Config,
    term_width: usize,
    expand_limit: Option<usize>,
    flatten: bool,
    flatten_sep: &str,
) -> Result<Option<String>, ShellError> {
    let theme = load_theme_from_config(config);
    let color_hm = get_color_config(config);
    let alignments = Alignments::default();

    // calculate the width of a key part + the rest of table so we know the rest of the table width available for value.
    let key_width = cols.iter().map(|col| string_width(col)).max().unwrap_or(0);
    let key = NuTable::create_cell(" ".repeat(key_width), TextStyle::default());
    let key_table = NuTable::new(vec![vec![key]], (1, 2), term_width, false, false);
    let key_width = key_table
        .draw_table(config, &color_hm, alignments, &theme, usize::MAX, false)
        .map(|table| string_width(&table))
        .unwrap_or(0);

    // 3 - count borders (left, center, right)
    // 2 - padding
    if key_width + 3 + 2 > term_width {
        return Ok(None);
    }

    let remaining_width = term_width - key_width - 3 - 2;

    let mut data = Vec::with_capacity(cols.len());
    for (key, value) in cols.into_iter().zip(vals) {
        // handle CTRLC event
        if let Some(ctrlc) = &ctrlc {
            if ctrlc.load(Ordering::SeqCst) {
                return Ok(None);
            }
        }

        let is_limited = matches!(expand_limit, Some(0));
        let mut is_expanded = false;
        let value = if is_limited {
            value_to_styled_string(&value, config, &color_hm).0
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
                        &color_hm,
                        &theme,
                        deep,
                        flatten,
                        flatten_sep,
                        remaining_width,
                    )?;

                    match table {
                        Some(mut table) => {
                            // controll width via removing table columns.
                            let theme = load_theme_from_config(config);
                            table.truncate(remaining_width, &theme);

                            is_expanded = true;

                            let val = table.draw_table(
                                config,
                                &color_hm,
                                alignments,
                                &theme,
                                remaining_width,
                                false,
                            );
                            match val {
                                Some(result) => result,
                                None => return Ok(None),
                            }
                        }
                        None => {
                            // it means that the list is empty
                            let value = Value::List { vals, span };
                            value_to_styled_string(&value, config, &color_hm).0
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
                        remaining_width,
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
                                &color_hm,
                            );

                            nu_table::wrap_string(&failed_value.0, remaining_width)
                        }
                    }
                }
                val => {
                    let text = value_to_styled_string(&val, config, &color_hm).0;
                    nu_table::wrap_string(&text, remaining_width)
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
    let table = NuTable::new(data, (data_len, 2), term_width, false, false);

    let table_s = table
        .clone()
        .draw_table(config, &color_hm, alignments, &theme, term_width, false);

    let table = match table_s {
        Some(s) => {
            // check whether we need to expand table or not,
            // todo: we can make it more effitient

            const EXPAND_TREASHHOLD: f32 = 0.80;

            let width = string_width(&s);
            let used_percent = width as f32 / term_width as f32;

            if width < term_width && used_percent > EXPAND_TREASHHOLD {
                table.draw_table(config, &color_hm, alignments, &theme, term_width, true)
            } else {
                Some(s)
            }
        }
        None => None,
    };

    Ok(table)
}

#[allow(clippy::too_many_arguments)]
fn handle_row_stream(
    engine_state: &EngineState,
    stack: &mut Stack,
    stream: ListStream,
    call: &Call,
    row_offset: usize,
    ctrlc: Option<Arc<AtomicBool>>,
    metadata: Option<PipelineMetadata>,
) -> Result<PipelineData, nu_protocol::ShellError> {
    let stream = match metadata {
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
        // Next, `into html -l` sources:
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
                config: engine_state.get_config().clone(),
                ctrlc: ctrlc.clone(),
                head,
                stream,
                width_param,
                view: table_view,
            }),
            ctrlc,
            head,
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

fn convert_to_table(
    row_offset: usize,
    input: &[Value],
    ctrlc: Option<Arc<AtomicBool>>,
    config: &Config,
    head: Span,
    termwidth: usize,
    color_hm: &NuColorMap,
) -> Result<Option<NuTable>, ShellError> {
    let mut headers = get_columns(input);
    let mut input = input.iter().peekable();
    let float_precision = config.float_precision as usize;
    let with_index = match config.table_index_mode {
        TableIndexMode::Always => true,
        TableIndexMode::Never => false,
        TableIndexMode::Auto => headers.iter().any(|header| header == INDEX_COLUMN_NAME),
    };

    if input.peek().is_none() {
        return Ok(None);
    }

    if !headers.is_empty() && with_index {
        headers.insert(0, "#".into());
    }

    // The header with the INDEX is removed from the table headers since
    // it is added to the natural table index
    let headers: Vec<_> = headers
        .into_iter()
        .filter(|header| header != INDEX_COLUMN_NAME)
        .map(|text| {
            NuTable::create_cell(
                text,
                TextStyle {
                    alignment: Alignment::Center,
                    color_style: Some(color_hm["header"]),
                },
            )
        })
        .collect();

    let with_header = !headers.is_empty();
    let mut count_columns = headers.len();

    let mut data: Vec<Vec<_>> = if headers.is_empty() {
        Vec::new()
    } else {
        vec![headers]
    };

    for (row_num, item) in input.enumerate() {
        if let Some(ctrlc) = &ctrlc {
            if ctrlc.load(Ordering::SeqCst) {
                return Ok(None);
            }
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

            let value = make_index_string(text, color_hm);
            let value = NuTable::create_cell(value.0, value.1);

            row.push(value);
        }

        if !with_header {
            let text = item.into_abbreviated_string(config);
            let text_type = item.get_type().to_string();
            let value = make_styled_string(text, &text_type, color_hm, float_precision);
            let value = NuTable::create_cell(value.0, value.1);

            row.push(value);
        } else {
            let skip_num = usize::from(with_index);
            for header in data[0].iter().skip(skip_num) {
                let value =
                    create_table2_entry_basic(item, header.as_ref(), head, config, color_hm);
                let value = NuTable::create_cell(value.0, value.1);
                row.push(value);
            }
        }

        count_columns = max(count_columns, row.len());

        data.push(row);
    }

    let count_rows = data.len();
    let table = NuTable::new(
        data,
        (count_rows, count_columns),
        termwidth,
        with_header,
        with_index,
    );

    Ok(Some(table))
}

#[allow(clippy::too_many_arguments)]
#[allow(clippy::into_iter_on_ref)]
fn convert_to_table2<'a>(
    row_offset: usize,
    input: impl Iterator<Item = &'a Value> + ExactSizeIterator + Clone,
    ctrlc: Option<Arc<AtomicBool>>,
    config: &Config,
    head: Span,
    color_hm: &NuColorMap,
    theme: &TableTheme,
    deep: Option<usize>,
    flatten: bool,
    flatten_sep: &str,
    available_width: usize,
) -> Result<Option<NuTable>, ShellError> {
    const PADDING_SPACE: usize = 2;
    const SPLIT_LINE_SPACE: usize = 1;
    const ADDITIONAL_CELL_SPACE: usize = PADDING_SPACE + SPLIT_LINE_SPACE;
    const TRUNCATE_CELL_WIDTH: usize = 3;
    const MIN_CELL_CONTENT_WIDTH: usize = 1;
    const OK_CELL_CONTENT_WIDTH: usize = 25;

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
        let mut column_width = 0;

        if with_header {
            data[0].push(NuTable::create_cell("#", header_style(color_hm)));
        }

        for (row, item) in input.clone().into_iter().enumerate() {
            if let Some(ctrlc) = &ctrlc {
                if ctrlc.load(Ordering::SeqCst) {
                    return Ok(None);
                }
            }

            if let Value::Error { error } = item {
                return Err(error.clone());
            }

            let index = row + row_offset;
            let text = matches!(item, Value::Record { .. })
                .then(|| lookup_index_value(item, config).unwrap_or_else(|| index.to_string()))
                .unwrap_or_else(|| index.to_string());

            let value = make_index_string(text, color_hm);

            let width = string_width(&value.0);
            column_width = max(column_width, width);

            let value = NuTable::create_cell(value.0, value.1);

            let row = if with_header { row + 1 } else { row };
            data[row].push(value);
        }

        if column_width + ADDITIONAL_CELL_SPACE > available_width {
            available_width = 0;
        } else {
            available_width -= column_width + ADDITIONAL_CELL_SPACE;
        }
    }

    if !with_header {
        for (row, item) in input.into_iter().enumerate() {
            if let Some(ctrlc) = &ctrlc {
                if ctrlc.load(Ordering::SeqCst) {
                    return Ok(None);
                }
            }

            if let Value::Error { error } = item {
                return Err(error.clone());
            }

            let value = convert_to_table2_entry(
                item,
                config,
                &ctrlc,
                color_hm,
                theme,
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
        let table = NuTable::new(data, size, usize::MAX, with_header, with_index);

        return Ok(Some(table));
    }

    let mut widths = Vec::new();
    let mut truncate = false;
    let count_columns = headers.len();
    for (col, header) in headers.into_iter().enumerate() {
        let is_last_col = col + 1 == count_columns;

        let mut nessary_space = PADDING_SPACE;
        if !is_last_col {
            nessary_space += SPLIT_LINE_SPACE;
        }

        if available_width == 0 || available_width <= nessary_space {
            // MUST NEVER HAPPEN (ideally)
            // but it does...

            truncate = true;
            break;
        }

        available_width -= nessary_space;

        let mut column_width = string_width(&header);

        data[0].push(NuTable::create_cell(&header, header_style(color_hm)));

        for (row, item) in input.clone().into_iter().enumerate() {
            if let Some(ctrlc) = &ctrlc {
                if ctrlc.load(Ordering::SeqCst) {
                    return Ok(None);
                }
            }

            if let Value::Error { error } = item {
                return Err(error.clone());
            }

            let value = create_table2_entry(
                item,
                &header,
                head,
                config,
                &ctrlc,
                color_hm,
                theme,
                deep,
                flatten,
                flatten_sep,
                available_width,
            );

            let value_width = string_width(&value.0);
            column_width = max(column_width, value_width);

            let value = NuTable::create_cell(value.0, value.1);

            data[row + 1].push(value);
        }

        if column_width >= available_width
            || (!is_last_col && column_width + nessary_space >= available_width)
        {
            // so we try to do soft landing
            // by doing a truncating in case there will be enough space for it.

            column_width = string_width(&header);

            for (row, item) in input.clone().into_iter().enumerate() {
                if let Some(ctrlc) = &ctrlc {
                    if ctrlc.load(Ordering::SeqCst) {
                        return Ok(None);
                    }
                }

                let value = create_table2_entry_basic(item, &header, head, config, color_hm);
                let value = wrap_nu_text(value, available_width);

                let value_width = string_width(&value.0);
                column_width = max(column_width, value_width);

                let value = NuTable::create_cell(value.0, value.1);

                *data[row + 1].last_mut().expect("unwrap") = value;
            }
        }

        let is_suitable_for_wrap =
            available_width >= string_width(&header) && available_width >= OK_CELL_CONTENT_WIDTH;
        if column_width >= available_width && is_suitable_for_wrap {
            // so we try to do soft landing ONCE AGAIN
            // but including a wrap

            column_width = string_width(&header);

            for (row, item) in input.clone().into_iter().enumerate() {
                if let Some(ctrlc) = &ctrlc {
                    if ctrlc.load(Ordering::SeqCst) {
                        return Ok(None);
                    }
                }

                let value = create_table2_entry_basic(item, &header, head, config, color_hm);
                let value = wrap_nu_text(value, OK_CELL_CONTENT_WIDTH);

                let value = NuTable::create_cell(value.0, value.1);

                *data[row + 1].last_mut().expect("unwrap") = value;
            }
        }

        if column_width > available_width {
            // remove just added column
            for row in &mut data {
                row.pop();
            }

            available_width += nessary_space;

            truncate = true;
            break;
        }

        available_width -= column_width;
        widths.push(column_width);
    }

    if truncate {
        if available_width <= TRUNCATE_CELL_WIDTH + PADDING_SPACE {
            // back up by removing last column.
            // it's ALWAYS MUST has us enough space for a shift column
            while let Some(width) = widths.pop() {
                for row in &mut data {
                    row.pop();
                }

                available_width += width + PADDING_SPACE + SPLIT_LINE_SPACE;

                if available_width > TRUNCATE_CELL_WIDTH + PADDING_SPACE {
                    break;
                }
            }
        }

        // this must be a RARE case or even NEVER happen,
        // but we do check it just in case.
        if widths.is_empty() {
            return Ok(None);
        }

        let shift = NuTable::create_cell(String::from("..."), TextStyle::default());
        for row in &mut data {
            row.push(shift.clone());
        }

        widths.push(3);
    }

    let count_columns = widths.len() + with_index as usize;
    let count_rows = data.len();
    let size = (count_rows, count_columns);

    let table = NuTable::new(data, size, usize::MAX, with_header, with_index);

    Ok(Some(table))
}

fn lookup_index_value(item: &Value, config: &Config) -> Option<String> {
    item.get_data_by_key(INDEX_COLUMN_NAME)
        .map(|value| value.into_string("", config))
}

fn header_style(color_hm: &NuColorMap) -> TextStyle {
    TextStyle {
        alignment: Alignment::Center,
        color_style: Some(color_hm["header"]),
    }
}

#[allow(clippy::too_many_arguments)]
fn create_table2_entry_basic(
    item: &Value,
    header: &str,
    head: Span,
    config: &Config,
    color_hm: &NuColorMap,
) -> NuText {
    match item {
        Value::Record { .. } => {
            let val = header.to_owned();
            let path = PathMember::String { val, span: head };
            let val = item.clone().follow_cell_path(&[path], false);

            match val {
                Ok(val) => value_to_styled_string(&val, config, color_hm),
                Err(_) => error_sign(color_hm),
            }
        }
        _ => value_to_styled_string(item, config, color_hm),
    }
}

#[allow(clippy::too_many_arguments)]
fn create_table2_entry(
    item: &Value,
    header: &str,
    head: Span,
    config: &Config,
    ctrlc: &Option<Arc<AtomicBool>>,
    color_hm: &NuColorMap,
    theme: &TableTheme,
    deep: Option<usize>,
    flatten: bool,
    flatten_sep: &str,
    width: usize,
) -> NuText {
    match item {
        Value::Record { .. } => {
            let val = header.to_owned();
            let path = PathMember::String { val, span: head };
            let val = item.clone().follow_cell_path(&[path], false);

            match val {
                Ok(val) => convert_to_table2_entry(
                    &val,
                    config,
                    ctrlc,
                    color_hm,
                    theme,
                    deep,
                    flatten,
                    flatten_sep,
                    width,
                ),
                Err(_) => wrap_nu_text(error_sign(color_hm), width),
            }
        }
        _ => convert_to_table2_entry(
            item,
            config,
            ctrlc,
            color_hm,
            theme,
            deep,
            flatten,
            flatten_sep,
            width,
        ),
    }
}

fn error_sign(color_hm: &HashMap<String, nu_ansi_term::Style>) -> (String, TextStyle) {
    make_styled_string(String::from("❎"), "empty", color_hm, 0)
}

fn wrap_nu_text(mut text: NuText, width: usize) -> NuText {
    text.0 = nu_table::wrap_string(&text.0, width);
    text
}

#[allow(clippy::too_many_arguments)]
fn convert_to_table2_entry(
    item: &Value,
    config: &Config,
    ctrlc: &Option<Arc<AtomicBool>>,
    color_hm: &NuColorMap,
    theme: &TableTheme,
    deep: Option<usize>,
    flatten: bool,
    flatten_sep: &str,
    width: usize,
) -> NuText {
    let is_limit_reached = matches!(deep, Some(0));
    if is_limit_reached {
        return wrap_nu_text(value_to_styled_string(item, config, color_hm), width);
    }

    match &item {
        Value::Record { span, cols, vals } => {
            if cols.is_empty() && vals.is_empty() {
                wrap_nu_text(value_to_styled_string(item, config, color_hm), width)
            } else {
                let table = convert_to_table2(
                    0,
                    std::iter::once(item),
                    ctrlc.clone(),
                    config,
                    *span,
                    color_hm,
                    theme,
                    deep.map(|i| i - 1),
                    flatten,
                    flatten_sep,
                    width,
                );

                let inner_table = table.map(|table| {
                    table.and_then(|table| {
                        let alignments = Alignments::default();
                        table.draw_table(config, color_hm, alignments, theme, usize::MAX, false)
                    })
                });

                if let Ok(Some(table)) = inner_table {
                    (table, TextStyle::default())
                } else {
                    // error so back down to the default
                    wrap_nu_text(value_to_styled_string(item, config, color_hm), width)
                }
            }
        }
        Value::List { vals, span } => {
            let is_simple_list = vals
                .iter()
                .all(|v| !matches!(v, Value::Record { .. } | Value::List { .. }));

            if flatten && is_simple_list {
                wrap_nu_text(
                    convert_value_list_to_string(vals, config, color_hm, flatten_sep),
                    width,
                )
            } else {
                let table = convert_to_table2(
                    0,
                    vals.iter(),
                    ctrlc.clone(),
                    config,
                    *span,
                    color_hm,
                    theme,
                    deep.map(|i| i - 1),
                    flatten,
                    flatten_sep,
                    width,
                );

                let inner_table = table.map(|table| {
                    table.and_then(|table| {
                        let alignments = Alignments::default();
                        table.draw_table(config, color_hm, alignments, theme, usize::MAX, false)
                    })
                });
                if let Ok(Some(table)) = inner_table {
                    (table, TextStyle::default())
                } else {
                    // error so back down to the default

                    wrap_nu_text(value_to_styled_string(item, config, color_hm), width)
                }
            }
        }
        _ => wrap_nu_text(value_to_styled_string(item, config, color_hm), width), // unknown type.
    }
}

fn convert_value_list_to_string(
    vals: &[Value],
    config: &Config,
    color_hm: &NuColorMap,
    flatten_sep: &str,
) -> NuText {
    let mut buf = Vec::new();
    for value in vals {
        let (text, _) = value_to_styled_string(value, config, color_hm);

        buf.push(text);
    }
    let text = buf.join(flatten_sep);
    (text, TextStyle::default())
}

fn value_to_styled_string(value: &Value, config: &Config, color_hm: &NuColorMap) -> NuText {
    let float_precision = config.float_precision as usize;
    make_styled_string(
        value.into_abbreviated_string(config),
        &value.get_type().to_string(),
        color_hm,
        float_precision,
    )
}

fn make_styled_string(
    text: String,
    text_type: &str,
    color_hm: &NuColorMap,
    float_precision: usize,
) -> NuText {
    if text_type == "float" {
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

fn make_index_string(text: String, color_hm: &NuColorMap) -> NuText {
    let style = TextStyle::new()
        .alignment(Alignment::Right)
        .style(color_hm["row_index"]);
    (text, style)
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
    Ok(format!("{:.prec$}", val_float, prec = precision))
}

struct PagingTableCreator {
    head: Span,
    stream: ListStream,
    ctrlc: Option<Arc<AtomicBool>>,
    config: Config,
    row_offset: usize,
    width_param: Option<i64>,
    view: TableView,
}

impl PagingTableCreator {
    fn build_extended(
        &self,
        batch: &[Value],
        limit: Option<usize>,
        flatten: bool,
        flatten_separator: Option<String>,
    ) -> Result<Option<String>, ShellError> {
        if batch.is_empty() {
            return Ok(None);
        }

        let theme = load_theme_from_config(&self.config);
        let term_width = get_width_param(self.width_param);
        let color_hm = get_color_config(&self.config);
        let table = convert_to_table2(
            self.row_offset,
            batch.iter(),
            self.ctrlc.clone(),
            &self.config,
            self.head,
            &color_hm,
            &theme,
            limit,
            flatten,
            flatten_separator.as_deref().unwrap_or(" "),
            term_width,
        )?;

        let mut table = match table {
            Some(table) => table,
            None => return Ok(None),
        };

        table.truncate(term_width, &theme);

        let table_s = table.clone().draw_table(
            &self.config,
            &color_hm,
            Alignments::default(),
            &theme,
            term_width,
            false,
        );

        let table = match table_s {
            Some(s) => {
                // check whether we need to expand table or not,
                // todo: we can make it more effitient

                const EXPAND_TREASHHOLD: f32 = 0.80;

                let width = string_width(&s);
                let used_percent = width as f32 / term_width as f32;

                if width < term_width && used_percent > EXPAND_TREASHHOLD {
                    table.draw_table(
                        &self.config,
                        &color_hm,
                        Alignments::default(),
                        &theme,
                        term_width,
                        true,
                    )
                } else {
                    Some(s)
                }
            }
            None => None,
        };

        Ok(table)
    }

    fn build_collapsed(&self, batch: Vec<Value>) -> Result<Option<String>, ShellError> {
        if batch.is_empty() {
            return Ok(None);
        }

        let color_hm = get_color_config(&self.config);
        let theme = load_theme_from_config(&self.config);
        let term_width = get_width_param(self.width_param);
        let need_footer = matches!(self.config.footer_mode, FooterMode::RowCount(limit) if batch.len() as u64 > limit)
            || matches!(self.config.footer_mode, FooterMode::Always);
        let value = Value::List {
            vals: batch,
            span: Span::new(0, 0),
        };

        let table = nu_table::NuTable::new(
            value,
            true,
            term_width,
            &self.config,
            &color_hm,
            &theme,
            need_footer,
        );

        Ok(table.draw())
    }

    fn build_general(&self, batch: &[Value]) -> Result<Option<String>, ShellError> {
        let term_width = get_width_param(self.width_param);
        let color_hm = get_color_config(&self.config);
        let theme = load_theme_from_config(&self.config);

        let table = convert_to_table(
            self.row_offset,
            batch,
            self.ctrlc.clone(),
            &self.config,
            self.head,
            term_width,
            &color_hm,
        )?;

        let table = match table {
            Some(table) => table,
            None => return Ok(None),
        };

        let table = table.draw_table(
            &self.config,
            &color_hm,
            Alignments::default(),
            &theme,
            term_width,
            false,
        );

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

            if idx % STREAM_TIMEOUT_CHECK_INTERVAL == 0 {
                let end_time = Instant::now();

                // If we've been buffering over a second, go ahead and send out what we have so far
                if (end_time - start_time).as_secs() >= 1 {
                    break;
                }
            }

            if idx == STREAM_PAGE_SIZE {
                break;
            }

            if let Some(ctrlc) = &self.ctrlc {
                if ctrlc.load(Ordering::SeqCst) {
                    break;
                }
            }
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
            Ok(Some(table)) => Some(Ok(table.as_bytes().to_vec())),
            Err(err) => Some(Err(err)),
            _ => None,
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
