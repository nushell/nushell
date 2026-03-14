use nu_engine::CallExt;
use nu_protocol::{
    Category, Example, PipelineData, ShellError, Signature, Span, SyntaxShape, Type, Value,
    ast::CellPath,
    engine::{Call, Command, EngineState},
    record,
};

#[derive(Clone)]
pub struct StrEscapeRegex;

impl Command for StrEscapeRegex {
    fn name(&self) -> &str {
        "str escape-regex"
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
                "For a data structure input, escape strings at the given cell paths.",
            )
            .category(Category::Strings)
    }

    fn description(&self) -> &str {
        "Escapes special characters in the input string with '\\'."
    }

    fn is_const(&self) -> bool {
        true
    }

    fn run(
        &self,
        engine_state: &nu_protocol::engine::EngineState,
        stack: &mut nu_protocol::engine::Stack,
        call: &nu_protocol::engine::Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let column_paths: Vec<CellPath> = call.rest(engine_state, stack, 0)?;
        operate(engine_state, call, input, column_paths)
    }

    fn run_const(
        &self,
        working_set: &nu_protocol::engine::StateWorkingSet,
        call: &nu_protocol::engine::Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let column_paths: Vec<CellPath> = call.rest_const(working_set, 0)?;
        operate(working_set.permanent(), call, input, column_paths)
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Escape dots in an IP address.",
                example: "'192.168.1.1' | str escape-regex",
                result: Some(Value::test_string("192\\.168\\.1\\.1")),
            },
            Example {
                description: "Escape a list of strings containing special characters.",
                example: "['(abc)', '1 + 1'] | str escape-regex",
                result: Some(Value::test_list(vec![
                    Value::test_string("\\(abc\\)"),
                    Value::test_string("1 \\+ 1"),
                ])),
            },
            Example {
                description: "Escape characters in a specific column of a table.",
                example: "[[pattern]; ['find.me'] ['(group)']] | str escape-regex pattern",
                result: Some(Value::test_list(vec![
                    Value::test_record(record! { "pattern" => Value::test_string("find\\.me") }),
                    Value::test_record(record! { "pattern" => Value::test_string("\\(group\\)") }),
                ])),
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
        Value::String { val, .. } => Value::string(fancy_regex::escape(val), head),
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
    fn test_examples() -> nu_test_support::Result {
        nu_test_support::test().examples(StrEscapeRegex)
    }
}
