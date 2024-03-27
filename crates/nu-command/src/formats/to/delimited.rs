use csv::{Writer, WriterBuilder};
use nu_cmd_base::formats::to::delimited::merge_descriptors;
use nu_protocol::{Config, IntoPipelineData, PipelineData, Record, ShellError, Span, Value};
use std::{collections::VecDeque, error::Error};

fn from_value_to_delimited_string(
    value: &Value,
    separator: char,
    config: &Config,
    head: Span,
) -> Result<String, ShellError> {
    let span = value.span();
    match value {
        Value::Record { val, .. } => record_to_delimited(val, span, separator, config, head),
        Value::List { vals, .. } => table_to_delimited(vals, span, separator, config, head),
        // Propagate errors by explicitly matching them before the final case.
        Value::Error { error, .. } => Err(*error.clone()),
        v => Err(make_unsupported_input_error(v, head, v.span())),
    }
}

fn record_to_delimited(
    record: &Record,
    span: Span,
    separator: char,
    config: &Config,
    head: Span,
) -> Result<String, ShellError> {
    let mut wtr = WriterBuilder::new()
        .delimiter(separator as u8)
        .from_writer(vec![]);
    let mut fields: VecDeque<String> = VecDeque::new();
    let mut values: VecDeque<String> = VecDeque::new();

    for (k, v) in record {
        fields.push_back(k.clone());

        values.push_back(to_string_tagged_value(v, config, head, span)?);
    }

    wtr.write_record(fields).expect("can not write.");
    wtr.write_record(values).expect("can not write.");

    writer_to_string(wtr).map_err(|_| make_conversion_error("record", span))
}

fn table_to_delimited(
    vals: &[Value],
    span: Span,
    separator: char,
    config: &Config,
    head: Span,
) -> Result<String, ShellError> {
    if let Some(val) = find_non_record(vals) {
        return Err(make_unsupported_input_error(val, head, span));
    }

    let mut wtr = WriterBuilder::new()
        .delimiter(separator as u8)
        .from_writer(vec![]);

    let merged_descriptors = merge_descriptors(vals);

    if merged_descriptors.is_empty() {
        let vals = vals
            .iter()
            .map(|ele| {
                to_string_tagged_value(ele, config, head, span).unwrap_or_else(|_| String::new())
            })
            .collect::<Vec<_>>();
        wtr.write_record(vals).expect("can not write");
    } else {
        wtr.write_record(merged_descriptors.iter().map(|item| &item[..]))
            .expect("can not write.");

        for l in vals {
            // should always be true because of `find_non_record` above
            if let Value::Record { val: l, .. } = l {
                let mut row = vec![];
                for desc in &merged_descriptors {
                    row.push(match l.get(desc) {
                        Some(s) => to_string_tagged_value(s, config, head, span)?,
                        None => String::new(),
                    });
                }
                wtr.write_record(&row).expect("can not write");
            }
        }
    }
    writer_to_string(wtr).map_err(|_| make_conversion_error("table", span))
}

fn writer_to_string(writer: Writer<Vec<u8>>) -> Result<String, Box<dyn Error>> {
    Ok(String::from_utf8(writer.into_inner()?)?)
}

fn make_conversion_error(type_from: &str, span: Span) -> ShellError {
    ShellError::CantConvert {
        to_type: type_from.to_string(),
        from_type: "string".to_string(),
        span,
        help: None,
    }
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
        | Value::Custom { .. }
        | Value::Filesize { .. }
        | Value::CellPath { .. }
        | Value::Float { .. } => Ok(v.clone().to_abbreviated_string(config)),
        Value::Date { val, .. } => Ok(val.to_string()),
        Value::Nothing { .. } => Ok(String::new()),
        // Propagate existing errors
        Value::Error { error, .. } => Err(*error.clone()),
        _ => Err(make_unsupported_input_error(v, head, span)),
    }
}

fn make_unsupported_input_error(value: &Value, head: Span, span: Span) -> ShellError {
    ShellError::UnsupportedInput {
        msg: "Unexpected type".to_string(),
        input: format!("input type: {:?}", value.get_type()),
        msg_span: head,
        input_span: span,
    }
}

pub fn find_non_record(values: &[Value]) -> Option<&Value> {
    values
        .iter()
        .find(|val| !matches!(val, Value::Record { .. }))
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
        Err(_) => Err(ShellError::CantConvert {
            to_type: format_name.into(),
            from_type: value.get_type().to_string(),
            span: value.span(),
            help: None,
        }),
    }?;
    Ok(Value::string(output, span).into_pipeline_data())
}
