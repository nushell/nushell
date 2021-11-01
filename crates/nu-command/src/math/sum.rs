use crate::math::reducers::{reducer_for, Reduce};
use crate::math::utils::run_with_function;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Example, PipelineData, ShellError, Signature, Span, Value};

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "math sum"
    }

    fn signature(&self) -> Signature {
        Signature::build("math sum")
    }

    fn usage(&self) -> &str {
        "Finds the sum of a list of numbers or tables"
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        run_with_function(call, input, summation)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Sum a list of numbers",
                example: "[1 2 3] | math sum",
                result: Some(Value::test_int(6)),
            },
            Example {
                description: "Get the disk usage for the current directory",
                example: "ls | get size | math sum",
                result: None,
            },
        ]
    }
}

pub fn summation(values: &[Value], head: &Span) -> Result<Value, ShellError> {
    let sum_func = reducer_for(Reduce::Summation);
    sum_func(Value::nothing(), values.to_vec(), *head)
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
