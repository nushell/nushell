use crate::semver::value::SemverValue;
use nu_cmd_base::input_handler::{CellPathOnlyArgs, operate};
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
                (Type::String, Type::custom("semver")),
                (Type::custom("semver"), Type::custom("semver")),
                (Type::record(), Type::custom("semver")),
                (Type::record(), Type::record()),
                (Type::table(), Type::table()),
                (Type::list(Type::String), Type::list(Type::custom("semver"))),
                (
                    Type::list(Type::custom("semver")),
                    Type::list(Type::custom("semver")),
                ),
                (Type::table(), Type::list(Type::custom("semver"))),
                // Relaxed case to support heterogeneous lists
                (Type::Any, Type::custom("semver")),
            ])
            .rest(
                "rest",
                SyntaxShape::CellPath,
                "For a data structure input, convert data at the given cell paths.",
            )
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
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;
        let cell_paths = call.rest(engine_state, stack, 0)?;
        operate(
            into_semver,
            cell_paths.into(),
            input,
            head,
            engine_state.signals(),
        )
    }

    fn examples(&self) -> Vec<Example<'static>> {
        vec![
            Example {
                description: "Convert a string to a semver value",
                example: "'1.2.3' | into semver",
                result: None,
            },
            Example {
                description: "Convert a string with prerelease",
                example: "'1.2.3-alpha.1+build.2' | into semver",
                result: None,
            },
            Example {
                description: "Convert a record to a semver value",
                example: "{major: 1, minor: 2, patch: 3} | into semver",
                result: None,
            },
        ]
    }
}

fn into_semver(input: &Value, _args: &CellPathOnlyArgs, head: Span) -> Value {
    match input {
        Value::Custom { val, .. } if val.type_name() == "semver" => input.clone(),
        Value::String { val, .. } => match semver::Version::parse(val) {
            Ok(version) => Value::custom(Box::new(SemverValue::new(version)), head),
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
        Value::Record { val, .. } => parse_record_to_semver(val, head),
        _ => Value::error(
            ShellError::Generic(GenericError::new(
                format!("Cannot convert {} to semver", input.get_type()),
                "expected a string, record, or semver value",
                head,
            )),
            head,
        ),
    }
}

fn parse_record_to_semver(record: &nu_protocol::Record, head: Span) -> Value {
    let major = record.get("major").and_then(|v| v.as_int().ok());
    let minor = record.get("minor").and_then(|v| v.as_int().ok());
    let patch = record.get("patch").and_then(|v| v.as_int().ok());

    let major = match major {
        Some(v) if v >= 0 => v as u64,
        _ => {
            return Value::error(
                ShellError::Generic(
                    GenericError::new(
                        "Cannot convert record to semver",
                        "missing or invalid 'major' field",
                        head,
                    )
                    .with_help("expected a non-negative integer"),
                ),
                head,
            );
        }
    };

    let minor = match minor {
        Some(v) if v >= 0 => v as u64,
        _ => {
            return Value::error(
                ShellError::Generic(
                    GenericError::new(
                        "Cannot convert record to semver",
                        "missing or invalid 'minor' field",
                        head,
                    )
                    .with_help("expected a non-negative integer"),
                ),
                head,
            );
        }
    };

    let patch = match patch {
        Some(v) if v >= 0 => v as u64,
        _ => {
            return Value::error(
                ShellError::Generic(
                    GenericError::new(
                        "Cannot convert record to semver",
                        "missing or invalid 'patch' field",
                        head,
                    )
                    .with_help("expected a non-negative integer"),
                ),
                head,
            );
        }
    };

    let pre = record
        .get("pre")
        .and_then(|v| v.as_str().ok())
        .unwrap_or("");

    let build = record
        .get("build")
        .and_then(|v| v.as_str().ok())
        .unwrap_or("");

    let pre = match semver::Prerelease::new(pre) {
        Ok(p) => p,
        Err(e) => {
            return Value::error(
                ShellError::Generic(GenericError::new(
                    "Cannot convert record to semver",
                    format!("invalid prerelease: {e}"),
                    head,
                )),
                head,
            );
        }
    };

    let build = match semver::BuildMetadata::new(build) {
        Ok(b) => b,
        Err(e) => {
            return Value::error(
                ShellError::Generic(GenericError::new(
                    "Cannot convert record to semver",
                    format!("invalid build metadata: {e}"),
                    head,
                )),
                head,
            );
        }
    };

    let version = semver::Version {
        major,
        minor,
        patch,
        pre,
        build,
    };

    Value::custom(Box::new(SemverValue::new(version)), head)
}

#[cfg(test)]
mod tests {
    use super::*;
    use nu_protocol::record;
    use nu_test_support::Result;
    use nu_test_support::prelude::*;

    fn get_custom_value(value: &Value) -> &SemverValue {
        match value {
            Value::Custom { val, .. } => val.as_any().downcast_ref::<SemverValue>().unwrap(),
            _ => panic!("Expected Custom value"),
        }
    }

    #[test]
    fn test_into_semver_from_string() {
        let value = Value::string("1.2.3", Span::test_data());
        let result = into_semver(&value, &vec![].into(), Span::test_data());

        assert!(matches!(result, Value::Custom { .. }));
        let semver_val = get_custom_value(&result);
        assert_eq!(semver_val.version.to_string(), "1.2.3");
    }

    #[test]
    fn test_into_semver_from_string_with_prerelease() {
        let value = Value::string("1.2.3-alpha.1+build.2", Span::test_data());
        let result = into_semver(&value, &vec![].into(), Span::test_data());

        let semver_val = get_custom_value(&result);
        assert_eq!(semver_val.version.to_string(), "1.2.3-alpha.1+build.2");
    }

    #[test]
    fn test_into_semver_from_invalid_string() {
        let value = Value::string("not-a-version", Span::test_data());
        let result = into_semver(&value, &vec![].into(), Span::test_data());

        assert!(matches!(result, Value::Error { .. }));
    }

    #[test]
    fn test_into_semver_from_semver() {
        let original = SemverValue::new(semver::Version::parse("1.2.3").unwrap());
        let value = Value::custom(Box::new(original), Span::test_data());
        let result = into_semver(&value, &vec![].into(), Span::test_data());

        // Should return the same value
        let semver_val = get_custom_value(&result);
        assert_eq!(semver_val.version.to_string(), "1.2.3");
    }

    #[test]
    fn test_into_semver_from_record_basic() {
        let record = record! {
            "major" => Value::int(1, Span::test_data()),
            "minor" => Value::int(2, Span::test_data()),
            "patch" => Value::int(3, Span::test_data()),
        };
        let value = Value::record(record, Span::test_data());
        let result = into_semver(&value, &vec![].into(), Span::test_data());

        let semver_val = get_custom_value(&result);
        assert_eq!(semver_val.version.to_string(), "1.2.3");
    }

    #[test]
    fn test_into_semver_from_record_with_prerelease() {
        let record = record! {
            "major" => Value::int(1, Span::test_data()),
            "minor" => Value::int(2, Span::test_data()),
            "patch" => Value::int(3, Span::test_data()),
            "pre" => Value::string("alpha.1", Span::test_data()),
        };
        let value = Value::record(record, Span::test_data());
        let result = into_semver(&value, &vec![].into(), Span::test_data());

        let semver_val = get_custom_value(&result);
        assert_eq!(semver_val.version.to_string(), "1.2.3-alpha.1");
    }

    #[test]
    fn test_into_semver_from_record_with_build() {
        let record = record! {
            "major" => Value::int(1, Span::test_data()),
            "minor" => Value::int(2, Span::test_data()),
            "patch" => Value::int(3, Span::test_data()),
            "build" => Value::string("build.2", Span::test_data()),
        };
        let value = Value::record(record, Span::test_data());
        let result = into_semver(&value, &vec![].into(), Span::test_data());

        let semver_val = get_custom_value(&result);
        assert_eq!(semver_val.version.to_string(), "1.2.3+build.2");
    }

    #[test]
    fn test_into_semver_from_record_with_both() {
        let record = record! {
            "major" => Value::int(1, Span::test_data()),
            "minor" => Value::int(2, Span::test_data()),
            "patch" => Value::int(3, Span::test_data()),
            "pre" => Value::string("alpha", Span::test_data()),
            "build" => Value::string("build", Span::test_data()),
        };
        let value = Value::record(record, Span::test_data());
        let result = into_semver(&value, &vec![].into(), Span::test_data());

        let semver_val = get_custom_value(&result);
        assert_eq!(semver_val.version.to_string(), "1.2.3-alpha+build");
    }

    #[test]
    fn test_into_semver_from_record_missing_major() {
        let record = record! {
            "minor" => Value::int(2, Span::test_data()),
            "patch" => Value::int(3, Span::test_data()),
        };
        let value = Value::record(record, Span::test_data());
        let result = into_semver(&value, &vec![].into(), Span::test_data());

        assert!(matches!(result, Value::Error { .. }));
    }

    #[test]
    fn test_into_semver_from_record_negative_major() {
        let record = record! {
            "major" => Value::int(-1, Span::test_data()),
            "minor" => Value::int(2, Span::test_data()),
            "patch" => Value::int(3, Span::test_data()),
        };
        let value = Value::record(record, Span::test_data());
        let result = into_semver(&value, &vec![].into(), Span::test_data());

        assert!(matches!(result, Value::Error { .. }));
    }

    #[test]
    fn test_into_semver_from_unsupported_type() {
        let value = Value::int(42, Span::test_data());
        let result = into_semver(&value, &vec![].into(), Span::test_data());

        assert!(matches!(result, Value::Error { .. }));
    }

    #[test]
    fn test_into_semver_from_list_of_records() -> Result {
        let value = Value::test_list(vec![
            Value::test_record(record! {
                "major" => Value::test_int(1),
                "minor" => Value::test_int(2),
                "patch" => Value::test_int(3),
            }),
            Value::test_record(record! {
                "major" => Value::test_int(0),
                "minor" => Value::test_int(1),
                "patch" => Value::test_int(6),
            }),
        ]);

        test()
            .run_with_data("into semver", value)
            .expect_value_eq(vec!["1.2.3", "0.1.6"])
    }

    #[test]
    fn test_into_semver_from_list_of_strings() -> Result {
        let value = Value::test_list(vec![
            Value::test_string("3.1.0"),
            Value::test_string("0.10.5"),
        ]);
        test()
            .run_with_data("into semver", value)
            .expect_value_eq(vec!["3.1.0", "0.10.5"])
    }

    #[test]
    fn test_into_semver_at_cell_paths() -> Result {
        let cell_a = Value::test_record(record! {
            "major" => Value::test_int(0),
            "minor" => Value::test_int(10),
            "patch" => Value::test_int(2),
        });
        let value = Value::test_record(record! {
            "a" => cell_a,
            "b" => Value::test_string("will not error"),
            "c" => Value::test_string("5.3.0"),
        });

        test()
            .run_with_data("into semver a c | values", value)
            .expect_value_eq(vec!["0.10.2", "will not error", "5.3.0"])
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
