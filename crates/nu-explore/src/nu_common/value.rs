use super::NuSpan;
use anyhow::Result;
use nu_engine::get_columns;
use nu_protocol::{record, ByteStream, ListStream, PipelineData, PipelineMetadata, Value};
use std::collections::HashMap;

pub fn collect_pipeline(input: PipelineData) -> Result<(Vec<String>, Vec<Vec<Value>>)> {
    match input {
        PipelineData::Empty => Ok((vec![], vec![])),
        PipelineData::Value(value, ..) => collect_input(value),
        PipelineData::ListStream(stream, ..) => Ok(collect_list_stream(stream)),
        PipelineData::ByteStream(stream, metadata) => Ok(collect_byte_stream(stream, metadata)),
    }
}

fn collect_list_stream(stream: ListStream) -> (Vec<String>, Vec<Vec<Value>>) {
    let mut records = vec![];
    for item in stream {
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

fn collect_byte_stream(
    stream: ByteStream,
    metadata: Option<PipelineMetadata>,
) -> (Vec<String>, Vec<Vec<Value>>) {
    let span = stream.span();

    let mut columns = vec![];
    let mut data = vec![];

    match stream.into_child() {
        Ok(child) => match child.wait_with_output() {
            Ok(output) => {
                let exit_code = output.exit_status.code();
                if let Some(stdout) = output.stdout {
                    columns.push(String::from("stdout"));
                    data.push(string_or_binary(stdout, span));
                }
                if let Some(stderr) = output.stderr {
                    columns.push(String::from("stderr"));
                    data.push(string_or_binary(stderr, span));
                }
                columns.push(String::from("exit_code"));
                data.push(Value::int(exit_code.into(), span));
            }
            Err(err) => {
                columns.push("".into());
                data.push(Value::error(err, span));
            }
        },
        Err(stream) => {
            let value = stream
                .into_value()
                .unwrap_or_else(|err| Value::error(err, span));

            columns.push("".into());
            data.push(value);
        }
    }

    if metadata.is_some() {
        let val = Value::record(record! { "data_source" => Value::string("ls", span) }, span);
        columns.push(String::from("metadata"));
        data.push(val);
    }
    (columns, vec![data])
}

fn string_or_binary(bytes: Vec<u8>, span: NuSpan) -> Value {
    match String::from_utf8(bytes) {
        Ok(str) => Value::string(str, span),
        Err(err) => Value::binary(err.into_bytes(), span),
    }
}

/// Try to build column names and a table grid.
pub fn collect_input(value: Value) -> Result<(Vec<String>, Vec<Vec<Value>>)> {
    let span = value.span();
    match value {
        Value::Record { val: record, .. } => {
            let (key, val): (_, Vec<Value>) = record.into_owned().into_iter().unzip();

            Ok((
                key,
                match val.is_empty() {
                    true => vec![],
                    false => vec![val],
                },
            ))
        }
        Value::List { vals, .. } => {
            let mut columns = get_columns(&vals);
            let data = convert_records_to_dataset(&columns, vals);

            if columns.is_empty() && !data.is_empty() {
                columns = vec![String::from("")];
            }

            Ok((columns, data))
        }
        Value::String { val, .. } => {
            let lines = val
                .lines()
                .map(|line| Value::string(line, span))
                .map(|val| vec![val])
                .collect();

            Ok((vec![String::from("")], lines))
        }
        Value::Nothing { .. } => Ok((vec![], vec![])),
        Value::Custom { val, .. } => {
            let materialized = val.to_base_value(span)?;
            collect_input(materialized)
        }
        value => Ok((vec![String::from("")], vec![vec![value]])),
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

fn unknown_error_value() -> Value {
    Value::string(String::from("❎"), NuSpan::unknown())
}
