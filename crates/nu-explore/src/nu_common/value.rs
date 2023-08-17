use std::collections::HashMap;

use nu_engine::get_columns;
use nu_protocol::{
    ast::PathMember, ListStream, PipelineData, PipelineMetadata, RawStream, SpannedValue,
};

use super::NuSpan;

pub fn collect_pipeline(input: PipelineData) -> (Vec<String>, Vec<Vec<SpannedValue>>) {
    match input {
        PipelineData::Empty => (vec![], vec![]),
        PipelineData::Value(value, ..) => collect_input(value),
        PipelineData::ListStream(stream, ..) => collect_list_stream(stream),
        PipelineData::ExternalStream {
            stdout,
            stderr,
            exit_code,
            metadata,
            span,
            ..
        } => collect_external_stream(stdout, stderr, exit_code, metadata.map(|m| *m), span),
    }
}

fn collect_list_stream(mut stream: ListStream) -> (Vec<String>, Vec<Vec<SpannedValue>>) {
    let mut records = vec![];
    for item in stream.by_ref() {
        records.push(item);
    }

    let mut cols = get_columns(&records);
    let data = convert_records_to_dataset(&cols, records);

    // trying to deal with 'non-standard input'
    if cols.is_empty() && !data.is_empty() {
        let min_column_length = data.iter().map(|row| row.len()).min().unwrap_or(0);
        if min_column_length > 0 {
            cols = (0..min_column_length).map(|i| i.to_string()).collect();
        }
    }

    (cols, data)
}

fn collect_external_stream(
    stdout: Option<RawStream>,
    stderr: Option<RawStream>,
    exit_code: Option<ListStream>,
    metadata: Option<PipelineMetadata>,
    span: NuSpan,
) -> (Vec<String>, Vec<Vec<SpannedValue>>) {
    let mut columns = vec![];
    let mut data = vec![];
    if let Some(stdout) = stdout {
        let value = stdout.into_string().map_or_else(
            |error| SpannedValue::Error {
                error: Box::new(error),
                span,
            },
            |string| SpannedValue::string(string.item, span),
        );

        columns.push(String::from("stdout"));
        data.push(value);
    }
    if let Some(stderr) = stderr {
        let value = stderr.into_string().map_or_else(
            |error| SpannedValue::Error {
                error: Box::new(error),
                span,
            },
            |string| SpannedValue::string(string.item, span),
        );

        columns.push(String::from("stderr"));
        data.push(value);
    }
    if let Some(exit_code) = exit_code {
        let list = exit_code.collect::<Vec<_>>();
        let val = SpannedValue::List { vals: list, span };

        columns.push(String::from("exit_code"));
        data.push(val);
    }
    if metadata.is_some() {
        let val = SpannedValue::Record {
            cols: vec![String::from("data_source")],
            vals: vec![SpannedValue::String {
                val: String::from("ls"),
                span,
            }],
            span,
        };

        columns.push(String::from("metadata"));
        data.push(val);
    }
    (columns, vec![data])
}

/// Try to build column names and a table grid.
pub fn collect_input(value: SpannedValue) -> (Vec<String>, Vec<Vec<SpannedValue>>) {
    match value {
        SpannedValue::Record { cols, vals, .. } => (cols, vec![vals]),
        SpannedValue::List { vals, .. } => {
            let mut columns = get_columns(&vals);
            let data = convert_records_to_dataset(&columns, vals);

            if columns.is_empty() && !data.is_empty() {
                columns = vec![String::from("")];
            }

            (columns, data)
        }
        SpannedValue::String { val, span } => {
            let lines = val
                .lines()
                .map(|line| SpannedValue::String {
                    val: line.to_string(),
                    span,
                })
                .map(|val| vec![val])
                .collect();

            (vec![String::from("")], lines)
        }
        SpannedValue::LazyRecord { val, span } => match val.collect() {
            Ok(value) => collect_input(value),
            Err(_) => (
                vec![String::from("")],
                vec![vec![SpannedValue::LazyRecord { val, span }]],
            ),
        },
        SpannedValue::Nothing { .. } => (vec![], vec![]),
        value => (vec![String::from("")], vec![vec![value]]),
    }
}

fn convert_records_to_dataset(
    cols: &Vec<String>,
    records: Vec<SpannedValue>,
) -> Vec<Vec<SpannedValue>> {
    if !cols.is_empty() {
        create_table_for_record(cols, &records)
    } else if cols.is_empty() && records.is_empty() {
        vec![]
    } else if cols.len() == records.len() {
        vec![records]
    } else {
        // I am not sure whether it's good to return records as its length LIKELY
        // will not match columns, which makes no sense......
        //
        // BUT...
        // we can represent it as a list; which we do

        records.into_iter().map(|record| vec![record]).collect()
    }
}

fn create_table_for_record(headers: &[String], items: &[SpannedValue]) -> Vec<Vec<SpannedValue>> {
    let mut data = vec![Vec::new(); items.len()];

    for (i, item) in items.iter().enumerate() {
        let row = record_create_row(headers, item);
        data[i] = row;
    }

    data
}

fn record_create_row(headers: &[String], item: &SpannedValue) -> Vec<SpannedValue> {
    let mut rows = vec![SpannedValue::default(); headers.len()];

    for (i, header) in headers.iter().enumerate() {
        let value = record_lookup_value(item, header);
        rows[i] = value;
    }

    rows
}

fn record_lookup_value(item: &SpannedValue, header: &str) -> SpannedValue {
    match item {
        SpannedValue::Record { .. } => {
            let path = PathMember::String {
                val: header.to_owned(),
                span: NuSpan::unknown(),
                optional: false,
            };

            item.clone()
                .follow_cell_path(&[path], false)
                .unwrap_or_else(|_| unknown_error_value())
        }
        item => item.clone(),
    }
}

pub fn create_map(value: &SpannedValue) -> Option<HashMap<String, SpannedValue>> {
    let (cols, inner_vals) = value.as_record().ok()?;

    let mut hm: HashMap<String, SpannedValue> = HashMap::new();
    for (k, v) in cols.iter().zip(inner_vals) {
        hm.insert(k.to_string(), v.clone());
    }

    Some(hm)
}

pub fn map_into_value(hm: HashMap<String, SpannedValue>) -> SpannedValue {
    let mut columns = Vec::with_capacity(hm.len());
    let mut values = Vec::with_capacity(hm.len());

    for (key, value) in hm {
        columns.push(key);
        values.push(value);
    }

    SpannedValue::Record {
        cols: columns,
        vals: values,
        span: NuSpan::unknown(),
    }
}

pub fn nu_str<S: AsRef<str>>(s: S) -> SpannedValue {
    SpannedValue::string(s.as_ref().to_owned(), NuSpan::unknown())
}

fn unknown_error_value() -> SpannedValue {
    SpannedValue::string(String::from("❎"), NuSpan::unknown())
}
