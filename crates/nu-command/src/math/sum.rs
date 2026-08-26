use crate::math::{
    reducers::{Reduce, reducer_for},
    utils::{run_with_function_with_cell_paths, run_with_function_with_cell_paths_const},
};
use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct MathSum;

impl Command for MathSum {
    fn name(&self) -> &str {
        "math sum"
    }

    fn signature(&self) -> Signature {
        Signature::build("math sum")
            .input_output_types(vec![
                (Type::List(Box::new(Type::Number)), Type::Number),
                (Type::List(Box::new(Type::Duration)), Type::Duration),
                (Type::List(Box::new(Type::Filesize)), Type::Filesize),
                (Type::Range, Type::Number),
                (Type::table(), Type::record()),
                (Type::record(), Type::record()),
            ])
            .allow_variants_without_examples(true)
            .rest(
                "columns",
                SyntaxShape::CellPath,
                "The cell-paths/columns to operate on.",
            )
            .category(Category::Math)
    }

    fn description(&self) -> &str {
        "Returns the sum of a list of numbers or of each column in a table."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["plus", "add", "total", "+"]
    }

    fn is_const(&self) -> bool {
        true
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        run_with_function_with_cell_paths(engine_state, stack, call, input, summation)
    }

    fn run_const(
        &self,
        working_set: &StateWorkingSet,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        run_with_function_with_cell_paths_const(working_set, call, input, summation)
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Sum a list of numbers.",
                example: "[1 2 3] | math sum",
                result: Some(Value::test_int(6)),
            },
            Example {
                description: "Get the disk usage for the current directory.",
                example: "ls | get size | math sum",
                result: None,
            },
            Example {
                description: "Compute the sum of each column in a table.",
                example: "[[a b]; [1 2] [3 4]] | math sum",
                result: Some(Value::test_record(record! {
                    "a" => Value::test_int(4),
                    "b" => Value::test_int(6),
                })),
            },
            Example {
                description: "Sum the values of list-valued columns in a record.",
                example: "{alice: [1 2 3], bob: [4 5 6]} | math sum",
                result: Some(Value::test_record(record! {
                    "alice" => Value::test_int(6),
                    "bob" => Value::test_int(15),
                })),
            },
            Example {
                description: "Sum a single column using a cell path.",
                example: "{alice: [1 2 3], bob: [4 5 6]} | math sum alice",
                result: Some(Value::test_record(record! {
                    "alice" => Value::test_int(6),
                    "bob" => Value::list(
                        vec![Value::test_int(4), Value::test_int(5), Value::test_int(6)],
                        Span::test_data(),
                    ),
                })),
            },
        ]
    }
}

pub fn summation(values: &[Value], span: Span, head: Span) -> Result<Value, ShellError> {
    let sum_func = reducer_for(Reduce::Summation);
    sum_func(Value::nothing(head), values.to_vec(), span, head)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() -> nu_test_support::Result {
        nu_test_support::test().examples(MathSum)
    }
}
