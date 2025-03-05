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

fn get_namespace_and_name(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    span: Span,
) -> Result<(Uuid, String), ShellError> {
    let namespace_str: Option<String> = call.get_flag(engine_state, stack, "namespace")?;
    let name: Option<String> = call.get_flag(engine_state, stack, "name")?;

    let namespace_str = match namespace_str {
        Some(ns) => ns,
        None => {
            return Err(ShellError::MissingParameter {
                param_name: "namespace".to_string(),
                span,
            });
        }
    };

    let name = match name {
        Some(n) => n,
        None => {
            return Err(ShellError::MissingParameter {
                param_name: "name".to_string(),
                span,
            });
        }
    };

    let namespace = match namespace_str.to_lowercase().as_str() {
        "dns" => Uuid::NAMESPACE_DNS,
        "url" => Uuid::NAMESPACE_URL,
        "oid" => Uuid::NAMESPACE_OID,
        "x500" => Uuid::NAMESPACE_X500,
        _ => match Uuid::parse_str(&namespace_str) {
            Ok(uuid) => uuid,
            Err(_) => {
                return Err(ShellError::IncorrectValue {
                    msg: "Namespace must be one of: dns, url, oid, x500, or a valid UUID string"
                        .to_string(),
                    val_span: span,
                    call_span: span,
                });
            }
        },
    };

    Ok((namespace, name))
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
