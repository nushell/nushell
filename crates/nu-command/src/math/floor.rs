use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, Example, PipelineData, ShellError, Signature, Span, Type, Value};

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "math floor"
    }

    fn signature(&self) -> Signature {
        Signature::build("math floor")
            .input_output_types(vec![(Type::Number, Type::Int)])
            .vectorizes_over_list(true)
            .category(Category::Math)
    }

    fn usage(&self) -> &str {
        "Returns the floor of a number (largest integer less than or equal to that number)"
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["round down", "rounding", "integer"]
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
                span: Span::test_data(),
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
