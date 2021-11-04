use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Example, PipelineData, ShellError, Signature, Span, Value};

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "math floor"
    }

    fn signature(&self) -> Signature {
        Signature::build("math floor")
    }

    fn usage(&self) -> &str {
        "Applies the floor function to a list of numbers"
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
            move |value| operate(value, head),
            engine_state.ctrlc.clone(),
        )
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Apply the floor function to a list of numbers",
            example: "[1.5 2.3 -3.1] | math floor",
            result: Some(Value::List {
                vals: vec![Value::test_int(1), Value::test_int(2), Value::test_int(-4)],
                span: Span::unknown(),
            }),
        }]
    }
}

fn operate(value: Value, head: Span) -> Value {
    match value {
        Value::Int { .. } => value,
        Value::Float { val, span } => Value::Float {
            val: val.floor(),
            span,
        },
        other => Value::Error {
            error: ShellError::UnsupportedInput(
                String::from("Only numerical values are supported"),
                other.span().unwrap_or(head),
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
