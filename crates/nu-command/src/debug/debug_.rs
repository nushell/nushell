use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct Debug;

impl Command for Debug {
    fn name(&self) -> &str {
        "debug"
    }

    fn description(&self) -> &str {
        "Debug print the value(s) piped in."
    }

    fn signature(&self) -> Signature {
        Signature::build("debug")
            .input_output_types(vec![
                (
                    Type::List(Box::new(Type::Any)),
                    Type::List(Box::new(Type::String)),
                ),
                (Type::Any, Type::String),
            ])
            .category(Category::Debug)
            .switch("raw", "Prints the raw value representation", Some('r'))
            .switch(
                "raw-value",
                "Prints the raw value representation but not the nushell value part",
                Some('v'),
            )
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;
        let config = stack.get_config(engine_state);
        let raw = call.has_flag(engine_state, stack, "raw")?;
        let raw_value = call.has_flag(engine_state, stack, "raw-value")?;

        // Should PipelineData::Empty result in an error here?

        input.map(
            move |x| {
                if raw {
                    Value::string(x.to_debug_string(), head)
                } else if raw_value {
                    Value::string(format!("{:#?}", x.to_expanded_string(", ", &config)), head)
                } else {
                    Value::string(x.to_expanded_string(", ", &config), head)
                }
            },
            engine_state.signals(),
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
                result: Some(Value::list(
                    vec![Value::test_string("hello")],
                    Span::test_data(),
                )),
            },
            Example {
                description: "Debug print a table",
                example:
                    "[[version patch]; ['0.1.0' false] ['0.1.1' true] ['0.2.0' false]] | debug",
                result: Some(Value::list(
                    vec![
                        Value::test_string("{version: 0.1.0, patch: false}"),
                        Value::test_string("{version: 0.1.1, patch: true}"),
                        Value::test_string("{version: 0.2.0, patch: false}"),
                    ],
                    Span::test_data(),
                )),
            },
            Example {
                description: "Debug print an ansi escape encoded string and get the raw value",
                example: "$'(ansi red)nushell(ansi reset)' | debug -v",
                result: Some(Value::test_string("\\u{1b}[31mnushell\\u{1b}[0m")),
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
