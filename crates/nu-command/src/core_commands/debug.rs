use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, Example, PipelineData, ShellError, Signature, Span, Type, Value};

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
            .input_output_types(vec![
                (
                    Type::List(Box::new(Type::Any)),
                    Type::List(Box::new(Type::String)),
                ),
                (Type::Table(vec![]), Type::List(Box::new(Type::String))),
                (Type::Any, Type::String),
            ])
            .category(Category::Core)
            .switch("raw", "Prints the raw value representation", Some('r'))
    }

    fn run(
        &self,
        engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;
        let config = engine_state.get_config().clone();
        let raw = call.has_flag("raw");

        input.map(
            move |x| {
                if raw {
                    Value::String {
                        val: x.debug_value(),
                        span: head,
                    }
                } else {
                    Value::String {
                        val: x.debug_string(", ", &config),
                        span: head,
                    }
                }
            },
            engine_state.ctrlc.clone(),
        )
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Debug print a string",
                example: "'hello' | debug",
                result: Some(Value::test_string("hello")),
            },
            Example {
                description: "Debug print a list",
                example: "['hello'] | debug",
                result: Some(Value::List {
                    vals: vec![Value::test_string("hello")],
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Debug print a table",
                example: "echo [[version patch]; [0.1.0 false] [0.1.1 true] [0.2.0 false]] | debug",
                result: Some(Value::List {
                    vals: vec![
                        Value::test_string("{version: 0.1.0, patch: false}"),
                        Value::test_string("{version: 0.1.1, patch: true}"),
                        Value::test_string("{version: 0.2.0, patch: false}"),
                    ],
                    span: Span::test_data(),
                }),
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
