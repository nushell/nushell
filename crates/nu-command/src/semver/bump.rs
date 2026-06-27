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
            .input_output_types(vec![
                (Type::Custom("semver".into()), Type::Custom("semver".into())),
                (Type::String, Type::Custom("semver".into())),
            ])
            .switch(
                "ignore-errors",
                "If the input is not a valid semver version, return the original input unchanged",
                Some('i'),
            )
            .named(
                "build-metadata",
                SyntaxShape::String,
                "Additionally set the build metadata",
                Some('b'),
            )
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

    fn examples(&self) -> Vec<Example<'static>> {
        vec![
            Example {
                description: "Bump major version",
                example: "'1.2.3' | into semver | semver bump major",
                result: Some(SemverValue::test_value("2.0.0")),
            },
            Example {
                description: "Bump minor version",
                example: "'1.2.3' | into semver | semver bump minor",
                result: Some(SemverValue::test_value("1.3.0")),
            },
            Example {
                description: "Bump patch version",
                example: "'1.2.3' | into semver | semver bump patch",
                result: Some(SemverValue::test_value("1.2.4")),
            },
            Example {
                description: "Bump patch version with string input",
                example: "'1.2.3' | semver bump patch",
                result: Some(SemverValue::test_value("1.2.4")),
            },
            Example {
                description: "Add alpha prerelease",
                example: "'1.2.3' | into semver | semver bump alpha",
                result: Some(SemverValue::test_value("1.2.3-alpha.1")),
            },
            Example {
                description: "Remove prerelease",
                example: "'1.2.3-alpha' | into semver | semver bump release",
                result: Some(SemverValue::test_value("1.2.3")),
            },
        ]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let level: String = call.req(engine_state, stack, 0)?;
        let ignore_errors = call.has_flag(engine_state, stack, "ignore-errors")?;
        let build_metadata: Option<String> =
            call.get_flag(engine_state, stack, "build-metadata")?;
        let head = call.head;

        input.map(
            move |value| {
                bump_value_with_options(
                    &value,
                    &level,
                    head,
                    ignore_errors,
                    build_metadata.as_deref(),
                )
                .unwrap_or_else(|err| Value::error(err, head))
            },
            engine_state.signals(),
        )
    }
}

fn bump_value_with_options(
    input: &Value,
    level: &str,
    head: Span,
    ignore_errors: bool,
    build_metadata: Option<&str>,
) -> Result<Value, ShellError> {
    let semver_val = match SemverValue::try_from(input) {
        Ok(semver) => semver,
        Err(err) => {
            if ignore_errors {
                return Ok(input.clone());
            }
            return Err(err);
        }
    };

    let result = match level {
        "major" => semver_val.bump_major(),
        "minor" => semver_val.bump_minor(),
        "patch" => semver_val.bump_patch(),
        "alpha" | "beta" | "rc" => semver_val.bump_prerelease(level)?,
        "release" => semver_val.bump_release(),
        _ => {
            return Err(ShellError::Generic(
                GenericError::new(
                    "Invalid bump level",
                    format!("'{}' is not a valid bump level", level),
                    head,
                )
                .with_help("valid levels: major, minor, patch, alpha, beta, rc, release"),
            ));
        }
    };

    let result = if let Some(metadata) = build_metadata {
        result.set_build_metadata(metadata)?
    } else {
        result
    };

    Ok(Value::custom(Box::new(result), head))
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
        let result =
            bump_value_with_options(&input, "major", Span::test_data(), false, None).unwrap();
        assert_eq!(get_semver_from_value(&result), "2.0.0");
    }

    #[test]
    fn test_bump_minor() {
        let input = create_semver_value("1.2.3");
        let result =
            bump_value_with_options(&input, "minor", Span::test_data(), false, None).unwrap();
        assert_eq!(get_semver_from_value(&result), "1.3.0");
    }

    #[test]
    fn test_bump_patch() {
        let input = create_semver_value("1.2.3");
        let result =
            bump_value_with_options(&input, "patch", Span::test_data(), false, None).unwrap();
        assert_eq!(get_semver_from_value(&result), "1.2.4");
    }

    #[test]
    fn test_bump_alpha() {
        let input = create_semver_value("1.2.3");
        let result =
            bump_value_with_options(&input, "alpha", Span::test_data(), false, None).unwrap();
        assert_eq!(get_semver_from_value(&result), "1.2.3-alpha.0");
    }

    #[test]
    fn test_bump_beta() {
        let input = create_semver_value("1.2.3");
        let result =
            bump_value_with_options(&input, "beta", Span::test_data(), false, None).unwrap();
        assert_eq!(get_semver_from_value(&result), "1.2.3-beta.0");
    }

    #[test]
    fn test_bump_rc() {
        let input = create_semver_value("1.2.3");
        let result = bump_value_with_options(&input, "rc", Span::test_data(), false, None).unwrap();
        assert_eq!(get_semver_from_value(&result), "1.2.3-rc.0");
    }

    #[test]
    fn test_bump_release() {
        let input = create_semver_value("1.2.3-alpha.1");
        let result =
            bump_value_with_options(&input, "release", Span::test_data(), false, None).unwrap();
        assert_eq!(get_semver_from_value(&result), "1.2.3");
    }

    #[test]
    fn test_bump_invalid_level() {
        let input = create_semver_value("1.2.3");
        let result = bump_value_with_options(&input, "invalid", Span::test_data(), false, None);
        assert!(result.is_err());
    }

    #[test]
    fn test_bump_string_input_is_supported() {
        let input = Value::string("1.2.3", Span::test_data());
        let result =
            bump_value_with_options(&input, "major", Span::test_data(), false, None).unwrap();
        assert_eq!(get_semver_from_value(&result), "2.0.0");
    }

    #[test]
    fn test_bump_string_input_with_build_metadata() {
        let input = Value::string("1.2.3", Span::test_data());
        let result =
            bump_value_with_options(&input, "minor", Span::test_data(), false, Some("build"))
                .unwrap();
        assert_eq!(get_semver_from_value(&result), "1.3.0+build");
    }

    #[test]
    fn test_bump_ignore_errors_for_invalid_input() {
        let input = Value::string("not-a-version", Span::test_data());
        let result =
            bump_value_with_options(&input, "major", Span::test_data(), true, None).unwrap();
        assert!(matches!(result, Value::String { .. }));
    }

    #[test]
    fn test_bump_wrong_custom_value() {
        // Create a different custom value (not semver)
        let input = Value::int(42, Span::test_data());
        let result = bump_value_with_options(&input, "major", Span::test_data(), false, None);
        assert!(result.is_err());
    }

    #[test]
    fn test_bump_with_prerelease() {
        let input = create_semver_value("1.2.3-alpha.1");
        let result =
            bump_value_with_options(&input, "major", Span::test_data(), false, None).unwrap();
        assert_eq!(get_semver_from_value(&result), "2.0.0");
    }

    #[test]
    fn test_bump_with_build_metadata() {
        let input = create_semver_value("1.2.3+build.1");
        let result =
            bump_value_with_options(&input, "minor", Span::test_data(), false, None).unwrap();
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
