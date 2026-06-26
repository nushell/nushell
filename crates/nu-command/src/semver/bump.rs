use super::value::SemverValue;
use nu_engine::command_prelude::*;
use nu_protocol::{Parameter, shell_error::generic::GenericError};

#[derive(Clone)]
pub struct SemverBump;

impl Command for SemverBump {
    fn name(&self) -> &str {
        "semver bump"
    }

    fn signature(&self) -> Signature {
        Signature::build("semver bump")
            .input_output_types(vec![(
                Type::Custom("semver".into()),
                Type::Custom("semver".into()),
            )])
            .param(Parameter::Required(
                PositionalArg::new("level", SyntaxShape::String)
                    .desc("The level to bump: major, minor, patch, alpha, beta, rc, release.")
                    .completion(Completion::new_list(&[
                        "major", "minor", "patch", "alpha", "beta", "rc", "release",
                    ])),
            ))
            .category(Category::Filters)
    }

    fn description(&self) -> &str {
        "Bump a semantic version to the next level."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["version", "increment", "major", "minor", "patch"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let level: String = call.req(engine_state, stack, 0)?;
        let head = call.head;

        input.map(
            move |value| bump_value(&value, &level, head),
            engine_state.signals(),
        )
    }

    fn examples(&self) -> Vec<Example<'static>> {
        vec![
            Example {
                description: "Bump major version",
                example: "'1.2.3' | into semver | semver bump major",
                result: None,
            },
            Example {
                description: "Bump minor version",
                example: "'1.2.3' | into semver | semver bump minor",
                result: None,
            },
            Example {
                description: "Bump patch version",
                example: "'1.2.3' | into semver | semver bump patch",
                result: None,
            },
            Example {
                description: "Add alpha prerelease",
                example: "'1.2.3' | into semver | semver bump alpha",
                result: None,
            },
            Example {
                description: "Remove prerelease",
                example: "'1.2.3-alpha' | into semver | semver bump release",
                result: None,
            },
        ]
    }
}

fn bump_value(input: &Value, level: &str, head: Span) -> Value {
    let semver_val = match input {
        Value::Custom { val, .. } => {
            if let Some(semver) = val.as_any().downcast_ref::<SemverValue>() {
                semver
            } else {
                return Value::error(
                    ShellError::Generic(GenericError::new(
                        "Value is not a semver",
                        "expected a semver value",
                        head,
                    )),
                    head,
                );
            }
        }
        _ => {
            return Value::error(
                ShellError::Generic(
                    GenericError::new(
                        "Value is not a semver",
                        format!("expected a semver value, got {}", input.get_type()),
                        head,
                    )
                    .with_help("Use `into semver` to convert a string to a semver value first"),
                ),
                head,
            );
        }
    };

    let result = match level {
        "major" => semver_val.bump_major(),
        "minor" => semver_val.bump_minor(),
        "patch" => semver_val.bump_patch(),
        "alpha" | "beta" | "rc" => match semver_val.bump_prerelease(level) {
            Ok(v) => v,
            Err(e) => return Value::error(e, head),
        },
        "release" => semver_val.bump_release(),
        _ => {
            return Value::error(
                ShellError::Generic(
                    GenericError::new(
                        "Invalid bump level",
                        format!("'{}' is not a valid bump level", level),
                        head,
                    )
                    .with_help("valid levels: major, minor, patch, alpha, beta, rc, release"),
                ),
                head,
            );
        }
    };

    Value::custom(Box::new(result), head)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_semver_value(version: &str) -> Value {
        let semver = SemverValue::new(semver::Version::parse(version).unwrap());
        Value::custom(Box::new(semver), Span::test_data())
    }

    fn get_semver_from_value(value: &Value) -> String {
        match value {
            Value::Custom { val, .. } => {
                let semver = val.as_any().downcast_ref::<SemverValue>().unwrap();
                semver.version.to_string()
            }
            _ => panic!("Expected Custom value"),
        }
    }

    #[test]
    fn test_bump_major() {
        let input = create_semver_value("1.2.3");
        let result = bump_value(&input, "major", Span::test_data());
        assert_eq!(get_semver_from_value(&result), "2.0.0");
    }

    #[test]
    fn test_bump_minor() {
        let input = create_semver_value("1.2.3");
        let result = bump_value(&input, "minor", Span::test_data());
        assert_eq!(get_semver_from_value(&result), "1.3.0");
    }

    #[test]
    fn test_bump_patch() {
        let input = create_semver_value("1.2.3");
        let result = bump_value(&input, "patch", Span::test_data());
        assert_eq!(get_semver_from_value(&result), "1.2.4");
    }

    #[test]
    fn test_bump_alpha() {
        let input = create_semver_value("1.2.3");
        let result = bump_value(&input, "alpha", Span::test_data());
        assert_eq!(get_semver_from_value(&result), "1.2.3-alpha.0");
    }

    #[test]
    fn test_bump_beta() {
        let input = create_semver_value("1.2.3");
        let result = bump_value(&input, "beta", Span::test_data());
        assert_eq!(get_semver_from_value(&result), "1.2.3-beta.0");
    }

    #[test]
    fn test_bump_rc() {
        let input = create_semver_value("1.2.3");
        let result = bump_value(&input, "rc", Span::test_data());
        assert_eq!(get_semver_from_value(&result), "1.2.3-rc.0");
    }

    #[test]
    fn test_bump_release() {
        let input = create_semver_value("1.2.3-alpha.1");
        let result = bump_value(&input, "release", Span::test_data());
        assert_eq!(get_semver_from_value(&result), "1.2.3");
    }

    #[test]
    fn test_bump_invalid_level() {
        let input = create_semver_value("1.2.3");
        let result = bump_value(&input, "invalid", Span::test_data());
        assert!(matches!(result, Value::Error { .. }));
    }

    #[test]
    fn test_bump_non_semver_value() {
        let input = Value::string("1.2.3", Span::test_data());
        let result = bump_value(&input, "major", Span::test_data());
        assert!(matches!(result, Value::Error { .. }));
    }

    #[test]
    fn test_bump_wrong_custom_value() {
        // Create a different custom value (not semver)
        let input = Value::int(42, Span::test_data());
        let result = bump_value(&input, "major", Span::test_data());
        assert!(matches!(result, Value::Error { .. }));
    }

    #[test]
    fn test_bump_with_prerelease() {
        let input = create_semver_value("1.2.3-alpha.1");
        let result = bump_value(&input, "major", Span::test_data());
        assert_eq!(get_semver_from_value(&result), "2.0.0");
    }

    #[test]
    fn test_bump_with_build_metadata() {
        let input = create_semver_value("1.2.3+build.1");
        let result = bump_value(&input, "minor", Span::test_data());
        assert_eq!(get_semver_from_value(&result), "1.3.0");
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() -> nu_test_support::Result {
        nu_test_support::test().examples(SemverBump)
    }
}
