use csv::{Writer, WriterBuilder};
use indexmap::{indexset, IndexSet};
use nu_protocol::{Config, IntoPipelineData, PipelineData, ShellError, Span, Value};
use std::collections::VecDeque;
use std::error::Error;

fn from_value_to_delimited_string(
    value: &Value,
    separator: char,
    config: &Config,
    head: Span,
) -> Result<String, ShellError> {
    match value {
        Value::Record { cols, vals, span } => {
            record_to_delimited(cols, vals, span, separator, config, head)
        }
        Value::List { vals, span } => table_to_delimited(vals, span, separator, config, head),
        // Propagate errors by explicitly matching them before the final case.
        Value::Error { error } => Err(error.clone()),
        other => to_string_tagged_value(value, config, other.expect_span(), head),
    }
}

fn record_to_delimited(
    cols: &Vec<String>,
    vals: &Vec<Value>,
    span: &Span,
    separator: char,
    config: &Config,
    head: Span,
) -> Result<String, ShellError> {
    let mut wtr = WriterBuilder::new()
        .delimiter(separator as u8)
        .from_writer(vec![]);
    let mut fields: VecDeque<String> = VecDeque::new();
    let mut values: VecDeque<String> = VecDeque::new();

    for (k, v) in cols.iter().zip(vals.iter()) {
        fields.push_back(k.clone());

        values.push_back(to_string_tagged_value(v, config, head, *span)?);
    }

    wtr.write_record(fields).expect("can not write.");
    wtr.write_record(values).expect("can not write.");

    writer_to_string(wtr).map_err(|_| make_conversion_error("record", span))
}

fn table_to_delimited(
    vals: &Vec<Value>,
    span: &Span,
    separator: char,
    config: &Config,
    head: Span,
) -> Result<String, ShellError> {
    let mut wtr = WriterBuilder::new()
        .delimiter(separator as u8)
        .from_writer(vec![]);

    let merged_descriptors = merge_descriptors(vals);

    if merged_descriptors.is_empty() {
        let vals = vals
            .iter()
            .map(|ele| {
                to_string_tagged_value(ele, config, head, *span).unwrap_or_else(|_| String::new())
            })
            .collect::<Vec<_>>();
        wtr.write_record(vals).expect("can not write");
    } else {
        wtr.write_record(merged_descriptors.iter().map(|item| &item[..]))
            .expect("can not write.");

        for l in vals {
            let mut row = vec![];
            for desc in &merged_descriptors {
                row.push(match l.to_owned().get_data_by_key(desc) {
                    Some(s) => to_string_tagged_value(&s, config, head, *span)?,
                    None => String::new(),
                });
            }
            wtr.write_record(&row).expect("can not write");
        }
    }
    writer_to_string(wtr).map_err(|_| make_conversion_error("table", span))
}

fn writer_to_string(writer: Writer<Vec<u8>>) -> Result<String, Box<dyn Error>> {
    Ok(String::from_utf8(writer.into_inner()?)?)
}

fn make_conversion_error(type_from: &str, span: &Span) -> ShellError {
    ShellError::CantConvert(type_from.to_string(), "string".to_string(), *span, None)
}

fn to_string_tagged_value(
    v: &Value,
    config: &Config,
    span: Span,
    head: Span,
) -> Result<String, ShellError> {
    match &v {
        Value::String { .. }
        | Value::Bool { .. }
        | Value::Int { .. }
        | Value::Duration { .. }
        | Value::Binary { .. }
        | Value::CustomValue { .. }
        | Value::Filesize { .. }
        | Value::CellPath { .. }
        | Value::List { .. }
        | Value::Record { .. }
        | Value::Float { .. } => Ok(v.clone().into_abbreviated_string(config)),
        Value::Date { val, .. } => Ok(val.to_string()),
        Value::Nothing { .. } => Ok(String::new()),
        // Propagate existing errors
        Value::Error { error } => Err(error.clone()),
        _ => Err(ShellError::UnsupportedInput(
            "Unexpected type".to_string(),
            format!("input type: {:?}", v.get_type()),
            head,
            span,
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
