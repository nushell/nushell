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
        "math max"
    }

    fn signature(&self) -> Signature {
        Signature::build("math max")
            .input_output_types(vec![
                (Type::List(Box::new(Type::Any)), Type::Any),
                (Type::Table(vec![]), Type::Record(vec![])),
            ])
            .allow_variants_without_examples(true)
            .category(Category::Math)
    }

    fn usage(&self) -> &str {
        "Returns the maximum of a list of values, or of columns in a table."
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
    ) -> Result<PipelineData, ShellError> {
        run_with_function(call, input, maximum)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Find the maximum of list of numbers",
                example: "[-50 100 25] | math max",
                result: Some(SpannedValue::test_int(100)),
            },
            Example {
                description: "Find the maxima of the columns of a table",
                example: "[{a: 1 b: 3} {a: 2 b: -1}] | math max",
                result: Some(SpannedValue::Record {
                    cols: vec!["a".to_string(), "b".to_string()],
                    vals: vec![SpannedValue::test_int(2), SpannedValue::test_int(3)],
                    span: Span::test_data(),
                }),
            },
        ]
    }
}

pub fn maximum(
    values: &[SpannedValue],
    span: Span,
    head: Span,
) -> Result<SpannedValue, ShellError> {
    let max_func = reducer_for(Reduce::Maximum);
    max_func(SpannedValue::nothing(head), values.to_vec(), span, head)
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
