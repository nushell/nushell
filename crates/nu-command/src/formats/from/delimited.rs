use csv::{ReaderBuilder, Trim};
use nu_protocol::{IntoPipelineData, PipelineData, ShellError, Span, Value};

fn from_delimited_string_to_value(
    DelimitedReaderConfig {
        separator,
        comment,
        quote,
        escape,
        noheaders,
        flexible,
        no_infer,
        trim,
    }: DelimitedReaderConfig,
    s: String,
    span: Span,
) -> Result<Value, csv::Error> {
    let mut reader = ReaderBuilder::new()
        .has_headers(!noheaders)
        .flexible(flexible)
        .delimiter(separator as u8)
        .comment(comment.map(|c| c as u8))
        .quote(quote as u8)
        .escape(escape.map(|c| c as u8))
        .trim(trim)
        .from_reader(s.as_bytes());

    let headers = if noheaders {
        (1..=reader.headers()?.len())
            .map(|i| format!("column{i}"))
            .collect::<Vec<String>>()
    } else {
        reader.headers()?.iter().map(String::from).collect()
    };

    let mut rows = vec![];
    for row in reader.records() {
        let mut output_row = vec![];
        for value in row?.iter() {
            if no_infer {
                output_row.push(Value::String {
                    span,
                    val: value.into(),
                });
                continue;
            }

            if let Ok(i) = value.parse::<i64>() {
                output_row.push(Value::Int { val: i, span });
            } else if let Ok(f) = value.parse::<f64>() {
                output_row.push(Value::Float { val: f, span });
            } else {
                output_row.push(Value::String {
                    val: value.into(),
                    span,
                });
            }
        }
        rows.push(Value::record_from_parts(headers.clone(), output_row, span));
    }

    Ok(Value::List { vals: rows, span })
}

pub(super) struct DelimitedReaderConfig {
    pub separator: char,
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
    name: Span,
) -> Result<PipelineData, ShellError> {
    let (concat_string, _span, metadata) = input.collect_string_strict(name)?;

    Ok(from_delimited_string_to_value(config, concat_string, name)
        .map_err(|x| ShellError::DelimiterError {
            msg: x.to_string(),
            span: name,
        })?
        .into_pipeline_data_with_metadata(metadata))
}

pub fn trim_from_str(trim: Option<Value>) -> Result<Trim, ShellError> {
    match trim {
        Some(Value::String { val: item, span }) => match item.as_str() {
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
        },
        _ => Ok(Trim::None),
    }
}
