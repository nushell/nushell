use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, Span, Type, Value,
};

use crate::math::{
    reducers::{reducer_for, Reduce},
    utils::run_with_function,
};

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "math product"
    }

    fn signature(&self) -> Signature {
        Signature::build("math product")
            .input_output_types(vec![(Type::List(Box::new(Type::Number)), Type::Number)])
            .category(Category::Math)
    }

    fn usage(&self) -> &str {
        "Returns the product of a list of numbers or the products of each column of a table."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["times", "multiply", "x", "*"]
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        run_with_function(call, input, product)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Compute the product of a list of numbers",
            example: "[2 3 3 4] | math product",
            result: Some(Value::test_int(72)),
        }]
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
    fn test_examples() {
        use crate::test_examples;

        test_examples(SubCommand {})
    }
}
