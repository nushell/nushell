use crate::math::reducers::{reducer_for, Reduce};
use crate::math::utils::run_with_function;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, PipelineData, ShellError, Signature, Span, SpannedValue, Type,
};

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "math min"
    }

    fn signature(&self) -> Signature {
        Signature::build("math min")
            .input_output_types(vec![
                (Type::List(Box::new(Type::Any)), Type::Any),
                (Type::Table(vec![]), Type::Record(vec![])),
            ])
            .allow_variants_without_examples(true)
            .category(Category::Math)
    }

    fn usage(&self) -> &str {
        "Finds the minimum within a list of values or tables."
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
    ) -> Result<PipelineData, ShellError> {
        run_with_function(call, input, minimum)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Compute the minimum of a list of numbers",
                example: "[-50 100 25] | math min",
                result: Some(SpannedValue::test_int(-50)),
            },
            Example {
                description: "Compute the minima of the columns of a table",
                example: "[{a: 1 b: 3} {a: 2 b: -1}] | math min",
                result: Some(SpannedValue::Record {
                    cols: vec!["a".to_string(), "b".to_string()],
                    vals: vec![SpannedValue::test_int(1), SpannedValue::test_int(-1)],
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Find the minimum of a list of arbitrary values (Warning: Weird)",
                example: "[-50 'hello' true] | math min",
                result: Some(SpannedValue::test_bool(true)),
            },
        ]
    }
}

pub fn minimum(
    values: &[SpannedValue],
    span: Span,
    head: Span,
) -> Result<SpannedValue, ShellError> {
    let min_func = reducer_for(Reduce::Minimum);
    min_func(SpannedValue::nothing(head), values.to_vec(), span, head)
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
