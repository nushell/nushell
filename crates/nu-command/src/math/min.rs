use crate::math::{
    reducers::{Reduce, reducer_for},
    utils::run_with_function,
};
use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct MathMin;

impl Command for MathMin {
    fn name(&self) -> &str {
        "math min"
    }

    fn signature(&self) -> Signature {
        Signature::build("math min")
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
        "Finds the minimum within a list of values or tables."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["minimum", "smallest"]
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
        run_with_function(call, input, minimum)
    }

    fn run_const(
        &self,
        _working_set: &StateWorkingSet,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        run_with_function(call, input, minimum)
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Compute the minimum of a list of numbers",
                example: "[-50 100 25] | math min",
                result: Some(Value::test_int(-50)),
            },
            Example {
                description: "Compute the minima of the columns of a table",
                example: "[{a: 1 b: 3} {a: 2 b: -1}] | math min",
                result: Some(Value::test_record(record! {
                    "a" => Value::test_int(1),
                    "b" => Value::test_int(-1),
                })),
            },
            Example {
                description: "Find the minimum of a list of arbitrary values (Warning: Weird)",
                example: "[-50 'hello' true] | math min",
                result: Some(Value::test_bool(true)),
            },
        ]
    }
}

pub fn minimum(values: &[Value], span: Span, head: Span) -> Result<Value, ShellError> {
    let min_func = reducer_for(Reduce::Minimum);
    min_func(Value::nothing(head), values.to_vec(), span, head)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(MathMin {})
    }
}
