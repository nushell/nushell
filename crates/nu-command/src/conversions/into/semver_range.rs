use crate::semver::range::SemverRangeValue;
use nu_engine::command_prelude::*;
use nu_protocol::shell_error::generic::GenericError;

#[derive(Clone)]
pub struct IntoSemverRange;

impl Command for IntoSemverRange {
    fn name(&self) -> &str {
        "into semver-range"
    }

    fn signature(&self) -> Signature {
        Signature::build("into semver-range")
            .input_output_types(vec![
                (Type::String, Type::Custom("semver-range".into())),
                (
                    Type::Custom("semver-range".into()),
                    Type::Custom("semver-range".into()),
                ),
            ])
            .category(Category::Conversions)
    }

    fn description(&self) -> &str {
        "Convert a string to a semver range."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["version", "requirement", "semantic"]
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
            move |value| into_semver_range(&value, head),
            engine_state.signals(),
        )
    }

    fn examples(&self) -> Vec<Example<'static>> {
        vec![
            Example {
                description: "Convert a string to a semver range",
                example: "'>=1.0.0' | into semver-range",
                result: None,
            },
            Example {
                description: "Convert a complex range",
                example: "'^1.2.3' | into semver-range",
                result: None,
            },
        ]
    }
}

fn into_semver_range(input: &Value, head: Span) -> Value {
    match input {
        Value::Custom { val, .. } if val.type_name() == "semver-range" => input.clone(),
        Value::String { val, .. } => match semver::VersionReq::parse(val) {
            Ok(requirement) => Value::custom(Box::new(SemverRangeValue::new(requirement)), head),
            Err(_) => Value::error(
                ShellError::Generic(
                    GenericError::new(
                        format!("Cannot convert \"{val}\" to a semver range"),
                        "the given string is not a valid semver requirement",
                        head,
                    )
                    .with_help("expected format: >=1.0.0, ^1.2.3, ~1.2, etc."),
                ),
                head,
            ),
        },
        _ => Value::error(
            ShellError::Generic(GenericError::new(
                format!("Cannot convert {} to semver range", input.get_type()),
                "expected a string or semver-range value",
                head,
            )),
            head,
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get_custom_value(value: &Value) -> &SemverRangeValue {
        match value {
            Value::Custom { val, .. } => val.as_any().downcast_ref::<SemverRangeValue>().unwrap(),
            _ => panic!("Expected Custom value"),
        }
    }

    #[test]
    fn test_into_semver_range_from_string() {
        let value = Value::string(">=1.0.0", Span::test_data());
        let result = into_semver_range(&value, Span::test_data());

        assert!(matches!(result, Value::Custom { .. }));
        let range_val = get_custom_value(&result);
        assert_eq!(range_val.requirement.to_string(), ">=1.0.0");
    }

    #[test]
    fn test_into_semver_range_caret() {
        let value = Value::string("^1.2.3", Span::test_data());
        let result = into_semver_range(&value, Span::test_data());

        let range_val = get_custom_value(&result);
        assert_eq!(range_val.requirement.to_string(), "^1.2.3");
    }

    #[test]
    fn test_into_semver_range_tilde() {
        let value = Value::string("~1.2", Span::test_data());
        let result = into_semver_range(&value, Span::test_data());

        let range_val = get_custom_value(&result);
        assert_eq!(range_val.requirement.to_string(), "~1.2");
    }

    #[test]
    fn test_into_semver_range_complex() {
        let value = Value::string(">=1.0.0, <2.0.0", Span::test_data());
        let result = into_semver_range(&value, Span::test_data());

        let range_val = get_custom_value(&result);
        assert_eq!(range_val.requirement.to_string(), ">=1.0.0, <2.0.0");
    }

    #[test]
    fn test_into_semver_range_wildcard() {
        let value = Value::string("*", Span::test_data());
        let result = into_semver_range(&value, Span::test_data());

        let range_val = get_custom_value(&result);
        assert_eq!(range_val.requirement.to_string(), "*");
    }

    #[test]
    fn test_into_semver_range_from_semver_range() {
        let original = SemverRangeValue::new(semver::VersionReq::parse(">=1.0.0").unwrap());
        let value = Value::custom(Box::new(original), Span::test_data());
        let result = into_semver_range(&value, Span::test_data());

        // Should return the same value
        let range_val = get_custom_value(&result);
        assert_eq!(range_val.requirement.to_string(), ">=1.0.0");
    }

    #[test]
    fn test_into_semver_range_invalid() {
        let value = Value::string("not-a-range", Span::test_data());
        let result = into_semver_range(&value, Span::test_data());

        assert!(matches!(result, Value::Error { .. }));
    }

    #[test]
    fn test_into_semver_range_unsupported_type() {
        let value = Value::int(42, Span::test_data());
        let result = into_semver_range(&value, Span::test_data());

        assert!(matches!(result, Value::Error { .. }));
    }

    #[test]
    fn test_into_semver_range_wrong_custom_value() {
        // Create a semver value instead of semver-range
        use crate::semver::value::SemverValue;
        let semver = SemverValue::new(semver::Version::parse("1.2.3").unwrap());
        let value = Value::custom(Box::new(semver), Span::test_data());
        let result = into_semver_range(&value, Span::test_data());

        assert!(matches!(result, Value::Error { .. }));
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() -> nu_test_support::Result {
        nu_test_support::test().examples(IntoSemverRange)
    }
}
