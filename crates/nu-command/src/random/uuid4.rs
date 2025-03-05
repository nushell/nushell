use nu_engine::command_prelude::*;
use uuid::Uuid;

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "random uuid4"
    }

    fn signature(&self) -> Signature {
        Signature::build("random uuid4")
            .category(Category::Random)
            .input_output_types(vec![(Type::Nothing, Type::String)])
            .allow_variants_without_examples(true)
    }

    fn description(&self) -> &str {
        "Generate a v4 (random) UUID string."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["uuidv4"]
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
            description: "Generate a uuid v4 string (random)",
            example: "random uuid4",
            result: None,
        }]
    }
}

fn uuid(call: &Call) -> Result<PipelineData, ShellError> {
    let span = call.head;

    let uuid = Uuid::new_v4();
    let uuid_str = uuid.hyphenated().to_string();

    Ok(PipelineData::Value(Value::string(uuid_str, span), None))
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
