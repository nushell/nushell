use nu_engine::command_prelude::*;
use nu_protocol::shell_error::generic::GenericError;

#[derive(Clone)]
pub struct IntoSemver;

impl Command for IntoSemver {
    fn name(&self) -> &str {
        "into semver"
    }

    fn signature(&self) -> Signature {
        Signature::build("into semver")
            .input_output_types(vec![
                (Type::String, Type::SemVer),
                (Type::SemVer, Type::SemVer),
            ])
            .category(Category::Conversions)
    }

    fn description(&self) -> &str {
        "Convert a value to a semantic version."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["version", "convert", "semantic"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;

        input.map(
            move |value| into_semver(&value, head),
            engine_state.signals(),
        )
    }

    fn examples(&self) -> Vec<Example<'static>> {
        vec![
            Example {
                description: "Convert a string to a semver value",
                example: "'1.2.3' | into semver",
                result: Some(Value::test_semver(semver::Version::new(1, 2, 3))),
            },
            Example {
                description: "Convert a string with prerelease",
                example: "'1.2.3-alpha.1+build.2' | into semver",
                result: Some(Value::test_semver(
                    semver::Version::parse("1.2.3-alpha.1+build.2")
                        .expect("hardcoded example should be valid"),
                )),
            },
        ]
    }
}

fn into_semver(input: &Value, head: Span) -> Value {
    match input {
        Value::SemVer { .. } => input.clone(),
        Value::String { val, .. } => match semver::Version::parse(val) {
            Ok(version) => Value::semver(version, head),
            Err(_) => Value::error(
                ShellError::Generic(
                    GenericError::new(
                        format!("Cannot convert \"{val}\" to a semver"),
                        "the given string is not a valid semver version",
                        head,
                    )
                    .with_help("expected format: major.minor.patch (e.g. 1.2.3)"),
                ),
                head,
            ),
        },
        _ => Value::error(
            ShellError::Generic(GenericError::new(
                format!("Cannot convert {} to semver", input.get_type()),
                "expected a string or semver value",
                head,
            )),
            head,
        ),
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() -> nu_test_support::Result {
        nu_test_support::test().examples(IntoSemver)
    }
}
