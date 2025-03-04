use nu_engine::command_prelude::*;
use uuid::{Timestamp, Uuid};

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
                "version",
                SyntaxShape::Int,
                "The UUID version to generate (1, 3, 4, 5, 7). Defaults to 4 if not specified.",
                Some('v'),
            )
            .named(
                "namespace",
                SyntaxShape::String,
                "The namespace for v3 and v5 UUIDs (dns, url, oid, x500). Required for v3 and v5.",
                Some('n'),
            )
            .named(
                "name",
                SyntaxShape::String,
                "The name string for v3 and v5 UUIDs. Required for v3 and v5.",
                Some('s'),
            )
            .allow_variants_without_examples(true)
    }

    fn description(&self) -> &str {
        "Generate a UUID string with specified version. Defaults to v4 (random) if no version is specified."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec![
            "generate", "uuid", "uuid4", "uuid1", "uuid3", "uuid5", "uuid7",
        ]
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
        vec![
            Example {
                description: "Generate a random uuid v4 string (default)",
                example: "random uuid",
                result: None,
            },
            Example {
                description: "Generate a uuid v1 string (timestamp-based)",
                example: "random uuid -v 1",
                result: None,
            },
            Example {
                description: "Generate a uuid v3 string (namespace with MD5)",
                example: "random uuid -v 3 -n dns -s example.com",
                result: None,
            },
            Example {
                description: "Generate a uuid v5 string (namespace with SHA1)",
                example: "random uuid -v 5 -n dns -s example.com",
                result: None,
            },
            Example {
                description: "Generate a uuid v7 string (timestamp + random)",
                example: "random uuid -v 7",
                result: None,
            },
        ]
    }
}

fn uuid(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
) -> Result<PipelineData, ShellError> {
    let span = call.head;

    let version: Option<i64> = call.get_flag(engine_state, stack, "version")?;
    let version = version.unwrap_or(4);

    let uuid_str = match version {
        1 => {
            let ts = Timestamp::now(uuid::timestamp::context::NoContext);
            let node_id = random_mac_address();
            let uuid = Uuid::new_v1(ts, &node_id);
            uuid.hyphenated().to_string()
        }
        3 => {
            let (namespace, name) = get_namespace_and_name(engine_state, stack, call, span)?;
            let uuid = Uuid::new_v3(&namespace, name.as_bytes());
            uuid.hyphenated().to_string()
        }
        4 => {
            let uuid = Uuid::new_v4();
            uuid.hyphenated().to_string()
        }
        5 => {
            let (namespace, name) = get_namespace_and_name(engine_state, stack, call, span)?;
            let uuid = Uuid::new_v5(&namespace, name.as_bytes());
            uuid.hyphenated().to_string()
        }
        7 => {
            let ts = Timestamp::now(uuid::timestamp::context::NoContext);
            let uuid = Uuid::new_v7(ts);
            uuid.hyphenated().to_string()
        }
        _ => {
            return Err(ShellError::GenericError {
                error: format!(
                    "Unsupported UUID version: {}. Supported versions are 1, 3, 4, 5, and 7.",
                    version
                ),
                msg: "Invalid UUID version".to_string(),
                span: Some(span),
                help: Some("Specify version with -v: 1, 3, 4, 5, or 7".to_string()),
                inner: Vec::new(),
            });
        }
    };

    Ok(PipelineData::Value(Value::string(uuid_str, span), None))
}

fn random_mac_address() -> [u8; 6] {
    let mut mac = [0u8; 6];
    for i in 0..6 {
        mac[i] = rand::random::<u8>();
    }
    mac[0] |= 0x01;
    mac
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
                return Err(ShellError::GenericError {
                        error: format!("Invalid namespace: {}", namespace_str),
                        msg: "Namespace must be one of: dns, url, oid, x500, or a valid UUID string".to_string(),
                        span: Some(span),
                        help: Some("Use one of the predefined namespaces (dns, url, oid, x500) or a valid UUID string".to_string()),
                        inner: Vec::new(),
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
