use nu_engine::command_prelude::*;
use nu_parser::parse_simple_cell_path;
use nu_protocol::{
    ast::{Expr, PathMember},
    engine::StateWorkingSet,
    Config, LabeledError,
};

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
                "The pattern to output (e.g., \"{foo}: {bar}\").",
            )
            .category(Category::Strings)
    }

    fn description(&self) -> &str {
        "Format columns into a string using a simple pattern.

A pattern is a string which can contain cell paths within curly braces.

If a template cell path refers to a column, then a string is created by filling the pattern for each row.
If a cell path refers to a value, then the value is inserted directly into the pattern.

When multiple cell paths are used, format pattern operates row-wise over the cell path with the fewest components.
Cell paths with additional components can be used to access nested data within each row."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["printf", "template"]
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
                example: r#"ls | format pattern "{name}: {size}""#,
                result: None,
            },
            Example {
                description: "Print elements from some columns of a table",
                example: r#"[[col1, col2]; [v1, v2] [v3, v4]] | format pattern "{col2}""#,
                result: Some(Value::list(
                    vec![Value::test_string("v2"), Value::test_string("v4")],
                    Span::test_data(),
                )),
            },
            Example {
                description: "Print specific elements of a list",
                example: r#"[world hello] | format pattern "{1}, {0}!""#,
                result: Some(Value::test_string("hello, world!")),
            },
            Example {
                description: "Print nested elements",
                example: r#"[foo bar baz qux] | window 2 | enumerate | format pattern "{index}: {item.0} -> {item.1}""#,
                result: Some(Value::test_list(vec![
                    Value::test_string("0: foo -> bar"),
                    Value::test_string("1: bar -> baz"),
                    Value::test_string("2: baz -> qux"),
                ])),
            },
        ]
    }
}

/// A format operation parsed from a pattern
enum FormatOperation {
    /// A portion of the pattern to be inserted without any further processing
    FixedText(String),
    /// A cell path referring to the value or column to be formatted into the pattern template
    CellPath(CellPath, Span),
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

        let span = expression.span(&working_set);
        if let Expr::CellPath(cell_path) = expression.expr {
            // successfully parsed pattern, start over
            output.push(FormatOperation::CellPath(cell_path, span));
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
    /// A single value, which might be different depending on the row
    Value(CellPath),
}

/// Format the incoming PipelineData according to the pattern
fn format(
    input_data: Value,
    format_operations: Vec<FormatOperation>,
    config: &Config,
    span: Span,
) -> Result<PipelineData, ShellError> {
    let mut extracted_operations = Vec::with_capacity(format_operations.len());
    let mut column_size: Option<(usize, Span)> = None;

    // use the cell path with the fewest members as our basis for operating row-wise
    let min_depth = format_operations
        .iter()
        .filter_map(|op| match op {
            FormatOperation::FixedText(_) => None,
            FormatOperation::CellPath(cell_path, _) => Some(cell_path.members.len()),
        })
        .min()
        .unwrap_or(0);

    for operation in format_operations {
        let extracted = match operation {
            FormatOperation::FixedText(text) => ExtractedOperation::FixedText(text),
            FormatOperation::CellPath(cell_path, span) => {
                let depth = cell_path.members.len();
                let inner = input_data
                    .clone()
                    .follow_cell_path(&cell_path.members, false);
                match inner {
                    Ok(Value::Error { error, .. }) => return Err(*error),
                    Ok(Value::List { vals, .. }) if depth == min_depth => {
                        match column_size {
                            Some((size, _)) if size == vals.len() => (),
                            Some((size, old_span)) => {
                                return Err(ShellError::LabeledError(Box::new(
                                    LabeledError::new("Mismatched column lengths")
                                        .with_help("Attempted to format pattern row-wise over columns of different lengths")
                                        .with_label(format!("this column has a length of {}", size), old_span)
                                        .with_label(format!("this column has a length of {}", vals.len()), span)
                                )))
                            }
                            None => column_size = Some((vals.len(), span)),
                        }
                        ExtractedOperation::Column(vals)
                    }
                    _ => ExtractedOperation::Value(cell_path),
                }
            }
        };
        extracted_operations.push(extracted);
    }

    let out = match column_size {
        Some((size, _)) => (0..size)
            .map(|row| format_row(&input_data, &extracted_operations, Some(row), span, &config))
            .collect::<Result<Vec<Value>, ShellError>>()?
            .into_value(span),
        None => format_row(&input_data, &extracted_operations, None, span, &config)?,
    };
    Ok(out.into_pipeline_data())
}

/// `row` must be `Some` if any operations are `ExtractedOperation::Column`
fn format_row(
    input_data: &Value,
    operations: &[ExtractedOperation],
    row: Option<usize>,
    span: Span,
    config: &Config,
) -> Result<Value, ShellError> {
    let mut output = String::new();
    for operation in operations.iter() {
        let text = match operation {
            ExtractedOperation::FixedText(text) => text,
            ExtractedOperation::Value(cell_path) => {
                let members: Vec<PathMember> = row
                    .map(|val| PathMember::int(val, false, span))
                    .into_iter()
                    .chain(cell_path.members.clone())
                    .collect();
                &input_data
                    .clone()
                    .follow_cell_path(&members, false)?
                    .to_expanded_string(", ", config)
            }
            ExtractedOperation::Column(values) => {
                let row = row.expect("Column operation found, but no row passed");
                &values[row].to_expanded_string(", ", config)
            }
        };
        output.push_str(text);
    }
    Ok(Value::string(output, span))
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
