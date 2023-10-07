use std::sync::{atomic::AtomicBool, Arc};

use csv::{ReaderBuilder, Trim};
use nu_protocol::{IntoInterruptiblePipelineData, PipelineData, Record, ShellError, Span, Value};

fn from_delimited_to_values<R>(
    DelimitedReaderConfig {
        separator,
        record_separator: _,
        comment,
        quote,
        escape,
        noheaders,
        flexible,
        no_infer,
        trim,
    }: DelimitedReaderConfig,
    reader: R,
    span: Span,
) -> csv::Result<impl Iterator<Item = Value> + Send + 'static>
where
    R: std::io::Read + Send + 'static,
{
    let mut reader = ReaderBuilder::new()
        .has_headers(!noheaders)
        .flexible(flexible)
        .delimiter(separator as u8)
        .comment(comment.map(|c| c as u8))
        .quote(quote as u8)
        .escape(escape.map(|c| c as u8))
        .trim(trim)
        .from_reader(reader);

    let headers = if noheaders {
        (1..=reader.headers()?.len())
            .map(|i| format!("column{i}"))
            .collect::<Vec<String>>()
    } else {
        reader.headers()?.iter().map(String::from).collect()
    };

    Ok(reader
        .into_records()
        .scan(
            headers,
            move |headers, row: csv::Result<csv::StringRecord>| {
                // Is there a better way to bubble the error up?
                let row = match row {
                    Ok(row) => row,
                    Err(err) => {
                        eprintln!("Error: {}", err);
                        return None;
                    }
                };

                let output_row = (0..headers.len())
                    .map(|i| {
                        row.get(i)
                            .map(|value| {
                                if no_infer {
                                    Value::string(value.to_string(), span)
                                } else if let Ok(i) = value.parse::<i64>() {
                                    Value::int(i, span)
                                } else if let Ok(f) = value.parse::<f64>() {
                                    Value::float(f, span)
                                } else {
                                    Value::string(value.to_string(), span)
                                }
                            })
                            .unwrap_or(Value::nothing(span))
                    })
                    .collect::<Vec<Value>>();

                Some(Value::record(
                    Record::from_raw_cols_vals(headers.clone(), output_row),
                    span,
                ))
            },
        )
        .fuse())
}

pub(super) struct DelimitedReaderConfig {
    pub separator: char,
    pub record_separator: char,
    pub comment: Option<char>,
    pub quote: char,
    pub escape: Option<char>,
    pub noheaders: bool,
    pub flexible: bool,
    pub no_infer: bool,
    pub trim: Trim,
}

pub(super) fn from_delimited_data(
    config: DelimitedReaderConfig,
    input: PipelineData,
    span: Span,
    ctrlc: Option<Arc<AtomicBool>>,
) -> Result<PipelineData, ShellError> {
    let (reader, span, metadata) = input.into_reader(
        span,
        Some(
            u8::try_from(config.record_separator).map_err(|err| ShellError::IncorrectValue {
                msg: format!("Invalid separator: {}", err),
                val_span: span,
                call_span: span,
            })?,
        ),
    )?;

    let csv_err = |err: csv::Error| ShellError::GenericError {
        error: "CSVError".into(),
        msg: err.to_string(),
        span: Some(span),
        help: None,
        inner: vec![],
    };

    Ok(from_delimited_to_values(config, reader, span)
        .map_err(csv_err)?
        .into_pipeline_data_with_metadata(metadata, ctrlc))
}

pub fn trim_from_str(trim: Option<Value>) -> Result<Trim, ShellError> {
    match trim {
        Some(v) => {
            let span = v.span();
            match v {
                Value::String {val: item, ..} => match item.as_str() {

            "all" => Ok(Trim::All),
            "headers" => Ok(Trim::Headers),
            "fields" => Ok(Trim::Fields),
            "none" => Ok(Trim::None),
            _ => Err(ShellError::TypeMismatch {
                err_message:
                    "the only possible values for trim are 'all', 'headers', 'fields' and 'none'"
                        .into(),
                span,
            }),
                }
                _ => Ok(Trim::None),
            }
        }
        _ => Ok(Trim::None),
    }
}
