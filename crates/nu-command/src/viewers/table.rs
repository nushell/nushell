use lscolors::Style;
use nu_color_config::{get_color_config, style_primitive};
use nu_engine::{column::get_columns, env_to_string, CallExt};
use nu_protocol::{
    ast::{Call, PathMember},
    engine::{Command, EngineState, Stack, StateWorkingSet},
    format_error, Category, Config, DataSource, Example, IntoPipelineData, ListStream,
    PipelineData, PipelineMetadata, RawStream, ShellError, Signature, Span, SyntaxShape, Value,
};
use nu_table::{Alignment, Alignments, Table as NuTable, TableTheme, TextStyle};
use nu_utils::get_ls_colors;
use std::time::Instant;
use std::{cmp::max, sync::Arc};
use std::{
    collections::HashMap,
    sync::atomic::{AtomicBool, Ordering},
};
use terminal_size::{Height, Width};

//use super::lscolor_ansiterm::ToNuAnsiStyle;

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
            .category(Category::Viewers)
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        let head = call.head;
        let ctrlc = engine_state.ctrlc.clone();
        let config = engine_state.get_config();
        let color_hm = get_color_config(config);
        let start_num: Option<i64> = call.get_flag(engine_state, stack, "start-number")?;
        let row_offset = start_num.unwrap_or_default() as usize;
        let list: bool = call.has_flag("list");

        let width_param: Option<i64> = call.get_flag(engine_state, stack, "width")?;
        let term_width = get_width_param(width_param);

        if list {
            let table_modes = vec![
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
            ];
            return Ok(Value::List {
                vals: table_modes,
                span: Span::test_data(),
            }
            .into_pipeline_data());
        }

        // reset vt processing, aka ansi because illbehaved externals can break it
        #[cfg(windows)]
        {
            let _ = nu_utils::enable_vt_processing();
        }

        match input {
            PipelineData::ExternalStream { .. } => Ok(input),
            PipelineData::Value(Value::Binary { val, .. }, ..) => {
                Ok(PipelineData::ExternalStream {
                    stdout: Some(RawStream::new(
                        Box::new(
                            vec![Ok(format!("{}\n", nu_pretty_hex::pretty_hex(&val))
                                .as_bytes()
                                .to_vec())]
                            .into_iter(),
                        ),
                        ctrlc,
                        head,
                    )),
                    stderr: None,
                    exit_code: None,
                    span: head,
                    metadata: None,
                })
            }
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
            PipelineData::Value(Value::Record { cols, vals, .. }, ..) => {
                let output = cols
                    .into_iter()
                    .zip(vals.into_iter())
                    .map(|(c, v)| {
                        vec![
                            NuTable::create_cell(c, TextStyle::default_field()),
                            NuTable::create_cell(
                                v.into_abbreviated_string(config),
                                TextStyle::default(),
                            ),
                        ]
                    })
                    .collect::<Vec<_>>();

                let output_len = output.len();
                let table = NuTable::new(output, (output_len, 2), term_width, false);

                let theme = load_theme_from_config(config);
                let result = table
                    .draw_table(config, &color_hm, Alignments::default(), &theme, term_width)
                    .unwrap_or_else(|| format!("Couldn't fit table into {} columns!", term_width));

                Ok(Value::String {
                    val: result,
                    span: call.head,
                }
                .into_pipeline_data())
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
                self.run(engine_state, stack, call, base_pipeline)
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

    fn examples(&self) -> Vec<Example> {
        let span = Span::test_data();
        vec![
            Example {
                description: "List the files in current directory with index number start from 1.",
                example: r#"ls | table -n 1"#,
                result: None,
            },
            Example {
                description: "Render data in table view",
                example: r#"echo [[a b]; [1 2] [3 4]] | table"#,
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

#[allow(clippy::too_many_arguments)]
fn handle_row_stream(
    engine_state: &EngineState,
    stack: &mut Stack,
    stream: ListStream,
    call: &Call,
    row_offset: usize,
    ctrlc: Option<Arc<AtomicBool>>,
    metadata: Option<PipelineMetadata>,
) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
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
                                if let Some(Value::String { val: path, span }) = vals.get(idx) {
                                    match std::fs::symlink_metadata(&path) {
                                        Ok(metadata) => {
                                            let style = ls_colors.style_for_path_with_metadata(
                                                path.clone(),
                                                Some(&metadata),
                                            );
                                            let ansi_style = style
                                                .map(Style::to_crossterm_style)
                                                // .map(ToNuAnsiStyle::to_nu_ansi_style)
                                                .unwrap_or_default();
                                            let use_ls_colors = config.use_ls_colors;

                                            if use_ls_colors {
                                                vals[idx] = Value::String {
                                                    val: ansi_style.apply(path).to_string(),
                                                    span: *span,
                                                };
                                            }
                                        }
                                        Err(_) => {
                                            let style = ls_colors.style_for_path(path.clone());
                                            let ansi_style = style
                                                .map(Style::to_crossterm_style)
                                                // .map(ToNuAnsiStyle::to_nu_ansi_style)
                                                .unwrap_or_default();
                                            let use_ls_colors = config.use_ls_colors;

                                            if use_ls_colors {
                                                vals[idx] = Value::String {
                                                    val: ansi_style.apply(path).to_string(),
                                                    span: *span,
                                                };
                                            }
                                        }
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

    Ok(PipelineData::ExternalStream {
        stdout: Some(RawStream::new(
            Box::new(PagingTableCreator {
                row_offset,
                config: engine_state.get_config().clone(),
                ctrlc: ctrlc.clone(),
                head,
                stream,
                width_param,
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

fn convert_to_table<'a>(
    row_offset: usize,
    input: &[Value],
    ctrlc: Option<Arc<AtomicBool>>,
    config: &Config,
    head: Span,
    termwidth: usize,
    color_hm: &HashMap<String, nu_ansi_term::Style>,
) -> Result<Option<NuTable<'a>>, ShellError> {
    let mut headers = get_columns(input);
    let mut input = input.iter().peekable();
    let float_precision = config.float_precision as usize;
    let disable_index = config.disable_table_indexes;

    if input.peek().is_none() {
        return Ok(None);
    }

    if !headers.is_empty() && !disable_index {
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
        if !disable_index {
            let text = match &item {
                Value::Record { .. } => item
                    .get_data_by_key(INDEX_COLUMN_NAME)
                    .map(|value| value.into_string("", config)),
                _ => None,
            }
            .unwrap_or_else(|| (row_num + row_offset).to_string());

            let value =
                make_styled_string(text, "string", 0, disable_index, color_hm, float_precision);
            let value = NuTable::create_cell(value.0, value.1);

            row.push(value);
        }

        if !with_header {
            let text = item.into_abbreviated_string(config);
            let text_type = item.get_type().to_string();
            let col = if !disable_index { 1 } else { 0 };
            let value = make_styled_string(
                text,
                &text_type,
                col,
                disable_index,
                color_hm,
                float_precision,
            );
            let value = NuTable::create_cell(value.0, value.1);

            row.push(value);
        } else {
            let skip_num = if !disable_index { 1 } else { 0 };
            for (col, header) in data[0].iter().enumerate().skip(skip_num) {
                let result = match item {
                    Value::Record { .. } => item.clone().follow_cell_path(
                        &[PathMember::String {
                            val: header.as_str().to_owned(),
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
                        disable_index,
                        color_hm,
                        float_precision,
                    ),
                    Err(_) => make_styled_string(
                        String::from("❎"),
                        "empty",
                        col,
                        disable_index,
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
    let table = NuTable::new(data, (count_rows, count_columns), termwidth, with_header);

    Ok(Some(table))
}

fn make_styled_string(
    text: String,
    text_type: &str,
    col: usize,
    disable_index: bool,
    color_hm: &HashMap<String, nu_ansi_term::Style>,
    float_precision: usize,
) -> (String, TextStyle) {
    if col == 0 && !disable_index {
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

        let color_hm = get_color_config(&self.config);
        let term_width = get_width_param(self.width_param);

        let table = convert_to_table(
            self.row_offset,
            &batch,
            self.ctrlc.clone(),
            &self.config,
            self.head,
            term_width,
            &color_hm,
        );
        self.row_offset += idx;

        match table {
            Ok(Some(table)) => {
                let theme = load_theme_from_config(&self.config);
                let result = table
                    .draw_table(
                        &self.config,
                        &color_hm,
                        Alignments::default(),
                        &theme,
                        term_width,
                    )
                    .unwrap_or_else(|| format!("Couldn't fit table into {} columns!", term_width));

                Some(Ok(result.as_bytes().to_vec()))
            }
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
