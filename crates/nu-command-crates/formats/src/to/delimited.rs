use csv::WriterBuilder;
use indexmap::{indexset, IndexSet};
use nu_protocol::{Config, IntoPipelineData, PipelineData, ShellError, Span, Value};
use std::collections::VecDeque;

fn from_value_to_delimited_string(
    value: &Value,
    separator: char,
    config: &Config,
    head: Span,
) -> Result<String, ShellError> {
    match value {
        Value::Record { cols, vals, span } => {
            let mut wtr = WriterBuilder::new()
                .delimiter(separator as u8)
                .from_writer(vec![]);
            let mut fields: VecDeque<String> = VecDeque::new();
            let mut values: VecDeque<String> = VecDeque::new();

            for (k, v) in cols.iter().zip(vals.iter()) {
                fields.push_back(k.clone());

                values.push_back(to_string_tagged_value(v, config, *span)?);
            }

            wtr.write_record(fields).expect("can not write.");
            wtr.write_record(values).expect("can not write.");

            let v = String::from_utf8(wtr.into_inner().map_err(|_| {
                ShellError::UnsupportedInput("Could not convert record".to_string(), *span)
            })?)
            .map_err(|_| {
                ShellError::UnsupportedInput("Could not convert record".to_string(), *span)
            })?;
            Ok(v)
        }
        Value::List { vals, span } => {
            let mut wtr = WriterBuilder::new()
                .delimiter(separator as u8)
                .from_writer(vec![]);

            let merged_descriptors = merge_descriptors(vals);

            if merged_descriptors.is_empty() {
                wtr.write_record(
                    vals.iter()
                        .map(|ele| {
                            to_string_tagged_value(ele, config, *span)
                                .unwrap_or_else(|_| String::new())
                        })
                        .collect::<Vec<_>>(),
                )
                .expect("can not write");
            } else {
                wtr.write_record(merged_descriptors.iter().map(|item| &item[..]))
                    .expect("can not write.");

                for l in vals {
                    let mut row = vec![];
                    for desc in &merged_descriptors {
                        row.push(match l.to_owned().get_data_by_key(desc) {
                            Some(s) => to_string_tagged_value(&s, config, *span)?,
                            None => String::new(),
                        });
                    }
                    wtr.write_record(&row).expect("can not write");
                }
            }
            let v = String::from_utf8(wtr.into_inner().map_err(|_| {
                ShellError::UnsupportedInput("Could not convert record".to_string(), *span)
            })?)
            .map_err(|_| {
                ShellError::UnsupportedInput("Could not convert record".to_string(), *span)
            })?;
            Ok(v)
        }
        _ => to_string_tagged_value(value, config, head),
    }
}

fn to_string_tagged_value(v: &Value, config: &Config, span: Span) -> Result<String, ShellError> {
    match &v {
        Value::String { .. }
        | Value::Bool { .. }
        | Value::Int { .. }
        | Value::Duration { .. }
        | Value::Binary { .. }
        | Value::CustomValue { .. }
        | Value::Error { .. }
        | Value::Filesize { .. }
        | Value::CellPath { .. }
        | Value::List { .. }
        | Value::Record { .. }
        | Value::Float { .. } => Ok(v.clone().into_abbreviated_string(config)),
        Value::Date { val, .. } => Ok(val.to_string()),
        Value::Nothing { .. } => Ok(String::new()),
        _ => Err(ShellError::UnsupportedInput(
            "Unexpected value".to_string(),
            v.span().unwrap_or(span),
        )),
    }
}

pub fn merge_descriptors(values: &[Value]) -> Vec<String> {
    let mut ret: Vec<String> = vec![];
    let mut seen: IndexSet<String> = indexset! {};
    for value in values {
        let data_descriptors = match value {
            Value::Record { cols, .. } => cols.to_owned(),
            _ => vec!["".to_string()],
        };
        for desc in data_descriptors {
            if !desc.is_empty() && !seen.contains(&desc) {
                seen.insert(desc.to_string());
                ret.push(desc.to_string());
            }
        }
    }
    ret
}

pub fn to_delimited_data(
    noheaders: bool,
    sep: char,
    format_name: &'static str,
    input: PipelineData,
    span: Span,
    config: &Config,
) -> Result<PipelineData, ShellError> {
    let value = input.into_value(span);
    let output = match from_value_to_delimited_string(&value, sep, config, span) {
        Ok(mut x) => {
            if noheaders {
                if let Some(second_line) = x.find('\n') {
                    let start = second_line + 1;
                    x.replace_range(0..start, "");
                }
            }
            Ok(x)
        }
        Err(_) => Err(ShellError::CantConvert(
            format_name.into(),
            value.get_type().to_string(),
            value.span().unwrap_or(span),
            None,
        )),
    }?;
    Ok(Value::string(output, span).into_pipeline_data())
}
