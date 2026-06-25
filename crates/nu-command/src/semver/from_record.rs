use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct SemverFromRecord;

impl Command for SemverFromRecord {
    fn name(&self) -> &str {
        "semver from-record"
    }

    fn signature(&self) -> Signature {
        Signature::build("semver from-record")
            .input_output_types(vec![(Type::Record(Default::default()), Type::SemVer)])
            .category(Category::Filters)
    }

    fn description(&self) -> &str {
        "Create a semver value from a record."
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
            move |value| from_record_value(&value, head),
            engine_state.signals(),
        )
    }

    fn examples(&self) -> Vec<Example<'static>> {
        vec![
            Example {
                description: "Create a semver from a record",
                example: "{major: 1, minor: 2, patch: 3} | semver from-record",
                result: Some(Value::test_semver(semver::Version::new(1, 2, 3))),
            },
            Example {
                description: "Create a semver with prerelease from a record",
                example: "{major: 1, minor: 2, patch: 3, pre: alpha.1} | semver from-record",
                result: Some(Value::test_semver(
                    semver::Version::parse("1.2.3-alpha.1")
                        .expect("hardcoded example should be valid"),
                )),
            },
        ]
    }
}

fn from_record_value(input: &Value, head: Span) -> Value {
    let record = match input.as_record() {
        Ok(r) => r,
        Err(e) => return Value::error(e, head),
    };

    let major = record
        .get("major")
        .and_then(|v| v.as_int().ok())
        .unwrap_or(0)
        .max(0) as u64;
    let minor = record
        .get("minor")
        .and_then(|v| v.as_int().ok())
        .unwrap_or(0)
        .max(0) as u64;
    let patch = record
        .get("patch")
        .and_then(|v| v.as_int().ok())
        .unwrap_or(0)
        .max(0) as u64;

    let mut version = semver::Version::new(major, minor, patch);

    if let Some(val) = record.get("pre").or_else(|| record.get("pre_identifiers"))
        && let Some(joined) = join_identifiers(val)
        && let Ok(pre) = semver::Prerelease::new(&joined)
    {
        version.pre = pre;
    }

    if let Some(val) = record
        .get("build")
        .or_else(|| record.get("build_identifiers"))
        && let Some(joined) = join_identifiers(val)
        && let Ok(build) = semver::BuildMetadata::new(&joined)
    {
        version.build = build;
    }

    Value::semver(version, head)
}

/// Read a record field as either a string (e.g. `"alpha.1"`) or a list of identifiers
/// (e.g. `[alpha, 1]`) and join them into a dot-separated string suitable for
/// [`semver::Prerelease::new`] or [`semver::BuildMetadata::new`].
fn join_identifiers(val: &Value) -> Option<String> {
    // String form: "alpha.1" → "alpha.1"
    if let Ok(s) = val.coerce_str() {
        let s = s.trim();
        if !s.is_empty() {
            return Some(s.to_string());
        }
    }

    // List form: [alpha, 1] → "alpha.1"
    if let Ok(list) = val.as_list() {
        let parts: Vec<String> = list
            .iter()
            .map(|v| {
                if let Ok(n) = v.as_int() {
                    n.to_string()
                } else {
                    v.coerce_str().unwrap_or_default().trim().to_string()
                }
            })
            .collect();
        if !parts.is_empty() {
            return Some(parts.join("."));
        }
    }

    None
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() -> nu_test_support::Result {
        nu_test_support::test().examples(SemverFromRecord)
    }
}
