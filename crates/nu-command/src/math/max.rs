use crate::math::{
    reducers::{Reduce, reducer_for},
    utils::run_with_function,
};
use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct MathMax;

impl Command for MathMax {
    fn name(&self) -> &str {
        "math max"
    }

    fn signature(&self) -> Signature {
        Signature::build("math max")
            .input_output_types(vec![
                (Type::List(Box::new(Type::Number)), Type::Number),
                (Type::List(Box::new(Type::Duration)), Type::Duration),
                (Type::List(Box::new(Type::Filesize)), Type::Filesize),
                (Type::List(Box::new(Type::Any)), Type::Any),
                (Type::Range, Type::Number),
                (Type::table(), Type::record()),
                (Type::record(), Type::record()),
            ])
            .allow_variants_without_examples(true)
            .category(Category::Math)
    }

    fn description(&self) -> &str {
        "Returns the maximum of a list of values, or of columns in a table."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["maximum", "largest"]
    }

    fn is_const(&self) -> bool {
        true
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

    fn run_const(
        &self,
        _working_set: &StateWorkingSet,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        run_with_function(call, input, maximum)
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Find the maximum of a list of numbers",
                example: "[-50 100 25] | math max",
                result: Some(Value::test_int(100)),
            },
            Example {
                description: "Find the maxima of the columns of a table",
                example: "[{a: 1 b: 3} {a: 2 b: -1}] | math max",
                result: Some(Value::test_record(record! {
                    "a" => Value::test_int(2),
                    "b" => Value::test_int(3),
                })),
            },
            Example {
                description: "Find the maximum of a list of dates",
                example: "[2022-02-02 2022-12-30 2012-12-12] | math max",
                result: Some(Value::test_date(
                    "2022-12-30 00:00:00Z".parse().unwrap_or_default(),
                )),
            },
        ]
    }
}

pub fn maximum(values: &[Value], span: Span, head: Span) -> Result<Value, ShellError> {
    let max_func = reducer_for(Reduce::Maximum);
    max_func(Value::nothing(head), values.to_vec(), span, head)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(MathMax {})
    }
}
