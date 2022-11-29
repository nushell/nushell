use nu_engine::get_columns;
use nu_protocol::{ast::PathMember, PipelineData, Value};

use super::NuSpan;

pub fn collect_pipeline(input: PipelineData) -> (Vec<String>, Vec<Vec<Value>>) {
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
            ..
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
pub fn collect_input(value: Value) -> (Vec<String>, Vec<Vec<Value>>) {
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
                span: NuSpan::unknown(),
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
