use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct SemverIsValid;

impl Command for SemverIsValid {
    fn name(&self) -> &str {
        "semver is-valid"
    }

    fn signature(&self) -> Signature {
        Signature::build("semver is-valid")
            .input_output_types(vec![(Type::String, Type::Bool)])
            .category(Category::Filters)
    }

    fn description(&self) -> &str {
        "Check if a string is a valid semver version."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["version", "validate", "check"]
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
            move |value| is_valid_value(&value, head),
            engine_state.signals(),
        )
    }

    fn examples(&self) -> Vec<Example<'static>> {
        vec![
            Example {
                description: "Check if a string is valid semver",
                example: "'1.2.3' | semver is-valid",
                result: Some(Value::test_bool(true)),
            },
            Example {
                description: "Invalid semver string",
                example: "'not-valid' | semver is-valid",
                result: Some(Value::test_bool(false)),
            },
        ]
    }
}

fn is_valid_value(input: &Value, head: Span) -> Value {
    let s = match input.coerce_str() {
        Ok(s) => s,
        Err(e) => return Value::error(e, head),
    };

    Value::bool(semver::Version::parse(&s).is_ok(), head)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() -> nu_test_support::Result {
        nu_test_support::test().examples(SemverIsValid)
    }
}
