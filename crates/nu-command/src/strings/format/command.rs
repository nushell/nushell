use nu_engine::CallExt;
use nu_protocol::ast::{Call, PathMember};
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Config, Example, ListStream, PipelineData, ShellError, Signature, Span, SyntaxShape,
    Value,
};

#[derive(Clone)]
pub struct Format;

impl Command for Format {
    fn name(&self) -> &str {
        "format"
    }

    fn signature(&self) -> Signature {
        Signature::build("format")
            .required(
                "pattern",
                SyntaxShape::String,
                "the pattern to output. e.g.) \"{foo}: {bar}\"",
            )
            .category(Category::Strings)
    }

    fn usage(&self) -> &str {
        "Format columns into a string using a simple pattern."
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let config = engine_state.get_config();
        let specified_pattern: Result<Value, ShellError> = call.req(engine_state, stack, 0);
        match specified_pattern {
            Err(e) => Err(e),
            Ok(pattern) => {
                let string_pattern = pattern.as_string()?;
                let ops = extract_formatting_operations(string_pattern);
                format(input, &ops, call.head, config)
            }
        }
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Print filenames with their sizes",
                example: "ls | format '{name}: {size}'",
                result: None,
            },
            Example {
                description: "Print elements from some columns of a table",
                example: "echo [[col1, col2]; [v1, v2] [v3, v4]] | format '{col2}'",
                result: Some(Value::List {
                    vals: vec![Value::test_string("v2"), Value::test_string("v4")],
                    span: Span::test_data(),
                }),
            },
        ]
    }
}

#[derive(Debug)]
enum FormatOperation {
    FixedText(String),
    ValueFromColumn(String),
}

/// Given a pattern that is fed into the Format command, we can process it and subdivide it
/// in two kind of operations.
/// FormatOperation::FixedText contains a portion of the pattern that has to be placed
/// there without any further processing.
/// FormatOperation::ValueFromColumn contains the name of a column whose values will be
/// formatted according to the input pattern.
fn extract_formatting_operations(input: String) -> Vec<FormatOperation> {
    let mut output = vec![];

    let mut characters = input.chars();
    'outer: loop {
        let mut before_bracket = String::new();

        for ch in &mut characters {
            if ch == '{' {
                break;
            }
            before_bracket.push(ch);
        }

        if !before_bracket.is_empty() {
            output.push(FormatOperation::FixedText(before_bracket.to_string()));
        }

        let mut column_name = String::new();

        for ch in &mut characters {
            if ch == '}' {
                break;
            }
            column_name.push(ch);
        }

        if !column_name.is_empty() {
            output.push(FormatOperation::ValueFromColumn(column_name.clone()));
        }

        if before_bracket.is_empty() && column_name.is_empty() {
            break 'outer;
        }
    }
    output
}

/// Format the incoming PipelineData according to the pattern
fn format(
    input_data: PipelineData,
    format_operations: &[FormatOperation],
    span: Span,
    config: &Config,
) -> Result<PipelineData, ShellError> {
    let data_as_value = input_data.into_value(span);

    //  We can only handle a Record or a List of Records
    match data_as_value {
        Value::Record { .. } => {
            match format_record(format_operations, &data_as_value, span, config) {
                Ok(value) => Ok(PipelineData::Value(Value::string(value, span), None)),
                Err(value) => Err(value),
            }
        }

        Value::List { vals, .. } => {
            let mut list = vec![];
            for val in vals.iter() {
                match val {
                    Value::Record { .. } => {
                        match format_record(format_operations, val, span, config) {
                            Ok(value) => {
                                list.push(Value::string(value, span));
                            }
                            Err(value) => {
                                return Err(value);
                            }
                        }
                    }

                    _ => {
                        return Err(ShellError::UnsupportedInput(
                            "Input data is not supported by this command.".to_string(),
                            span,
                        ))
                    }
                }
            }

            Ok(PipelineData::ListStream(
                ListStream::from_stream(list.into_iter(), None),
                None,
            ))
        }
        _ => Err(ShellError::UnsupportedInput(
            "Input data is not supported by this command.".to_string(),
            span,
        )),
    }
}

fn format_record(
    format_operations: &[FormatOperation],
    data_as_value: &Value,
    span: Span,
    config: &Config,
) -> Result<String, ShellError> {
    let mut output = String::new();
    for op in format_operations {
        match op {
            FormatOperation::FixedText(s) => output.push_str(s.as_str()),

            //  The referenced code suggests to use the correct Spans
            //  See: https://github.com/nushell/nushell/blob/c4af5df828135159633d4bc3070ce800518a42a2/crates/nu-command/src/commands/strings/format/command.rs#L61
            FormatOperation::ValueFromColumn(col_name) => {
                match data_as_value
                    .clone()
                    .follow_cell_path(&[PathMember::String {
                        val: col_name.clone(),
                        span,
                    }]) {
                    Ok(value_at_column) => {
                        output.push_str(value_at_column.into_string(", ", config).as_str())
                    }
                    Err(se) => return Err(se),
                }
            }
        }
    }
    Ok(output)
}

#[cfg(test)]
mod test {
    #[test]
    fn test_examples() {
        use super::Format;
        use crate::test_examples;
        test_examples(Format {})
    }
}
