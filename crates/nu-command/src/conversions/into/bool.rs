use nu_cmd_base::input_handler::{operate, CellPathOnlyArgs};
use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "into bool"
    }

    fn signature(&self) -> Signature {
        Signature::build("into bool")
            .input_output_types(vec![
                (Type::Int, Type::Bool),
                (Type::Number, Type::Bool),
                (Type::String, Type::Bool),
                (Type::Bool, Type::Bool),
                (Type::List(Box::new(Type::Any)), Type::table()),
                (Type::table(), Type::table()),
                (Type::record(), Type::record()),
            ])
            .allow_variants_without_examples(true)
            .rest(
                "rest",
                SyntaxShape::CellPath,
                "For a data structure input, convert data at the given cell paths.",
            )
            .category(Category::Conversions)
    }

    fn usage(&self) -> &str {
        "Convert value to boolean."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["convert", "boolean", "true", "false", "1", "0"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        into_bool(engine_state, stack, call, input)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Convert value to boolean in table",
                example: "[[value]; ['false'] ['1'] [0] [1.0] [true]] | into bool value",
                result: Some(Value::test_list(vec![
                    Value::test_record(record! {
                        "value" => Value::test_bool(false),
                    }),
                    Value::test_record(record! {
                        "value" => Value::test_bool(true),
                    }),
                    Value::test_record(record! {
                        "value" => Value::test_bool(false),
                    }),
                    Value::test_record(record! {
                        "value" => Value::test_bool(true),
                    }),
                    Value::test_record(record! {
                        "value" => Value::test_bool(true),
                    }),
                ])),
            },
            Example {
                description: "Convert bool to boolean",
                example: "true | into bool",
                result: Some(Value::test_bool(true)),
            },
            Example {
                description: "convert int to boolean",
                example: "1 | into bool",
                result: Some(Value::test_bool(true)),
            },
            Example {
                description: "convert float to boolean",
                example: "0.3 | into bool",
                result: Some(Value::test_bool(true)),
            },
            Example {
                description: "convert float string to boolean",
                example: "'0.0' | into bool",
                result: Some(Value::test_bool(false)),
            },
            Example {
                description: "convert string to boolean",
                example: "'true' | into bool",
                result: Some(Value::test_bool(true)),
            },
        ]
    }
}

fn into_bool(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let cell_paths: Vec<CellPath> = call.rest(engine_state, stack, 0)?;
    let args = CellPathOnlyArgs::from(cell_paths);
    operate(action, args, input, call.head, engine_state.ctrlc.clone())
}

fn string_to_boolean(s: &str, span: Span) -> Result<bool, ShellError> {
    match s.trim().to_ascii_lowercase().as_str() {
        "true" => Ok(true),
        "false" => Ok(false),
        o => {
            let val = o.parse::<f64>();
            match val {
                Ok(f) => Ok(f != 0.0),
                Err(_) => Err(ShellError::CantConvert {
                    to_type: "boolean".to_string(),
                    from_type: "string".to_string(),
                    span,
                    help: Some(
                        r#"the strings "true" and "false" can be converted into a bool"#
                            .to_string(),
                    ),
                }),
            }
        }
    }
}

fn action(input: &Value, _args: &CellPathOnlyArgs, span: Span) -> Value {
    match input {
        Value::Bool { .. } => input.clone(),
        Value::Int { val, .. } => Value::bool(*val != 0, span),
        Value::Float { val, .. } => Value::bool(val.abs() >= f64::EPSILON, span),
        Value::String { val, .. } => match string_to_boolean(val, span) {
            Ok(val) => Value::bool(val, span),
            Err(error) => Value::error(error, span),
        },
        // Propagate errors by explicitly matching them before the final case.
        Value::Error { .. } => input.clone(),
        other => Value::error(
            ShellError::OnlySupportsThisInputType {
                exp_input_type: "bool, int, float or string".into(),
                wrong_type: other.get_type().to_string(),
                dst_span: span,
                src_span: other.span(),
            },
            span,
        ),
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(SubCommand {})
    }
}
