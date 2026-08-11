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
            .switch(
                "preserve-build-metadata",
                "Preserve the existing build metadata from the input version",
                Some('p'),
            )
            .switch(
                "loose",
                "Allow common non-strict prefixes such as v1.2.3, v.1.2.3, v:1.2.3, v-1.2.3, or v_1.2.3 when parsing string input; the prefix is preserved on the result",
                Some('l'),
            )
            .named(
                "build-metadata",
                SyntaxShape::String,
                "Additionally set the build metadata. Takes precedence over --preserve-build-metadata",
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
            Example {
                description: "Bump with preserved build metadata",
                example: "'1.2.3+build.5' | into semver | semver bump patch --preserve-build-metadata",
                result: Some(SemverValue::test_value("1.2.4+build.5")),
            },
            Example {
                description: "Bump a loosely-prefixed version string",
                example: "'v1.2.3' | semver bump patch --loose",
                result: Some(SemverValue::test_value("v1.2.4")),
            },
            Example {
                description: "Bump after converting with --loose (prefix is preserved)",
                example: "'v1.2.3' | into semver --loose | semver bump major",
                result: Some(SemverValue::test_value("v2.0.0")),
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
        let preserve_build_metadata =
            call.has_flag(engine_state, stack, "preserve-build-metadata")?;
        let loose = call.has_flag(engine_state, stack, "loose")?;
        let head = call.head;

        input.map(
            move |value| {
                bump_value_with_options(
                    &value,
                    &level,
                    head,
                    ignore_errors,
                    build_metadata.as_deref(),
                    preserve_build_metadata,
                    loose,
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
    preserve_build_metadata: bool,
    loose: bool,
) -> Result<Value, ShellError> {
    let semver_val = match SemverValue::try_from_value(input, loose) {
        Ok(semver) => semver,
        Err(err) => {
            if ignore_errors {
                return Ok(input.clone());
            }
            return Err(err);
        }
    };

    let original_build = semver_val.version.build.clone();

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

    let result = match (build_metadata, preserve_build_metadata) {
        (Some(metadata), _) => result.set_build_metadata(metadata)?,
        (None, true) => SemverValue {
            version: semver::Version {
                build: original_build,
                ..result.version
            },
            prefix: result.prefix,
        },
        (None, false) => result,
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
                semver.display()
            }
            _ => panic!("Expected Custom value"),
        }
    }

    fn bump(
        input: &Value,
        level: &str,
        ignore_errors: bool,
        build_metadata: Option<&str>,
        preserve_build_metadata: bool,
        loose: bool,
    ) -> Result<Value, ShellError> {
        bump_value_with_options(
            input,
            level,
            Span::test_data(),
            ignore_errors,
            build_metadata,
            preserve_build_metadata,
            loose,
        )
    }

    #[test]
    fn test_bump_major() {
        let input = create_semver_value("1.2.3");
        let result = bump(&input, "major", false, None, false, false).unwrap();
        assert_eq!(get_semver_from_value(&result), "2.0.0");
    }

    #[test]
    fn test_bump_minor() {
        let input = create_semver_value("1.2.3");
        let result = bump(&input, "minor", false, None, false, false).unwrap();
        assert_eq!(get_semver_from_value(&result), "1.3.0");
    }

    #[test]
    fn test_bump_patch() {
        let input = create_semver_value("1.2.3");
        let result = bump(&input, "patch", false, None, false, false).unwrap();
        assert_eq!(get_semver_from_value(&result), "1.2.4");
    }

    #[test]
    fn test_bump_alpha() {
        let input = create_semver_value("1.2.3");
        let result = bump(&input, "alpha", false, None, false, false).unwrap();
        assert_eq!(get_semver_from_value(&result), "1.2.3-alpha.1");
    }

    #[test]
    fn test_bump_beta() {
        let input = create_semver_value("1.2.3");
        let result = bump(&input, "beta", false, None, false, false).unwrap();
        assert_eq!(get_semver_from_value(&result), "1.2.3-beta.1");
    }

    #[test]
    fn test_bump_rc() {
        let input = create_semver_value("1.2.3");
        let result = bump(&input, "rc", false, None, false, false).unwrap();
        assert_eq!(get_semver_from_value(&result), "1.2.3-rc.1");
    }

    #[test]
    fn test_bump_release() {
        let input = create_semver_value("1.2.3-alpha.1");
        let result = bump(&input, "release", false, None, false, false).unwrap();
        assert_eq!(get_semver_from_value(&result), "1.2.3");
    }

    #[test]
    fn test_bump_invalid_level() {
        let input = create_semver_value("1.2.3");
        let result = bump(&input, "invalid", false, None, false, false);
        assert!(result.is_err());
    }

    #[test]
    fn test_bump_string_input_is_supported() {
        let input = Value::string("1.2.3", Span::test_data());
        let result = bump(&input, "major", false, None, false, false).unwrap();
        assert_eq!(get_semver_from_value(&result), "2.0.0");
    }

    #[test]
    fn test_bump_string_input_with_build_metadata() {
        let input = Value::string("1.2.3", Span::test_data());
        let result = bump(&input, "minor", false, Some("build"), false, false).unwrap();
        assert_eq!(get_semver_from_value(&result), "1.3.0+build");
    }

    #[test]
    fn test_bump_ignore_errors_for_invalid_input() {
        let input = Value::string("not-a-version", Span::test_data());
        let result = bump(&input, "major", true, None, false, false).unwrap();
        assert!(matches!(result, Value::String { .. }));
    }

    #[test]
    fn test_bump_wrong_custom_value() {
        let input = Value::int(42, Span::test_data());
        let result = bump(&input, "major", false, None, false, false);
        assert!(result.is_err());
    }

    #[test]
    fn test_bump_with_prerelease() {
        let input = create_semver_value("1.2.3-alpha.1");
        let result = bump(&input, "major", false, None, false, false).unwrap();
        assert_eq!(get_semver_from_value(&result), "2.0.0");
    }

    #[test]
    fn test_bump_with_build_metadata() {
        let input = create_semver_value("1.2.3+build.1");
        let result = bump(&input, "minor", false, None, false, false).unwrap();
        assert_eq!(get_semver_from_value(&result), "1.3.0");
    }

    #[test]
    fn test_bump_preserve_build_metadata() {
        let input = create_semver_value("1.2.3+build.5");
        let result = bump(&input, "patch", false, None, true, false).unwrap();
        assert_eq!(get_semver_from_value(&result), "1.2.4+build.5");
    }

    #[test]
    fn test_bump_preserve_build_metadata_major() {
        let input = create_semver_value("1.2.3+build.1");
        let result = bump(&input, "major", false, None, true, false).unwrap();
        assert_eq!(get_semver_from_value(&result), "2.0.0+build.1");
    }

    #[test]
    fn test_bump_build_metadata_takes_precedence() {
        let input = create_semver_value("1.2.3+build.1");
        let result = bump(&input, "patch", false, Some("override"), true, false).unwrap();
        assert_eq!(get_semver_from_value(&result), "1.2.4+override");
    }

    #[test]
    fn test_bump_loose_string_preserves_prefix() {
        let input = Value::string("v1.2.3", Span::test_data());
        let result = bump(&input, "major", false, None, false, true).unwrap();
        assert_eq!(get_semver_from_value(&result), "v2.0.0");

        let input = Value::string("v.1.2.3", Span::test_data());
        let result = bump(&input, "patch", false, None, false, true).unwrap();
        assert_eq!(get_semver_from_value(&result), "v.1.2.4");

        let input = Value::string("v:1.2.3", Span::test_data());
        let result = bump(&input, "minor", false, None, false, true).unwrap();
        assert_eq!(get_semver_from_value(&result), "v:1.3.0");

        let input = Value::string("v-1.2.3", Span::test_data());
        let result = bump(&input, "major", false, None, false, true).unwrap();
        assert_eq!(get_semver_from_value(&result), "v-2.0.0");

        let input = Value::string("v_1.2.3", Span::test_data());
        let result = bump(&input, "patch", false, None, false, true).unwrap();
        assert_eq!(get_semver_from_value(&result), "v_1.2.4");
    }

    #[test]
    fn test_bump_loose_required_for_prefixed_string() {
        let input = Value::string("v1.2.3", Span::test_data());
        let result = bump(&input, "major", false, None, false, false);
        assert!(result.is_err());
    }

    #[test]
    fn test_bump_preserves_prefix_from_semver_value() {
        let input = Value::custom(
            Box::new(SemverValue::parse("v1.2.3", true).unwrap()),
            Span::test_data(),
        );
        // Prefix is already on the value; --loose is not required to keep it.
        let result = bump(&input, "major", false, None, false, false).unwrap();
        assert_eq!(get_semver_from_value(&result), "v2.0.0");
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
