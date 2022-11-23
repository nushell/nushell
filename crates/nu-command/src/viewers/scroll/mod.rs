mod pager;

use std::collections::HashMap;

use self::pager::{InformationView, Pager, RecordView, StyleConfig, TableConfig, ViewConfig};
use nu_ansi_term::{Color, Style};
use nu_color_config::{get_color_config, get_color_map};
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

        Signature::build("scroll")
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
            .switch("peek", "Return a last seen cell content", Some('p'))
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
        let peek_value: bool = call.has_flag("peek");
        let table_cfg = TableConfig {
            show_index,
            show_head,
            peek_value,
            reverse: is_reverse,
            show_help: false,
        };

        let ctrlc = engine_state.ctrlc.clone();

        let config = engine_state.get_config();
        let color_hm = get_color_config(config);
        let scroll_colors = get_color_map(&config.scroll_config);
        let style = style_from_colors(&config.scroll_config, &scroll_colors);

        let (columns, data) = collect_pipeline(input);

        let view_cfg = ViewConfig::new(config, &color_hm, &style);

        let mut p = Pager::new(table_cfg.clone(), view_cfg);

        let result = if columns.is_empty() && data.is_empty() {
            p.run(engine_state, stack, ctrlc, Some(InformationView))
        } else {
            let mut view = RecordView::new(columns, data, table_cfg.clone());

            if table_cfg.reverse {
                if let Some((terminal_size::Width(w), terminal_size::Height(h))) =
                    terminal_size::terminal_size()
                {
                    view.reverse(w, h);
                }
            }

            p.run(engine_state, stack, ctrlc, Some(view))
        };

        match result {
            Ok(Some(value)) => Ok(PipelineData::Value(value, None)),
            Ok(None) => Ok(PipelineData::Value(Value::default(), None)),
            Err(err) => Ok(PipelineData::Value(
                Value::Error { error: err.into() },
                None,
            )),
        }
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
            Example {
                description: "Inspect $nu information and return an entity where you've stopped.",
                example: r#"$nu | scroll --peek"#,
                result: None,
            },
        ]
    }
}

pub(crate) fn collect_pipeline(input: PipelineData) -> (Vec<String>, Vec<Vec<Value>>) {
    match input {
        PipelineData::Value(value, ..) => collect_input(value),
        PipelineData::ListStream(mut stream, ..) => {
            let mut records = vec![];
            for item in stream.by_ref() {
                records.push(item);
            }

            let mut cols = get_columns(&records);
            let data = convert_records_to_dataset(&cols, records);

            // trying to deal with 'not standart input'
            if cols.is_empty() && !data.is_empty() {
                let min_column_length = data.iter().map(|row| row.len()).min().unwrap_or(0);
                if min_column_length > 0 {
                    cols = (0..min_column_length).map(|i| i.to_string()).collect();
                }
            }

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
        Value::Nothing { .. } => (vec![], vec![]),
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
        status_error: Style {
            background: Some(Color::Red),
            foreground: Some(Color::White),
            ..Default::default()
        },
        status_info: Style::default(),
        status_warn: Style::default(),
        selected_cell: None,
        selected_column: None,
        selected_row: None,
        show_cursow: true,
    }
}
