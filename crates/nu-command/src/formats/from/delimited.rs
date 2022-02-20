use csv::ReaderBuilder;
use nu_protocol::{Config, IntoPipelineData, PipelineData, ShellError, Span, Value};

fn from_delimited_string_to_value(
    s: String,
    noheaders: bool,
    separator: char,
    span: Span,
) -> Result<Value, csv::Error> {
    let mut reader = ReaderBuilder::new()
        .has_headers(!noheaders)
        .delimiter(separator as u8)
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
    sep: char,
    input: PipelineData,
    name: Span,
    config: &Config,
) -> Result<PipelineData, ShellError> {
    let concat_string = input.collect_string("", config)?;

    Ok(
        from_delimited_string_to_value(concat_string, noheaders, sep, name)
            .map_err(|x| ShellError::DelimiterError(x.to_string(), name))?
            .into_pipeline_data(),
    )
}
