use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct StrCapitalize;

impl Command for StrCapitalize {
    fn name(&self) -> &str {
        "str capitalize"
    }

    fn signature(&self) -> Signature {
        Signature::build("str capitalize")
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
        "Capitalize first letter of text."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["convert", "style", "caps", "upper"]
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
        vec![
            Example {
                description: "Capitalize contents",
                example: "'good day' | str capitalize",
                result: Some(Value::test_string("Good day")),
            },
            Example {
                description: "Capitalize contents",
                example: "'anton' | str capitalize",
                result: Some(Value::test_string("Anton")),
            },
            Example {
                description: "Capitalize a column in a table",
                example: "[[lang, gems]; [nu_test, 100]] | str capitalize lang",
                result: Some(Value::test_list(vec![Value::test_record(record! {
                    "lang" => Value::test_string("Nu_test"),
                    "gems" => Value::test_int(100),
                })])),
            },
        ]
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
        Value::String { val, .. } => Value::string(uppercase_helper(val), head),
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

fn uppercase_helper(s: &str) -> String {
    // apparently more performant https://stackoverflow.com/questions/38406793/why-is-capitalizing-the-first-letter-of-a-string-so-convoluted-in-rust
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(StrCapitalize {})
    }
}
