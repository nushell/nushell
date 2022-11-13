use nu_engine::{eval_expression, CallExt};
use nu_parser::parse_expression;
use nu_protocol::ast::{Call, PathMember};
use nu_protocol::engine::{Command, EngineState, Stack, StateWorkingSet};
use nu_protocol::{
    Category, Example, ListStream, PipelineData, ShellError, Signature, Span, SyntaxShape, Type,
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
            .input_output_types(vec![(
                Type::Table(vec![]),
                Type::List(Box::new(Type::String)),
            )])
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
        let mut working_set = StateWorkingSet::new(engine_state);

        let specified_pattern: Result<Value, ShellError> = call.req(engine_state, stack, 0);
        let input_val = input.into_value(call.head);
        // add '$it' variable to support format like this: $it.column1.column2.
        let it_id = working_set.add_variable(b"$it".to_vec(), call.head, Type::Any, false);
        stack.add_var(it_id, input_val.clone());

        match specified_pattern {
            Err(e) => Err(e),
            Ok(pattern) => {
                let string_pattern = pattern.as_string()?;
                let string_span = pattern.span()?;
                // the string span is start as `"`, we don't need the character
                // to generate proper span for sub expression.
                let ops = extract_formatting_operations(string_pattern, string_span.start + 1);

                format(
                    input_val,
                    &ops,
                    engine_state,
                    &mut working_set,
                    stack,
                    call.head,
                )
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

// NOTE: The reason to split {column1.column2} and {$it.column1.column2}:
// for {column1.column2}, we just need to follow given record or list.
// for {$it.column1.column2} or {$variable}, we need to manually evaluate the expression.
//
// Have thought about converting from {column1.column2} to {$it.column1.column2}, but that
// will extend input relative span, finally make `nu` panic out with message: span missing in file
// contents cache.
#[derive(Debug)]
enum FormatOperation {
    FixedText(String),
    // raw input is something like {column1.column2}
    ValueFromColumn(String, Span),
    // raw input is something like {$it.column1.column2} or {$var}.
    ValueNeedEval(String, Span),
}

/// Given a pattern that is fed into the Format command, we can process it and subdivide it
/// in two kind of operations.
/// FormatOperation::FixedText contains a portion of the pattern that has to be placed
/// there without any further processing.
/// FormatOperation::ValueFromColumn contains the name of a column whose values will be
/// formatted according to the input pattern.
/// FormatOperation::ValueNeedEval contains expression which need to eval, it has the following form:
/// "$it.column1.column2" or "$variable"
fn extract_formatting_operations(input: String, span_start: usize) -> Vec<FormatOperation> {
    let mut output = vec![];

    let mut characters = input.char_indices();

    let mut column_span_start = 0;
    let mut column_span_end = 0;
    loop {
        let mut before_bracket = String::new();

        for (index, ch) in &mut characters {
            if ch == '{' {
                column_span_start = index + 1; // not include '{' character.
                break;
            }
            before_bracket.push(ch);
        }

        if !before_bracket.is_empty() {
            output.push(FormatOperation::FixedText(before_bracket.to_string()));
        }

        let mut column_name = String::new();
        let mut column_need_eval = false;
        for (index, ch) in &mut characters {
            if ch == '$' {
                column_need_eval = true;
            }

            if ch == '}' {
                column_span_end = index; // not include '}' character.
                break;
            }
            column_name.push(ch);
        }

        if !column_name.is_empty() {
            if column_need_eval {
                output.push(FormatOperation::ValueNeedEval(
                    column_name.clone(),
                    Span {
                        start: span_start + column_span_start,
                        end: span_start + column_span_end,
                    },
                ));
            } else {
                output.push(FormatOperation::ValueFromColumn(
                    column_name.clone(),
                    Span {
                        start: span_start + column_span_start,
                        end: span_start + column_span_end,
                    },
                ));
            }
        }

        if before_bracket.is_empty() && column_name.is_empty() {
            break;
        }
    }
    output
}

/// Format the incoming PipelineData according to the pattern
fn format(
    input_data: Value,
    format_operations: &[FormatOperation],
    engine_state: &EngineState,
    working_set: &mut StateWorkingSet,
    stack: &mut Stack,
    head_span: Span,
) -> Result<PipelineData, ShellError> {
    let data_as_value = input_data;

    //  We can only handle a Record or a List of Records
    match data_as_value {
        Value::Record { .. } => {
            match format_record(
                format_operations,
                &data_as_value,
                engine_state,
                working_set,
                stack,
            ) {
                Ok(value) => Ok(PipelineData::Value(Value::string(value, head_span), None)),
                Err(value) => Err(value),
            }
        }

        Value::List { vals, .. } => {
            let mut list = vec![];
            for val in vals.iter() {
                match val {
                    Value::Record { .. } => {
                        match format_record(
                            format_operations,
                            val,
                            engine_state,
                            working_set,
                            stack,
                        ) {
                            Ok(value) => {
                                list.push(Value::string(value, head_span));
                            }
                            Err(value) => {
                                return Err(value);
                            }
                        }
                    }

                    _ => {
                        return Err(ShellError::UnsupportedInput(
                            "Input data is not supported by this command.".to_string(),
                            head_span,
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
            head_span,
        )),
    }
}

fn format_record(
    format_operations: &[FormatOperation],
    data_as_value: &Value,
    engine_state: &EngineState,
    working_set: &mut StateWorkingSet,
    stack: &mut Stack,
) -> Result<String, ShellError> {
    let config = engine_state.get_config();
    let mut output = String::new();

    for op in format_operations {
        match op {
            FormatOperation::FixedText(s) => output.push_str(s.as_str()),
            FormatOperation::ValueFromColumn(col_name, span) => {
                // path member should split by '.' to handle for nested structure.
                let path_members: Vec<PathMember> = col_name
                    .split('.')
                    .map(|path| PathMember::String {
                        val: path.to_string(),
                        span: *span,
                    })
                    .collect();
                match data_as_value.clone().follow_cell_path(&path_members, false) {
                    Ok(value_at_column) => {
                        output.push_str(value_at_column.into_string(", ", config).as_str())
                    }
                    Err(se) => return Err(se),
                }
            }
            FormatOperation::ValueNeedEval(_col_name, span) => {
                let (exp, may_parse_err) = parse_expression(working_set, &[*span], &[]);
                match may_parse_err {
                    None => {
                        let parsed_result = eval_expression(engine_state, stack, &exp);
                        if let Ok(val) = parsed_result {
                            output.push_str(&val.into_abbreviated_string(config))
                        }
                    }
                    Some(err) => {
                        return Err(ShellError::UnsupportedInput(
                            format!("expression is invalid, detail message: {:?}", err),
                            *span,
                        ))
                    }
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
