use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, Example, PipelineData, ShellError, Signature, Span, Value};

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
        Signature::build("debug").category(Category::Core).switch(
            "raw",
            "Prints the raw value representation",
            Some('r'),
        )
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
                    Value::String(x.debug_value())
                } else {
                    Value::String(x.debug_string(", ", &config))
                }
            },
            engine_state.ctrlc.clone(),
        )
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Print the value of a string",
                example: "'hello' | debug",
                result: Some(Value::String("hello".into())),
            },
            Example {
                description: "Print the value of a table",
                example: "echo [[version patch]; [0.1.0 false] [0.1.1 true] [0.2.0 false]] | debug",
                result: Some(Value::List(vec![
                    Value::String("{version: 0.1.0, patch: false}".into()),
                    Value::String("{version: 0.1.1, patch: true}".into()),
                    Value::String("{version: 0.2.0, patch: false}".into()),
                ])),
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
