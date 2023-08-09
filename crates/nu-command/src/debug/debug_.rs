use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, Example, PipelineData, ShellError, Signature, Type, Value};

use crate::formats::convert_string_to_value;

#[derive(Clone)]
pub struct Debug;

impl Command for Debug {
    fn name(&self) -> &str {
        "debug"
    }

    fn usage(&self) -> &str {
        "Debug print the value(s) piped in."
    }

    fn signature(&self) -> Signature {
        Signature::build("debug")
            .input_output_type(Type::Any, Type::Record(vec![]))
            .category(Category::Debug)
    }

    fn run(
        &self,
        engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;

        // Should PipelineData::Empty result in an error here?
        input.map(
            move |x| match convert_string_to_value(x.debug_value(), head) {
                Ok(value) => value,
                Err(err) => Value::error(err),
            },
            engine_state.ctrlc.clone(),
        )
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Debug print a string",
                example: r#""hello" | debug | reject String.span"#,
                result: Some(Value::test_record(
                    vec!["String".to_string()],
                    vec![Value::test_record(
                        vec!["val".to_string()],
                        vec![Value::test_string("hello")],
                    )],
                )),
            },
            Example {
                description: "Debug print a list",
                example: "[1 2 3] | debug | reject Int.span",
                result: Some(Value::test_list(vec![
                    Value::test_record(
                        vec!["Int".to_string()],
                        vec![Value::test_record(
                            vec!["val".to_string()],
                            vec![Value::test_int(1)],
                        )],
                    ),
                    Value::test_record(
                        vec!["Int".to_string()],
                        vec![Value::test_record(
                            vec!["val".to_string()],
                            vec![Value::test_int(2)],
                        )],
                    ),
                    Value::test_record(
                        vec!["Int".to_string()],
                        vec![Value::test_record(
                            vec!["val".to_string()],
                            vec![Value::test_int(3)],
                        )],
                    ),
                ])),
            },
            Example {
                description: "Debug print a table",
                example: "{foo: 1.23, bar: true} | debug | reject Record.span Record.vals.0.Float.span Record.vals.1.Bool.span",
                result: Some(Value::test_record(
                    vec!["Record".to_string()],
                    vec![Value::test_record(
                        vec!["cols".to_string(), "vals".to_string()],
                        vec![
                            Value::test_list(vec![Value::test_string("foo"), Value::test_string("bar")]),
                            Value::test_list(vec![
                                Value::test_record(vec!["Float".to_string()], vec![Value::test_record(vec!["val".to_string()], vec![Value::test_float(1.23)])]),
                                Value::test_record(vec!["Bool".to_string()], vec![Value::test_record(vec!["val".to_string()], vec![Value::test_bool(true)])]),
                            ]),
                        ]
                    )]
                )),
            },
        ]
    }
}

#[cfg(test)]
mod test {
    #[test]
    fn test_examples() {
        use super::Debug;
        use crate::test_examples;
        test_examples(Debug {})
    }
}
