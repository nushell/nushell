use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EvaluationContext};
use nu_protocol::{Example, ShellError, Signature, Span, Type, Value};

pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "math abs"
    }

    fn signature(&self) -> Signature {
        Signature::build("math abs")
    }

    fn usage(&self) -> &str {
        "Returns absolute values of a list of numbers"
    }

    fn run(&self, _context: &EvaluationContext, call: &Call, input: Value) -> Result<Value, ShellError> {
        let head = call.head;
        input.map(head, move |val| match val {
            Value::Int { val, span } => Value::int(val.abs(), span),
            Value::Float { val, span } => Value::Float{val: val.abs(), span: span},
            Value::Duration { val, span } => Value::Duration{val: val.abs(), span: span},
            other => abs_default(other, head),
        })
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Get absolute of each value in a list of numbers",
            example: "echo [-50 25] | math abs",
            result: Some(Value:: List {
                vals: vec![
                    Value::test_int(50),
                    Value::test_int(25),
                ],
                span: Span::unknown(),
            }),
        }]
    }
}

fn abs_default(_: Value, head: Span) -> Value {
    Value::Error {error: ShellError::UnsupportedInput(
        String::from("Only numerical values are supported"),
        head
    )}
}

#[cfg(test)]
mod tests {
    use super::ShellError;
    use super::SubCommand;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(SubCommand {})
    }
}
