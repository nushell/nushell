use csv::{Writer, WriterBuilder};
use nu_cmd_base::formats::to::delimited::merge_descriptors;
use nu_protocol::{Config, IntoPipelineData, PipelineData, ShellError, Span, SpannedValue};
use std::collections::VecDeque;
use std::error::Error;

fn from_value_to_delimited_string(
    value: &SpannedValue,
    separator: char,
    config: &Config,
    head: Span,
) -> Result<String, ShellError> {
    match value {
        SpannedValue::Record { cols, vals, span } => {
            record_to_delimited(cols, vals, *span, separator, config, head)
        }
        SpannedValue::List { vals, span } => {
            table_to_delimited(vals, *span, separator, config, head)
        }
        // Propagate errors by explicitly matching them before the final case.
        SpannedValue::Error { error, .. } => Err(*error.clone()),
        v => Err(make_unsupported_input_error(v, head, v.span())),
    }
}

fn record_to_delimited(
    cols: &[String],
    vals: &[SpannedValue],
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

    for (k, v) in cols.iter().zip(vals.iter()) {
        fields.push_back(k.clone());

        values.push_back(to_string_tagged_value(v, config, head, span)?);
    }

    wtr.write_record(fields).expect("can not write.");
    wtr.write_record(values).expect("can not write.");

    writer_to_string(wtr).map_err(|_| make_conversion_error("record", span))
}

fn table_to_delimited(
    vals: &Vec<SpannedValue>,
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
            let mut row = vec![];
            for desc in &merged_descriptors {
                row.push(match l.to_owned().get_data_by_key(desc) {
                    Some(s) => to_string_tagged_value(&s, config, head, span)?,
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

fn make_conversion_error(type_from: &str, span: Span) -> ShellError {
    ShellError::CantConvert {
        to_type: type_from.to_string(),
        from_type: "string".to_string(),
        span,
        help: None,
    }
}

fn to_string_tagged_value(
    v: &SpannedValue,
    config: &Config,
    span: Span,
    head: Span,
) -> Result<String, ShellError> {
    match &v {
        SpannedValue::String { .. }
        | SpannedValue::Bool { .. }
        | SpannedValue::Int { .. }
        | SpannedValue::Duration { .. }
        | SpannedValue::Binary { .. }
        | SpannedValue::CustomValue { .. }
        | SpannedValue::Filesize { .. }
        | SpannedValue::CellPath { .. }
        | SpannedValue::Float { .. } => Ok(v.clone().into_abbreviated_string(config)),
        SpannedValue::Date { val, .. } => Ok(val.to_string()),
        SpannedValue::Nothing { .. } => Ok(String::new()),
        // Propagate existing errors
        SpannedValue::Error { error, .. } => Err(*error.clone()),
        _ => Err(make_unsupported_input_error(v, head, span)),
    }
}

fn make_unsupported_input_error(value: &SpannedValue, head: Span, span: Span) -> ShellError {
    ShellError::UnsupportedInput(
        "Unexpected type".to_string(),
        format!("input type: {:?}", value.get_type()),
        head,
        span,
    )
}

pub fn find_non_record(values: &[SpannedValue]) -> Option<&SpannedValue> {
    values
        .iter()
        .find(|val| !matches!(val, SpannedValue::Record { .. }))
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
            span: value.span().unwrap_or(span),
            help: None,
        }),
    }?;
    Ok(SpannedValue::string(output, span).into_pipeline_data())
}
