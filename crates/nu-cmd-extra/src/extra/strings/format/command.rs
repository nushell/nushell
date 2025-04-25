use nu_engine::command_prelude::*;
use nu_protocol::{ast::PathMember, engine::StateWorkingSet, Config, ListStream};
use uucore::format::{parse_spec_and_escape, FormatArgument, FormatError, FormatItem};

#[derive(Clone)]
pub struct FormatPattern;

impl Command for FormatPattern {
    fn name(&self) -> &str {
        "format pattern"
    }

    fn signature(&self) -> Signature {
        Signature::build("format pattern")
            .input_output_types(vec![
                (Type::list(Type::Any), Type::String),
                (Type::table(), Type::List(Box::new(Type::String))),
                (Type::record(), Type::Any),
                (Type::Any, Type::String),
            ])
            .required(
                "pattern",
                SyntaxShape::String,
                "The pattern used to format the input values.",
            )
            .switch("printf", "Use `printf`-compatible pattern.", None)
            .allow_variants_without_examples(true)
            .category(Category::Strings)
    }

    fn description(&self) -> &str {
        r"Format values into a string using either a simple pattern or `printf`-compatible pattern.
Simple pattern supports input of type list<any>, table, and record;
`printf` pattern supports input types that are coercible to string, namely bool, int, float, string, glob, binary and date, and also list of a mix of those types.
        "
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let mut working_set = StateWorkingSet::new(engine_state);

        let use_printf = call.has_flag(engine_state, stack, "printf")?;
        let specified_pattern: Result<Value, ShellError> = call.req(engine_state, stack, 0);
        let input_val = input.into_value(call.head)?;
        // add '$it' variable to support format like this: $it.column1.column2.
        let it_id = working_set.add_variable(b"$it".to_vec(), call.head, Type::Any, false);
        stack.add_var(it_id, input_val.clone());

        let config = stack.get_config(engine_state);

        match (specified_pattern, use_printf) {
            (Err(e), _) => Err(e),
            (Ok(pattern), false) => {
                let string_span = pattern.span();
                let string_pattern = pattern.coerce_into_string()?;
                // the string span is start as `"`, we don't need the character
                // to generate proper span for sub expression.
                let ops = extract_formatting_operations(
                    string_pattern,
                    call.head,
                    string_span.start + 1,
                )?;

                format(input_val, &ops, engine_state, &config, call.head)
            }
            (Ok(pattern), true) => format_printf(input_val, pattern, call.head),
        }
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
            Example {
                description: "Unescape a fully quoted json using printf",
                example: r#""\{\\\"foo\\\": \\\"bar\\\"\}" | format pattern --printf "%b""#,
                result: Some(Value::test_string(r#"{"foo": "bar"}"#)),
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
}

/// Given a pattern that is fed into the Format command, we can process it and subdivide it
/// in two kind of operations.
/// FormatOperation::FixedText contains a portion of the pattern that has to be placed
/// there without any further processing.
/// FormatOperation::ValueFromColumn contains the name of a column whose values will be
/// formatted according to the input pattern.
/// "$it.column1.column2" or "$variable"
fn extract_formatting_operations(
    input: String,
    error_span: Span,
    span_start: usize,
) -> Result<Vec<FormatOperation>, ShellError> {
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

        if column_span_end < column_span_start {
            return Err(ShellError::DelimiterError {
                msg: "there are unmatched curly braces".to_string(),
                span: error_span,
            });
        }

        if !column_name.is_empty() {
            if column_need_eval {
                return Err(ShellError::GenericError {
                    error: "Removed functionality".into(),
                    msg: "The ability to use variables ($it) in `format pattern` has been removed."
                        .into(),
                    span: Some(error_span),
                    help: Some(
                        "You can use other formatting options, such as string interpolation."
                            .into(),
                    ),
                    inner: vec![],
                });
            } else {
                output.push(FormatOperation::ValueFromColumn(
                    column_name.clone(),
                    Span::new(span_start + column_span_start, span_start + column_span_end),
                ));
            }
        }

        if before_bracket.is_empty() && column_name.is_empty() {
            break;
        }
    }
    Ok(output)
}

/// Format the incoming PipelineData according to the pattern
fn format(
    input_data: Value,
    format_operations: &[FormatOperation],
    engine_state: &EngineState,
    config: &Config,
    head_span: Span,
) -> Result<PipelineData, ShellError> {
    let data_as_value = input_data;

    //  We can only handle a Record or a List of Records
    match data_as_value {
        Value::Record { .. } => match format_record(format_operations, &data_as_value, config) {
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

            Ok(ListStream::new(list.into_iter(), head_span, engine_state.signals().clone()).into())
        }
        // Unwrapping this ShellError is a bit unfortunate.
        // Ideally, its Span would be preserved.
        Value::Error { error, .. } => Err(*error),
        _ => Err(ShellError::OnlySupportsThisInputType {
            exp_input_type: "record".to_string(),
            wrong_type: data_as_value.get_type().to_string(),
            dst_span: head_span,
            src_span: data_as_value.span(),
        }),
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
            FormatOperation::ValueFromColumn(col_name, span) => {
                // path member should split by '.' to handle for nested structure.
                let path_members: Vec<PathMember> = col_name
                    .split('.')
                    .map(|path| PathMember::String {
                        val: path.to_string(),
                        span: *span,
                        optional: false,
                    })
                    .collect();
                match data_as_value.clone().follow_cell_path(&path_members, false) {
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

fn assert_specifier_count_eq_arg_count(
    spec_count: usize,
    arg_count: usize,
    span: Span,
) -> Result<(), ShellError> {
    if spec_count != arg_count {
        Err(ShellError::IncompatibleParametersSingle {
            msg: format!(
                "Number of arguments ({}) provided does not match the number of specifiers ({}) given in the pattern.",
                arg_count, spec_count,
            )
            .into(),
            span: span,
        })
    } else {
        Ok(())
    }
}

fn format_printf(
    input_data: Value,
    pattern: Value,
    head_span: Span,
) -> Result<PipelineData, ShellError> {
    let pattern_str = pattern.coerce_into_string()?;
    let spec_count = parse_spec_and_escape(pattern_str.as_ref())
        .filter_map(|item| match item {
            Ok(FormatItem::Spec(_)) => Some(()),
            _ => None,
        })
        .count();
    let args: Vec<String> = match input_data {
        v @ Value::List { .. } => {
            let span = v.span();
            let vals = v.into_list()?;
            let arg_count = Vec::len(&vals);
            assert_specifier_count_eq_arg_count(spec_count, arg_count, span)?;
            vals.into_iter()
                .map(Value::coerce_into_string)
                .collect::<Result<Vec<_>, _>>()?
        }
        v @ Value::Nothing {..} => {
            assert_specifier_count_eq_arg_count(spec_count, 0, v.span())?;
            vec![]
        }
        v => {
            assert_specifier_count_eq_arg_count(spec_count, 1, v.span())?;
            vec![v.coerce_into_string()?]
        }
    };

    match printf_spec_escape(pattern_str, args) {
        Ok(value) => Ok(PipelineData::Value(Value::string(value, head_span), None)),
        Err(err) => Err(ShellError::GenericError {
            error: err.to_string(),
            msg: err.to_string(),
            span: Some(head_span),
            help: None,
            inner: vec![],
        }),
    }
}

pub fn printf_spec_escape(pattern: String, args: Vec<String>) -> Result<String, FormatError> {
    let mut writer: Vec<_> = Vec::new();
    let args: Vec<FormatArgument> = args.into_iter().map(FormatArgument::Unparsed).collect();
    let mut args = args.iter().peekable();

    for item in parse_spec_and_escape(pattern.as_ref()) {
        match item {
            Ok(item) => {
                item.write(&mut writer, &mut args)?;
            }
            Err(e) => return Err(e),
        }
    }

    Ok(String::from_utf8_lossy(&writer).to_string())
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
