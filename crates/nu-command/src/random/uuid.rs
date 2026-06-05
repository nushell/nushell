use nu_engine::command_prelude::*;
use nu_protocol::FromValue;
use uuid::{Timestamp, Uuid};

#[derive(Clone)]
pub struct RandomUuid;

impl Command for RandomUuid {
    fn name(&self) -> &str {
        "random uuid"
    }

    fn signature(&self) -> Signature {
        Signature::build("random uuid")
            .category(Category::Random)
            .input_output_types(vec![(Type::Nothing, Type::String)])
            .param(
                Flag::new("version")
                    .short('v')
                    .arg(SyntaxShape::Int)
                    .desc(
                        "The UUID version to generate (1, 3, 4, 5, 7). \
                            Defaults to 4 if not specified.",
                    )
                    .completion(Completion::new_list(&["1", "3", "4", "5", "7"])),
            )
            .param(
                Flag::new("namespace")
                    .short('n')
                    .arg(SyntaxShape::String)
                    .desc(
                        "The namespace for v3 and v5 UUIDs (dns, url, oid, x500). \
                            Required for v3 and v5.",
                    )
                    .completion(Completion::new_list(&["dns", "url", "oid", "x500"])),
            )
            .named(
                "name",
                SyntaxShape::String,
                "The name string for v3 and v5 UUIDs. Required for v3 and v5.",
                Some('s'),
            )
            .named(
                "mac",
                SyntaxShape::String,
                "The MAC address (node ID) used to generate v1 UUIDs. Required for v1.",
                Some('m'),
            )
            .allow_variants_without_examples(true)
    }

    fn description(&self) -> &str {
        "Generate a random uuid string of the specified version."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["generate", "uuid4", "uuid1", "uuid3", "uuid5", "uuid7"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let fix_call_span = |err: ShellError| match err {
            ShellError::IncorrectValue { msg, val_span, .. } => ShellError::IncorrectValue {
                msg,
                val_span,
                call_span: call.head,
            },
            _ => err,
        };
        uuid(engine_state, stack, call).map_err(fix_call_span)
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Generate a random uuid v4 string (default).",
                example: "random uuid",
                result: None,
            },
            Example {
                description: "Generate a uuid v1 string (timestamp-based).",
                example: "random uuid -v 1 -m 00:11:22:33:44:55",
                result: None,
            },
            Example {
                description: "Generate a uuid v3 string (namespace with MD5).",
                example: "random uuid -v 3 -n dns -s example.com",
                result: None,
            },
            Example {
                description: "Generate a uuid v4 string (random).",
                example: "random uuid -v 4",
                result: None,
            },
            Example {
                description: "Generate a uuid v5 string (namespace with SHA1).",
                example: "random uuid -v 5 -n dns -s example.com",
                result: None,
            },
            Example {
                description: "Generate a uuid v7 string (timestamp + random).",
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

    validate_flags(engine_state, stack, call, span, version)?;

    let uuid_str = match version {
        1 => {
            let ts = Timestamp::now(uuid::timestamp::context::NoContext);
            let MacAddr(node_id) = call
                .get_flag::<MacAddr>(engine_state, stack, "mac")?
                .ok_or_else(|| ShellError::MissingParameter {
                    param_name: "mac".to_string(),
                    span,
                })?;
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
            return Err(ShellError::IncorrectValue {
                msg: format!(
                    "Unsupported UUID version: {version}. Supported versions are 1, 3, 4, 5, and 7."
                ),
                val_span: span,
                call_span: span,
            });
        }
    };

    Ok(PipelineData::value(Value::string(uuid_str, span), None))
}

fn validate_flags(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    span: Span,
    version: i64,
) -> Result<(), ShellError> {
    match version {
        1 => {
            if call
                .get_flag::<Option<String>>(engine_state, stack, "namespace")?
                .is_some()
            {
                return Err(ShellError::IncompatibleParametersSingle {
                    msg: "version 1 uuid does not take namespace as a parameter".to_string(),
                    span,
                });
            }
            if call
                .get_flag::<Option<String>>(engine_state, stack, "name")?
                .is_some()
            {
                return Err(ShellError::IncompatibleParametersSingle {
                    msg: "version 1 uuid does not take name as a parameter".to_string(),
                    span,
                });
            }
        }
        3 | 5 => {
            if call
                .get_flag::<Option<String>>(engine_state, stack, "mac")?
                .is_some()
            {
                return Err(ShellError::IncompatibleParametersSingle {
                    msg: "version 3 and 5 uuids do not take mac as a parameter".to_string(),
                    span,
                });
            }
        }
        v => {
            if v != 4 && v != 7 {
                return Err(ShellError::IncorrectValue {
                    msg: format!(
                        "Unsupported UUID version: {v}. Supported versions are 1, 3, 4, 5, and 7."
                    ),
                    val_span: span,
                    call_span: span,
                });
            }
            if call
                .get_flag::<Option<String>>(engine_state, stack, "mac")?
                .is_some()
            {
                return Err(ShellError::IncompatibleParametersSingle {
                    msg: format!("version {v} uuid does not take mac as a parameter"),
                    span,
                });
            }
            if call
                .get_flag::<Option<String>>(engine_state, stack, "namespace")?
                .is_some()
            {
                return Err(ShellError::IncompatibleParametersSingle {
                    msg: format!("version {v} uuid does not take namespace as a parameter"),
                    span,
                });
            }
            if call
                .get_flag::<Option<String>>(engine_state, stack, "name")?
                .is_some()
            {
                return Err(ShellError::IncompatibleParametersSingle {
                    msg: format!("version {v} uuid does not take name as a parameter"),
                    span,
                });
            }
        }
    }
    Ok(())
}

struct MacAddr([u8; 6]);

impl FromValue for MacAddr {
    fn from_value(v: Value) -> Result<Self, ShellError> {
        let span = v.span();
        let s = v.into_string()?;

        let mut buf = [0_u8; 6];
        let mut mac_parts = s.split(':').map(|x| u8::from_str_radix(x, 0x10));

        let has_6_hexadecimal_parts = buf.iter_mut().all(|ele| match mac_parts.next() {
            Some(Ok(n)) => {
                *ele = n;
                true
            }
            _ => false,
        });

        if has_6_hexadecimal_parts && mac_parts.next().is_none() {
            Ok(Self(buf))
        } else {
            Err(ShellError::IncorrectValue {
                msg: "MAC address must be in the format XX:XX:XX:XX:XX:XX".to_string(),
                val_span: span,
                call_span: span,
            })
        }
    }
}

struct NameSpace(Uuid);

impl FromValue for NameSpace {
    fn from_value(v: Value) -> Result<Self, ShellError> {
        let span = v.span();
        let s = v.into_string()?;

        let namespace = match s.to_lowercase().as_str() {
            "dns" => Uuid::NAMESPACE_DNS,
            "url" => Uuid::NAMESPACE_URL,
            "oid" => Uuid::NAMESPACE_OID,
            "x500" => Uuid::NAMESPACE_X500,
            _ => match Uuid::parse_str(&s) {
                Ok(uuid) => uuid,
                Err(_) => {
                    return Err(ShellError::IncorrectValue {
                        msg:
                            "Namespace must be one of: dns, url, oid, x500, or a valid UUID string"
                                .to_string(),
                        val_span: span,
                        call_span: span,
                    });
                }
            },
        };

        Ok(Self(namespace))
    }
}

fn get_namespace_and_name(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    span: Span,
) -> Result<(Uuid, String), ShellError> {
    let NameSpace(namespace) = call
        .get_flag::<NameSpace>(engine_state, stack, "namespace")?
        .ok_or_else(|| ShellError::MissingParameter {
            param_name: "namespace".to_string(),
            span,
        })?;

    let name = call
        .get_flag::<String>(engine_state, stack, "name")?
        .ok_or_else(|| ShellError::MissingParameter {
            param_name: "name".to_string(),
            span,
        })?;

    Ok((namespace, name))
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() -> nu_test_support::Result {
        nu_test_support::test().examples(RandomUuid)
    }
}
