use nu_cmd_base::input_handler::{CmdArgument, operate};
use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct IntoBool;

impl Command for IntoBool {
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
                (Type::Nothing, Type::Bool),
                (Type::List(Box::new(Type::Any)), Type::table()),
                (Type::table(), Type::table()),
                (Type::record(), Type::record()),
            ])
            .switch(
                "relaxed",
                "Relaxes conversion to also allow null and any strings.",
                None,
            )
            .allow_variants_without_examples(true)
            .rest(
                "rest",
                SyntaxShape::CellPath,
                "For a data structure input, convert data at the given cell paths.",
            )
            .category(Category::Conversions)
    }

    fn description(&self) -> &str {
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
        let relaxed = call
            .has_flag(engine_state, stack, "relaxed")
            .unwrap_or(false);
        into_bool(engine_state, stack, call, input, relaxed)
    }

    fn examples(&self) -> Vec<Example<'_>> {
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
            Example {
                description: "interpret a null as false",
                example: "null | into bool --relaxed",
                result: Some(Value::test_bool(false)),
            },
            Example {
                description: "interpret any non-false, non-zero string as true",
                example: "'something' | into bool --relaxed",
                result: Some(Value::test_bool(true)),
            },
        ]
    }
}

struct IntoBoolCmdArgument {
    cell_paths: Option<Vec<CellPath>>,
    relaxed: bool,
}

impl CmdArgument for IntoBoolCmdArgument {
    fn take_cell_paths(&mut self) -> Option<Vec<CellPath>> {
        self.cell_paths.take()
    }
}

fn into_bool(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input: PipelineData,
    relaxed: bool,
) -> Result<PipelineData, ShellError> {
    let cell_paths = Some(call.rest(engine_state, stack, 0)?).filter(|v| !v.is_empty());
    let args = IntoBoolCmdArgument {
        cell_paths,
        relaxed,
    };
    operate(action, args, input, call.head, engine_state.signals())
}

fn strict_string_to_boolean(s: &str, span: Span) -> Result<bool, ShellError> {
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

fn action(input: &Value, args: &IntoBoolCmdArgument, span: Span) -> Value {
    let err = || {
        Value::error(
            ShellError::OnlySupportsThisInputType {
                exp_input_type: "bool, int, float or string".into(),
                wrong_type: input.get_type().to_string(),
                dst_span: span,
                src_span: input.span(),
            },
            span,
        )
    };

    match (input, args.relaxed) {
        (Value::Error { .. } | Value::Bool { .. }, _) => input.clone(),
        // In strict mode is this an error, while in relaxed this is just `false`
        (Value::Nothing { .. }, false) => err(),
        (Value::String { val, .. }, false) => match strict_string_to_boolean(val, span) {
            Ok(val) => Value::bool(val, span),
            Err(error) => Value::error(error, span),
        },
        _ => match input.coerce_bool() {
            Ok(val) => Value::bool(val, span),
            Err(_) => err(),
        },
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(IntoBool {})
    }

    #[test]
    fn test_strict_handling() {
        let span = Span::test_data();
        let args = IntoBoolCmdArgument {
            cell_paths: vec![].into(),
            relaxed: false,
        };

        assert!(action(&Value::test_nothing(), &args, span).is_error());
        assert!(action(&Value::test_string("abc"), &args, span).is_error());
        assert!(action(&Value::test_string("true"), &args, span).is_true());
        assert!(action(&Value::test_string("FALSE"), &args, span).is_false());
    }

    #[test]
    fn test_relaxed_handling() {
        let span = Span::test_data();
        let args = IntoBoolCmdArgument {
            cell_paths: vec![].into(),
            relaxed: true,
        };

        assert!(action(&Value::test_nothing(), &args, span).is_false());
        assert!(action(&Value::test_string("abc"), &args, span).is_true());
        assert!(action(&Value::test_string("true"), &args, span).is_true());
        assert!(action(&Value::test_string("FALSE"), &args, span).is_false());
    }
}
