use super::utils;
use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct Any;

impl Command for Any {
    fn name(&self) -> &str {
        "any"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .input_output_types(vec![(Type::List(Box::new(Type::Any)), Type::Bool)])
            .required(
                "predicate",
                SyntaxShape::RowCondition,
                "Row condition or closure that evaluates to a boolean.",
            )
            .category(Category::Filters)
    }

    fn description(&self) -> &str {
        "Tests if any element of the input fulfills a predicate expression."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["some", "or"]
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Check if a list contains any true values.",
                example: "[false true true false] | any {}",
                result: Some(Value::test_bool(true)),
            },
            Example {
                description: "Check if any row's status is the string 'DOWN'.",
                example: "[[status]; [UP] [DOWN] [UP]] | any status == DOWN",
                result: Some(Value::test_bool(true)),
            },
            Example {
                description: "Check that any item is a string.",
                example: "[1 2 3 4] | any {|| ($in | describe) == 'string' }",
                result: Some(Value::test_bool(false)),
            },
            Example {
                description: "Check if any value is equal to twice its own index.",
                example: "[9 8 7 6] | enumerate | any item == index * 2",
                result: Some(Value::test_bool(true)),
            },
            Example {
                description: "Check if any of the values are odd, using a stored closure.",
                example: "let cond = {|e| $e mod 2 == 1 }; [2 4 1 6 8] | any $cond",
                result: Some(Value::test_bool(true)),
            },
        ]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        utils::boolean_fold(engine_state, stack, call, input, true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_examples() -> nu_test_support::Result {
        nu_test_support::test().examples(Any)
    }
}
