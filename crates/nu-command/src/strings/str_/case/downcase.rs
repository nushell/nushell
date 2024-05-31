use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "str downcase"
    }

    fn signature(&self) -> Signature {
        Signature::build("str downcase")
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

    fn usage(&self) -> &str {
        "Make text lowercase."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["lower case", "lowercase"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        operate(engine_state, stack, call, input)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Downcase contents",
                example: "'NU' | str downcase",
                result: Some(Value::test_string("nu")),
            },
            Example {
                description: "Downcase contents",
                example: "'TESTa' | str downcase",
                result: Some(Value::test_string("testa")),
            },
            Example {
                description: "Downcase contents",
                example: "[[ColA ColB]; [Test ABC]] | str downcase ColA",
                result: Some(Value::test_list(vec![Value::test_record(record! {
                    "ColA" => Value::test_string("test"),
                    "ColB" => Value::test_string("ABC"),
                })])),
            },
            Example {
                description: "Downcase contents",
                example: "[[ColA ColB]; [Test ABC]] | str downcase ColA ColB",
                result: Some(Value::test_list(vec![Value::test_record(record! {
                    "ColA" => Value::test_string("test"),
                    "ColB" => Value::test_string("abc"),
                })])),
            },
        ]
    }
}

fn operate(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let head = call.head;
    let column_paths: Vec<CellPath> = call.rest(engine_state, stack, 0)?;
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
        engine_state.ctrlc.clone(),
    )
}

fn action(input: &Value, head: Span) -> Value {
    match input {
        Value::String { val, .. } => Value::string(val.to_lowercase(), head),
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
mod test {
    use super::*;
    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(SubCommand {})
    }
}
