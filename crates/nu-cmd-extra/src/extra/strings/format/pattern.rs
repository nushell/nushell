use nu_engine::command_prelude::*;
use nu_parser::parse_simple_cell_path;
use nu_protocol::{ast::Expr, engine::StateWorkingSet, Config};

#[derive(Clone)]
pub struct FormatPattern;

impl Command for FormatPattern {
    fn name(&self) -> &str {
        "format pattern"
    }

    fn signature(&self) -> Signature {
        Signature::build("format pattern")
            .input_output_types(vec![
                (Type::table(), Type::List(Box::new(Type::String))),
                (Type::record(), Type::String),
                // TODO: only string types
                (Type::Any, Type::Any),
            ])
            .required(
                "pattern",
                SyntaxShape::String,
                "The pattern to output. e.g.) \"{foo}: {bar}\".",
            )
            .allow_variants_without_examples(true)
            .category(Category::Strings)
    }

    fn description(&self) -> &str {
        "Format columns into a string using a simple pattern."
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let pattern: Value = call.req(engine_state, stack, 0)?;
        let input_val = input.into_value(call.head)?;

        let config = stack.get_config(engine_state);

        let string_span = pattern.span();
        let string_pattern = pattern.coerce_into_string()?;
        // the string span is start as `"`, we don't need the character
        // to generate proper span for sub expression.
        let ops =
            extract_formatting_operations(engine_state, string_pattern, string_span.start + 1)?;

        format(input_val, &ops, &config, call.head)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Print filenames with their sizes",
                example: "ls | format pattern '{name}: {size}'",
                result: None,
            },
            Example {
                description: "Print elements from some columns of a table",
                example: "[[col1, col2]; [v1, v2] [v3, v4]] | format pattern '{col2}'",
                result: Some(Value::list(
                    vec![Value::test_string("v2"), Value::test_string("v4")],
                    Span::test_data(),
                )),
            },
        ]
    }
}

#[derive(Debug)]
enum FormatOperation {
    FixedText(String),
    CellPath(CellPath),
}

/// Given a pattern that is fed into the `format pattern` command, we can process it and subdivide it
/// in two kind of operations.
/// FormatOperation::FixedText contains a portion of the pattern that has to be placed
/// there without any further processing.
/// FormatOperation::CellPath contains the cell path whose values will be
/// formatted according to the input pattern.
fn extract_formatting_operations(
    engine_state: &EngineState,
    input: String,
    span_start: usize,
) -> Result<Vec<FormatOperation>, ShellError> {
    let mut output = vec![];

    let mut characters = input.char_indices().peekable();

    let mut pattern_range = (None, None);
    loop {
        let mut before_bracket = String::new();

        // scan for opening curly brace
        for (index, ch) in &mut characters {
            if ch == '{' {
                pattern_range.0 = Some(index + 1); // not include '{' character.
                break;
            }
            before_bracket.push(ch);
        }

        if !before_bracket.is_empty() {
            output.push(FormatOperation::FixedText(before_bracket.to_string()));
        }

        // scan for closing curly brace
        for (index, ch) in &mut characters {
            if ch == '}' {
                pattern_range.1 = Some(index); // not include '}' character.
                break;
            }
        }

        let pattern_span = match pattern_range {
            // found start and end of pattern
            (Some(start), Some(end)) => Span::new(span_start + start, span_start + end),
            // missing closing curly brace
            (Some(start), None) if characters.peek().is_none() => {
                return Err(ShellError::DelimiterError {
                    msg: "unmatched curly brace".to_string(),
                    span: Span::new(span_start + start - 1, span_start + start - 1),
                })
            }
            //  we're done parsing
            _ if characters.peek().is_none() => break,
            _ => continue,
        };

        // parse the pattern contents into a cell path
        let mut working_set = StateWorkingSet::new(engine_state);
        let expression = parse_simple_cell_path(&mut working_set, pattern_span);

        // return if parsing error
        match working_set.parse_errors.first() {
            Some(err) => return Err(ShellError::LabeledError(Box::new(err.clone().into()))),
            None => (),
        }

        if let Expr::CellPath(cell_path) = expression.expr {
            // successfully parsed pattern, start over
            output.push(FormatOperation::CellPath(cell_path));
            pattern_range = (None, None);
        } else {
            return Err(ShellError::NushellFailed {
                msg: "received non cell path expression".into(),
            });
        }
    }

    Ok(output)
}

/// Format the incoming PipelineData according to the pattern
fn format(
    input_data: Value,
    format_operations: &[FormatOperation],
    config: &Config,
    head_span: Span,
) -> Result<PipelineData, ShellError> {
    //  We can only handle a Record or a List of Records
    match input_data {
        Value::Record { .. } => match format_record(format_operations, &input_data, config) {
            Ok(value) => Ok(PipelineData::Value(Value::string(value, head_span), None)),
            Err(value) => Err(value),
        },

        Value::List { vals, .. } => {
            let mut list = vec![];
            for val in vals.iter() {
                match val {
                    Value::Record { .. } => match format_record(format_operations, val, config) {
                        Ok(value) => {
                            list.push(Value::string(value, head_span));
                        }
                        Err(value) => {
                            return Err(value);
                        }
                    },
                    Value::Error { error, .. } => return Err(*error.clone()),
                    _ => {
                        return Err(ShellError::OnlySupportsThisInputType {
                            exp_input_type: "record".to_string(),
                            wrong_type: val.get_type().to_string(),
                            dst_span: head_span,
                            src_span: val.span(),
                        })
                    }
                }
            }

            Ok(Value::list(list, head_span).into_pipeline_data())
        }
        Value::Error { error, .. } => Err(*error),
        _ => Ok(Value::string(
            format_record(format_operations, &input_data, config)?,
            head_span,
        )
        .into_pipeline_data()),
        // _ => Err(ShellError::OnlySupportsThisInputType {
        //     exp_input_type: "record".to_string(),
        //     wrong_type: data_as_value.get_type().to_string(),
        //     dst_span: head_span,
        //     src_span: data_as_value.span(),
        // }),
    }
}

fn format_record(
    format_operations: &[FormatOperation],
    data_as_value: &Value,
    config: &Config,
) -> Result<String, ShellError> {
    let mut output = String::new();

    for op in format_operations {
        match op {
            FormatOperation::FixedText(s) => output.push_str(s.as_str()),
            FormatOperation::CellPath(cell_path) => {
                match data_as_value
                    .clone()
                    .follow_cell_path(&cell_path.members, false)
                {
                    Ok(value_at_column) => {
                        output.push_str(value_at_column.to_expanded_string(", ", config).as_str())
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
        use super::FormatPattern;
        use crate::test_examples;
        test_examples(FormatPattern {})
    }
}
