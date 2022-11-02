use csv::{ReaderBuilder, Trim};
use nu_protocol::{Config, IntoPipelineData, PipelineData, ShellError, Span, Value};

fn from_delimited_string_to_value(
    s: String,
    noheaders: bool,
    no_infer: bool,
    separator: char,
    trim: Trim,
    span: Span,
) -> Result<Value, csv::Error> {
    let mut reader = ReaderBuilder::new()
        .has_headers(!noheaders)
        .delimiter(separator as u8)
        .trim(trim)
        .from_reader(s.as_bytes());

    let headers = if noheaders {
        (1..=reader.headers()?.len())
            .map(|i| format!("column{}", i))
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
        rows.push(Value::Record {
            cols: headers.clone(),
            vals: output_row,
            span,
        });
    }

    Ok(Value::List { vals: rows, span })
}

pub fn from_delimited_data(
    noheaders: bool,
    no_infer: bool,
    sep: char,
    trim: Trim,
    input: PipelineData,
    name: Span,
    config: &Config,
) -> Result<PipelineData, ShellError> {
    let concat_string = input.collect_string("", config)?;

    Ok(
        from_delimited_string_to_value(concat_string, noheaders, no_infer, sep, trim, name)
            .map_err(|x| ShellError::DelimiterError(x.to_string(), name))?
            .into_pipeline_data(),
    )
}

pub fn trim_from_str(trim: Option<Value>) -> Result<Trim, ShellError> {
    match trim {
        Some(Value::String { val: item, span }) => match item.as_str() {
            "all" => Ok(Trim::All),
            "headers" => Ok(Trim::Headers),
            "fields" => Ok(Trim::Fields),
            "none" => Ok(Trim::None),
            _ => Err(ShellError::UnsupportedInput(
                "the only possible values for trim are 'all', 'headers', 'fields' and 'none'"
                    .into(),
                span,
            )),
        },
        _ => Ok(Trim::None),
    }
}
