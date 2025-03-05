use nu_engine::command_prelude::*;
use nu_protocol::{report_parse_warning, ParseWarning};
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

    fn description(&self) -> &str {
        "Generate a random uuid4 string."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["generate", "uuid4"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;
        report_parse_warning(
            &StateWorkingSet::new(engine_state),
            &ParseWarning::DeprecatedWarning {
                old_command: "random uuid".into(),
                new_suggestion: "use `random uuid[version]`".into(),
                span: head,
                url: "`help random uuid[version]`".into(),
            },
        );
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

// NOTE: Do not remove this function. It is used in the `uuid3` and `uuid5` modules.
pub fn get_namespace_and_name(
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
