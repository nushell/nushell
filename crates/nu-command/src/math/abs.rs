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

    fn run(&self, context: &EvaluationContext, call: &Call, input: Value) -> Result<Value, ShellError> {
        let mapped = input.map(move |val| match val.value {
            Value::Int { val, span } => Value::int(val.abs(), span),
            Value::Float { val, span } => Value::Float{val: val.abs(), span: span},
            Value::Duration { val, span } => Value::Duration{val: val.abs(), span: span},
            other => abs_default(other)?,
        });
        Ok(mapped)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Get absolute of each value in a list of numbers",
            example: "echo [-50 25] | math abs",
            result: Some(vec![
                Value::test_int(50),
                Value::test_int(25),
            ]),
        }]
    }
}

fn abs_default(_: Value) -> Result<Value, ShellError> {
    Value::Error(ShellError::unexpected(
        "Only numerical values are supported",
    ))
    .into()
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
