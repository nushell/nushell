use nu_engine::command_prelude::*;
use uuid::Uuid;

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
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        uuid(call)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Generate a random uuid4 string",
            example: "random uuid",
            result: None,
        }]
    }
}

fn uuid(call: &Call) -> Result<PipelineData, ShellError> {
    let span = call.head;
    let uuid_4 = Uuid::new_v4().hyphenated().to_string();

    Ok(PipelineData::Value(Value::string(uuid_4, span), None))
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
