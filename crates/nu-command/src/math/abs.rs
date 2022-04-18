use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, Example, PipelineData, ShellError, Signature, Span, Value};

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "math abs"
    }

    fn signature(&self) -> Signature {
        Signature::build("math abs").category(Category::Math)
    }

    fn usage(&self) -> &str {
        "Returns absolute values of a list of numbers"
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["absolute", "modulus", "positive", "distance"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        let head = call.head;
        input.map(
            move |value| abs_helper(value, head),
            engine_state.ctrlc.clone(),
        )
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Get absolute of each value in a list of numbers",
            example: "[-50 -100.0 25] | math abs",
            result: Some(Value::List {
                vals: vec![
                    Value::test_int(50),
                    Value::Float {
                        val: 100.0,
                        span: Span::test_data(),
                    },
                    Value::test_int(25),
                ],
                span: Span::test_data(),
            }),
        }]
    }
}

fn abs_helper(val: Value, head: Span) -> Value {
    match val {
        Value::Int { val, span } => Value::int(val.abs(), span),
        Value::Float { val, span } => Value::Float {
            val: val.abs(),
            span,
        },
        Value::Duration { val, span } => Value::Duration {
            val: val.abs(),
            span,
        },
        other => Value::Error {
            error: ShellError::UnsupportedInput(
                format!(
                    "Only numerical values are supported, input type: {:?}",
                    other.get_type()
                ),
                head,
            ),
        },
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
