use super::NuSpan;
use nu_engine::get_columns;
use nu_protocol::{record, ListStream, PipelineData, PipelineMetadata, RawStream, Value};
use std::collections::HashMap;

pub fn collect_pipeline(input: PipelineData) -> (Vec<String>, Vec<Vec<Value>>) {
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
        } => collect_external_stream(stdout, stderr, exit_code, metadata, span),
    }
}

fn collect_list_stream(mut stream: ListStream) -> (Vec<String>, Vec<Vec<Value>>) {
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
) -> (Vec<String>, Vec<Vec<Value>>) {
    let mut columns = vec![];
    let mut data = vec![];
    if let Some(stdout) = stdout {
        let value = stdout.into_string().map_or_else(
            |error| Value::error(error, span),
            |string| Value::string(string.item, span),
        );

        columns.push(String::from("stdout"));
        data.push(value);
    }
    if let Some(stderr) = stderr {
        let value = stderr.into_string().map_or_else(
            |error| Value::error(error, span),
            |string| Value::string(string.item, span),
        );

        columns.push(String::from("stderr"));
        data.push(value);
    }
    if let Some(exit_code) = exit_code {
        let list = exit_code.collect::<Vec<_>>();
        let val = Value::list(list, span);

        columns.push(String::from("exit_code"));
        data.push(val);
    }
    if metadata.is_some() {
        let val = Value::record(record! { "data_source" => Value::string("ls", span) }, span);

        columns.push(String::from("metadata"));
        data.push(val);
    }
    (columns, vec![data])
}

/// Try to build column names and a table grid.
pub fn collect_input(value: Value) -> (Vec<String>, Vec<Vec<Value>>) {
    let span = value.span();
    match value {
        Value::Record { val: record, .. } => {
            let (key, val) = record.into_owned().into_iter().unzip();
            (key, vec![val])
        }
        Value::List { vals, .. } => {
            let mut columns = get_columns(&vals);
            let data = convert_records_to_dataset(&columns, vals);

            if columns.is_empty() && !data.is_empty() {
                columns = vec![String::from("")];
            }

            (columns, data)
        }
        Value::String { val, .. } => {
            let lines = val
                .lines()
                .map(|line| Value::string(line, span))
                .map(|val| vec![val])
                .collect();

            (vec![String::from("")], lines)
        }
        Value::LazyRecord { val, .. } => match val.collect() {
            Ok(value) => collect_input(value),
            Err(_) => (
                vec![String::from("")],
                vec![vec![Value::lazy_record(val, span)]],
            ),
        },
        Value::Nothing { .. } => (vec![], vec![]),
        value => (vec![String::from("")], vec![vec![value]]),
    }
}

fn convert_records_to_dataset(cols: &[String], records: Vec<Value>) -> Vec<Vec<Value>> {
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

fn create_table_for_record(headers: &[String], items: &[Value]) -> Vec<Vec<Value>> {
    let mut data = vec![Vec::new(); items.len()];

    for (i, item) in items.iter().enumerate() {
        let row = record_create_row(headers, item);
        data[i] = row;
    }

    data
}

fn record_create_row(headers: &[String], item: &Value) -> Vec<Value> {
    if let Value::Record { val, .. } = item {
        headers
            .iter()
            .map(|col| val.get(col).cloned().unwrap_or_else(unknown_error_value))
            .collect()
    } else {
        // should never reach here due to `get_columns` above which will return
        // empty columns if any value in the list is not a record
        vec![Value::default(); headers.len()]
    }
}

pub fn create_map(value: &Value) -> Option<HashMap<String, Value>> {
    Some(
        value
            .as_record()
            .ok()?
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect(),
    )
}

pub fn map_into_value(hm: HashMap<String, Value>) -> Value {
    Value::record(hm.into_iter().collect(), NuSpan::unknown())
}

fn unknown_error_value() -> Value {
    Value::string(String::from("❎"), NuSpan::unknown())
}
