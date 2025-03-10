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
                (Type::Any, Type::String),
                (Type::Any, Type::list(Type::String)),
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
        let input_as_value = input.into_value(call.head)?;

        let config = stack.get_config(engine_state);

        let string_span = pattern.span();
        let string_pattern = pattern.coerce_into_string()?;
        // the string span is start as `"`, we don't need the character
        // to generate proper span for sub expression.
        let ops = parse_formatting_operations(engine_state, string_pattern, string_span.start + 1)?;

        format(input_as_value, ops, &config, call.head)
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

/// A format operation parsed from a pattern
enum FormatOperation {
    /// A portion of the pattern to be inserted without any further processing
    FixedText(String),
    /// A cell path referring to the value or column to be formatted into the pattern template
    CellPath(CellPath),
}

fn parse_formatting_operations(
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

/// Extract values and columns by following cell paths ahead of time
enum ExtractedOperation {
    /// Fixed text, equivalent to FormatOperation::FixedText
    FixedText(String),
    /// A column of values. All columns must have the same number of rows.
    Column(Vec<Value>),
    /// A single value, which is the same regardless of row.
    Value(Value),
}

/// Format the incoming PipelineData according to the pattern
fn format(
    input_data: Value,
    format_operations: Vec<FormatOperation>,
    config: &Config,
    span: Span,
) -> Result<PipelineData, ShellError> {
    let mut extracted_operations = Vec::with_capacity(format_operations.len());
    let mut column_size: Option<usize> = None;

    for operation in format_operations {
        let extracted = match operation {
            FormatOperation::FixedText(text) => ExtractedOperation::FixedText(text),
            FormatOperation::CellPath(cell_path) => {
                let inner = input_data
                    .clone()
                    .follow_cell_path(&cell_path.members, false)?;
                match inner {
                    Value::Error { error, .. } => return Err(*error),
                    Value::List { vals, .. } => {
                        if let Some(size) = column_size {
                            assert!(vals.len() == size);
                        }
                        column_size = Some(vals.len());
                        ExtractedOperation::Column(vals)
                    }
                    value => ExtractedOperation::Value(value),
                }
            }
        };
        extracted_operations.push(extracted);
    }

    let out = match column_size {
        Some(size) => (0..size)
            .map(|row| format_row(&extracted_operations, row, span, &config))
            .collect::<Vec<Value>>()
            .into_value(span),
        None => format_row(&extracted_operations, 0, span, &config),
    };
    Ok(out.into_pipeline_data())
}

fn format_row(operations: &[ExtractedOperation], row: usize, span: Span, config: &Config) -> Value {
    let mut output = String::new();
    for operation in operations.iter() {
        let text = match operation {
            ExtractedOperation::FixedText(text) => text,
            ExtractedOperation::Column(values) => &values[row].to_expanded_string(", ", config),
            ExtractedOperation::Value(value) => &value.to_expanded_string(", ", config),
        };
        output.push_str(text);
    }
    Value::string(output, span)
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
