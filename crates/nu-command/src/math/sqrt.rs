use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, Example, PipelineData, ShellError, Signature, Span, Value};

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "math sqrt"
    }

    fn signature(&self) -> Signature {
        Signature::build("math sqrt").category(Category::Math)
    }

    fn usage(&self) -> &str {
        "Applies the square root function to a list of numbers"
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["square", "root"]
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
            description: "Apply the square root function to a list of numbers",
            example: "[9 16] | math sqrt",
            result: Some(Value::List {
                vals: vec![Value::test_int(3), Value::test_int(4)],
                span: Span::test_data(),
            }),
        }]
    }
}

fn operate(value: Value, head: Span) -> Value {
    match value {
        Value::Int { val, span } => {
            let squared = (val as f64).sqrt();
            if squared.is_nan() {
                return error_negative_sqrt(span);
            }
            Value::Float { val: squared, span }
        }
        Value::Float { val, span } => {
            let squared = val.sqrt();
            if squared.is_nan() {
                return error_negative_sqrt(span);
            }
            Value::Float { val: squared, span }
        }
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

fn error_negative_sqrt(span: Span) -> Value {
    Value::Error {
        error: ShellError::UnsupportedInput(
            String::from("Can't square root a negative number"),
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
