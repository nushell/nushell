use nu_engine::command_prelude::*;
use uuid::Builder;

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "random uuid"
    }

    fn signature(&self) -> Signature {
        Signature::build("random uuid")
            .category(Category::Random)
            .input_output_types(vec![(Type::Nothing, Type::String)])
            .named(
                "seed",
                SyntaxShape::Int,
                "Seeds the RNG to get reproducible results.",
                None,
            )
            .allow_variants_without_examples(true)
    }

    fn usage(&self) -> &str {
        "Generate a random uuid4 string."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["generate", "uuid4"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let span = call.head;

        let mut rng = super::rng(engine_state, stack, call)?;
        let mut random_bytes = [0u8; 16];
        rng.fill_bytes(&mut random_bytes);
        let uuid_4 = Builder::from_random_bytes(random_bytes)
            .into_uuid()
            .hyphenated()
            .to_string();

        Ok(PipelineData::Value(Value::string(uuid_4, span), None))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Generate a random uuid4 string",
            example: "random uuid",
            result: None,
        }]
    }
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
