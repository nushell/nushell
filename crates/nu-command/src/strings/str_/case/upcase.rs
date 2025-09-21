use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct StrUpcase;

impl Command for StrUpcase {
    fn name(&self) -> &str {
        "str upcase"
    }

    fn signature(&self) -> Signature {
        Signature::build("str upcase")
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
        "Make text uppercase."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["uppercase", "upper case"]
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
        vec![Example {
            description: "Upcase contents",
            example: "'nu' | str upcase",
            result: Some(Value::test_string("NU")),
        }]
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
    use super::{StrUpcase, action};

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(StrUpcase {})
    }

    #[test]
    fn upcases() {
        let word = Value::test_string("andres");

        let actual = action(&word, Span::test_data());
        let expected = Value::test_string("ANDRES");
        assert_eq!(actual, expected);
    }
}
