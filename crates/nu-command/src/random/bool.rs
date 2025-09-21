use nu_engine::command_prelude::*;
use rand::random_bool;

#[derive(Clone)]
pub struct RandomBool;

impl Command for RandomBool {
    fn name(&self) -> &str {
        "random bool"
    }

    fn signature(&self) -> Signature {
        Signature::build("random bool")
            .input_output_types(vec![(Type::Nothing, Type::Bool)])
            .allow_variants_without_examples(true)
            .named(
                "bias",
                SyntaxShape::Number,
                "Adjusts the probability of a \"true\" outcome",
                Some('b'),
            )
            .category(Category::Random)
    }

    fn description(&self) -> &str {
        "Generate a random boolean value."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["generate", "boolean", "true", "false", "1", "0"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        bool(engine_state, stack, call)
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Generate a random boolean value",
                example: "random bool",
                result: None,
            },
            Example {
                description: "Generate a random boolean value with a 75% chance of \"true\"",
                example: "random bool --bias 0.75",
                result: None,
            },
        ]
    }
}

fn bool(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
) -> Result<PipelineData, ShellError> {
    let span = call.head;
    let bias: Option<Spanned<f64>> = call.get_flag(engine_state, stack, "bias")?;

    let mut probability = 0.5;

    if let Some(prob) = bias {
        probability = prob.item;

        let probability_is_valid = (0.0..=1.0).contains(&probability);

        if !probability_is_valid {
            return Err(ShellError::InvalidProbability { span: prob.span });
        }
    }

    let bool_result: bool = random_bool(probability);

    Ok(PipelineData::value(Value::bool(bool_result, span), None))
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(RandomBool {})
    }
}
