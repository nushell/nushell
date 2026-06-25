use super::semver_from_input;
use nu_engine::command_prelude::*;
use nu_protocol::shell_error::generic::GenericError;

#[derive(Clone)]
pub struct SemverMatchReq;

impl Command for SemverMatchReq {
    fn name(&self) -> &str {
        "semver match-req"
    }

    fn signature(&self) -> Signature {
        Signature::build("semver match-req")
            .input_output_types(vec![(Type::SemVer, Type::Bool)])
            .required(
                "requirement",
                SyntaxShape::String,
                "the semver requirement to match against (e.g. '>=1.0.0')",
            )
            .category(Category::Filters)
    }

    fn description(&self) -> &str {
        "Check if a semver value matches a semver requirement."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["version", "check", "match", "satisfy"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let requirement: String = call.req(engine_state, stack, 0)?;
        let head = call.head;

        let version_req = match semver::VersionReq::parse(&requirement) {
            Ok(r) => r,
            Err(_) => {
                return Err(ShellError::Generic(GenericError::new(
                    format!("Invalid semver requirement: {requirement}"),
                    "expected a valid semver requirement like '>=1.0.0'",
                    head,
                )));
            }
        };

        input.map(
            move |value| match_value(&value, &version_req, head),
            engine_state.signals(),
        )
    }

    fn examples(&self) -> Vec<Example<'static>> {
        vec![
            Example {
                description: "Check if a version matches a requirement",
                example: "1.2.3 | semver match-req '>=1.0.0'",
                result: Some(Value::test_bool(true)),
            },
            Example {
                description: "Check if a version does not match",
                example: "1.0.0 | semver match-req '>=2.0.0'",
                result: Some(Value::test_bool(false)),
            },
            Example {
                description: "Match with prerelease versions",
                example: "1.3.0-alpha | semver match-req '>=1.2.3'",
                result: Some(Value::test_bool(false)),
            },
        ]
    }
}

fn match_value(input: &Value, version_req: &semver::VersionReq, head: Span) -> Value {
    let version = match semver_from_input(input, head) {
        Ok(v) => v,
        Err(e) => return Value::error(e, head),
    };

    Value::bool(version_req.matches(&version), head)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() -> nu_test_support::Result {
        nu_test_support::test().examples(SemverMatchReq)
    }
}
