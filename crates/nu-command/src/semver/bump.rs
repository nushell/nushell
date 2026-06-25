use super::semver_from_input;
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
            .input_output_types(vec![(Type::SemVer, Type::SemVer)])
            .param(Parameter::Required(
                PositionalArg::new("level", SyntaxShape::String)
                    .desc("the level to bump: major, minor, patch, alpha, beta, rc, release")
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
                example: "1.2.3 | semver bump major",
                result: Some(Value::test_semver(semver::Version::new(2, 0, 0))),
            },
            Example {
                description: "Bump minor version",
                example: "1.2.3 | semver bump minor",
                result: Some(Value::test_semver(semver::Version::new(1, 3, 0))),
            },
            Example {
                description: "Bump patch version",
                example: "1.2.3 | semver bump patch",
                result: Some(Value::test_semver(semver::Version::new(1, 2, 4))),
            },
            Example {
                description: "Add alpha prerelease",
                example: "1.2.3 | semver bump alpha",
                result: Some(Value::test_semver(
                    semver::Version::parse("1.2.3-alpha")
                        .expect("hardcoded example should be valid"),
                )),
            },
            Example {
                description: "Remove prerelease",
                example: "1.2.3-alpha | semver bump release",
                result: Some(Value::test_semver(semver::Version::new(1, 2, 3))),
            },
        ]
    }
}

fn bump_value(input: &Value, level: &str, head: Span) -> Value {
    let version = match semver_from_input(input, head) {
        Ok(v) => v,
        Err(e) => return Value::error(e, head),
    };

    let mut result = version;

    match level {
        "major" => {
            result.major += 1;
            result.minor = 0;
            result.patch = 0;
            result.pre = semver::Prerelease::EMPTY;
            result.build = semver::BuildMetadata::EMPTY;
        }
        "minor" => {
            result.minor += 1;
            result.patch = 0;
            result.pre = semver::Prerelease::EMPTY;
            result.build = semver::BuildMetadata::EMPTY;
        }
        "patch" => {
            result.patch += 1;
            result.pre = semver::Prerelease::EMPTY;
            result.build = semver::BuildMetadata::EMPTY;
        }
        "alpha" | "beta" | "rc" => {
            bump_prerelease(&mut result, level);
        }
        "release" => {
            result.pre = semver::Prerelease::EMPTY;
            result.build = semver::BuildMetadata::EMPTY;
        }
        _ => {
            return Value::error(
                ShellError::Generic(GenericError::new(
                    format!("Unknown bump level: {level}"),
                    "expected major, minor, patch, alpha, beta, rc, or release",
                    head,
                )),
                head,
            );
        }
    }

    Value::semver(result, head)
}

fn bump_prerelease(version: &mut semver::Version, level: &str) {
    let pre_str = version.pre.as_str();

    // If there's no existing prerelease, start at `<level>.1`
    if pre_str.is_empty() {
        version.pre = semver::Prerelease::new(level).unwrap_or(semver::Prerelease::EMPTY);
        return;
    }

    let parts: Vec<&str> = pre_str.split('.').collect();

    // If the level doesn't match the current prerelease prefix, start fresh
    if parts.is_empty() || parts[0] != level {
        version.pre = semver::Prerelease::new(level).unwrap_or(semver::Prerelease::EMPTY);
        return;
    }

    // No numeric suffix yet: start at `<level>.1`
    if parts.len() == 1 {
        let new_pre = format!("{level}.1");
        version.pre = semver::Prerelease::new(&new_pre).unwrap_or(semver::Prerelease::EMPTY);
        return;
    }

    // Increment the numeric suffix if present, otherwise start at 1
    if let Ok(n) = parts[1].parse::<u64>() {
        let new_pre = format!("{level}.{}", n + 1);
        version.pre = semver::Prerelease::new(&new_pre).unwrap_or(semver::Prerelease::EMPTY);
    } else {
        let new_pre = format!("{level}.1");
        version.pre = semver::Prerelease::new(&new_pre).unwrap_or(semver::Prerelease::EMPTY);
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
