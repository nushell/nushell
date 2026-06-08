use nu_engine::command_prelude::*;
use nu_protocol::{DeprecationEntry, DeprecationType, ReportMode};

#[derive(Clone)]
pub struct StrUpcaseLike(pub &'static str, pub bool);

pub const STR_UPCASE: StrUpcaseLike = StrUpcaseLike("str upcase", true);
pub const STR_UPPERCASE: StrUpcaseLike = StrUpcaseLike("str uppercase", false);

impl Command for StrUpcaseLike {
    fn name(&self) -> &str {
        self.0
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .input_output_types(vec![
                (Type::String, Type::String),
                (
                    Type::List(Box::new(Type::String)),
                    Type::List(Box::new(Type::String)),
                ),
                (Type::table(), Type::table()),
                (Type::record(), Type::record()),
            ])
            .allow_variants_without_examples(true)
            .rest(
                "rest",
                SyntaxShape::CellPath,
                "For a data structure input, convert strings at the given cell paths.",
            )
            .category(Category::Strings)
    }

    fn description(&self) -> &str {
        "Convert text to uppercase."
    }

    fn search_terms(&self) -> Vec<&str> {
        match self.0 {
            "str upcase" => vec!["uppercase", "upper case", "upper-case"],
            _ => vec!["upcase", "upper case", "upper-case"],
        }
    }

    fn is_const(&self) -> bool {
        true
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let column_paths: Vec<CellPath> = call.rest(engine_state, stack, 0)?;
        operate(engine_state, call, input, column_paths)
    }

    fn run_const(
        &self,
        working_set: &StateWorkingSet,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let column_paths: Vec<CellPath> = call.rest_const(working_set, 0)?;
        operate(working_set.permanent(), call, input, column_paths)
    }

    fn examples(&self) -> Vec<Example<'_>> {
        let example = match self.0 {
            "str upcase" => "'nu' | str upcase",
            _ => "'nu' | str uppercase",
        };
        vec![Example {
            description: "Uppercase contents.",
            example,
            result: Some(Value::test_string("NU")),
        }]
    }

    fn deprecation_info(&self) -> Vec<DeprecationEntry> {
        if self.1 {
            vec![DeprecationEntry {
                ty: DeprecationType::Command,
                report_mode: ReportMode::FirstUse,
                since: Some("0.105.0".into()),
                expected_removal: None,
                help: Some("Use `str uppercase` instead.".into()),
            }]
        } else {
            vec![]
        }
    }
}

fn operate(
    engine_state: &EngineState,
    call: &Call,
    input: PipelineData,
    column_paths: Vec<CellPath>,
) -> Result<PipelineData, ShellError> {
    let head = call.head;
    input.map(
        move |v| {
            if column_paths.is_empty() {
                action(&v, head)
            } else {
                let mut ret = v;
                for path in &column_paths {
                    let r =
                        ret.update_cell_path(&path.members, Box::new(move |old| action(old, head)));
                    if let Err(error) = r {
                        return Value::error(error, head);
                    }
                }
                ret
            }
        },
        engine_state.signals(),
    )
}

fn action(input: &Value, head: Span) -> Value {
    match input {
        Value::String { val: s, .. } => Value::string(s.to_uppercase(), head),
        Value::Error { .. } => input.clone(),
        _ => Value::error(
            ShellError::OnlySupportsThisInputType {
                exp_input_type: "string".into(),
                wrong_type: input.get_type().to_string(),
                dst_span: head,
                src_span: input.span(),
            },
            head,
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_upcase_examples() -> nu_test_support::Result {
        nu_test_support::test().examples(STR_UPCASE)
    }

    #[test]
    fn test_uppercase_examples() -> nu_test_support::Result {
        nu_test_support::test().examples(STR_UPPERCASE)
    }

    #[test]
    fn upcases() {
        let word = Value::test_string("andres");

        let actual = action(&word, Span::test_data());
        let expected = Value::test_string("ANDRES");
        assert_eq!(actual, expected);
    }
}
