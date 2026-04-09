use csv::WriterBuilder;
use nu_cmd_base::formats::to::delimited::merge_descriptors;
use nu_protocol::{
    ByteStream, ByteStreamType, Config, PipelineData, ShellError, Signals, Span, Spanned, Value,
    shell_error::generic::GenericError, shell_error::io::IoError,
};
use std::{iter, sync::Arc};

fn make_csv_error(error: csv::Error, format_name: &str, head: Span) -> ShellError {
    if let csv::ErrorKind::Io(error) = error.kind() {
        IoError::new(error, head, None).into()
    } else {
        ShellError::Generic(GenericError::new(
            format!("Failed to generate {format_name} data"),
            error.to_string(),
            head,
        ))
    }
}

fn to_string_tagged_value(
    v: &Value,
    config: &Config,
    format_name: &'static str,
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
        _ => Err(make_cant_convert_error(v, format_name)),
    }
}

fn make_unsupported_input_error(
    r#type: impl std::fmt::Display,
    head: Span,
    span: Span,
) -> ShellError {
    ShellError::UnsupportedInput {
        msg: "expected table or record".to_string(),
        input: format!("input type: {type}"),
        msg_span: head,
        input_span: span,
    }
}

fn make_cant_convert_error(value: &Value, format_name: &'static str) -> ShellError {
    ShellError::CantConvert {
        to_type: "string".into(),
        from_type: value.get_type().to_string(),
        span: value.span(),
        help: Some(format!(
            "only simple values are supported for {format_name} output"
        )),
    }
}

fn make_schema_drift_error(format_name: &str, new_column: &str, head: Span) -> ShellError {
    let command_name = format_name.to_ascii_lowercase();

    ShellError::Generic(
        GenericError::new(
            format!(
                "streamed {command_name} schema changed: new column '{new_column}' appeared after output started"
            ),
            format!(
                "{format_name} output started with a specific set of columns, but a later row contains an unexpected column."
            ),
            head,
        )
        .with_help(format!(
            "use `to {command_name} --columns [...]` or `collect` before `to {command_name}`"
        )),
    )
}

pub struct ToDelimitedDataArgs {
    pub noheaders: bool,
    pub separator: Spanned<char>,
    pub columns: Option<Vec<String>>,
    pub format_name: &'static str,
    pub input: PipelineData,
    pub head: Span,
    pub content_type: Option<String>,
}

/// Helper function to extract column names from a Value
fn extract_columns(value: &Value) -> Result<Vec<String>, ShellError> {
    match value {
        Value::Record { val, .. } => Ok(val.columns().cloned().collect()),
        Value::List { vals, .. } => Ok(merge_descriptors(vals)),
        Value::Error { error, .. } => Err(*error.clone()),
        other => Err(make_unsupported_input_error(
            other.get_type(),
            other.span(),
            other.span(),
        )),
    }
}

fn current_stream_columns<'a>(
    explicit_columns: bool,
    columns: &'a [String],
    detected_columns: Option<&'a [String]>,
    format_name: &'static str,
    head: Span,
) -> Result<&'a [String], ShellError> {
    if explicit_columns {
        Ok(columns)
    } else {
        detected_columns.ok_or_else(|| {
            ShellError::Generic(GenericError::new(
                format!("failed to initialize streamed {format_name} columns"),
                "the input stream ended before Nushell could determine its schema".to_string(),
                head,
            ))
        })
    }
}

/// Helper function to write a single row to CSV
fn write_csv_row(
    wtr: &mut csv::Writer<&mut Vec<u8>>,
    record: &nu_protocol::Record,
    columns: &[String],
    config: &Config,
    format_name: &'static str,
    head: Span,
) -> Result<(), ShellError> {
    for column in columns {
        let field = record
            .get(column)
            .map(|v| to_string_tagged_value(v, config, format_name))
            .unwrap_or(Ok(String::new()))?;
        wtr.write_field(field)
            .map_err(|err| make_csv_error(err, format_name, head))?;
    }
    wtr.write_record(iter::empty::<String>())
        .map_err(|err| make_csv_error(err, format_name, head))?;
    Ok(())
}

pub fn to_delimited_data(
    ToDelimitedDataArgs {
        noheaders,
        separator,
        columns,
        format_name,
        mut input,
        head,
        content_type,
    }: ToDelimitedDataArgs,
    config: Arc<Config>,
) -> Result<PipelineData, ShellError> {
    let span = input.span().unwrap_or(head);
    let metadata = Some(
        input
            .take_metadata()
            .unwrap_or_default()
            .with_content_type(content_type),
    );

    let separator = u8::try_from(separator.item).map_err(|_| ShellError::IncorrectValue {
        msg: "separator must be an ASCII character".into(),
        val_span: separator.span,
        call_span: head,
    })?;

    // Check to ensure the input is likely one of our supported types first. We can't check a stream
    // without consuming it though
    match input {
        PipelineData::Value(Value::List { .. } | Value::Record { .. }, _) => (),
        PipelineData::Value(Value::Error { error, .. }, _) => return Err(*error),
        PipelineData::Value(other, _) => {
            return Err(make_unsupported_input_error(other.get_type(), head, span));
        }
        PipelineData::ByteStream(..) => {
            return Err(make_unsupported_input_error("byte stream", head, span));
        }
        PipelineData::ListStream(..) => (),
        PipelineData::Empty => (),
    }

    // When the input is already materialized (Value::List or Value::Record), we can inspect all
    // rows upfront to compute the union of all columns via merge_descriptors. This preserves the
    // existing behavior where heterogeneous tables get all columns filled in.
    if let PipelineData::Value(ref value @ (Value::List { .. } | Value::Record { .. }), _) = input {
        let columns = match columns {
            Some(cols) => cols,
            None => extract_columns(value)?,
        };
        let mut iter = input.into_iter();
        let mut header_written = noheaders; // If noheaders is true, consider it already written

        let stream = ByteStream::from_fn(
            head,
            Signals::empty(),
            ByteStreamType::String,
            move |buffer| {
                let mut wtr = WriterBuilder::new()
                    .delimiter(separator)
                    .from_writer(buffer);

                // Write header once if not already written
                if !header_written {
                    wtr.write_record(&columns)
                        .map_err(|err| make_csv_error(err, format_name, head))?;
                    header_written = true;
                }

                if let Some(row) = iter.next() {
                    if let Value::Error { error, .. } = &row {
                        return Err(*error.clone());
                    }
                    let record = row.into_record()?;
                    write_csv_row(&mut wtr, &record, &columns, &config, format_name, head)?;
                    Ok(true)
                } else {
                    Ok(false)
                }
            },
        );
        return Ok(PipelineData::byte_stream(stream, metadata));
    }

    // For streaming input (ListStream), we cannot look ahead to compute the column union.
    // Instead, columns are either provided explicitly via --columns or detected from the first
    // row. Subsequent rows with new columns produce a schema drift error.
    let explicit_columns = columns.is_some();
    let columns = columns.unwrap_or_default();

    let mut iter = input.into_iter();
    let mut detected_columns: Option<Vec<String>> = None;
    let mut first_row_processed = false;
    let mut header_written = noheaders;

    let stream = ByteStream::from_fn(
        head,
        Signals::empty(),
        ByteStreamType::String,
        move |buffer| {
            let mut wtr = WriterBuilder::new()
                .delimiter(separator)
                .from_writer(buffer);

            if explicit_columns && !header_written {
                wtr.write_record(&columns)
                    .map_err(|err| make_csv_error(err, format_name, head))?;
                header_written = true;
            }

            if !first_row_processed {
                match iter.next() {
                    Some(row) => {
                        if let Value::Error { error, .. } = &row {
                            return Err(*error.clone());
                        }

                        // Detect columns from first row if not explicitly provided
                        if !explicit_columns && detected_columns.is_none() {
                            detected_columns = Some(extract_columns(&row)?);
                        }

                        let cols = current_stream_columns(
                            explicit_columns,
                            &columns,
                            detected_columns.as_deref(),
                            format_name,
                            head,
                        )?;

                        if !header_written {
                            wtr.write_record(cols)
                                .map_err(|err| make_csv_error(err, format_name, head))?;
                            header_written = true;
                        }

                        let record = row.into_record()?;
                        write_csv_row(&mut wtr, &record, cols, &config, format_name, head)?;

                        first_row_processed = true;
                        Ok(true)
                    }
                    None => Ok(false),
                }
            } else {
                match iter.next() {
                    Some(row) => {
                        if let Value::Error { error, .. } = &row {
                            return Err(*error.clone());
                        }

                        let cols = current_stream_columns(
                            explicit_columns,
                            &columns,
                            detected_columns.as_deref(),
                            format_name,
                            head,
                        )?;

                        let record = row.into_record()?;

                        // Schema drift detection: error on new columns in streaming mode
                        // without explicit columns. Missing columns are intentionally allowed
                        // and filled with empty strings by write_csv_row, matching the
                        // explicit-columns behavior.
                        if !explicit_columns {
                            for col in record.columns() {
                                if !cols.iter().any(|c| c.as_str() == col) {
                                    return Err(make_schema_drift_error(format_name, col, head));
                                }
                            }
                        }

                        write_csv_row(&mut wtr, &record, cols, &config, format_name, head)?;

                        Ok(true)
                    }
                    None => Ok(false),
                }
            }
        },
    );

    Ok(PipelineData::byte_stream(stream, metadata))
}
