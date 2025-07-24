use csv::WriterBuilder;
use nu_cmd_base::formats::to::delimited::merge_descriptors;
use nu_protocol::{
    ByteStream, ByteStreamType, Config, PipelineData, ShellError, Signals, Span, Spanned, Value,
    shell_error::io::IoError,
};
use std::{iter, sync::Arc};

fn make_csv_error(error: csv::Error, format_name: &str, head: Span) -> ShellError {
    if let csv::ErrorKind::Io(error) = error.kind() {
        IoError::new(error, head, None).into()
    } else {
        ShellError::GenericError {
            error: format!("Failed to generate {format_name} data"),
            msg: error.to_string(),
            span: Some(head),
            help: None,
            inner: vec![],
        }
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

pub struct ToDelimitedDataArgs {
    pub noheaders: bool,
    pub separator: Spanned<char>,
    pub columns: Option<Vec<String>>,
    pub format_name: &'static str,
    pub input: PipelineData,
    pub head: Span,
    pub content_type: Option<String>,
}

pub fn to_delimited_data(
    ToDelimitedDataArgs {
        noheaders,
        separator,
        columns,
        format_name,
        input,
        head,
        content_type,
    }: ToDelimitedDataArgs,
    config: Arc<Config>,
) -> Result<PipelineData, ShellError> {
    let mut input = input;
    let span = input.span().unwrap_or(head);
    let metadata = Some(
        input
            .metadata()
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

    // Determine the columns we'll use. This is necessary even if we don't write the header row,
    // because we need to write consistent columns.
    let columns = match columns {
        Some(columns) => columns,
        None => {
            // The columns were not provided. We need to detect them, and in order to do so, we have
            // to convert the input into a value first, so that we can find all of them
            let value = input.into_value(span)?;
            let columns = match &value {
                Value::List { vals, .. } => merge_descriptors(vals),
                Value::Record { val, .. } => val.columns().cloned().collect(),
                _ => return Err(make_unsupported_input_error(value.get_type(), head, span)),
            };
            input = PipelineData::Value(value, metadata.clone());
            columns
        }
    };

    // Generate a byte stream of all of the values in the pipeline iterator, with a non-strict
    // iterator so we can still accept plain records.
    let mut iter = input.into_iter();

    // If we're configured to generate a header, we generate it first, then set this false
    let mut is_header = !noheaders;

    let stream = ByteStream::from_fn(
        head,
        Signals::empty(),
        ByteStreamType::String,
        move |buffer| {
            let mut wtr = WriterBuilder::new()
                .delimiter(separator)
                .from_writer(buffer);

            if is_header {
                // Unless we are configured not to write a header, we write the header row now, once,
                // before everything else.
                wtr.write_record(&columns)
                    .map_err(|err| make_csv_error(err, format_name, head))?;
                is_header = false;
                Ok(true)
            } else if let Some(row) = iter.next() {
                // Write each column of a normal row, in order
                let record = row.into_record()?;
                for column in &columns {
                    let field = record
                        .get(column)
                        .map(|v| to_string_tagged_value(v, &config, format_name))
                        .unwrap_or(Ok(String::new()))?;
                    wtr.write_field(field)
                        .map_err(|err| make_csv_error(err, format_name, head))?;
                }
                // End the row
                wtr.write_record(iter::empty::<String>())
                    .map_err(|err| make_csv_error(err, format_name, head))?;
                Ok(true)
            } else {
                Ok(false)
            }
        },
    );

    Ok(PipelineData::ByteStream(stream, metadata))
}
