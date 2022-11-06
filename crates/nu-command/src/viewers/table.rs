use lscolors::{LsColors, Style};
use nu_color_config::{get_color_config, style_primitive};
use nu_engine::{column::get_columns, env_to_string, CallExt};
use nu_protocol::{
    ast::{Call, PathMember},
    engine::{Command, EngineState, Stack, StateWorkingSet},
    format_error, Category, Config, DataSource, Example, FooterMode, IntoPipelineData, ListStream,
    PipelineData, PipelineMetadata, RawStream, ShellError, Signature, Span, SyntaxShape,
    TableIndexMode, Value,
};
use nu_table::{Alignment, Alignments, Table as NuTable, TableTheme, TextStyle};
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
                TableView::General => build_general_table2(cols, vals, ctrlc, config, term_width),
                TableView::Expanded {
                    limit,
                    flatten,
                    flatten_separator,
                } => {
                    let sep = flatten_separator.as_deref().unwrap_or(" ");
                    build_expanded_table(
                        cols, vals, span, ctrlc, config, term_width, limit, flatten, sep,
                    )
                }
                TableView::Collapsed => build_collapsed_table(cols, vals, config, term_width),
            }?;

            let result = result
                .unwrap_or_else(|| format!("Couldn't fit table into {} columns!", term_width));

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

    let table = table.draw_table(config, &color_hm, Alignments::default(), &theme, term_width);

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
    let key_width = cols
        .iter()
        .map(|col| nu_table::string_width(col))
        .max()
        .unwrap_or(0);
    let key = NuTable::create_cell(" ".repeat(key_width), TextStyle::default());
    let key_table = NuTable::new(vec![vec![key]], (1, 2), term_width, false, false);
    let key_width = key_table
        .draw_table(config, &color_hm, alignments, &theme, usize::MAX)
        .map(|table| nu_table::string_width(&table))
        .unwrap_or(0);

    if key_width > term_width {
        return Ok(None);
    }

    let remaining_width = term_width - key_width;
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
            value_to_styled_string(&value, 0, config, &color_hm).0
        } else {
            let mut is_record = false;
            let mut vals = match value {
                Value::List { vals, .. } => vals,
                value => {
                    is_record = true;
                    vec![value]
                }
            };

            let deep = expand_limit.map(|i| i - 1);
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
            )?;

            match table {
                Some(mut table) => {
                    // controll width via removing table columns.
                    let count_cols = table.size().1;
                    let is_empty = table.truncate(remaining_width, &theme);
                    let was_left_only_index =
                        table.is_with_index() && table.size().1 == 2 && count_cols != 2;
                    let was_truncated = is_empty || was_left_only_index;

                    if is_record && vals.len() == 1 && was_truncated {
                        match vals.remove(0) {
                            Value::Record { cols, vals, .. } => {
                                let t = build_general_table2(
                                    cols,
                                    vals,
                                    ctrlc.clone(),
                                    config,
                                    remaining_width,
                                )?;

                                match t {
                                    Some(val) => val,
                                    None => return Ok(None),
                                }
                            }
                            _ => unreachable!(),
                        }
                    } else {
                        let theme = load_theme_from_config(config);
                        let result = table.draw_table(
                            config,
                            &color_hm,
                            alignments,
                            &theme,
                            remaining_width,
                        );
                        is_expanded = true;
                        match result {
                            Some(result) => result,
                            None => return Ok(None),
                        }
                    }
                }
                None => {
                    // it means that the list is empty
                    let value = Value::List { vals, span };
                    value_to_styled_string(&value, 0, config, &color_hm).0
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

        let key = Value::String {
            val: key,
            span: Span::new(0, 0),
        };

        let key = value_to_styled_string(&key, 0, config, &color_hm);

        let key = NuTable::create_cell(key.0, key.1);
        let val = NuTable::create_cell(value, TextStyle::default());

        let row = vec![key, val];
        data.push(row);
    }

    let data_len = data.len();
    let table = NuTable::new(data, (data_len, 2), term_width, false, false);

    let table = table.draw_table(config, &color_hm, alignments, &theme, usize::MAX);

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
    color_hm: &HashMap<String, nu_ansi_term::Style>,
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

            let value =
                make_styled_string(text, "string", 0, with_index, color_hm, float_precision);
            let value = NuTable::create_cell(value.0, value.1);

            row.push(value);
        }

        if !with_header {
            let text = item.into_abbreviated_string(config);
            let text_type = item.get_type().to_string();
            let col = if with_index { 1 } else { 0 };
            let value =
                make_styled_string(text, &text_type, col, with_index, color_hm, float_precision);
            let value = NuTable::create_cell(value.0, value.1);

            row.push(value);
        } else {
            let skip_num = if with_index { 1 } else { 0 };
            for (col, header) in data[0].iter().enumerate().skip(skip_num) {
                let result = match item {
                    Value::Record { .. } => item.clone().follow_cell_path(
                        &[PathMember::String {
                            val: header.as_ref().to_owned(),
                            span: head,
                        }],
                        false,
                    ),
                    _ => Ok(item.clone()),
                };

                let value = match result {
                    Ok(value) => make_styled_string(
                        value.into_abbreviated_string(config),
                        &value.get_type().to_string(),
                        col,
                        with_index,
                        color_hm,
                        float_precision,
                    ),
                    Err(_) => make_styled_string(
                        String::from("❎"),
                        "empty",
                        col,
                        with_index,
                        color_hm,
                        float_precision,
                    ),
                };

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
    color_hm: &HashMap<String, nu_ansi_term::Style>,
    theme: &TableTheme,
    deep: Option<usize>,
    flatten: bool,
    flatten_sep: &str,
) -> Result<Option<NuTable>, ShellError> {
    if input.len() == 0 {
        return Ok(None);
    }

    let float_precision = config.float_precision as usize;

    let mut headers = get_columns(input.clone());
    let with_index = match config.table_index_mode {
        TableIndexMode::Always => true,
        TableIndexMode::Never => false,
        TableIndexMode::Auto => headers.iter().any(|header| header == INDEX_COLUMN_NAME),
    };

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

    for (row_num, item) in input.into_iter().enumerate() {
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

            let value =
                make_styled_string(text, "string", 0, with_index, color_hm, float_precision);
            let value = NuTable::create_cell(value.0, value.1);

            row.push(value);
        }

        if !with_header {
            let value = convert_to_table2_entry(
                Some(item),
                config,
                &ctrlc,
                color_hm,
                0,
                theme,
                with_index,
                deep,
                flatten,
                flatten_sep,
            );

            let value = NuTable::create_cell(value.0, value.1);
            row.push(value);
        } else {
            let skip_num = if with_index { 1 } else { 0 };
            for (col, header) in data[0].iter().enumerate().skip(skip_num) {
                let value = match item {
                    Value::Record { .. } => {
                        let val = item.clone().follow_cell_path(
                            &[PathMember::String {
                                val: header.as_ref().to_owned(),
                                span: head,
                            }],
                            false,
                        );

                        match val {
                            Ok(val) => convert_to_table2_entry(
                                Some(&val),
                                config,
                                &ctrlc,
                                color_hm,
                                col,
                                theme,
                                with_index,
                                deep,
                                flatten,
                                flatten_sep,
                            ),
                            Err(_) => make_styled_string(
                                item.into_abbreviated_string(config),
                                &item.get_type().to_string(),
                                col,
                                with_index,
                                color_hm,
                                float_precision,
                            ),
                        }
                    }
                    _ => convert_to_table2_entry(
                        Some(item),
                        config,
                        &ctrlc,
                        color_hm,
                        col,
                        theme,
                        with_index,
                        deep,
                        flatten,
                        flatten_sep,
                    ),
                };

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
        usize::MAX,
        with_header,
        with_index,
    );

    Ok(Some(table))
}

#[allow(clippy::too_many_arguments)]
fn convert_to_table2_entry(
    item: Option<&Value>,
    config: &Config,
    ctrlc: &Option<Arc<AtomicBool>>,
    color_hm: &HashMap<String, nu_ansi_term::Style>,
    col: usize,
    theme: &TableTheme,
    with_index: bool,
    deep: Option<usize>,
    flatten: bool,
    flatten_sep: &str,
) -> (String, TextStyle) {
    let float_precision = config.float_precision as usize;
    let alignments = Alignments::default();

    let item = match item {
        Some(item) => item,
        None => {
            return make_styled_string(
                String::from("❎"),
                "empty",
                col,
                with_index,
                color_hm,
                float_precision,
            )
        }
    };

    let is_limit_reached = matches!(deep, Some(0));
    if is_limit_reached {
        return make_styled_string(
            item.into_abbreviated_string(config),
            &item.get_type().to_string(),
            col,
            with_index,
            color_hm,
            float_precision,
        );
    }

    match &item {
        Value::Record { span, cols, vals } => {
            if cols.is_empty() && vals.is_empty() {
                make_styled_string(
                    item.into_abbreviated_string(config),
                    &item.get_type().to_string(),
                    col,
                    with_index,
                    color_hm,
                    float_precision,
                )
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
                );

                let inner_table = table.map(|table| {
                    table.and_then(|table| {
                        table.draw_table(config, color_hm, alignments, theme, usize::MAX)
                    })
                });
                if let Ok(Some(table)) = inner_table {
                    (table, TextStyle::default())
                } else {
                    // error so back down to the default
                    make_styled_string(
                        item.into_abbreviated_string(config),
                        &item.get_type().to_string(),
                        col,
                        with_index,
                        color_hm,
                        float_precision,
                    )
                }
            }
        }
        Value::List { vals, span } => {
            let is_simple_list = vals
                .iter()
                .all(|v| !matches!(v, Value::Record { .. } | Value::List { .. }));

            if flatten && is_simple_list {
                let mut buf = Vec::new();
                for value in vals {
                    let (text, _) = make_styled_string(
                        value.into_abbreviated_string(config),
                        &value.get_type().to_string(),
                        col,
                        with_index,
                        color_hm,
                        float_precision,
                    );

                    buf.push(text);
                }

                let text = buf.join(flatten_sep);

                (text, TextStyle::default())
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
                );

                let inner_table = table.map(|table| {
                    table.and_then(|table| {
                        table.draw_table(config, color_hm, alignments, theme, usize::MAX)
                    })
                });
                if let Ok(Some(table)) = inner_table {
                    (table, TextStyle::default())
                } else {
                    // error so back down to the default
                    make_styled_string(
                        item.into_abbreviated_string(config),
                        &item.get_type().to_string(),
                        col,
                        with_index,
                        color_hm,
                        float_precision,
                    )
                }
            }
        }
        _ => {
            // unknown type.
            make_styled_string(
                item.into_abbreviated_string(config),
                &item.get_type().to_string(),
                col,
                with_index,
                color_hm,
                float_precision,
            )
        }
    }
}

fn value_to_styled_string(
    value: &Value,
    col: usize,
    config: &Config,
    color_hm: &HashMap<String, nu_ansi_term::Style>,
) -> (String, TextStyle) {
    let float_precision = config.float_precision as usize;
    make_styled_string(
        value.into_abbreviated_string(config),
        &value.get_type().to_string(),
        col,
        false,
        color_hm,
        float_precision,
    )
}

fn make_styled_string(
    text: String,
    text_type: &str,
    col: usize,
    with_index: bool,
    color_hm: &HashMap<String, nu_ansi_term::Style>,
    float_precision: usize,
) -> (String, TextStyle) {
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
        )?;

        let mut table = match table {
            Some(table) => table,
            None => return Ok(None),
        };

        table.truncate(term_width, &theme);

        let table = table.draw_table(
            &self.config,
            &color_hm,
            Alignments::default(),
            &theme,
            term_width,
        );

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
