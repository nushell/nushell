use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, Example, PipelineData, ShellError, Signature, Span, Value};

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "math bit-not"
    }

    fn signature(&self) -> Signature {
        Signature::build("math bit-not").category(Category::Math)
    }

    fn usage(&self) -> &str {
        "performs logical negation on each bit"
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["negation"]
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
            example: "[4 3 2] | math bit-not",
            result: Some(Value::List {
                vals: vec![
                    Value::test_int(-5),
                    Value::test_int(-4),
                    Value::test_int(-3),
                ],
                span: Span::test_data(),
            }),
        }]
    }
}

fn operate(value: Value, head: Span) -> Value {
    match value {
        Value::Int { val, span } => Value::Int { val: !val, span },
        other => Value::Error {
            error: ShellError::UnsupportedInput(
                format!(
                    "Only numerical values are supported, input type: {:?}",
                    other.get_type()
                ),
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
