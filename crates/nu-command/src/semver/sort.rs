use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct SemverSort;

impl Command for SemverSort {
    fn name(&self) -> &str {
        "semver sort"
    }

    fn signature(&self) -> Signature {
        Signature::build("semver sort")
            .input_output_types(vec![(
                Type::List(Box::new(Type::SemVer)),
                Type::List(Box::new(Type::SemVer)),
            )])
            .switch("reverse", "Sort in reverse order", Some('r'))
            .category(Category::Filters)
    }

    fn description(&self) -> &str {
        "Sort a list of semver values."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["version", "order"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let reverse = call.has_flag(engine_state, stack, "reverse")?;
        let span = input.span().unwrap_or(call.head);
        let value = input.into_value(span)?;
        let result = sort_value(&value, reverse, span);
        Ok(result.into_pipeline_data())
    }

    fn examples(&self) -> Vec<Example<'static>> {
        vec![
            Example {
                description: "Sort a list of semver values",
                example: "[1.2.3, 2.0.0, 1.0.0] | semver sort",
                result: Some(Value::test_list(vec![
                    Value::test_semver(semver::Version::new(1, 0, 0)),
                    Value::test_semver(semver::Version::new(1, 2, 3)),
                    Value::test_semver(semver::Version::new(2, 0, 0)),
                ])),
            },
            Example {
                description: "Sort in reverse order",
                example: "[1.2.3, 2.0.0, 1.0.0] | semver sort --reverse",
                result: Some(Value::test_list(vec![
                    Value::test_semver(semver::Version::new(2, 0, 0)),
                    Value::test_semver(semver::Version::new(1, 2, 3)),
                    Value::test_semver(semver::Version::new(1, 0, 0)),
                ])),
            },
        ]
    }
}

fn sort_value(input: &Value, reverse: bool, head: Span) -> Value {
    let list = match input.as_list() {
        Ok(l) => l,
        Err(e) => return Value::error(e, head),
    };

    let mut sorted: Vec<Value> = list.to_vec();
    sorted.sort_by(|a, b| {
        let cmp = a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal);
        if reverse { cmp.reverse() } else { cmp }
    });

    Value::list(sorted, head)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() -> nu_test_support::Result {
        nu_test_support::test().examples(SemverSort)
    }
}
