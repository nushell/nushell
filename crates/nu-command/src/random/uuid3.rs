use crate::get_namespace_and_name;
use nu_engine::command_prelude::*;
use uuid::Uuid;

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "random uuid3"
    }

    fn signature(&self) -> Signature {
        Signature::build("random uuid3")
            .category(Category::Random)
            .input_output_types(vec![(Type::Nothing, Type::String)])
            .named(
                "namespace",
                SyntaxShape::String,
                "The namespace for generating UUID (dns, url, oid, x500).",
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
        "Generate a v3 (namespace with MD5) UUID string."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["uuidv3"]
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
            description: "Generate a uuid v3 string (namespace with MD5)",
            example: "random uuid3 -n dns -s example.com",
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
    let uuid = Uuid::new_v3(&namespace, name.as_bytes());
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
