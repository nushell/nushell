mod pager;

use std::collections::HashMap;

use self::pager::{pager, StyleConfig, TableConfig};
use nu_ansi_term::{Color, Style};
use nu_color_config::get_color_map;
use nu_engine::{get_columns, CallExt};
use nu_protocol::{
    ast::{Call, PathMember},
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, Span, SyntaxShape, Value,
};

/// A `less` like program to render a [Value] as a table.
#[derive(Clone)]
pub struct Scroll;

impl Command for Scroll {
    fn name(&self) -> &str {
        "scroll"
    }

    fn usage(&self) -> &str {
        "Scroll acts as a simple table pager, just like `less` does for text"
    }

    fn signature(&self) -> nu_protocol::Signature {
        // todo: Fix error message when it's empty
        // if we set h i short flags it panics????

        Signature::build("tabless")
            .named(
                "head",
                SyntaxShape::Boolean,
                "Setting it to false makes it doesn't show column headers",
                None,
            )
            .switch("index", "A flag to show a index beside the rows", Some('i'))
            .switch(
                "reverse",
                "Makes it start from the end. (like `more`)",
                Some('r'),
            )
            .category(Category::Viewers)
    }

    fn extra_usage(&self) -> &str {
        r#"Press <i> to get into cursor mode; which will allow you to get inside the cells to see its inner structure.
Press <ESC> to get out of the mode and the inner view.
        "#
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let show_head: bool = call.get_flag(engine_state, stack, "head")?.unwrap_or(true);
        let show_index: bool = call.has_flag("index");
        let is_reverse: bool = call.has_flag("reverse");
        let table_config = TableConfig {
            show_index,
            show_head,
            reverse: is_reverse,
        };

        let ctrlc = engine_state.ctrlc.clone();

        let config = engine_state.get_config();
        let colors = get_color_map(&config.scroll_config);
        let style = style_from_colors(&config.scroll_config, &colors);

        let (columns, data) = collect_pipeline(input);

        let _ = pager(&columns, &data, config, ctrlc, table_config, style);

        Ok(PipelineData::Value(Value::default(), None))
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "List the files in current directory, an looking at them via scroll.",
                example: r#"ls | scroll"#,
                result: None,
            },
            Example {
                description: "Inspect system information (scroll with index).",
                example: r#"sys | scroll -i"#,
                result: None,
            },
            Example {
                description: "Inspect $nu information (scroll with no column names).",
                example: r#"$nu | scroll --head false"#,
                result: None,
            },
        ]
    }
}

fn collect_pipeline(input: PipelineData) -> (Vec<String>, Vec<Vec<Value>>) {
    match input {
        PipelineData::Value(value, ..) => collect_input(value),
        PipelineData::ListStream(mut stream, ..) => {
            let mut records = vec![];
            for item in stream.by_ref() {
                records.push(item);
            }

            let cols = get_columns(&records);
            let data = convert_records_to_dataset(&cols, records);

            (cols, data)
        }
        PipelineData::ExternalStream {
            stdout,
            stderr,
            exit_code,
            metadata,
            span,
        } => {
            let mut columns = vec![];
            let mut data = vec![];

            if let Some(stdout) = stdout {
                let value = stdout.into_string().map_or_else(
                    |error| Value::Error { error },
                    |string| Value::string(string.item, span),
                );

                columns.push(String::from("stdout"));
                data.push(vec![value]);
            }

            if let Some(stderr) = stderr {
                let value = stderr.into_string().map_or_else(
                    |error| Value::Error { error },
                    |string| Value::string(string.item, span),
                );

                columns.push(String::from("stderr"));
                data.push(vec![value]);
            }

            if let Some(exit_code) = exit_code {
                let list = exit_code.collect::<Vec<_>>();

                columns.push(String::from("exit_code"));
                data.push(list);
            }

            if metadata.is_some() {
                columns.push(String::from("metadata"));
                data.push(vec![Value::Record {
                    cols: vec![String::from("data_source")],
                    vals: vec![Value::String {
                        val: String::from("ls"),
                        span,
                    }],
                    span,
                }]);
            }

            (columns, data)
        }
    }
}

/// Try to build column names and a table grid.
pub(crate) fn collect_input(value: Value) -> (Vec<String>, Vec<Vec<Value>>) {
    match value {
        Value::Record { cols, vals, .. } => (cols, vec![vals]),
        Value::List { vals, .. } => {
            let mut columns = get_columns(&vals);
            let data = convert_records_to_dataset(&columns, vals);

            if columns.is_empty() && !data.is_empty() {
                columns = vec![String::from("")];
            }

            (columns, data)
        }
        Value::String { val, span } => {
            let lines = val
                .lines()
                .map(|line| Value::String {
                    val: line.to_string(),
                    span,
                })
                .map(|val| vec![val])
                .collect();

            (vec![String::from("")], lines)
        }
        value => (vec![String::from("")], vec![vec![value]]),
    }
}

fn convert_records_to_dataset(cols: &Vec<String>, records: Vec<Value>) -> Vec<Vec<Value>> {
    if !cols.is_empty() {
        create_table_for_record(cols, &records)
    } else if cols.is_empty() && records.is_empty() {
        vec![]
    } else if cols.len() == records.len() {
        vec![records]
    } else {
        // I am not sure whether it's good to return records as its length LIKELY will not match columns,
        // which makes no scense......
        //
        // BUT...
        // we can represent it as a list; which we do

        records.into_iter().map(|record| vec![record]).collect()
    }
}

fn create_table_for_record(headers: &[String], items: &[Value]) -> Vec<Vec<Value>> {
    let mut data = vec![Vec::new(); items.len()];

    for (i, item) in items.iter().enumerate() {
        let row = record_create_row(headers, item);
        data[i] = row;
    }

    data
}

fn record_create_row(headers: &[String], item: &Value) -> Vec<Value> {
    let mut rows = vec![Value::default(); headers.len()];

    for (i, header) in headers.iter().enumerate() {
        let value = record_lookup_value(item, header);
        rows[i] = value;
    }

    rows
}

fn record_lookup_value(item: &Value, header: &str) -> Value {
    match item {
        Value::Record { .. } => {
            let path = PathMember::String {
                val: header.to_owned(),
                span: Span::unknown(),
            };

            let value = item.clone().follow_cell_path(&[path], false);
            match value {
                Ok(value) => value,
                Err(_) => item.clone(),
            }
        }
        item => item.clone(),
    }
}

fn style_from_colors(
    config: &HashMap<String, Value>,
    colors: &HashMap<String, Style>,
) -> StyleConfig {
    let mut style = default_style();

    if let Some(s) = colors.get("status_bar") {
        style.status_bar = *s;
    }

    if let Some(s) = colors.get("command_bar") {
        style.cmd_bar = *s;
    }

    if let Some(s) = colors.get("split_line") {
        style.split_line = *s;
    }

    if let Some(s) = colors.get("highlight") {
        style.highlight = *s;
    }

    if let Some(s) = colors.get("selected_cell") {
        style.selected_cell = Some(*s);
    }

    if let Some(s) = colors.get("selected_row") {
        style.selected_row = Some(*s);
    }

    if let Some(s) = colors.get("selected_column") {
        style.selected_column = Some(*s);
    }

    if let Some(show_cursor) = config.get("cursor").and_then(|v| v.as_bool().ok()) {
        style.show_cursow = show_cursor;
    }

    style
}

fn default_style() -> StyleConfig {
    StyleConfig {
        status_bar: Style {
            background: Some(Color::Rgb(196, 201, 198)),
            foreground: Some(Color::Rgb(29, 31, 33)),
            ..Default::default()
        },
        highlight: Style {
            background: Some(Color::Yellow),
            foreground: Some(Color::Black),
            ..Default::default()
        },
        split_line: Style {
            foreground: Some(Color::Rgb(64, 64, 64)),
            ..Default::default()
        },
        cmd_bar: Style {
            foreground: Some(Color::Rgb(196, 201, 198)),
            ..Default::default()
        },
        selected_cell: None,
        selected_column: None,
        selected_row: None,
        show_cursow: true,
    }
}
