use super::semver_from_input;
use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct SemverToRecord;

impl Command for SemverToRecord {
    fn name(&self) -> &str {
        "semver to-record"
    }

    fn signature(&self) -> Signature {
        Signature::build("semver to-record")
            .input_output_types(vec![(Type::SemVer, Type::Record(Default::default()))])
            .category(Category::Filters)
    }

    fn description(&self) -> &str {
        "Convert a semver value to a record with useful fields."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["version", "convert"]
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
            move |value| to_record_value(&value, head),
            engine_state.signals(),
        )
    }

    fn examples(&self) -> Vec<Example<'static>> {
        vec![
            Example {
                description: "Convert a semver to a record",
                example: "1.2.3 | semver to-record",
                result: Some(Value::test_record(record! {
                    "major" => Value::test_int(1),
                    "minor" => Value::test_int(2),
                    "patch" => Value::test_int(3),
                    "pre" => Value::test_string(""),
                    "build" => Value::test_string(""),
                    "pre_identifiers" => Value::test_list(vec![]),
                    "build_identifiers" => Value::test_list(vec![]),
                })),
            },
            Example {
                description: "Convert a semver with prerelease and build metadata",
                example: "1.2.3-alpha.1+build.2 | semver to-record",
                result: Some(Value::test_record(record! {
                    "major" => Value::test_int(1),
                    "minor" => Value::test_int(2),
                    "patch" => Value::test_int(3),
                    "pre" => Value::test_string("alpha.1"),
                    "build" => Value::test_string("build.2"),
                    "pre_identifiers" => Value::test_list(vec![
                        Value::test_string("alpha"),
                        Value::test_int(1),
                    ]),
                    "build_identifiers" => Value::test_list(vec![
                        Value::test_string("build"),
                        Value::test_int(2),
                    ]),
                })),
            },
        ]
    }
}

fn to_record_value(input: &Value, head: Span) -> Value {
    let version = match semver_from_input(input, head) {
        Ok(v) => v,
        Err(e) => return Value::error(e, head),
    };

    let pre_str = version.pre.as_str().to_string();
    let build_str = version.build.as_str().to_string();

    let pre_idents = parse_identifiers(version.pre.as_str(), head);
    let build_idents = parse_identifiers(version.build.as_str(), head);

    Value::record(
        record! {
            "major" => Value::int(version.major as i64, head),
            "minor" => Value::int(version.minor as i64, head),
            "patch" => Value::int(version.patch as i64, head),
            "pre" => Value::string(pre_str, head),
            "build" => Value::string(build_str, head),
            "pre_identifiers" => Value::list(pre_idents, head),
            "build_identifiers" => Value::list(build_idents, head),
        },
        head,
    )
}

fn parse_identifiers(s: &str, head: Span) -> Vec<Value> {
    if s.is_empty() {
        return vec![];
    }

    s.split('.')
        .map(|part| {
            if let Ok(n) = part.parse::<i64>() {
                Value::int(n, head)
            } else {
                Value::string(part, head)
            }
        })
        .collect()
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() -> nu_test_support::Result {
        nu_test_support::test().examples(SemverToRecord)
    }
}
