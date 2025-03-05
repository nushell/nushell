use crate::get_namespace_and_name;
use nu_engine::command_prelude::*;
use uuid::Uuid;

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "random uuid5"
    }

    fn signature(&self) -> Signature {
        Signature::build("random uuid5")
            .category(Category::Random)
            .input_output_types(vec![(Type::Nothing, Type::String)])
            .named(
                "namespace",
                SyntaxShape::String,
                "The namespace for generating the UUID (dns, url, oid, x500).",
                Some('n'),
            )
            .named(
                "name",
                SyntaxShape::String,
                "The name string for generating the UUID.",
                Some('s'),
            )
            .allow_variants_without_examples(true)
    }

    fn description(&self) -> &str {
        "Generate a UUID string with specified version. Defaults to v4 (random) if no version is specified."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["uuidv5"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        uuid(engine_state, stack, call)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Generate a uuid v5 string (namespace with SHA1)",
            example: "random uuid5 -n dns -s example.com",
            result: None,
        }]
    }
}

fn uuid(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
) -> Result<PipelineData, ShellError> {
    let span = call.head;

    let (namespace, name) = get_namespace_and_name(engine_state, stack, call, span)?;
    let uuid = Uuid::new_v5(&namespace, name.as_bytes());
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
