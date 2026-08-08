use crate::math::{
    reducers::{Reduce, reducer_for},
    utils::{run_with_function_with_cell_paths, run_with_function_with_cell_paths_const},
};
use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct MathProduct;

impl Command for MathProduct {
    fn name(&self) -> &str {
        "math product"
    }

    fn signature(&self) -> Signature {
        Signature::build("math product")
            .input_output_types(vec![
                (Type::List(Box::new(Type::Number)), Type::Number),
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
        "Returns the product of a list of numbers or the products of each column of a table."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["times", "multiply", "x", "*"]
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
        run_with_function_with_cell_paths(engine_state, stack, call, input, product)
    }

    fn run_const(
        &self,
        working_set: &StateWorkingSet,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        run_with_function_with_cell_paths_const(working_set, call, input, product)
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Compute the product of a list of numbers.",
                example: "[2 3 3 4] | math product",
                result: Some(Value::test_int(72)),
            },
            Example {
                description: "Compute the product of each column in a table.",
                example: "[[a b]; [1 2] [3 4]] | math product",
                result: Some(Value::test_record(record! {
                    "a" => Value::test_int(3),
                    "b" => Value::test_int(8),
                })),
            },
            Example {
                description: "Compute the product of list-valued columns in a record.",
                example: "{alice: [2 3 4], bob: [4 5 6]} | math product",
                result: Some(Value::test_record(record! {
                    "alice" => Value::test_int(24),
                    "bob" => Value::test_int(120),
                })),
            },
            Example {
                description: "Compute the product of a single column using a cell path.",
                example: "{alice: [2 3 4], bob: [4 5 6]} | math product alice",
                result: Some(Value::test_record(record! {
                    "alice" => Value::test_int(24),
                    "bob" => Value::list(
                        vec![Value::test_int(4), Value::test_int(5), Value::test_int(6)],
                        Span::test_data(),
                    ),
                })),
            },
        ]
    }
}

/// Calculate product of given values
pub fn product(values: &[Value], span: Span, head: Span) -> Result<Value, ShellError> {
    let product_func = reducer_for(Reduce::Product);
    product_func(Value::nothing(head), values.to_vec(), span, head)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() -> nu_test_support::Result {
        nu_test_support::test().examples(MathProduct)
    }
}
