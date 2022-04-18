use crate::math::reducers::{reducer_for, Reduce};
use crate::math::utils::run_with_function;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, Example, PipelineData, ShellError, Signature, Span, Value};

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "math min"
    }

    fn signature(&self) -> Signature {
        Signature::build("math min").category(Category::Math)
    }

    fn usage(&self) -> &str {
        "Finds the minimum within a list of numbers or tables"
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["minimum", "smallest"]
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        run_with_function(call, input, minimum)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Get the minimum of a list of numbers",
            example: "[-50 100 25] | math min",
            result: Some(Value::test_int(-50)),
        }]
    }
}

pub fn minimum(values: &[Value], head: &Span) -> Result<Value, ShellError> {
    let min_func = reducer_for(Reduce::Minimum);
    min_func(Value::nothing(*head), values.to_vec(), *head)
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
