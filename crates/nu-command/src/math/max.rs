use crate::math::reducers::{reducer_for, Reduce};
use crate::math::utils::run_with_function;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, Example, PipelineData, ShellError, Signature, Span, Type, Value};

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "math max"
    }

    fn signature(&self) -> Signature {
        Signature::build("math max")
            .input_output_types(vec![
                (Type::List(Box::new(Type::Number)), Type::Number),
                (Type::Table(vec![]), Type::Record(vec![])),
            ])
            .category(Category::Math)
    }

    fn usage(&self) -> &str {
        "Returns the maximum of a list of numbers, or of columns in a table"
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["maximum", "largest"]
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        run_with_function(call, input, maximum)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Find the maximum of list of numbers",
                example: "[-50 100 25] | math max",
                result: Some(Value::test_int(100)),
            },
            Example {
                description: "Find the maxima of the columns of a table",
                example: "[{a: 1 b: 3} {a: 2 b: -1}] | math max",
                result: Some(Value::Record {
                    cols: vec!["a".to_string(), "b".to_string()],
                    vals: vec![Value::test_int(2), Value::test_int(3)],
                    span: Span::test_data(),
                }),
            },
        ]
    }
}

pub fn maximum(values: &[Value], head: &Span) -> Result<Value, ShellError> {
    let max_func = reducer_for(Reduce::Maximum);
    max_func(Value::nothing(*head), values.to_vec(), *head)
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
