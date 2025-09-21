use super::utils;
use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct All;

impl Command for All {
    fn name(&self) -> &str {
        "all"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .input_output_types(vec![(Type::List(Box::new(Type::Any)), Type::Bool)])
            .required(
                "predicate",
                SyntaxShape::Closure(Some(vec![SyntaxShape::Any])),
                "A closure that must evaluate to a boolean.",
            )
            .category(Category::Filters)
    }

    fn description(&self) -> &str {
        "Test if every element of the input fulfills a predicate expression."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["every", "and"]
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Check if a list contains only true values",
                example: "[false true true false] | all {}",
                result: Some(Value::test_bool(false)),
            },
            Example {
                description: "Check if each row's status is the string 'UP'",
                example: "[[status]; [UP] [UP]] | all {|el| $el.status == UP }",
                result: Some(Value::test_bool(true)),
            },
            Example {
                description: "Check that each item is a string",
                example: "[foo bar 2 baz] | all {|| ($in | describe) == 'string' }",
                result: Some(Value::test_bool(false)),
            },
            Example {
                description: "Check that all values are equal to twice their index",
                example: "[0 2 4 6] | enumerate | all {|i| $i.item == $i.index * 2 }",
                result: Some(Value::test_bool(true)),
            },
            Example {
                description: "Check that all of the values are even, using a stored closure",
                example: "let cond = {|el| ($el mod 2) == 0 }; [2 4 6 8] | all $cond",
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
        utils::boolean_fold(engine_state, stack, call, input, false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(All)
    }
}
